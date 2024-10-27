use mlua::{Error, ErrorContext, Lua, MultiValue, Result, Table, TableExt, Value};

/// Parser for luamark
pub(crate) struct Parser<'a> {
    /// current input
    input: &'a str,

    /// Current row
    row: usize,

    /// Current column
    col: usize,
}

// TODO: fix functions failing also causing parsing to fail

impl<'a> Parser<'a> {
    /// Parse a luamark string
    pub(crate) fn parse<'lua>(
        lua: &'lua Lua,
        input: &'a str,
        macros: Table<'lua>,
        row: usize,
        col: usize,
    ) -> Result<Value<'lua>> {
        let mut parser = Self {

            input,

            row,
            col,
        };

        // Current document
        let mut document = lua.create_table()?;

        // current paragraph
        let mut paragraph = lua.create_table()?;

        // current text
        let mut text = String::new();

        // as long as there's text
        while !parser.input.is_empty() {
            // if we start with whitespace, take that
            if parser.input.starts_with(char::is_whitespace) {
                // TODO: more than 2 whitespaces?
                // push the paragraph
                if parser.until_pred(|c| !c.is_whitespace()).is_none() {
                    parser.input = "";
                }

                // string is not empty or does not end in a whitespace? add a whitespace
                if !text.is_empty() && !text.ends_with(char::is_whitespace) {
                    text.push(' ');
                }
            }
            // if we start with a %, parse a comment
            else if parser.input.starts_with('%') {
                // take until the newline
                if parser.until_pat("\n").is_none() {
                    parser.input = "";
                }

                // string is not empty or does not end in a whitespace? add a whitespace
                if !text.is_empty() && !text.ends_with(char::is_whitespace) {
                    text.push(' ');
                }
            }
            // if we start with a \, parse an escape sequence
            else if parser.input.starts_with('\\') {
                // take the first \
                parser.pat("\\");

                // take until the whitespace
                if let Some(content) = parser.until_pred(|c| !c.is_whitespace()) {
                    // add it to the string
                    text.push_str(content);
                } else {
                    // else, end of file, push the rest
                    text.push_str(parser.input);
                    parser.input = "";
                }
            }
            // if we start with a @begin, parse a block macro
            else if parser.input.starts_with("@begin") {
                // take the @begin
                parser.pat("@begin");

                // take the @
                // TODO: parse position
                parser.pat("@").ok_or(mlua::Error::external(
                    "Expected a @name after the @begin of a block macro",
                ))?;

                // take the tag name
                let tag = parser
                    .until_pred(char::is_whitespace)
                    .filter(|x| x.len() > 0)
                    .ok_or_else(|| {
                        mlua::Error::external("Expected a macro name, but got nothing")
                    })?;

                // name of the macro
                let name = tag.split_once('@').map(|x| x.0).unwrap_or(tag);

                // read until the newline
                // TODO: proper escape and comment handling
                let arguments = parser.until_pat("\n").ok_or(mlua::Error::external(
                    "Expected a newline, but did not get it",
                ))?;

                // read the newline
                parser.pat("\n");

                // closing tag
                let closing = format!("@end@{tag}");

                // read until the closing tag
                let content = parser.until_pat(&closing).ok_or(mlua::Error::external(format!(
                    "Expected a closing {closing}, but did not get it",
                )))?;

                // read the closing tag
                parser.pat(&closing);

                // run the macro
                let result: Value = macros.call_method(name, ())?;

                // push the macro to the paragrah
                paragraph.push(result)?;
            }
            // if we start with a @, parse a normal macro
            else if parser.input.starts_with('@') {
            }
            // we now have normal text, eat up until the next special character
            else {
            }
        }

        todo!()
    }

    /// take until we match a predicate
    fn until_pred<F: FnMut(char) -> bool>(&mut self, pred: F) -> Option<&'a str> {
        // where it is
        let mid = self.input.find(pred)?;

        // split
        let (content, rest) = self.input.split_at(mid);

        // advance cursor
        self.input = rest;
        self.col = if content.contains('\n') {
            1 + content.rsplit_once('\n').map(|x| x.1.len()).unwrap_or(0)
        } else {
            self.col + content.len()
        };
        self.row += content.chars().filter(|x| *x == '\n').count();

        Some(content)
    }

    /// take until we find the specific pattern
    fn until_pat(&mut self, pat: &str) -> Option<&'a str> {
        // where it is
        let mid = self.input.find(pat)?;

        // split
        let (content, rest) = self.input.split_at(mid);

        // advance cursor
        self.input = rest;
        self.col = if content.contains('\n') {
            1 + content.rsplit_once('\n').map(|x| x.1.len()).unwrap_or(0)
        } else {
            self.col + content.len()
        };
        self.row += content.chars().filter(|x| *x == '\n').count();

        Some(content)
    }

    /// take the pattern at the start
    fn pat(&mut self, pat: &str) -> Option<()> {
        let rest = self.input.strip_prefix(pat)?;

        // advance cursor
        self.input = rest;
        self.col = if pat.contains('\n') {
            1 + pat.rsplit_once('\n').map(|x| x.1.len()).unwrap_or(0)
        } else {
            self.col + pat.len()
        };
        self.row += pat.chars().filter(|x| *x == '\n').count();

        Some(())
    }
}

#[cfg(test)]
mod tests {
    //use nom::Parser;

    use mlua::FromLua;

    use super::*;

    impl<'a> Parser<'a> {
        fn new(input: &'a str) -> Self {
            Self {
                input,
                row: 1,
                col: 1,
            }
        }
    }
}
