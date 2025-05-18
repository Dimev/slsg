use std::collections::{binary_heap::PeekMut, BTreeMap};

use winnow::{
    combinator::{alt, delimited, peek, preceded},
    stream::AsChar,
    token::{one_of, take_until, take_while},
    LocatingSlice, Parser, Result,
};

/// Single content item
pub enum Content {
    /// standalone text
    Text(String),

    /// Macro call
    Call(String, Args),

    /// Heading
    Head(u8, Vec<Content>)

    
}

/// Arguments
pub struct Args {
    /// Non-key arguments
    list: Vec<String>,

    /// Keyed arguments
    keyed: BTreeMap<String, String>
}

/// Luamark document
pub struct Luamark {
    /// Meta values
    meta: BTreeMap<String, String>,

    /// Document, aka the AST
    document: Vec<Content>,
}

// name of a macro @name
fn name<'s>(input: &mut LocatingSlice<&'s str>) -> Result<&'s str> {
    preceded("@", take_while(1.., ('a'..='z', 'A'..='Z', '_', '0'..='9'))).parse_next(input)
}

// any string, "content" and 'content'
fn string(input: &mut LocatingSlice<&str>) -> Result<String> {
    // taken from the example
    todo!()
}

// content, enclosed by []
// can contain macros
fn content(input: &mut LocatingSlice<&str>) -> Result<String> {
    todo!()
}

// % any \n
fn comment(input: &mut LocatingSlice<&str>) -> Result<()> {
    ("%", take_until(0.., "\n"), "\n").parse_next(input)?;
    Ok(())
}

// arguments { arg1, arg2, arg3 = '' }
fn args(input: &mut LocatingSlice<&str>) -> Result<()> {
    todo!()
}

// @name { arg1, arg2, arg3 = '', arg4 = [] }
fn call<'s>(input: &mut LocatingSlice<&'s str>) -> Result<(&'s str, String)> {
    todo!()
}

// meta @? { arg1, arg2, arg3 = '', arg4 = [] }
fn meta<'s>(input: &mut LocatingSlice<&'s str>) -> Result<(&'s str, String)> {
    todo!()
}

#[cfg(test)]
mod tests {
    fn meta() {
        let meta = r#"
        @? {
            'arg 1',
            "arg 2",
            [arg 3],
            arg4 = 'text',
            arg5 = "text",
            arg6 = [text],
        }
        "#;
        todo!()
    }
    fn paragraph() {
        todo!()
    }
    fn header() {
        todo!()
    }
    fn italic() {
        todo!()
    }
    fn bold() {
        todo!()
    }
    fn code() {
        todo!()
    }
    fn call() {
        todo!()
    }
}
