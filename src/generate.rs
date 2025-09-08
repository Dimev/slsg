use std::collections::BTreeMap;

use mlua::{ErrorContext, Lua, Result, chunk};
use relative_path::RelativePathBuf;
use syntect::parsing::SyntaxSet;

pub(crate) struct Site {
    /// resulting files
    pub files: BTreeMap<RelativePathBuf, Vec<u8>>,

    /// which file to use as 404
    pub not_found: Option<RelativePathBuf>,

    /// loaded syntaxes
    syntaxes: SyntaxSet,

    /// loaded user syntaxes
    user_syntaxes: SyntaxSet,
    
    // TODO: see how to do themes?
}

impl Site {
    pub fn new() -> Result<Site> {
        

        // TODO: separate functions for stuff below
        // set up functions
        // TODO lol_html
        // TODO see how this works out later? not sure how easy it is to only rerun specific things

        // load syntaxes

        // load sass

        // run markdown processing

        // compile tera

        // load lua

        // replace html

        // subset fonts

        // load everything
        todo!()
    }

    /// mark file as dirty
    pub fn mark_dirty(&mut self, path: RelativePathBuf) {
        // TODO: update the file
    }
}
