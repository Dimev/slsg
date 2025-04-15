use std::collections::{binary_heap::PeekMut, BTreeMap};

use winnow::{
    combinator::{alt, delimited, peek, preceded},
    stream::AsChar,
    token::{one_of, take_until, take_while},
    LocatingSlice, Parser, Result,
};

/// Ast
/// Each node refers to the nodes that follow up on it
#[derive(Clone, Debug)]
enum Ast {
    /// Paragraph or block, inside <p> tag
    Paragraph(Vec<Ast>),

    /// Raw text
    Text(String),

    /// Italic content <i>
    Italic(Vec<Ast>),

    /// Bold content <b>
    Bold(Vec<Ast>),

    /// Monospaced content <code>
    Mono(Vec<Ast>),

    /// Math block, inline or not
    Math(String, bool),

    /// Heading content <h1>, <h2>, ...
    Heading(u8, String),

    /// Call to a macro
    Macro(String, Vec<Vec<Ast>>),

    /// Call to a block macro
    Block(String, Vec<Vec<Ast>>, String),
}

/// Luamark document
pub struct Luamark {
    /// Meta values
    meta: BTreeMap<String, String>,

    /// Document, aka the AST
    document: Vec<Ast>,
}

// name of a macro @name
fn name<'s>(input: &mut LocatingSlice<&'s str>) -> Result<&'s str> {
    preceded("@", take_while(0.., ('a'..='z', 'A'..='Z', '_', '0'..='9'))).parse_next(input)
}

// any string, "content" and 'content'
fn string(input: &mut LocatingSlice<&str>) -> Result<String> {
    // taken from the example
    todo!()
}

// % any \n
fn comment(input: &mut LocatingSlice<&str>) -> Result<()> {
    ("%", take_until(0.., "\n"), "\n").parse_next(input)?;
    Ok(())
}

// @name = 'text'
fn attr<'s>(input: &mut LocatingSlice<&'s str>) -> Result<(&'s str, String)> {
    let (name, _, _, _, meta) = (
        name,
        take_while(0.., AsChar::is_space),
        "=",
        take_while(0.., AsChar::is_space),
        string,
    )
        .parse_next(input)?;
    Ok((name, meta))
}

// @name { arg1, arg2, arg3 = '', arg4 = [] }
fn call<'s>(input: &mut LocatingSlice<&'s str>) -> Result<(&'s str, String)> {
    todo!()
}

// arguments { arg1, arg2
fn args() {
    todo!()
}

#[cfg(test)]
mod tests {
    fn meta() {
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
