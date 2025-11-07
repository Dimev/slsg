use std::{collections::BTreeSet, fs};

use mlua::{ErrorContext, ExternalResult, Result};
use pulldown_cmark::{html::push_html, Event, MetadataBlockKind, Options, Parser, Tag, TagEnd};
use relative_path::{RelativePath, RelativePathBuf};

pub(crate) struct Page {
    /// markdown content
    markdown: String,

    /// Html content
    pub html: String,

    /// Output path, where it will be in the final site
    output: RelativePathBuf,

    /// Resulting html, after processing
    result: Result<String>,

    /// Template to use
    template: String,

    /// Files to include
    include: BTreeSet<RelativePathBuf>,
    // TODO funny page index stuff
    // TODO files to include
    // TODO marker to see if it needs to be updated
}

impl Page {
    pub(crate) fn from_path_and_previous(path: &RelativePath, prev: Option<Page>) -> Result<Page> {
        // TODO: check if the markdown changed
        // if not, no need to redo

        // load the markdown
        let md = fs::read_to_string(path.to_path("./"))
            .into_lua_err()
            .with_context(|_| format!("Failed to load markdown file at `{path}`"))?;

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
            ) => Ok(t),
            _ => Err(mlua::Error::external("Expected front matter")),
        }?;

        // turn the rest into html
        let mut html = String::with_capacity(md.len());

        // TODO: see if code block and math block can be changed slightly?
        push_html(&mut html, parser);

        // TODO: read first metadata block
        Ok(Self {
            markdown: md,
            html: html,
            output: RelativePathBuf::new(),
            result: Ok(String::new()),
            template: String::new(),
            include: BTreeSet::new(),
        })
    }
}
