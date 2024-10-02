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
    bytes::complete::{is_not, tag, take_till, take_until, take_while},
    character::complete::{anychar, none_of},
    combinator::{map, opt, peek, success},
    multi::{
        count, fold_many0, fold_many1, many0, many0_count, many1, many_till, separated_list0,
        separated_list1,
    },
    sequence::{delimited, preceded, tuple},
    IResult,
};

/// Escaped character
fn escaped(s: &str) -> IResult<&str, char> {
    preceded(tag("\\"), anychar)(s)
}

/// Parse a comment -- any \n
fn comment(s: &str) -> IResult<&str, &str> {
    delimited(tag("--"), take_until("\n"), tag("\n"))(s)
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
    // this is an offset from the start
    let literal = &start[..start.len() - s.len()];

    // parse the closing tag
    let (s, _) = delimited(tag("]"), count(tag("="), closing_count), tag("]"))(s)?;

    // success
    success(literal)(s)
}

/// Any of the argument characters we can find
enum ArgChar<'a> {
    Str(&'a str),
    String(String),
    Call(String),
    Nothing,
}

/// Single argument for an argument list
fn argument(s: &str) -> IResult<&str, String> {
    let arg_char = map(is_not(","), ArgChar::Str);
    let arg_comment = map(comment, |_| ArgChar::Nothing);
    let arg_call = map(macro_call, |_| ArgChar::Call(String::new()));
    let arg_literal = map(string_literal, |_| ArgChar::String(String::new()));

    // collect the string
    fold_many0(
        alt((arg_literal, arg_call, arg_comment, arg_char)),
        || String::new(),
        |mut acc, item| match item {
            ArgChar::Str(s) => {
                acc.push_str(s);
                acc
            }
            ArgChar::String(s) => {
                acc.push_str(&s);
                acc
            }
            ArgChar::Call(s) => {
                acc.push_str(&s);
                acc
            }
            ArgChar::Nothing => acc,
        },
    )(s)
}

/// Parse an argument list
fn arg_list(s: &str) -> IResult<&str, String> {
    let separator = tuple((tag(","), take_while(char::is_whitespace)));
    let arguments = separated_list1(separator, argument);

    map(delimited(tag("("), arguments, tag(")")), |v| v.concat())(s)
}

/// Parse a macro call
fn macro_call(s: &str) -> IResult<&str, &str> {
    // @ followed by the name
    let (s, name) = preceded(
        tag("@"),
        take_while(|c: char| c.is_alphanumeric() || c == '_'),
    )(s)?;

    // skip any whitespace
    let (s, _) = take_while(char::is_whitespace)(s)?;

    // either parse a string or argument list
    let (s, argument) = alt((arg_list, map(string_literal, String::from)))(s)?;

    success("sus mogus")(s)
}

/// Luamark paragraph
fn paragraph(s: &str) -> IResult<&str, ()> {
    // line is any character, string literal or call that is not a newline
    let line_char = map(is_not("\n"), ArgChar::Str);
    let line_comment = map(tuple((tag("--"), take_until("\n"))), |_| ArgChar::Nothing);
    let line_call = map(macro_call, |_| ArgChar::Call(String::new()));
    let line_literal = map(string_literal, |_| ArgChar::String(String::new()));

    let line = fold_many1(
        alt((line_literal, line_call, line_comment, line_char)),
        || String::new(),
        |acc, item| acc,
    );

    // paragraph is multiple lines OR comments seperated by newlines
    fold_many1(line, || (), |acc, item| acc)(s)
}

/// Entire luamark file
fn luamark(s: &str) -> IResult<&str, ()> {
    // an entire luamark file is a number of paragraphs seperated by whitespace
    map(
        tuple((
            separated_list0(
                tuple((tag("\n"), take_while(|c: char| c.is_whitespace()))),
                paragraph,
            ),
            opt(take_while(|c: char| c.is_whitespace())),
        )),
        |_| (),
    )(s)
}

#[cfg(test)]
mod tests {
    use nom::Parser;

    use super::*;
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
