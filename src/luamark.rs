use mlua::{Error, ErrorContext, Lua, Result, Table, TableExt, Value};

/// Parser for luamark
pub(crate) struct Parser<'a, 'b> {
    /// current input
    input: &'a str,

    /// Current row
    row: usize,

    /// Current column
    col: usize,

    /// Lua context
    lua: &'b Lua,

    /// Macro table
    macros: Table<'b>,
}

// TODO: fix functions failing also causing parsing to fail

impl<'a, 'lua: 'a> Parser<'a, 'lua> {
    /// Parse a luamark string
    pub(crate) fn parse(
        lua: &'lua Lua,
        input: &'a str,
        macros: Table<'lua>,
        row: usize,
        col: usize,
    ) -> Result<Value<'lua>> {
        let mut parser = Self {
            lua,
            input,
            macros,
            row,
            col,
        };

        // skip initial whitespace
        parser.untill_pred(|c| !c.is_whitespace())?;

        let paragraphs = lua.create_table()?;

        // take paragraphs
        while let Ok(x) = parser.paragraph() {
            // add to the paragraph
            paragraphs.push(x?)?;

            // take all empty lines
            let mut counter = 0;
            while parser.empty_line().is_ok() {
                counter += 1;
            }

            // stop if no progress
            if counter == 0 {
                break;
            }
        }

        // remaining whitespace
        if !parser.input.trim().is_empty() {
            Err(parser.fail(&format!("Unexpected end of input: {}", parser.input)))
        } else {
            parser
                .macros
                .clone()
                .call_method("document", (paragraphs, Value::Nil, row, col))
                .context(format!("Failed to call macro `document`"))
        }
    }

    /// Fail the parse
    fn fail(&mut self, reason: &str) -> Error {
        mlua::Error::external(format!("Failed to parse at {}: {reason}", self.input))
    }

    /// Take a tag
    fn tag(&mut self, tag: &str) -> Result<()> {
        self.input = self
            .input
            .strip_prefix(tag)
            .ok_or_else(|| self.fail(&format!("Could not find tag {tag}")))?;

        // TODO: advance cursor

        Ok(())
    }

    /// Take until we find a specific string
    fn untill_tag(&mut self, tag: &str) -> Result<&'a str> {
        // find where it is
        let mid = self
            .input
            .find(tag)
            .ok_or_else(|| self.fail(&format!("could not find tag {tag}")))?;

        // split
        let (content, rest) = self.input.split_at(mid);

        // advance the cursor
        self.input = rest;
        self.col += content.len();

        Ok(content)
    }

    /// Take until we the predicate is true
    fn untill_pred<P: FnMut(char) -> bool>(&mut self, pred: P) -> Result<&'a str> {
        // find where it is
        let mid = self
            .input
            .find(pred)
            .ok_or_else(|| self.fail("could not find predicate"))?;

        // split
        let (content, rest) = self.input.split_at(mid);

        // advance the cursor
        self.input = rest;
        self.col += content.len();

        Ok(content)
    }

    /// Take until any of the characters are found
    fn untill_any(&mut self, list: &str) -> Result<&'a str> {
        // find where it is
        let mid = self
            .input
            .find(|c| list.contains(c))
            .ok_or_else(|| self.fail("could not find tag"))?;

        // split
        let (content, rest) = self.input.split_at(mid);

        // advance the cursor
        self.input = rest;
        self.col += content.len();

        Ok(content)
    }

    /// Ensure none of the following characters are present
    fn none(&mut self, list: &str) -> Result<()> {
        if self.input.starts_with(|c| list.contains(c)) {
            Err(self.fail(&format!("Did not expect any of {list}")))
        } else {
            Ok(())
        }
    }
    /// Parse a comment
    fn comment(&mut self) -> Result<()> {
        self.tag("%")?;
        self.untill_tag("\n")?;
        Ok(())
    }

    /// Parse an escaped sequence
    fn escaped(&mut self) -> Result<&'a str> {
        self.tag("\\")?;
        self.untill_pred(char::is_whitespace)
    }

    /// Parse a delimited inline argument
    fn inline_argument(&mut self, open: char, close: char) -> Result<String> {
        let mut buf_a = [0; 4];
        let mut buf_b = [0; 4];
        let open_str = open.encode_utf8(&mut buf_a);
        let close_str = close.encode_utf8(&mut buf_b);

        self.tag(open_str)?;

        let mut result = String::new();

        // parse the argument, until it reaches a closing tag
        while let Ok(content) = self
            .escaped()
            .or_else(|_| self.comment().and_then(|_| self.tag("\n")).map(|_| "\n"))
            .or_else(|_| self.untill_pred(|c| "%\\".contains(c) || c == close))
        {
            // ensure progress
            if content.is_empty() {
                break;
            }

            // push result
            result.push_str(content);
        }

        // closing tag
        self.tag(&close_str)?;
        Ok(result)
    }

    /// Parse an inline macro
    fn inline_macro(&mut self) -> Result<Result<Value<'a>>> {
        self.tag("@")?;

        // name of the macro
        let name = self.untill_pred(|c| c.is_whitespace() || "[({<|$".contains(c))?;

        // position
        let (row, col) = (self.row, self.col + 1);

        // get the delimited argument
        let argument = self
            .inline_argument('[', ']')
            .or_else(|_| self.inline_argument('(', ')'))
            .or_else(|_| self.inline_argument('{', '}'))
            .or_else(|_| self.inline_argument('<', '>'))
            .or_else(|_| self.inline_argument('|', '|'))
            .or_else(|_| self.inline_argument('$', '$'))
            .map_err(|_| self.fail("Expected any of `], ), }, >, |, $`"))?;

        // call the command
        Ok(self
            .macros
            .call_method(name, (argument.trim(), Value::Nil, row, col))
            .context(format!("Failed to call macro `{name}'")))
    }

    /// parse a macro on a line
    fn line_macro(&mut self) -> Result<Result<Value<'a>>> {
        // skip empty space
        self.untill_pred(|c| !c.is_whitespace())?;

        // first at
        self.tag("@")?;

        // name of the macro
        let name = self.untill_pred(char::is_whitespace)?;

        // argument to the macro
        let mut argument = String::new();

        // position
        let (row, col) = (self.row, self.col);

        // parse the argument, until it reaches a comment or newline
        while let Ok(content) = self.escaped().or_else(|_| self.untill_any("%\\\n")) {
            // ensure progress
            if content.is_empty() {
                break;
            }

            // push content
            argument.push_str(content);
        }

        // optional comment
        let _ = self.comment();

        // newline
        self.tag("\n")?;

        // call the macro
        Ok(self
            .macros
            .call_method(name, (argument.trim(), Value::Nil, row, col))
            .context(format!("Failed to call macro `{name}'")))
    }

    /// Parse a block macro
    fn block_macro(&mut self) -> Result<Result<Value<'a>>> {
        // @begin@name
        self.tag("@begin@")?;

        // name and any closing tag
        let name_and_close = self.untill_pred(char::is_whitespace)?;

        // name
        let name = name_and_close
            .split_once('@')
            .map(|x| x.0)
            .unwrap_or(name_and_close);

        // argument to the macro
        let mut argument = String::new();

        // parse the argument, until it reaches a comment or newline
        while let Ok(content) = self.escaped().or_else(|_| self.untill_any("%\\\n")) {
            // ensure progress
            if content.is_empty() {
                break;
            }

            // push content
            argument.push_str(content);
        }

        // optional comment
        let _ = self.comment();

        // read the newline
        self.tag("\n").context("failure")?;

        // position
        let (row, col) = (self.row, self.col);

        // tag to end with
        let end_tag = format!("@end@{name_and_close}");

        // read everything until the end tag
        let content = self.untill_tag(&end_tag)?;

        // read the end tag
        self.tag(&end_tag)?;

        // call the macro
        Ok(self
            .macros
            .call_method(name, (argument.trim(), content, row, col))
            .context(format!("Failed to call macro `{name}`")))
    }

    /// Parse a single line
    fn line(&mut self) -> Result<Value<'a>> {
        // read the empty line first
        self.untill_pred(|c| !c.is_whitespace() || c == '\n')?;

        // ensure that the next character in line is not a newline or comment
        //self.none("%\n")?;

        // parse the content
        let line_content = self.lua.create_table()?;
        while let Ok(content) = self
            .block_macro()
            .or_else(|_| self.inline_macro())
            .or_else(|_| {
                self.escaped()
                    .and_then(|x| self.lua.create_string(x))
                    .map(|x| Ok(Value::String(x)))
            })
            .or_else(|_| {
                self.untill_any("\n\\@%")
                    .and_then(|x| {
                        if x.len() > 0 {
                            Ok(x)
                        } else {
                            Err(self.fail("No progress"))
                        }
                    })
                    .and_then(|x| self.lua.create_string(x).map(|x| Ok(Value::String(x))))
            })
        {
            // optional comment
            let _ = self.comment();

            // TODO: proper string
            line_content.push(content?)?;
        }

        // newline
        self.tag("\n")?;

        // TODO: proper string concat
        Ok(Value::Table(line_content))
    }

    /// Parse an empty line
    fn empty_line(&mut self) -> Result<()> {
        // any series of whitespace with an optional comment
        self.untill_pred(|c| !c.is_whitespace() || c == '\n')
            .context("no whitespace!")?;

        // optional comment
        let _ = self.comment();

        // ending whitespace
        self.tag("\n")?;
        Ok(())
    }

    /// Parse a paragraph
    fn paragraph(&mut self) -> Result<Result<Value<'a>>> {
        let paragraph_content = self.lua.create_table()?;
        let (row, col) = (self.row, self.col);
        while let Ok(content) = self.line().map(|x| Ok(x)).or_else(|_| self.line_macro()) {
            // push content
            paragraph_content.push(content?)?;
        }

        // call the macro
        Ok(self
            .macros
            .call_method("paragraph", (paragraph_content, Value::Nil, row, col))
            .context(format!("Failed to call macro `document`")))
    }
}

#[cfg(test)]
mod tests {
    //use nom::Parser;

    use mlua::FromLua;

    use super::*;

    impl<'a, 'b> Parser<'a, 'b> {
        fn new(lua: &'b Lua, input: &'a str, macros: Table<'b>) -> Self {
            Self {
                input,
                lua,
                macros,
                row: 1,
                col: 1,
            }
        }
    }

    #[test]
    fn parse_comment() {
        let lua = Lua::new();
        let macros = lua.create_table().unwrap();

        Parser::new(&lua, "% hello\nrest", macros.clone())
            .comment()
            .unwrap();
        Parser::new(&lua, "text % hello\nrest", macros)
            .comment()
            .unwrap_err();
    }

    #[test]
    fn parse_escaped() {
        let lua = Lua::new();
        let macros = lua.create_table().unwrap();
        assert_eq!(
            Parser::new(&lua, r#"\\\@#^%() "#, macros.clone())
                .escaped()
                .unwrap(),
            r#"\\@#^%()"#
        );
        Parser::new(&lua, r#"hello \@ rest"#, macros)
            .escaped()
            .unwrap_err();
    }

    #[test]
    fn parse_inline_argument() {
        let lua = Lua::new();
        let macros = lua.create_table().unwrap();
        assert_eq!(
            Parser::new(&lua, "(Hello there!)rest)", macros.clone())
                .inline_argument('(', ')')
                .unwrap(),
            "Hello there!"
        );
        assert_eq!(
            Parser::new(&lua, "(We allow \\) in here)rest)", macros.clone())
                .inline_argument('(', ')')
                .unwrap(),
            "We allow ) in here"
        );

        Parser::new(&lua, " (space first)", macros)
            .inline_argument('(', ')')
            .unwrap_err();
    }

    #[test]
    fn parse_inline_command() {
        let lua = Lua::new();
        let macros = lua.create_table().unwrap();
        macros
            .set(
                "command",
                lua.create_function(|_, (_, arg): (Table, String)| Ok(arg))
                    .unwrap(),
            )
            .unwrap();

        assert_eq!(
            Parser::new(&lua, "@command(Hello there!)rest", macros.clone())
                .inline_macro()
                .unwrap()
                .unwrap()
                .as_str()
                .unwrap(),
            "Hello there!"
        );

        assert_eq!(
            Parser::new(&lua, "@command$Hello there!$rest", macros.clone())
                .inline_macro()
                .unwrap()
                .unwrap()
                .as_str()
                .unwrap(),
            "Hello there!"
        );

        Parser::new(&lua, "@command rest", macros.clone())
            .inline_macro()
            .unwrap_err();
    }

    #[test]
    fn parse_line_command() {
        let lua = Lua::new();
        let macros = lua.create_table().unwrap();
        macros
            .set(
                "command",
                lua.create_function(|_, (_, arg): (Table, String)| Ok(arg))
                    .unwrap(),
            )
            .unwrap();

        assert_eq!(
            Parser::new(&lua, "@command a b c\nrest", macros.clone())
                .line_macro()
                .unwrap()
                .unwrap()
                .as_str()
                .unwrap(),
            "a b c"
        );

        assert_eq!(
            Parser::new(&lua, "@command a b \\( \\\\ c\nrest", macros.clone())
                .line_macro()
                .unwrap()
                .unwrap()
                .as_str()
                .unwrap(),
            "a b ( \\ c"
        );

        assert_eq!(
            Parser::new(&lua, "    @command a b c\nrest", macros.clone())
                .line_macro()
                .unwrap()
                .unwrap()
                .as_str()
                .unwrap(),
            "a b c"
        );

        assert_eq!(
            Parser::new(&lua, "@command a b % c\nrest", macros.clone())
                .line_macro()
                .unwrap()
                .unwrap()
                .as_str()
                .unwrap(),
            "a b"
        );

        Parser::new(&lua, "before @command\nrest", macros.clone())
            .line_macro()
            .unwrap_err();

        Parser::new(&lua, "before @command arg\nrest", macros.clone())
            .line_macro()
            .unwrap_err();
    }

    #[test]
    fn parse_line() {
        let lua = Lua::new();
        let macros = lua.create_table().unwrap();
        macros
            .set(
                "command",
                lua.create_function(|_, (_, arg): (Table, String)| Ok(arg))
                    .unwrap(),
            )
            .unwrap();

        assert_eq!(
            Vec::<String>::from_lua(
                Parser::new(&lua, "Hello world!\nrest", macros.clone())
                    .line()
                    .unwrap(),
                &lua
            )
            .unwrap(),
            vec!["Hello world!".to_string()]
        );

        assert_eq!(
            Vec::<String>::from_lua(
                Parser::new(&lua, "Hello world!\\% Not a comment!\nrest", macros.clone())
                    .line()
                    .unwrap(),
                &lua
            )
            .unwrap(),
            vec![
                "Hello world!".to_string(),
                "%".to_string(),
                " Not a comment!".to_string()
            ]
        );

        assert_eq!(
            Vec::<String>::from_lua(
                Parser::new(&lua, "   Hello world!\nrest", macros.clone())
                    .line()
                    .unwrap(),
                &lua
            )
            .unwrap(),
            vec!["Hello world!".to_string()]
        );

        assert_eq!(
            Vec::<String>::from_lua(
                Parser::new(&lua, "Hello world! % comment\nrest", macros.clone())
                    .line()
                    .unwrap(),
                &lua
            )
            .unwrap(),
            vec!["Hello world! ".to_string()]
        );

        assert_eq!(
            Vec::<String>::from_lua(
                Parser::new(&lua, "Hello world! @command(hello)\nrest", macros.clone())
                    .line()
                    .unwrap(),
                &lua
            )
            .unwrap(),
            vec!["Hello world! ".to_string(), "hello".to_string()]
        );

        assert_eq!(
            Vec::<String>::from_lua(
                Parser::new(
                    &lua,
                    "Hello world! @command(hello\nworld% ignore\n)\nrest",
                    macros.clone()
                )
                .line()
                .unwrap(),
                &lua
            )
            .unwrap(),
            vec!["Hello world! ".to_string(), "hello\nworld".to_string()]
        );

        assert_eq!(
            Vec::<String>::from_lua(
                Parser::new(&lua, "% hello\nrest", macros.clone())
                    .line()
                    .unwrap(),
                &lua
            )
            .unwrap(),
            Vec::<String>::new()
        );
        /*        assert_eq!(
            line::<VerboseError<&str>>.parse("   % hello\nrest"),
            Ok(("rest", vec![]))
        );
        assert_eq!(
            line::<VerboseError<&str>>
                .parse("Hello @begin@command a b c\nverbatim @end@command after % comment\nrest"),
            Ok((
                "rest",
                vec![
                    Node::Text("Hello ".to_string()),
                    Node::Block {
                        name: "command".to_string(),
                        arguments: "a b c".to_string(),
                        content: "verbatim ".to_string()
                    },
                    Node::Text(" after ".to_string())
                ]
            ))
        );
        assert_eq!(
            line::<VerboseError<&str>>.parse("@begin@command a b c\nverbatim\n@end@command\nrest"),
            Ok((
                "rest",
                vec![Node::Block {
                    name: "command".to_string(),
                    arguments: "a b c".to_string(),
                    content: "verbatim\n".to_string()
                },]
            ))
        );
        assert!(line::<VerboseError<&str>>.parse("  \t  \n").is_err());
        assert!(line::<VerboseError<&str>>.parse("      \n").is_err());
        */
    }
    /*

    #[test]
    fn parse_empty_line() {
        assert_eq!(
            empty_line::<VerboseError<&str>>.parse("  \t  \nrest"),
            Ok(("rest", ()))
        );
        assert_eq!(
            empty_line::<VerboseError<&str>>.parse("      \nrest"),
            Ok(("rest", ()))
        );
        assert_eq!(
            empty_line::<VerboseError<&str>>.parse(" % ho \nrest"),
            Ok(("rest", ()))
        );
        assert!(empty_line::<VerboseError<&str>>.parse("  hi \n").is_err());
    }

    #[test]
    fn parse_block_command() {
        assert_eq!(
            block_command::<VerboseError<&str>>
                .parse("@begin@command a b c\nverbatim \\ @h % hi\n@end@command rest"),
            Ok((
                " rest",
                Node::Block {
                    name: "command".to_string(),
                    arguments: "a b c".to_string(),
                    content: "verbatim \\ @h % hi\n".to_string()
                }
            ))
        );
        assert_eq!(
            block_command::<VerboseError<&str>>.parse("@begin@command@stop a b c\nverbatim \\ @end@command @h % hi\n@end@command@stop rest"),
            Ok((
                " rest",
                Node::Block {
                    name: "command".to_string(),
                    arguments: "a b c".to_string(),
                    content: "verbatim \\ @end@command @h % hi\n".to_string()
                }
            ))
        );
    }

    #[test]
    fn parse_paragraph() {
        // TODO; check for right output!
        let par = r#"Open!
            This is a single paragraph
            We have: %comments
            lines
            @command commands
            % more comments
            and @command(inline commands!)
            and even @begin@command args
                Block commands!
            @end@command

            Rest goes here!"#;

        assert!(paragraph::<VerboseError<&str>>
            .parse(par)
            .map(|(rest, _)| rest.trim().starts_with("Rest goes here!"))
            .unwrap_or(false));
    }

    #[test]
    fn parse_document() {
        // TODO: test to see if all items occur here
        // Also tes
        let doc = r#"

            % we start with a comment
            and a line
            @command and a command % with another comment
            @begin@command
                and a verbatim
            @end@command

            % now we take a break

            and start another paragraph
            with more lines

            and more
            and stop
        "#;

        assert_eq!(
            document::<VerboseError<&str>>
                .parse(doc)
                .map(|(rest, res)| match res {
                    Node::Document(v) => (v.len(), rest),
                    _ => (0, ""),
                }),
            Ok((3, ""))
        );
    }
    */
}
