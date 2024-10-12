use mlua::{ErrorContext, Lua, Result, Table, TableExt, Value};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_till, take_till1, take_until, take_while},
    character::complete::{newline, space0},
    combinator::{map, not, opt, peek, success},
    multi::{fold_many0, fold_many1, many0, separated_list0},
    sequence::{delimited, pair, preceded, terminated},
    Finish, IResult,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
        match document(s).finish() {
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
    pub fn run_lua<'a>(self, lua: &'a Lua, commands: &Table<'a>) -> Result<Value<'a>> {
        match self {
            Self::Document(nodes) => {
                let arguments = lua.create_table()?;
                for node in nodes {
                    arguments.push(node.run_lua(lua, commands)?)?;
                }
                commands.call_method("document", arguments)
            }
            Self::Paragraph(nodes) => {
                let arguments = lua.create_table()?;
                for node in nodes {
                    arguments.push(node.run_lua(lua, commands)?)?;
                }
                commands.call_method("paragraph", arguments)
            }
            Self::Block {
                name,
                arguments,
                content,
            } => commands.call_method(&name, (arguments, content)),
            Self::Command { name, arguments } => commands.call_method(&name, arguments),
            Self::Text(t) => commands.call_method("text", t),
        }
    }
}

#[derive(Copy, Clone)]
struct Span<'a> {
    string: &'a str,
    row: usize,
    col: usize,
}

impl<'a> Span<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            string: s,
            row: 1,
            col: 1,
        }
    }

    /// Take a pattern
    fn take_one(self, pattern: &str) -> Option<(Self, &str)> {
        if self.string.starts_with(pattern) {
            Some((
                Self {
                    string: &self.string[pattern.len()..],
                    row: self.row,
                    col: pattern.width_cjk(),
                },
                pattern,
            ))
        } else {
            None
        }
    }

    /// Take any of the next characters
    fn take_any_one(self, any: &[char]) -> Option<(Self, char)> {
        for c in any.into_iter() {
            if self.string.starts_with(*c) {
                return Some((
                    Self {
                        string: &self.string[c.len_utf8()..],
                        row: self.row,
                        col: self.col + c.width_cjk().unwrap_or(0),
                    },
                    *c,
                ));
            }
        }

        None
    }

    /// Take until the predicate is true
    fn take_until<F: FnMut(char) -> bool>(self, predicate: F) -> Option<(Self, &'a str)> {
        let mid = self.string.find(predicate).filter(|mid| mid > &0)?;
        let (left, right) = self.string.split_at(mid);
        let (row, col) = left
            .chars()
            .fold((self.row, self.col), |(row, col), c| match c {
                '\t' => (row, col + 4),
                '\n' => (row, 1),
                _ => (row, col + c.width_cjk().unwrap_or(0)),
            });

        Some((
            Self {
                string: right,
                row,
                col,
            },
            left,
        ))
    }

    /// Take untill we find the pattern
    fn take_till_pattern(self, pattern: &str) -> Option<(Self, &'a str)> {
        let mid = self.string.find(pattern).filter(|mid| mid > &0)?;
        let (left, right) = self.string.split_at(mid);
        let (row, col) = left
            .chars()
            .fold((self.row, self.col), |(row, col), c| match c {
                '\t' => (row, col + 4),
                '\n' => (row, 1),
                _ => (row, col + c.width_cjk().unwrap_or(0)),
            });

        Some((
            Self {
                string: right,
                row,
                col,
            },
            left,
        ))
    }

    /// Take untill we find any of the characters
    fn take_till_any(self, pattern: &[char]) -> Option<(Self, &'a str)> {
        let mid = self.string.find(pattern).filter(|mid| mid > &0)?;
        let (left, right) = self.string.split_at(mid);
        let (row, col) = left
            .chars()
            .fold((self.row, self.col), |(row, col), c| match c {
                '\t' => (row, col + 4),
                '\n' => (row, 1),
                _ => (row, col + c.width_cjk().unwrap_or(0)),
            });

        Some((
            Self {
                string: right,
                row,
                col,
            },
            left,
        ))
    }
}

fn before<
    'a,
    T1,
    T2,
    F1: FnMut(Span<'a>) -> Option<(Span<'a>, T1)>,
    F2: FnMut(Span<'a>) -> Option<(Span<'a>, T2)>,
>(
    mut f1: F1,
    mut f2: F2,
) -> impl FnMut(Span<'a>) -> Option<(Span<'a>, T2)> {
    move |s| {
        let (s, _) = f1(s)?;
        f2(s)
    }
}

fn parse_document<'a>(lua: &'a Lua, string: &str, macros: Table<'a>) -> Result<Value<'a>> {
    let span = Span::new(string);

    // skip opening whitespace
    let mut span = span
        .take_until(|c| !c.is_whitespace())
        .map(|x| x.0)
        .unwrap_or(span);

    // document so far
    let mut document = lua.create_table()?;

    // string so far
    let mut accumulator = String::new();

    // all values in this paragraph
    let mut values = lua.create_table()?;

    // parse paragraph
    while let Some((s, _)) = span.take_until(|c| !c.is_whitespace()) {
        // TODO
        // read text
        if let Some((s, t)) = s.take_till_any(&['\n', '\\', '%', '@']) {
            accumulator.push_str(t);
            span = s;
        }
        // read escaped
        else if let Some((s, t)) =
            before(|s| s.take_one("\\"), |s| s.take_until(char::is_whitespace))(s)
        {
            accumulator.push_str(t);
            span = s;
        }
        // read inline macro
        else if let Some((s, name)) = before(
            |s| s.take_one("@"),
            |s| s.take_until(|c| "<{([|$".contains(c) || c.is_whitespace()),
        )(s)
        {
            // what the opening is
            let (s, closing) = s
                .take_any_one(&['<', '{', '(', '[', '|', '$'])
                .ok_or_else(|| mlua::Error::external("sus mogus"))?;
            span = s;

            // find the closing character
            let closing = match closing {
                '<' => '>',
                '{' => '}',
                ')' => ')',
                '[' => ']',
                x => x,
            };

            // content of the macro
            let mut argument = String::new();

            // take until we get the closing one
            while let Some((s, t)) = span.take_till_any(&['\\', closing]) {
                // parse the escaped character if any
                let (s, t) = before(|s| s.take_one("\\"), |s| s.take_until(char::is_whitespace))(s)
                    .unwrap_or((s, t));
                argument.push_str(t);
                span = s;
            }

            // get the closing character
            let (s, _) = span
                .take_any_one(&[closing])
                .ok_or_else(|| mlua::Error::external("sus mogus"))?;

            // push the current string
            if !accumulator.is_empty() {
                values.push(accumulator.as_str())?;
                accumulator.clear();
            }

            // push the call the macro
            values.push(
                macros
                    .call_method::<String, Value>(name, argument)
                    .context(&format!("Failed to call macro `{name}` on the macro table"))?,
            )?;

            span = s;
        }

        // read block macro

        // read comment

        // if we find an empty line or end of file, push the accumulator and values
        // TODO
    }

    // call the document function
    macros
        .call_method("document", document)
        .context("Failed to call macro `document` on the macro table")
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
fn line(s: &str) -> IResult<&str, Vec<Node>> {
    let line_content = fold_many0(
        alt((
            block_command,
            inline_command,
            map(alt((escaped, is_not("\n\\@%"))), |s| {
                Node::Text(s.to_string())
            }),
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
        alt((map(line, OneMore::More), map(line_command, OneMore::One))),
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
                    arguments: "a b c".to_string(),
                }
            ))
        );

        assert_eq!(
            line_command.parse("  @command a b \\) \\\\ c\nrest"),
            Ok((
                "rest",
                Node::Command {
                    name: "command".to_string(),
                    arguments: "a b \\) \\\\ c".to_string(),
                }
            ))
        );

        assert_eq!(
            line_command.parse("  @command a b % c \nrest"),
            Ok((
                "rest",
                Node::Command {
                    name: "command".to_string(),
                    arguments: "a b ".to_string(),
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
            line.parse("Hello world!\\% not a comment!\nrest"),
            Ok((
                "rest",
                vec![Node::Text("Hello world!% not a comment!".to_string())]
            ))
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
                        arguments: "a b c".to_string(),
                        content: "verbatim ".to_string()
                    },
                    Node::Text(" after ".to_string())
                ]
            ))
        );
        assert_eq!(
            line.parse("@begin@command a b c\nverbatim\n@end@command\nrest"),
            Ok((
                "rest",
                vec![Node::Block {
                    name: "command".to_string(),
                    arguments: "a b c".to_string(),
                    content: "verbatim\n".to_string()
                },]
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
                    arguments: "a b c".to_string(),
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

        assert!(paragraph
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
            document.parse(doc).map(|(rest, res)| match res {
                Node::Document(v) => (v.len(), rest),
                _ => (0, ""),
            }),
            Ok((3, ""))
        );
    }
}
