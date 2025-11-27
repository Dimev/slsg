use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::{self},
    path::{Path, PathBuf},
};

use anyhow::{Context as AnyhowCtx, Result, anyhow, bail, ensure};
use globwalk::{GlobWalkerBuilder, glob};

use mlua::{Function, Lua, LuaSerdeExt, chunk};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html::push_html};
use relative_path::RelativePathBuf;

/// Escape a '' lua string
fn escape_lua_str(s: &str) -> String {
    s.replace("\\", "\\\\")
        .replace("'", "\\'")
        .replace("\n", "\\\n")
}

pub(crate) struct Files {
    /// resulting files
    pub files: HashMap<RelativePathBuf, Vec<u8>>,

    /// which file to use as 404
    pub not_found: Option<RelativePathBuf>,
}

impl Files {
    pub fn write_to_path(&self, path: &Path, force: bool) -> Result<()> {
        if path.exists() {
            // remove the directory if it does exist and we are forced to do it
            if force {
                fs::remove_dir_all(path).context("Failed to remove output directory")?;
            }
            // fail if the output directory is not empty and we are not forced to overwrite it
            else if path
                .read_dir()
                .context("Failed to check if path is empty")?
                .next()
                .is_none()
            {
                return Err(anyhow!("Output directory is not empty"));
            }
        }

        // ensure the path exists
        fs::create_dir_all(path)?;

        // write out all files here
        for (output, contents) in self.files.iter() {
            // output path
            let path = path.join(output.to_logical_path(""));

            // ensure it exists
            fs::create_dir_all(
                &path
                    .parent()
                    .ok_or_else(|| anyhow!("Path '{}' did not have a parent", path.display()))?,
            )
            .with_context(|| format!("Failed to create directories for '{output}'"))?;

            // write out
            fs::write(&path, contents).with_context(|| {
                format!("Failet to write file '{output}' to '{}'", path.display())
            })?;
        }

        Ok(())
    }
}

/// Cached state to generate a site with
pub(crate) struct SiteCache;

impl SiteCache {
    pub(crate) fn new() -> SiteCache {
        Self {}
    }

    /// Generate the files for the current site state, from the current working directory
    /// `development` is passed on to the lua state
    pub(crate) fn generate(&mut self, development: bool) -> Result<Files> {
        // TODO: use a lua syntax highlighter and call into that?
        // TODO: use etlua for templates https://github.com/leafo/etlua
        // TODO: the rust part will then just be running the script, and parsing and passing in the markdown
        // aka, inspire from https://log.schemescape.com/posts/static-site-generators/smallest-static-site-generator.html
        // TODO: also don't do fennel maybe?
        // TODO: don't do grass either

        // ensure lua files, pages and templates directory are present
        ensure!(fs::exists("site.lua")?, "'site.lua' not present");

        // load lua
        // SAFETY: we want all lua libraries
        let lua = unsafe { Lua::unsafe_new() };

        // load pages and create the page index
        let pages = lua.create_table()?;
        for path in glob("**/*.md")? {
            let path = path?;

            // load markdown
            let md = fs::read_to_string(path.path()).with_context(|| {
                format!("Failed to load markdown at '{}'", path.path().display())
            })?;

            // parse front matter
            // pulldown-cmark expects the metadata block to start and end with a +++
            // without any preceding spaces
            // find the range of the first metadata block
            let (frontmatter, rest) = md
                // opens with a +++, so no preceding newline
                .starts_with("+++")
                .then_some(3)
                // or if there's spaces between, there is a preceding newline
                .or_else(|| md.find("+++").map(|x| x + 3))
                // then, find the closing tag
                .and_then(|start| {
                    // read the frontmatter that appears after the onening tag
                    md[start..]
                        // find the closing tag in the rest of the text
                        // it must come after the opening tag
                        .find("\n+++")
                        // starts at the first +++, ends at the second +++
                        // start + end because the end is relative
                        // for the rest, skip the closing +++
                        .map(|end| (&md[start..start + end], &md[start + end + 4..]))
                })
                .ok_or_else(|| {
                    anyhow!(
                        "No toml-style ('+++' delimited) frontmatter for page '{}'",
                        path.path().display()
                    )
                })?;

            // parser state
            // inside a lua string?
            let mut inside_lua = false;

            // parse the markdown
            let parser = Parser::new_ext(
                rest,
                Options::ENABLE_MATH
                    | Options::ENABLE_HEADING_ATTRIBUTES
                    | Options::ENABLE_FOOTNOTES,
            )
            .map(|e| match e {
                // code block, convert to a tag
                // TODO: consider making a simple html parser so this is not needed?
                Event::Start(Tag::CodeBlock(info)) => {
                    match info {
                        CodeBlockKind::Indented => {
                            // we are now emitting lua
                            inside_lua = true;
                            Event::InlineHtml("<pre><code><%- highlight(nil, '".into())
                        }
                        CodeBlockKind::Fenced(info) => {
                            // we are now emitting lua
                            inside_lua = true;
                            let lang = info.split(' ').next().unwrap();
                            if lang.is_empty() {
                                Event::InlineHtml("<pre><code><%- highlight(nil, '".into())
                            } else {
                                Event::InlineHtml(
                                    format!(
                                        "<pre><code><%- highlight('{}', '",
                                        escape_lua_str(lang)
                                    )
                                    .into(),
                                )
                            }
                        }
                    }
                }
                // end tag
                // no need to deal with the middle tag, as that simply exports the html
                Event::End(TagEnd::CodeBlock) => {
                    // end of lua text
                    inside_lua = false;
                    Event::Html(format!("') -%></code></pre>").into())
                }

                // math, convert to math replace block
                Event::InlineMath(x) => Event::InlineHtml(
                    format!("<%- math_inline '{}' -%>", escape_lua_str(&x)).into(),
                ),
                Event::DisplayMath(x) => Event::InlineHtml(
                    format!("<%- math_display '{}' -%>", escape_lua_str(&x)).into(),
                ),

                // text tag, inside a codeblock
                Event::Text(t) if inside_lua => {
                    // no escape for html, but do escape for lua
                    Event::Html(escape_lua_str(&t).into())
                }

                // other cases
                x => x,
            });

            // convert to html
            let mut html = String::with_capacity(rest.len());
            push_html(&mut html, parser);

            // parse into a toml table
            let mut frontmatter = frontmatter.parse::<toml::Table>().with_context(|| {
                format!(
                    "Failed to parse frontmatter for page '{}'",
                    path.path().display()
                )
            })?;

            // add the markdown and content to the front matter
            frontmatter.insert("markdown".into(), rest.into());
            frontmatter.insert("content".into(), html.into());

            // frontmatter table
            let frontmatter = lua.to_value(&frontmatter)?;

            // and to the page list
            pages.push(&frontmatter)?;
        }

        // API
        let api = lua.create_table()?;
        api.set("pages", pages)?;

        // etlua
        let etlua = lua
            .load(include_str!("lua/etlua.lua"))
            .set_name("=etlua.lua")
            .into_function()?;

        // preload libraries
        lua.load(chunk! {
            // TODO: see if there's a way to make the render error out? as we have feedback anyway
            // TODO: consider making this in rust instead?
            // TODO: Do this in rust instead, then it can be used from inside markdown too
            // TODO: Also allows a bit better error messages?
            package.preload.etlua = $etlua;

            // TODO: the highlighter
        })
        .exec()?;

        // run the site generator
        let res: mlua::Table = lua
            .load(Path::new("site.lua"))
            .call(api)
            .context("'site.lua' did not return a table of results")?;

        // get the pages
        let pages: mlua::Table = res.get("pages").context(
            "'site.lua' must return a table with a key 'pages' with the table of files to emit",
        )?;

        // not found page
        let not_found: Option<String> = res.get("not_found")?;

        // TODO subset font?

        Ok(Files {
            // TODO build from the script
            files: Default::default(),

            // returned from the script
            not_found: not_found.map(RelativePathBuf::from),
        })
    }

    /// Generate files from the current directory
    pub fn new_site() -> Result<Files> {
        // set up self, then generate all files
        // no development mode
        Self::new().generate(false)
    }

    /// mark file as dirty
    pub fn mark_dirty(&mut self, path: &Path) {
        // template changed?
        if path.starts_with("templates") {
            //self.templates = None;
        }
    }
}
