use mlua::{Lua, Result, Table};
use regex::Regex;

// Inspired by the highlighter used by `micro`
// this allows reusing the syntax files, after converting them

/// Highlighter
pub(crate) struct Highlighter {
    language: String,
    regex: Regex,
    rules: Vec<Rule>,
}

enum Rule {
    Match {
        re: Regex,
        name: String,
    },
    Complex {
        begin: Regex,
        end: Regex,
        skip: Option<Regex>,
        name: String,
        rules: Vec<Rule>,
    },
}

impl Highlighter {
    fn from_table(lua: &Lua, table: Table) -> Result<Highlighter> {
        todo!()
    }
}
