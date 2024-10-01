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
    bytes::complete::{tag, take_till, take_until, take_while},
    character::complete::anychar,
    combinator::{map, peek, success},
    multi::{count, many0_count, many1, many_till, separated_list0, separated_list1},
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

/// Parse an argument list
fn arg_list(s: &str) -> IResult<&str, &str> {
    let separator = tuple((tag(","), take_while(char::is_whitespace)));
    let arguments = separated_list1(separator, take_while(|c| c != ','));

    delimited(tag("("), arguments, tag(")"))(s)?;

    todo!()
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
    let (s, argument) = alt((arg_list, string_literal))(s)?;

    todo!()
}

pub(crate) struct Parser<'a> {
    row: usize,
    column: usize,
    remaining: &'a str,
    lua: &'a Lua,
    commands: Table<'a>,
}

impl<'a> Parser<'a> {
    pub(crate) fn parse(lua: &Lua, commands: Table, string: &str) -> Result<()> {
        // make the parser
        let parser = Parser {
            row: 1,
            column: 1,
            remaining: string,
            lua,
            commands,
        };

        // start parsing

        Ok(())
    }

    /// comment -- text \n
    fn comment(&mut self) {
        // skip the --
        self.remaining = self.remaining.trim_start_matches("--");

        // skip the rest that is not a newline
        let mut chars = self.remaining.chars();
        while chars.next().unwrap_or('\n') != '\n' {
            self.remaining = chars.as_str();
        }

        // we already skipped the newline
        self.column = 0;
        self.row += 1;
    }

    /// Macro @name(arg1, arg2, arg3) or @name [[ string ]]
    fn macro_call(&mut self) -> Option<Result<Value>> {
        // skip the @
        let mut rest = self.remaining.strip_prefix('@')?;
        let mut row = self.row;
        let mut column = self.column;

        // read the name
        let mut name = String::new();
        let mut chars = rest.chars();
        while let Some(c) = chars.next() {
            rest = chars.as_str();
            name.push(c);
            column += 1;

            // stop if the next character is not a name character
            if !chars
                .as_str()
                .starts_with(|c: char| c.is_alphanumeric() || c == '_')
            {
                break;
            }
        }

        // name needs at least one character
        if name.len() == 0 {
            return None;
        }

        // trim the whitespaces
        let mut chars = rest.chars();
        while let Some(c) = chars.next() {
            rest = chars.as_str();
            // stop if the next character is not a whitespace
            if !chars.as_str().starts_with(char::is_whitespace) {
                break;
            }
            // row
            else if c == '\n' {
                column = 0;
                row += 1;
            }
            // column
            else {
                column += 1;
            }
        }

        // now parse either a single string
        let result: Result<Value> = if let Some(string) = self.string() {
            // call the function
            self.commands.call_method(&name, string)
        }
        // or an argument list
        else if rest.starts_with('(') {
            // trim starting (
            rest = rest.trim_start_matches('(');
            column += 1;

            // read the arguments, seperated by ,
            todo!();
        }
        // or no arguments, which isn't allowed
        else {
            return None;
        };

        // reset state
        self.remaining = rest;
        self.row = row;
        self.column = column;

        todo!()
    }

    /// String literal [[ ]]
    fn string(&mut self) -> Option<String> {
        // skip the opening [
        let mut rest = self.remaining.strip_prefix('[')?;
        let mut row = self.row;
        let mut column = self.column;

        // read the amount of =
        let mut count = 0usize;
        while rest.starts_with('[') {
            rest = &rest[1..];
            column += 1;
            count += 1;
        }

        // read the other [
        rest = rest.strip_prefix('[')?;

        // what to expect for the closing bracket
        let closing = format!(
            "]{}]",
            std::iter::repeat('=').take(count).collect::<String>()
        );

        let mut chars = rest.chars();
        let mut string = String::new();
        while let Some(c) = chars.next() {
            // stop if we find the end
            if chars.as_str().starts_with(&closing) {
                column += closing.len();
                break;
            }
            // next row if we see a newline
            else if c == '\n' {
                row += 1;
                column = 0;
            // next column
            } else {
                column += 1;
            }

            // add to the string
            string.push(c);
        }

        // reset state
        self.column = column;
        self.row = row;
        self.remaining = &chars.as_str()[closing.len()..];

        // we found a string
        Some(string)
    }

    /// Escaped backslash
    fn escape(&mut self) {
        todo!()
    }
}

// TODO: test
