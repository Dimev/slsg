use std::{collections::BTreeSet, fs};

use mlua::{ErrorContext, ExternalResult, Result};
use pulldown_cmark::{Event, MetadataBlockKind, Options, Parser, Tag, TagEnd, html::push_html};
use relative_path::{RelativePath, RelativePathBuf};
use toml::{Table, Value};

#[derive(Debug)]
pub(crate) struct Page {
    /// markdown content
    markdown: String,

    /// Html content
    pub html: String,

    /// Front matter table
    front: Table,

    /// Output path, where it will be in the final site
    output: RelativePathBuf,

    /// Resulting html, after processing
    result: Result<String>,

    /// Template to use
    template: String,

    /// Files to include
    include: BTreeSet<RelativePathBuf>,

    /// What tags this page is part of
    tags: Vec<String>,

    /// Has this been changed?
    update: bool,
}

impl Page {
    pub(crate) fn from_path_and_previous(path: &RelativePath, prev: Option<Page>) -> Result<Page> {
        // load the markdown
        let md = fs::read_to_string(path.to_path("./"))
            .into_lua_err()
            .with_context(|_| format!("Failed to load markdown file at `{path}`"))?;

        // if the previous page exists and the markdown is the same, quit early
        if let Some(prev) = prev
            && prev.markdown == md
        {
            // retunr the previous version
            return Ok(prev);
        }

        // parse the markdown
        let mut parser = Parser::new_ext(
            &md,
            Options::ENABLE_MATH | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS,
        );

        // front matter
        // this should always be at the start, with the +++ ... +++
        let front = match (parser.next(), parser.next(), parser.next()) {
            (
                Some(Event::Start(Tag::MetadataBlock(MetadataBlockKind::PlusesStyle))),
                Some(Event::Text(t)),
                Some(Event::End(TagEnd::MetadataBlock(MetadataBlockKind::PlusesStyle))),
            ) if md.starts_with("+++") => Ok(format!("###\n{t}")),
            _ => Err(mlua::Error::external("Expected front matter")),
        }?;

        // parse the front matter toml
        let mut front = front
            .parse::<Table>()
            .into_lua_err()
            .with_context(|_| format!("Failed to parse front matter of `{path}`"))?;

        // get the output path
        let output = front
            .remove("out")
            .and_then(|x| {
                x.as_str()
                    .map(RelativePath::new)
                    .map(|x| x.to_relative_path_buf())
            })
            .ok_or_else(|| {
                mlua::Error::external(format!(
                    "Page `{path}` lacks an output path `out = \"path\"` in the front matter"
                ))
            })?;

        // and template to use
        let template = front
            .remove("template")
            .and_then(|x| match x {
                Value::String(s) => Some(s),
                _ => None,
            })
            .ok_or_else(|| {
                mlua::Error::external(format!(
                    "Page `{path}` lacks a template `template = \"template\"` in the front matter"
                ))
            })?;

        // tags, if any
        let tags = match front.remove("tags") {
            Some(Value::String(s)) => Some(vec![s]),
            Some(Value::Array(s)) => s.into_iter().map(|x| x.as_str().map(|y| y.to_string())).collect(),
            Some(_) => None,
            None => Some(Vec::new()),
        }.ok_or(mlua::Error::external("`tags` in the front matter of `{path}` must be a single string, or an array of strings"))?;

        // turn the rest into html
        let mut html = String::with_capacity(md.len());

        // TODO: see if code block and math block can be changed slightly?
        push_html(&mut html, parser);

        // TODO: read first metadata block
        Ok(dbg!(Self {
            markdown: md,
            html: html,
            output,
            front,
            result: Ok(String::new()),
            template,
            include: BTreeSet::new(),
            tags,
            update: true, // reloaded, so this changed
        }))
    }
}
