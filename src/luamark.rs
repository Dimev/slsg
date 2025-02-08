use mlua::{ErrorContext, IntoLua, Lua, MultiValue, ObjectLike, Result, Table, Value};
use unicode_width::UnicodeWidthStr;

/// Parser for luamark
pub(crate) struct Parser<'a> {
    /// current remaining input
    input: &'a str,

    /// Current row
    row: usize,

    /// Current column
    col: usize,
}

impl<'a> Parser<'a> {
    /// Parse a luamark string
    pub(crate) fn parse(
        lua: &Lua,
        input: &'a str,
        macros: Table,
        row: usize,
        col: usize,
    ) -> Result<Value> {
        let mut parser = Self { input, row, col };

        // Current document
        let document = lua.create_table()?;

        // current paragraph
        let mut paragraph = lua.create_table()?;

        // current text
        let mut text = String::new();

        // as long as there's text
        while !parser.input.is_empty() {
            // if we start with whitespace, take that
            if parser.input.starts_with(char::is_whitespace) {
                // push the paragraph
                if let Some(content) = parser.until_pred(|c| !c.is_whitespace()) {
                    // more than 2 whitespaces, aka an empty line? round off the paragraph
                    // if it's not empty at least
                    if content.chars().filter(|c| *c == '\n').count() >= 2
                        && (!text.is_empty() || !paragraph.is_empty())
                    {
                        if !text.is_empty() {
                            let res: Value = macros
                                .call_function("text", (text, Value::Nil, parser.row, parser.col))
                                .context(format!(
                                    "[string]:{}:{}: Failed to call macro `text`",
                                    parser.row, parser.col
                                ))?;

                            paragraph.push(res)?;
                            text = String::new();
                        }

                        // call the paragraph macro
                        let res: Value = macros
                            .call_function(
                                "paragraph",
                                (paragraph, Value::Nil, parser.row, parser.col),
                            )
                            .context(format!(
                                "[string]:{}:{}: Failed to call macro `paragraph`",
                                parser.row, parser.col
                            ))?;

                        // push the paragraph to the document
                        document.push(res)?;

                        // new paragraph
                        paragraph = lua.create_table()?;
                    }
                } else {
                    // no match means end of input, so stop
                    break;
                }

                // text or paragraph is not empty and text does not end in a whitespace? add a whitespace, as we just skipped one
                // if the parargaph is not empty this means this whitespace is likely desired
                if (!paragraph.is_empty() || !text.is_empty())
                    && !text.ends_with(char::is_whitespace)
                {
                    text.push(' ');
                }
            }
            // if we start with a %, parse a comment
            else if parser.input.starts_with('%') {
                // take until the newline
                if parser.until_pat("\n").is_none() {
                    break;
                }

                // string is not empty or does not end in a whitespace? add a whitespace, as we just skipped one
                if !text.is_empty() && !text.ends_with(char::is_whitespace) {
                    text.push(' ');
                }
            }
            // if we start with a \, parse an escape sequence
            else if parser.input.starts_with('\\') {
                // take the first \
                parser.pat("\\");

                // take until the whitespace
                if let Some(content) = parser.until_pred(char::is_whitespace) {
                    // add it to the string
                    text.push_str(content);
                } else {
                    // else, end of file, push the rest
                    text.push_str(parser.input);
                    break;
                }
            }
            // if we start with a @begin, parse a block macro
            else if parser.input.starts_with("@begin") {
                // take the @begin
                parser.pat("@begin");

                // take the @
                parser.pat("@").ok_or(mlua::Error::external(format!(
                    "[string]:{}:{}: Expected a @name after the @begin of a block macro",
                    parser.row, parser.col
                )))?;

                // take the tag name
                let tag = parser
                    .until_pred(|c| c.is_whitespace() || "<{([|$".contains(c))
                    .filter(|x| x.len() > 0)
                    .ok_or_else(|| {
                        mlua::Error::external(format!(
                            "[string]:{}:{}: Expected a macro name",
                            parser.row, parser.col
                        ))
                    })?;

                // name of the macro
                let name = tag.split_once('@').map(|x| x.0).unwrap_or(tag);

                // parse the argument list
                let mut arguments = parser.arg_list(lua)?;

                // content starts here
                let (row, col) = (parser.row, parser.col);

                // closing tag
                let closing = format!("@end@{tag}");

                // read until the closing tag
                let content = parser
                    .until_pat(&closing)
                    .ok_or(mlua::Error::external(format!(
                        "[string]:{}:{}: Expected a closing {closing}, but did not get it",
                        parser.row, parser.col
                    )))?;

                // read the closing tag
                parser.pat(&closing);

                // other arguments
                arguments.push_back(content.trim().into_lua(lua)?);
                arguments.push_back(Value::Integer(row as i64));
                arguments.push_back(Value::Integer(col as i64));

                // run the macro
                let result: Value = macros.call_function(name, arguments).context(format!(
                    "[string]:{}:{}: Failed to call macro `{name}`",
                    parser.row, parser.col
                ))?;

                // push the current string, as that's valid content
                if !text.is_empty() {
                    let res: Value = macros
                        .call_function("text", (text, Value::Nil, parser.row, parser.col))
                        .context(format!(
                            "[string]:{}:{}: Failed to call macro `text`",
                            parser.row, parser.col
                        ))?;

                    paragraph.push(res)?;
                    text = String::new();
                }

                // push the macro to the paragrah
                paragraph.push(result)?;
            }
            // if we start with a @, parse a normal macro
            else if parser.input.starts_with('@') {
                // take the @
                parser.pat("@");

                // take the name
                let name = parser
                    .until_pred(|c| c.is_whitespace() || "<{([|$".contains(c))
                    .filter(|x| x.len() > 0)
                    .ok_or_else(|| {
                        mlua::Error::external(format!(
                            "[string]:{}:{}: Expected a macro name",
                            parser.row, parser.col
                        ))
                    })?;

                // parse the argument list
                let mut arguments = parser.arg_list(lua)?;
                arguments.push_back(Value::Nil);
                arguments.push_back(Value::Integer(parser.row as i64));
                arguments.push_back(Value::Integer(parser.col as i64));

                // run the macro
                let result: Value = macros.call_function(name, arguments).context(format!(
                    "[string]:{}:{}: Failed to call macro `{name}`",
                    parser.row, parser.col
                ))?;

                // push the current string, as that's valid content
                if !text.is_empty() {
                    let res: Value = macros
                        .call_function("text", (text, Value::Nil, parser.row, parser.col))
                        .context(format!(
                            "[string]:{}:{}: Failed to call macro `text`",
                            parser.row, parser.col
                        ))?;

                    paragraph.push(res)?;
                    text = String::new();
                }

                // push the macro to the paragrah
                paragraph.push(result)?;
            }
            // we now have normal text, eat up until the next special character
            else {
                if let Some(content) =
                    parser.until_pred(|c| c.is_whitespace() || "\\@%".contains(c))
                {
                    // add content
                    if !content.is_empty() {
                        text.push_str(content)
                    }
                } else {
                    // content was the rest of the input
                    text.push_str(parser.input.trim_end());
                    parser.input = "";
                }
            }
        }

        // close the paragraph
        if !text.is_empty() {
            let res: Value = macros
                .call_function("text", (text, Value::Nil, parser.row, parser.col))
                .context(format!(
                    "[string]:{}:{}: Failed to call macro `text`",
                    parser.row, parser.col
                ))?;

            paragraph.push(res)?;
        }

        if !paragraph.is_empty() {
            let res: Value = macros
                .call_function("paragraph", (paragraph, Value::Nil, parser.row, parser.col))
                .context(format!(
                    "[string]:{}:{}: Failed to call macro `paragraph`",
                    parser.row, parser.col
                ))?;

            // push the paragraph to the document
            document.push(res)?;
        }

        // close the document
        macros
            .call_function("document", (document, Value::Nil, row, col))
            .context(format!(
                "[string]:{}:{}: Failed to call macro `document`",
                parser.row, parser.col
            ))
    }

    /// Parse an argument list, for macros
    fn arg_list(&mut self, lua: &Lua) -> Result<MultiValue> {
        // characters to open/close the argument with
        let opening = "<{([|$";
        let closing = ">})]|$";

        // take the delimiter
        let delimiter = self
            .pat("<")
            .or_else(|| self.pat("{"))
            .or_else(|| self.pat("("))
            .or_else(|| self.pat("["))
            .or_else(|| self.pat("|"))
            .or_else(|| self.pat("$"))
            .or_else(|| self.until_pred(|c| !c.is_whitespace()))
            .ok_or(mlua::Error::external(format!(
                "[string]:{}:{}: Expected any of `<`, `{{`, `(`, `[`, `|`, `$` or a whitespace",
                self.row, self.col
            )))?;

        // get the closing delimiter
        let closing = if delimiter.starts_with(char::is_whitespace) {
            '\n'
        } else {
            closing
                .chars()
                .nth(opening.find(delimiter).unwrap())
                .unwrap()
        };

        // parse the argument
        let mut argument = String::new();
        let mut arguments = MultiValue::new();
        while !self.input.starts_with(closing) {
            // end of input, fail
            if self.input.is_empty() {
                Err(mlua::Error::external(format!(
                    "[string]:{}:{}: Expected a newline and rest of block macro, got end of input",
                    self.row, self.col
                )))?
            }
            // whitespace
            else if self
                .input
                .starts_with(|c: char| c.is_whitespace() && c != closing)
            {
                // consume the whitespace
                self.until_pred(|c| !c.is_whitespace() || c == closing);

                // push a space if the string is not empty or does not have a whitespace already
                if !argument.is_empty() && !argument.ends_with(char::is_whitespace) {
                    argument.push(' ');
                }
            }
            // comment
            else if self.input.starts_with('%') {
                self.until_pat("\n");
            }
            // argument delimiter
            else if self.input.starts_with(",") {
                self.pat(",");
                // in () and [], acts as an actual delimiter
                if "])".contains(closing) {
                    // push to the argument list
                    if !argument.trim().is_empty() {
                        arguments.push_back(argument.trim().into_lua(lua)?);
                    }
                } else {
                    // just push it
                    argument.push(',');
                }
            }
            // math environment escape, parse \$ as a $
            else if self.input.starts_with("\\$") && closing == '$' {
                self.pat("\\$");
                argument.push('$');
            }
            // math environment escape, parse \% as a %
            else if self.input.starts_with("\\%") && closing == '$' {
                self.pat("\\%");
                argument.push('%');
            }
            // other escape for the math environment does nothing
            else if self.input.starts_with('\\') && closing == '$' {
                self.pat("\\");
                argument.push('\\');
            }
            // escaped, only count if it's not enclosed in a math environment (aka $)
            else if self.input.starts_with('\\') && closing != '$' {
                // take \
                self.pat("\\");

                // take until a whitespace
                let content = self.until_pred(char::is_whitespace)
                            .ok_or(mlua::Error::external(format!(
                                "[string]:{}:{}: Expected a closing delimiter to the inline macro, got end of input",
                                self.row, self.col
                            )))?;

                // push the escaped value
                argument.push_str(content);
            }
            // rest, read until the next special character
            else if let Some(content) =
                self.until_pred(|c| c.is_whitespace() || "\\%,".contains(c) || c == closing)
            {
                argument.push_str(content);
            }
        }

        // closing tag as a string
        let mut buf = [0 as u8; 4];
        let closing = closing.encode_utf8(&mut buf);

        // read the closing tag
        self.pat(closing);

        // push the last string
        if !argument.trim().is_empty() {
            arguments.push_back(argument.trim().into_lua(lua)?);
        }

        Ok(arguments)
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
            1 + content
                .rsplit_once('\n')
                .map(|x| x.1.width_cjk())
                .unwrap_or(0)
        } else {
            self.col + content.width_cjk()
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
            1 + content
                .rsplit_once('\n')
                .map(|x| x.1.width_cjk())
                .unwrap_or(0)
        } else {
            self.col + content.width_cjk()
        };
        self.row += content.chars().filter(|x| *x == '\n').count();

        Some(content)
    }

    /// take the pattern at the start
    fn pat<'b>(&mut self, pat: &'b str) -> Option<&'b str> {
        let rest = self.input.strip_prefix(pat)?;

        // advance cursor
        self.input = rest;
        self.col = if pat.contains('\n') {
            1 + pat.rsplit_once('\n').map(|x| x.1.width_cjk()).unwrap_or(0)
        } else {
            self.col + pat.width_cjk()
        };
        self.row += pat.chars().filter(|x| *x == '\n').count();

        Some(pat)
    }
}

#[cfg(test)]
mod tests {
    //use nom::Parser;
    // TODO: tests
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
