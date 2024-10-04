/*

Features:
Comment, --
Commands, start with @, followed by name, optional string argument or function call
String, same as lua string literal
Escaping: done with \, not needed in string literal
Example:

@article(Hello world!, 1-1-1970)

-- Comment
This is a paragraph.
It consists of multiple lines

This is a second paragraph
Again, multiple lines

@section(And a third paragraph)
Now with a header if it's set up that way

@code(lua, [[
-- and some lua sample code!
print("Hello, world!")
]])

*/

use mlua::{Lua, Result, Table, TableExt, Value};
use nom::{
    branch::alt,
    bytes::complete::{is_a, is_not, tag, take_till, take_till1, take_until, take_while},
    character::complete::{anychar, none_of},
    combinator::{map, not, opt, peek, success},
    multi::{
        count, fold_many0, fold_many1, many0, many0_count, many1, many_till, separated_list0,
        separated_list1,
    },
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};

/// AST for luamark
#[derive(Clone, Debug, PartialEq, Eq)]
enum Ast<'a> {
    /// Entire document, consists of multiple paragraphs
    Document(Vec<Ast<'a>>),

    /// Paragraph
    Paragraph(Vec<Ast<'a>>),

    /// Normal text
    Text(String),

    /// Normal text, but as a reference
    TextRef(&'a str),

    /// Single character
    Char(char),

    /// A number of items
    Many(Vec<Ast<'a>>),

    /// Macro call, string argument
    CallString(&'a str, &'a str),

    /// Macro call, normal arguments
    CallMany(&'a str, Vec<Ast<'a>>),
}

/// Any of the argument characters we can find
#[derive(Debug)]
enum TextElem<'a> {
    Str(&'a str),
    String(String),
    Char(char),
    Value(Ast<'a>),
    Nothing,
}

impl<'a> TextElem<'a> {
    /// Create a new item stack
    fn new() -> (String, Vec<Ast<'a>>) {
        (String::new(), Vec::new())
    }

    /// Combine the item stack
    fn combine(acc: (String, Vec<Ast<'a>>), item: TextElem<'a>) -> (String, Vec<Ast<'a>>) {
        let (mut text, mut stack) = acc;
        match item {
            TextElem::Str(s) => {
                text.push_str(s);
                (text, stack)
            }
            TextElem::String(s) => {
                text.push_str(&s);
                (text, stack)
            }
            TextElem::Char(c) => {
                text.push(c);
                (text, stack)
            }
            TextElem::Value(v) => {
                stack.push(v);
                (String::new(), stack)
            }
            TextElem::Nothing => (text, stack),
        }
    }

    fn finish(mut acc: (String, Vec<Ast<'a>>)) -> Ast<'a> {
        if acc.0.len() > 0 {
            acc.1.push(Ast::Text(acc.0));
        }

        Ast::Many(acc.1)
    }
}

/// Escaped character
fn escaped(s: &str) -> IResult<&str, char> {
    preceded(tag("\\"), anychar)(s)
}

/// Parse a comment -- any \n
fn comment(s: &str) -> IResult<&str, &str> {
    delimited(tag("--"), take_until("\n"), peek(tag("\n")))(s)
}

/// Parse a string literal [=[ any ]=]
fn string_literal(s: &str) -> IResult<&str, &str> {
    // opening string pattern
    let (s, closing_count) = peek(delimited(tag("["), many0_count(tag("=")), tag("[")))(s)?;

    // parse the opening string pattern
    let (s, _) = delimited(tag("["), count(tag("="), closing_count), tag("["))(s)?;

    // where we started
    let start = s;

    // any character until we can get the closing tag
    let (s, _) = many_till(
        map(anychar, drop),
        delimited(tag("]"), count(tag("="), closing_count), tag("]")),
    )(s)?;

    // get the contained string
    // this is an offset from the start + the closing ]==]
    let literal = &start[..start.len() - s.len() - closing_count - 2];

    // success
    success(literal)(s)
}

/// Single argument for an argument list
fn argument(s: &str) -> IResult<&str, Ast> {
    let arg_string = map(is_not(",)\\@"), TextElem::Str);
    let arg_escaped = map(escaped, TextElem::Char);
    let arg_comment = map(comment, |_| TextElem::Nothing);
    let arg_call = map(macro_call, TextElem::Value);
    let arg_literal = map(string_literal, |s| TextElem::Value(Ast::TextRef(s)));

    // collect the string
    let arg = fold_many1(
        alt((arg_literal, arg_call, arg_comment, arg_escaped, arg_string)),
        TextElem::new,
        TextElem::combine,
    );

    map(arg, TextElem::finish)(s)
}

/// Parse an argument list
fn arg_list(s: &str) -> IResult<&str, Vec<Ast>> {
    let separator = tuple((tag(","), take_while(char::is_whitespace)));
    let arguments = separated_list0(separator, argument);

    delimited(tag("("), arguments, tag(")"))(s)
}

/// Parse a macro call
fn macro_call(s: &str) -> IResult<&str, Ast> {
    // @ followed by the name
    let (s, name) = preceded(
        tag("@"),
        take_while(|c: char| c.is_alphanumeric() || c == '_'),
    )(s)?;

    // skip any whitespace
    let (s, _) = take_while(char::is_whitespace)(s)?;

    // either parse a string or argument list
    alt((
        map(arg_list, |l| Ast::CallMany(name, l)),
        map(string_literal, |s| Ast::CallString(name, s)),
    ))(s)
}

/// Luamark paragraph
fn paragraph(s: &str) -> IResult<&str, Ast> {
    // line is any character, string literal or call that is not a newline
    let line_string = map(is_not("\n\\@-["), TextElem::Str);
    let line_escaped = map(escaped, TextElem::Char);
    let line_comment = map(tuple((tag("--"), take_until("\n"))), |_| TextElem::Nothing);
    let line_call = map(macro_call, TextElem::Value);
    let line_literal = map(string_literal, |s| TextElem::Value(Ast::TextRef(s)));
    let line_progress = map(
        tuple((
            tag("\n"),
            take_while(|c: char| c.is_whitespace() && c != '\n'),
            peek(is_not("\n"))
        )),
        |_| TextElem::Str("\n"),
    );

    map(
        fold_many1(
            alt((
                line_literal,
                line_call,
                line_comment,
                line_escaped,
                line_string,
                line_progress,
            )),
            TextElem::new,
            TextElem::combine,
        ),
        TextElem::finish,
    )(s)
}

/// Entire luamark file
fn luamark(s: &str) -> IResult<&str, ()> {
    // an entire luamark file is a number of paragraphs seperated by whitespace
    let file = map(
        separated_list0(
            tuple((tag("\n"), take_while(|c: char| c.is_whitespace()))),
            paragraph,
        ),
        |_| (),
    );

    // remove any leading or trailing whitespace
    delimited(
        take_while(|c: char| c.is_whitespace()),
        file,
        take_while(|c: char| c.is_whitespace()),
    )(s)
}

#[cfg(test)]
mod tests {
    use nom::Parser;

    use super::*;

    #[test]
    fn prim_comment() {
        assert_eq!(
            comment.parse("-- a comment\nrest"),
            Ok(("\nrest", " a comment"))
        );
    }

    #[test]
    fn prim_string_literal() {
        assert_eq!(
            string_literal.parse("[[ string! ]] rest"),
            Ok((" rest", " string! "))
        );

        assert_eq!(
            string_literal.parse("[=[ [[string!]] ]=] rest"),
            Ok((" rest", " [[string!]] "))
        );
    }

    #[test]
    fn prim_argument() {
        assert_eq!(
            argument.parse("arg1"),
            Ok(("", Ast::Many(vec![Ast::Text(String::from("arg1"))])))
        );
    }

    #[test]
    fn prim_arg_list() {
        assert_eq!(
            arg_list.parse("(arg1) rest"),
            Ok((
                " rest",
                vec![Ast::Many(vec![Ast::Text(String::from("arg1"))])]
            ))
        );
        assert_eq!(
            arg_list.parse("(arg1, arg2) rest"),
            Ok((
                " rest",
                vec![
                    Ast::Many(vec![Ast::Text(String::from("arg1"))]),
                    Ast::Many(vec![Ast::Text(String::from("arg2"))])
                ]
            ))
        );
    }

    #[test]
    fn prim_macro_call() {
        assert_eq!(
            macro_call.parse("@name1234_() rest"),
            Ok((" rest", Ast::CallMany("name1234_", Vec::new())))
        );

        assert_eq!(
            macro_call.parse("@name [[ arg ]] rest"),
            Ok((" rest", Ast::CallString("name", " arg ")))
        );

        assert_eq!(
            macro_call.parse("@name(arg1, arg2) rest"),
            Ok((
                " rest",
                Ast::CallMany(
                    "name",
                    vec![
                        Ast::Many(vec![Ast::Text(String::from("arg1"))]),
                        Ast::Many(vec![Ast::Text(String::from("arg2"))])
                    ]
                )
            ))
        );
    }

    #[test]
    fn prim_macro_call_recursive() {
        assert_eq!(
            macro_call.parse("@name(arg1, --comment\n @name2 [[ arg2 ]]) rest"),
            Ok((
                " rest",
                Ast::CallMany(
                    "name",
                    vec![
                        Ast::Many(vec![Ast::Text(String::from("arg1"))]),
                        Ast::Many(vec![Ast::CallString("name2", " arg2 ")])
                    ]
                )
            ))
        );
    }

    #[test]
    fn prim_paragraph_text() {
        assert_eq!(
            paragraph.parse("line 1\nline 2\n-- comment\nline 3\n\nrest"),
            Ok(("\nrest", Ast::TextRef("")))
        );
    }

    #[test]
    fn comments() {
        let s = "
            -- first paragraph
            Paragraph 1
            test test test -- end of line comment
            another line
            before
            -- comment
            and after!
        ";

        assert_eq!(luamark.parse(s), Ok(("", ())));
    }

    #[test]
    fn paragraphs() {
        let s = "
        -- first paragraph
        Paragraph 1
        test test test
        another line

        paragraph 2
        more lines
        before
        -- comment inbetween
        after

        and another paragraph
        line line line

        -- start with another comment
        more more more 
        paragraph!
        ";

        assert_eq!(luamark.parse(s), Ok(("", ())));
    }

    #[test]
    fn literals() {
        let s = r#"
            paragraph 1
            line
            \[ escape! \\

            paragraph 2
            [[ has a literal in it! ]]
            [==[ and a literal with a [[ literal ]] in it! ]==]
            [[ inside literals, we skip -- comments and escapes \\ \[ ]]
        "#;

        assert_eq!(luamark.parse(s), Ok(("", ())));
    }

    #[test]
    fn macro_calls() {
        let s = "
            @title(This is a test)
            @description [[ This is a short test for macros ]]
            -- we can nest them too
            @table(
                line 1,
                line 2,
                -- comment!
                line 3,
                [[ line 4 ]],
                @line(5, 6, 7)
            )
        ";

        assert_eq!(luamark.parse(s), Ok(("", ())));
    }
}
