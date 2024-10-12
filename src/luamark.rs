use mlua::{ErrorContext, Lua, Result, Table, TableExt, Value};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_till, take_till1, take_until, take_while},
    character::complete::{newline, space0},
    combinator::{all_consuming, map, not, opt, peek, success},
    error::{context, ContextError, ParseError, VerboseError},
    multi::{fold_many0, fold_many1, many0, separated_list0},
    sequence::{delimited, pair, preceded, terminated},
    Finish, IResult,
};

/// AST node for luamark
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    /// Entire document
    Document(Vec<Node>),

    /// Paragraph, contains multiple nodes
    Paragraph(Vec<Node>),

    /// Block, contains the string verbatim
    Block {
        name: String,
        arguments: String,
        content: String,
    },

    /// Command, contains only the name and the arguments
    Command { name: String, arguments: String },

    /// Text, regular text
    Text(String),
}

impl Node {
    /// Makes a node from a string
    pub fn from_str(s: &str) -> Result<Self> {
        match all_consuming(document::<VerboseError<&str>>)(s).finish() {
            Ok(("", node)) => Ok(node),
            Ok((rest, _)) => Err(mlua::Error::external(format!(
                "Parser did not complete: {rest}"
            ))),
            Err(e) => Err(mlua::Error::external(e.to_string())),
        }
    }

    /// Convert a node to lua values
    pub fn to_lua(self, lua: &Lua) -> Result<Value> {
        match self {
            Self::Document(nodes) => {
                let table = lua.create_table()?;
                for node in nodes {
                    table.push(node.to_lua(lua)?)?;
                }
                Ok(Value::Table(table))
            }
            Self::Paragraph(nodes) => {
                let table = lua.create_table()?;
                table.set("type", "paragraph")?;
                for node in nodes {
                    table.push(node.to_lua(lua)?)?;
                }
                Ok(Value::Table(table))
            }
            Self::Block {
                name,
                arguments,
                content,
            } => {
                let table = lua.create_table()?;
                table.set("type", "block")?;
                table.set("name", name)?;
                table.set("arguments", arguments)?;
                table.set("content", content)?;
                Ok(Value::Table(table))
            }
            Self::Command { name, arguments } => {
                let table = lua.create_table()?;
                table.set("type", "command")?;
                table.set("name", name)?;
                table.set("arguments", arguments)?;
                Ok(Value::Table(table))
            }
            Self::Text(t) => Ok(Value::String(lua.create_string(t)?)),
        }
    }

    /// Run the commands in the parser
    pub fn run_lua<'a>(self, lua: &'a Lua, macros: &Table<'a>) -> Result<Value<'a>> {
        match self {
            Self::Document(nodes) => {
                let arguments = lua.create_table()?;
                for node in nodes {
                    arguments.push(node.run_lua(lua, macros)?)?;
                }
                macros
                    .call_method("document", arguments)
                    .context("Failed to call function `document` on the macros table")
            }
            Self::Paragraph(nodes) => {
                let arguments = lua.create_table()?;
                for node in nodes {
                    arguments.push(node.run_lua(lua, macros)?)?;
                }
                macros
                    .call_method("paragraph", arguments)
                    .context("Failed to call function `document` on the macros table")
            }
            Self::Block {
                name,
                arguments,
                content,
            } => macros.call_method(&name, (arguments, content)),
            Self::Command { name, arguments } => macros.call_method(&name, arguments).context(
                format!("Failed to call function `{name}` on the macros table"),
            ),
            Self::Text(t) => macros.call_method("text", t),
        }
    }
}

/// Parse a comment
fn comment<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, &'a str, E> {
    context("comment", delimited(tag("%"), is_not("\n"), peek(newline)))(s)
}

/// Parse an escaped sequence
/// these start with \, and will return the sequence of non-whitespace character followed by this \
fn escaped<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, &'a str, E> {
    context(
        "escaped",
        preceded(tag("\\"), take_till1(char::is_whitespace)),
    )(s)
}

/// Delimited argument, for an inline command
fn inline_argument<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    open: char,
    close: char,
) -> impl FnMut(&'a str) -> IResult<&'a str, String, E> {
    move |s| {
        context(
            "inline argument",
            delimited(
                nom::character::complete::char(open),
                fold_many0(
                    alt((
                        preceded(comment, tag("\n")),
                        escaped,
                        is_not(['\\', '%', close].as_slice()),
                    )),
                    || String::new(),
                    |mut acc, item| {
                        acc.push_str(item);
                        acc
                    },
                ),
                nom::character::complete::char(close),
            ),
        )(s)
    }
}

/// Inline command
/// Can be found in the middle of a line, and contains it's argument
/// Is allowed to flow across lines
fn inline_command<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Node, E> {
    // get the name of the command
    let (s, name) = preceded(
        tag("@"),
        take_till(|c: char| c.is_whitespace() || "([{<".contains(c)),
    )(s)?;

    // get the inner arguments
    let (s, arguments) = alt((
        inline_argument('(', ')'),
        inline_argument('[', ']'),
        inline_argument('{', '}'),
        inline_argument('<', '>'),
    ))(s)?;

    // succeed at parsing
    success(Node::Command {
        name: name.to_string(),
        arguments: arguments.to_string(),
    })(s)
}

/// Parse a command, on a line
fn line_command<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Node, E> {
    let command = map(
        terminated(
            pair(
                // parse the name
                preceded(tag("@"), take_till1(char::is_whitespace)),
                // parse the rest of the argument
                preceded(space0, many0(alt((is_not("\n%"), escaped)))),
            ),
            opt(comment),
        ),
        |(name, arguments)| Node::Command {
            name: name.to_string(),
            arguments: arguments.concat(),
        },
    );
    delimited(space0, command, newline)(s)
}

/// Parse a block command
fn block_command<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Node, E> {
    // opening @begin@name
    let (s, (name, tagname)) = preceded(
        tag("@begin@"),
        pair(
            take_till1(|c: char| c.is_whitespace() || c == '@'),
            opt(preceded(tag("@"), take_till1(char::is_whitespace))),
        ),
    )(s)?;

    // arguments
    let (s, arguments) = terminated(
        preceded(space0, many0(alt((is_not("\n%"), escaped)))),
        pair(opt(comment), tag("\n")),
    )(s)?;

    // what to end on
    let end = format!(
        "@end@{name}{}{}",
        if tagname.is_some() { "@" } else { "" },
        tagname.unwrap_or("")
    );

    // parse the entire block
    let (s, content) = take_until(end.as_str())(s)?;

    // parse the end block
    let (s, _) = tag(end.as_str())(s)?;

    // succeed with the contents
    success(Node::Block {
        name: name.to_string(),
        arguments: arguments.concat(),
        content: content.to_string(),
    })(s)
}

/// Parse a single line
fn line<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Vec<Node>, E> {
    let line_content = fold_many0(
        alt((
            context("block command", block_command),
            context("inline command", inline_command),
            context(
                "text",
                map(alt((escaped, is_not("\n\\@%"))), |s| {
                    Node::Text(s.to_string())
                }),
            ),
        )),
        || Vec::new(),
        |mut acc, item| {
            // if the last item on the accumulator is text, append it directly
            match (acc.last_mut(), item) {
                (Some(Node::Text(t1)), Node::Text(t2)) => {
                    t1.push_str(&t2);
                }
                (_, i) => acc.push(i),
            };

            acc
        },
    );
    delimited(
        // make sure we have content on this line
        pair(space0, peek(not(newline))),
        line_content,
        pair(opt(comment), newline),
    )(s)
}

/// Parse an empty line
fn empty_line<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, (), E> {
    context(
        "empty line",
        map(delimited(space0, opt(comment), newline), |_| ()),
    )(s)
}

/// Either 0, 1 or more nodes
enum OneMore {
    One(Node),
    More(Vec<Node>),
}

/// Parse a single paragraph
fn paragraph<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Node, E> {
    // paragraph consists of many lines
    let (s, items) = fold_many1(
        context(
            "line",
            alt((map(line, OneMore::More), map(line_command, OneMore::One))),
        ),
        || Vec::new(),
        |mut acc, item| match item {
            OneMore::One(x) => {
                acc.push(x);
                acc
            }
            OneMore::More(x) => match (acc.last_mut(), &x.as_slice()) {
                (Some(Node::Text(t1)), &[Node::Text(t2)]) => {
                    t1.push('\n');
                    t1.push_str(&t2);
                    acc
                }
                (_, x) => {
                    acc.extend_from_slice(&x);
                    acc
                }
            },
        },
    )(s)?;

    // we parsed the paragraph
    success(Node::Paragraph(items))(s)
}

/// Parse an entire document
fn document<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, Node, E> {
    // skip the initial emptiness
    let (s, _) = take_while(char::is_whitespace)(s)?;

    // get the paragraph
    let (s, paragraphs) = separated_list0(fold_many1(empty_line, || (), |_, _| ()), paragraph)(s)?;

    // get the optional empty line termination
    let (s, _) = take_while(char::is_whitespace)(s)?;

    // succeed
    success(Node::Document(paragraphs))(s)
}

#[cfg(test)]
mod tests {
    use nom::Parser;

    use super::*;

    #[test]
    fn parse_comment() {
        assert_eq!(
            comment::<VerboseError<&str>>.parse("% hello\nrest"),
            Ok(("\nrest", " hello"))
        );
        assert!(comment::<VerboseError<&str>>
            .parse("line % hello\nrest")
            .is_err());
    }

    #[test]
    fn parse_escaped() {
        assert_eq!(
            escaped::<VerboseError<&str>>.parse(r#"\\\@#^%() rest"#),
            Ok((" rest", r#"\\@#^%()"#))
        );
        assert!(escaped::<VerboseError<&str>>
            .parse(r#"hello \@ rest"#)
            .is_err());
    }

    #[test]
    fn parse_inline_argument() {
        assert_eq!(
            inline_argument::<VerboseError<&str>>('(', ')').parse("(Hello there!)rest"),
            Ok(("rest", "Hello there!".to_string()))
        );
        assert_eq!(
            inline_argument::<VerboseError<&str>>('(', ')').parse(r#"(We allow \) in here)rest"#),
            Ok(("rest", r#"We allow ) in here"#.to_string()))
        );
        assert!(inline_argument::<VerboseError<&str>>('(', ')')
            .parse(r#" (We allow \) in here)rest"#)
            .is_err());
    }
    #[test]
    fn parse_inline_command() {
        assert_eq!(
            inline_command::<VerboseError<&str>>.parse("@command(Hello there!)rest"),
            Ok((
                "rest",
                Node::Command {
                    name: "command".to_string(),
                    arguments: "Hello there!".to_string()
                }
            ))
        );
        assert_eq!(
            inline_command::<VerboseError<&str>>.parse("@command[Hello there!]rest"),
            Ok((
                "rest",
                Node::Command {
                    name: "command".to_string(),
                    arguments: "Hello there!".to_string()
                }
            ))
        );
        assert!(inline_command::<VerboseError<&str>>
            .parse("@commmand rest")
            .is_err());
    }

    #[test]
    fn parse_line_command() {
        assert_eq!(
            line_command::<VerboseError<&str>>.parse("@command a b c\nrest"),
            Ok((
                "rest",
                Node::Command {
                    name: "command".to_string(),
                    arguments: "a b c".to_string(),
                }
            ))
        );

        assert_eq!(
            line_command::<VerboseError<&str>>.parse("  @command a b \\) \\\\ c\nrest"),
            Ok((
                "rest",
                Node::Command {
                    name: "command".to_string(),
                    arguments: "a b \\) \\\\ c".to_string(),
                }
            ))
        );

        assert_eq!(
            line_command::<VerboseError<&str>>.parse("  @command a b % c \nrest"),
            Ok((
                "rest",
                Node::Command {
                    name: "command".to_string(),
                    arguments: "a b ".to_string(),
                }
            ))
        );

        assert!(line_command::<VerboseError<&str>>
            .parse("before @command a b % c \nrest")
            .is_err());
    }

    #[test]
    fn parse_line() {
        assert_eq!(
            line::<VerboseError<&str>>.parse("Hello world!\nrest"),
            Ok(("rest", vec![Node::Text("Hello world!".to_string())]))
        );
        assert_eq!(
            line::<VerboseError<&str>>.parse("Hello world!\\% not a comment!\nrest"),
            Ok((
                "rest",
                vec![Node::Text("Hello world!% not a comment!".to_string())]
            ))
        );
        assert_eq!(
            line::<VerboseError<&str>>.parse("  Hello world!\nrest"),
            Ok(("rest", vec![Node::Text("Hello world!".to_string())]))
        );
        assert_eq!(
            line::<VerboseError<&str>>.parse("Hello world! % ignore\nrest"),
            Ok(("rest", vec![Node::Text("Hello world! ".to_string())]))
        );
        assert_eq!(
            line::<VerboseError<&str>>.parse("Hello world! @command(hello)\nrest"),
            Ok((
                "rest",
                vec![
                    Node::Text("Hello world! ".to_string()),
                    Node::Command {
                        name: "command".to_string(),
                        arguments: "hello".to_string()
                    }
                ]
            ))
        );
        assert_eq!(
            line::<VerboseError<&str>>.parse("Hello world! @command(hello\nworld% ignore\n)\nrest"),
            Ok((
                "rest",
                vec![
                    Node::Text("Hello world! ".to_string()),
                    Node::Command {
                        name: "command".to_string(),
                        arguments: "hello\nworld\n".to_string()
                    }
                ]
            ))
        );
        assert_eq!(
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
    }

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
}
