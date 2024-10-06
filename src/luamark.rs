/*

Features:
Comments: started with %
Commands: started with @name, begin of the line means it's parsed on all lines,
    mid of the line means it needs to be parsed between opening and closing brackets
Blocks: started with @begin@name, closed with @end@name, gives the entire content back verbatim
    Can have optional third @ that is ignored in case there needs to be an extra tag

Example:
% this is a comment!
@article Hello world!
@section Paragraph start!
This is a paragraph
and here is code with @bold(Bold text!)

@begin@code lua
-- some lua sample code!
print 'Hello world!'
@end@code
*/

use mlua::{Lua, Table, TableExt, Value};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_till, take_till1, take_until, take_while},
    character::complete::{newline, space0},
    combinator::{map, not, opt, peek, success},
    multi::{fold_many0, fold_many1, many0, separated_list0},
    sequence::{delimited, pair, preceded, terminated},
    Err, Finish, IResult, Parser,
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
    pub fn from_str(s: &str) -> Result<Node, ()> {
        todo!()
    }
}

/// Parse a comment
fn comment(s: &str) -> IResult<&str, &str> {
    delimited(tag("%"), is_not("\n"), peek(newline))(s)
}

/// Parse an escaped sequence
/// these start with \, and will return the sequence of non-whitespace character followed by this \
fn escaped(s: &str) -> IResult<&str, &str> {
    preceded(tag("\\"), take_till1(char::is_whitespace))(s)
}

/// Delimited argument, for an inline command
fn inline_argument(open: char, close: char) -> impl FnMut(&str) -> IResult<&str, String> {
    move |s| {
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
        )(s)
    }
}

/// Inline command
/// Can be found in the middle of a line, and contains it's argument
/// Is allowed to flow across lines
fn inline_command(s: &str) -> IResult<&str, Node> {
    // get the name of the command
    let (s, name) = preceded(
        tag("@"),
        take_till(|c: char| c.is_whitespace() || "([{<".contains(c)),
    )(s)?;

    // TODO: better escaping? [==[ ]==] style?

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
fn line_command(s: &str) -> IResult<&str, Node> {
    let command = map(
        terminated(
            pair(
                // parse the name
                preceded(tag("@"), take_till(char::is_whitespace)),
                // parse the rest of the argument
                many0(alt((is_not("\n%"), escaped))),
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
fn block_command(s: &str) -> IResult<&str, Node> {
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
        many0(alt((is_not("\n%"), escaped))),
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
fn line(s: &str) -> IResult<&str, Vec<Node>> {
    let line_content = many0(alt((
        inline_command,
        block_command,
        map(alt((escaped, is_not("\n\\@%"))), |s| {
            Node::Text(s.to_string())
        }),
    )));
    delimited(
        // make sure we have content on this line
        pair(space0, peek(not(newline))),
        line_content,
        pair(opt(comment), newline),
    )(s)
}

/// Parse an empty line
fn empty_line(s: &str) -> IResult<&str, ()> {
    map(delimited(space0, opt(comment), newline), |_| ())(s)
}

/// Either 0, 1 or more nodes
enum OneMore {
    One(Node),
    More(Vec<Node>),
}

/// Parse a single paragraph
fn paragraph(s: &str) -> IResult<&str, Node> {
    // paragraph consists of many lines
    let (s, items) = fold_many1(
        alt((map(line_command, OneMore::One), map(line, OneMore::More))),
        || Vec::new(),
        |mut acc, item| match item {
            OneMore::One(x) => {
                acc.push(x);
                acc
            }
            OneMore::More(mut x) => {
                acc.append(&mut x);
                acc
            }
        },
    )(s)?;

    // we parsed the paragraph
    success(Node::Paragraph(items))(s)
}

/// Parse an entire document
fn document(s: &str) -> IResult<&str, Node> {
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
        assert_eq!(comment.parse("% hello\nrest"), Ok(("\nrest", " hello")));
        assert!(comment.parse("line % hello\nrest").is_err());
    }

    #[test]
    fn parse_escaped() {
        assert_eq!(
            escaped.parse(r#"\\\@#^%() rest"#),
            Ok((" rest", r#"\\@#^%()"#))
        );
        assert!(escaped.parse(r#"hello \@ rest"#).is_err());
    }

    #[test]
    fn parse_inline_argument() {
        assert_eq!(
            inline_argument('(', ')').parse("(Hello there!)rest"),
            Ok(("rest", "Hello there!".to_string()))
        );
        assert_eq!(
            inline_argument('(', ')').parse(r#"(We allow \) in here)rest"#),
            Ok(("rest", r#"We allow ) in here"#.to_string()))
        );
        assert!(inline_argument('(', ')')
            .parse(r#" (We allow \) in here)rest"#)
            .is_err());
    }
    #[test]
    fn parse_inline_command() {
        assert_eq!(
            inline_command.parse("@command(Hello there!)rest"),
            Ok((
                "rest",
                Node::Command {
                    name: "command".to_string(),
                    arguments: "Hello there!".to_string()
                }
            ))
        );
        assert_eq!(
            inline_command.parse("@command[Hello there!]rest"),
            Ok((
                "rest",
                Node::Command {
                    name: "command".to_string(),
                    arguments: "Hello there!".to_string()
                }
            ))
        );
        assert!(inline_command.parse("@commmand rest").is_err());
    }

    #[test]
    fn parse_line_command() {
        assert_eq!(
            line_command.parse("@command a b c\nrest"),
            Ok((
                "rest",
                Node::Command {
                    name: "command".to_string(),
                    arguments: " a b c".to_string(),
                }
            ))
        );

        assert_eq!(
            line_command.parse("  @command a b \\) \\\\ c\nrest"),
            Ok((
                "rest",
                Node::Command {
                    name: "command".to_string(),
                    arguments: " a b \\) \\\\ c".to_string(),
                }
            ))
        );

        assert_eq!(
            line_command.parse("  @command a b % c \nrest"),
            Ok((
                "rest",
                Node::Command {
                    name: "command".to_string(),
                    arguments: " a b ".to_string(),
                }
            ))
        );

        assert!(line_command
            .parse("before @command a b % c \nrest")
            .is_err());
    }

    #[test]
    fn parse_line() {
        assert_eq!(
            line.parse("Hello world!\nrest"),
            Ok(("rest", vec![Node::Text("Hello world!".to_string())]))
        );
        assert_eq!(
            line.parse("  Hello world!\nrest"),
            Ok(("rest", vec![Node::Text("Hello world!".to_string())]))
        );
        assert_eq!(
            line.parse("Hello world! % ignore\nrest"),
            Ok(("rest", vec![Node::Text("Hello world! ".to_string())]))
        );
        assert_eq!(
            line.parse("Hello world! @command(hello)\nrest"),
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
            line.parse("Hello world! @command(hello\nworld% ignore\n)\nrest"),
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
        assert_eq!(line.parse("   % hello\nrest"), Ok(("rest", vec![])));
        assert_eq!(
            line.parse("Hello @begin@command a b c\nverbatim @end@command after % comment\nrest"),
            Ok((
                "rest",
                vec![
                    Node::Text("Hello ".to_string()),
                    Node::Block {
                        name: "command".to_string(),
                        arguments: " a b c".to_string(),
                        content: "verbatim ".to_string()
                    },
                    Node::Text(" after ".to_string())
                ]
            ))
        );
        assert!(line.parse("  \t  \n").is_err());
        assert!(line.parse("      \n").is_err());
    }

    #[test]
    fn parse_empty_line() {
        assert_eq!(empty_line.parse("  \t  \nrest"), Ok(("rest", ())));
        assert_eq!(empty_line.parse("      \nrest"), Ok(("rest", ())));
        assert_eq!(empty_line.parse(" % ho \nrest"), Ok(("rest", ())));
        assert!(empty_line.parse("  hi \n").is_err());
    }

    #[test]
    fn parse_block_command() {
        assert_eq!(
            block_command.parse("@begin@command a b c\nverbatim \\ @h % hi\n@end@command rest"),
            Ok((
                " rest",
                Node::Block {
                    name: "command".to_string(),
                    arguments: " a b c".to_string(),
                    content: "verbatim \\ @h % hi\n".to_string()
                }
            ))
        );
        assert_eq!(
            block_command.parse("@begin@command@stop a b c\nverbatim \\ @end@command @h % hi\n@end@command@stop rest"),
            Ok((
                " rest",
                Node::Block {
                    name: "command".to_string(),
                    arguments: " a b c".to_string(),
                    content: "verbatim \\ @end@command @h % hi\n".to_string()
                }
            ))
        );
    }

    #[test]
    fn parse_paragraph() {
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

        assert!(paragraph
            .parse(par)
            .map(|(rest, _)| rest.trim().starts_with("Rest goes here!"))
            .unwrap_or(false));
    }

    #[test]
    fn parse_document() {
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
            document.parse(doc).map(|(rest, res)| match res {
                Node::Document(v) => (v.len(), rest),
                _ => (0, ""),
            }),
            Ok((3, ""))
        );
    }
}
