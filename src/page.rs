use std::{collections::BTreeSet, fs};

use mlua::{ErrorContext, ExternalResult, Result};
use pulldown_cmark::{Event, Options, Parser};
use relative_path::{RelativePath, RelativePathBuf};

pub(crate) struct Page {
    /// Html content
    html: String,

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
    //
}

impl Page {
    fn from_path(path: &RelativePath) -> Result<Page> {
        // load the markdown
        let md = fs::read_to_string(path.to_path("./"))
            .into_lua_err()
            .with_context(|_| format!("Failed to load markdown file at `{path}`"))?;

        // future front matter
        let mut front = String::new();

        // parse the markdown
        let parser = Parser::new_ext(
            &md,
            Options::ENABLE_MATH | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS,
        ).collect::<Vec<_>>();

        // TODO: read first metadata block
        todo!()
    }
}
