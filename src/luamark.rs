use std::collections::BTreeMap;

use mlua::{ErrorContext, IntoLua, Lua, MultiValue, ObjectLike, Result, Table, Value};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Ast
/// Each node refers to the nodes that follow up on it
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

    /// Heading content <h1>, <h2>, ...
    Head(u8, Vec<Ast>),

    /// Call to a macro
    Macro(String, Vec<Ast>),

    /// Call to a block macro
    Block(String, Vec<Ast>, String),
}

/// Luamark document
struct Luamark {
    /// Meta values
    meta: BTreeMap<String, String>,

    /// Document, aka the AST
    document: Vec<Ast>,
}

impl Luamark {
    fn from_str(lmk: &str) -> Result<Self> {
        // parser state
        let mut state = Parser {
            input: lmk,
            row: 1,
            col: 1,
        };

        todo!()
    }
}

/// Parser for luamark
#[derive(Clone)]
pub(crate) struct Parser<'a> {
    /// current remaining input
    input: &'a str,

    /// Current row
    row: usize,

    /// Current column
    col: usize,
}

impl<'a> Parser<'a> {
    /// parse a comment
    /// --.*\n
    fn comment(&mut self) -> Option<()> {
        self.pat("%")?;
        self.until_pat("\n")?;
        Some(())
    }

    /// Parse escaped content
    /// \anything-non-whitespace
    fn escaped(&mut self) -> Option<&str> {
        self.pat("\\")?;
        self.until_pred(char::is_whitespace)
    }

    /// Parse a macro name
    fn name(&mut self) -> Option<&str> {
        // alphanum or _
        self.pred(|x| x.is_alphanumeric() || x == '_')
    }

    /// Parse a meta variable
    fn meta(&mut self) -> Option<(String, String)> {
        // @ and name
        let mut cx = self.clone();
        cx.pat("@")?;

        // meta name
        let name = cx.name()?.to_string();

        // trim any whitespace
        cx.pred(char::is_whitespace)?;
        cx.pat("=")?;
        cx.pred(char::is_whitespace)?;

        // split based on how the string is escaped
        // content
        let mut content = String::new();
        if cx.pat("\"").is_some() {
            while !cx.input.starts_with('"') {
                // text
                if let Some(x) = cx.until_pred(|x| "\"\\".contains(x)) {
                    content.push_str(x);
                }

                // escape
                if let Some(x) = cx.escaped() {
                    content.push_str(x);
                }
            }

            // close
            cx.pat("\"")?;
        } else if cx.pat("'").is_some() {
            while !cx.input.starts_with('\'') {
                // text
                if let Some(x) = cx.until_pred(|x| "'\\".contains(x)) {
                    content.push_str(x);
                }

                // escape
                if let Some(x) = cx.escaped() {
                    content.push_str(x);
                }
            }

            // close
            cx.pat("'")?;
        } else {
            while !cx.input.starts_with(['\n', '%']) {
                // text
                if let Some(x) = cx.until_pred(|x| "%\n\\".contains(x)) {
                    content.push_str(x);
                }

                // escape
                if let Some(x) = cx.escaped() {
                    content.push_str(x);
                }
            }
        }

        // advance own state
        *self = cx;
        Some((name, content))
    }

    /// Parse macro arguments
    fn args(&mut self) -> Option<Vec<Ast>> {
        let mut cx = self.clone();
        cx.pat("(")?;

        // TODO: paragraph

        // close
        cx.pat(")")?;

        // advance own state
        *self = cx;
        todo!()
    }

    /// Parse a macro call
    fn call(&mut self) -> Option<Ast> {
        let mut cx = self.clone();
        cx.pat("@")?;

        // macro name
        let name = cx.name()?.to_string();

        // arguments
        cx.pred(char::is_whitespace)?;
        let args = cx.args()?;

        // advance own state
        *self = cx;
        Some(Ast::Macro(name, args))
    }

    /// Parse a block macro
    fn block(&mut self) -> Option<Ast> {
        let mut cx = self.clone();
        cx.pat("@begin")?;

        cx.pred(char::is_whitespace)?;

        // macro name
        let name = cx.name()?.to_string();

        // args
        cx.pred(char::is_whitespace)?;
        let args = cx.args()?;

        // read until the @end
        // escape only escapes the @end here
        let mut content = String::new();
        while !cx.input.ends_with("@end") {
            // up till the escape or @ character
            if let Some(x) = cx.until_pred(|x| "@\\".contains(x)) {
                content.push_str(x);
            }

            // consume an \@end
            if let Some(x) = cx.pat("\\@end") {
                content.push_str(x);
            } else if let Some(x) = cx.pat("\\") {
                // or a normal escape
                content.push_str(x);
            } else if cx.input.starts_with("@end") {
                // escaped versions have been consumed
                // now we have the real one
                break;
            } else {
                // otherwise, a normal @
                cx.pat("@")?;
                content.push('@');
            }
        }

        // advance own state
        *self = cx;
        Some(Ast::Block(name, args, content))
    }

    /// take until we match a predicate
    fn until_pred<F: FnMut(char) -> bool>(&mut self, pred: F) -> Option<&'a str> {
        // where it is
        let mid = self.input.find(pred)?;

        // split
        let (content, rest) = self.input.split_at(mid);

        // advance cursor
        self.input = rest;
        self.col = content
            .rsplit_once('\n')
            .map(|x| 1 + x.1.width())
            .unwrap_or(self.col + content.width());
        self.row += content.chars().filter(|x| *x == '\n').count();

        Some(content)
    }

    /// Take while we match a predicate
    fn pred<F: FnMut(char) -> bool>(&mut self, mut pred: F) -> Option<&'a str> {
        self.until_pred(|x| !pred(x))
    }

    /// take until we find the specific pattern
    fn until_pat(&mut self, pat: &str) -> Option<&'a str> {
        // where it is
        let mid = self.input.find(pat)?;

        // split
        let (content, rest) = self.input.split_at(mid);

        // advance cursor
        self.input = rest;
        self.col = content
            .rsplit_once('\n')
            .map(|x| 1 + x.1.width())
            .unwrap_or(self.col + content.width());
        self.row += content.chars().filter(|x| *x == '\n').count();

        Some(content)
    }

    /// take the pattern at the start
    fn pat<'b>(&mut self, pat: &'b str) -> Option<&'b str> {
        let rest = self.input.strip_prefix(pat)?;

        // advance cursor
        self.input = rest;

        self.col = pat
            .rsplit_once('\n')
            .map(|x| 1 + x.1.width())
            .unwrap_or(self.col + pat.width());
        self.row += pat.chars().filter(|x| *x == '\n').count();

        Some(pat)
    }
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
