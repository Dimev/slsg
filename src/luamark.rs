use std::collections::BTreeMap;

use crossterm::style::Stylize;
use mlua::{ErrorContext, IntoLua, Lua, MultiValue, ObjectLike, Result, Table, Value};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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

impl Luamark {
    pub fn parse(lmk: &str) -> Result<Self> {
        // parser state
        let mut cx = Parser {
            input: lmk,
            row: 1,
            col: 1,
        };

        // meta flags
        let mut meta = BTreeMap::new();

        // document
        let mut document = Vec::new();

        // parse
        while !cx.input.is_empty() {
            // skip comment
            cx.comment();

            // skip empty
            cx.take_pred(char::is_whitespace);

            // try parsing a meta
            if let Some((key, value)) = cx.meta() {
                println!("@{} = {}", key, value);
                // add to the map
                meta.insert(key, value.trim().to_string());
            }

            if let Some(x) = cx.heading() {
                // print the heading
                println!("= {:?}", x);
                document.push(x);
            }

            // paragraph, stop on headings as we parse those here
            if let Some(x) = cx.paragraph("=") {
                println!("Paragraph! {:?}", x);
                // TODO: ignore if empty
                document.push(x);
            }
        }

        Ok(Luamark { meta, document })
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

// TODO: only escape needed characters?

impl<'a> Parser<'a> {
    /// parse a comment
    /// %.*\n
    fn comment(&mut self) -> Option<()> {
        self.pat("%")?;
        self.until_pat("\n")?;
        Some(())
    }

    /// Parse escaped content
    /// \anything-non-whitespace
    fn escaped(&mut self) -> Option<&str> {
        self.pat("\\")?;
        Some(self.take_until_pred(char::is_whitespace))
    }

    /// Parse a heading
    fn heading(&mut self) -> Option<Ast> {
        self.pat("=")?;

        // count the remaining
        let mut depth = 1u8;
        while self.pat("=").is_some() {
            depth = depth.saturating_add(1);
        }

        // read until the newline or comment start
        let content = self.take_until_pred(|x| x == '%' || x == '\n');
        Some(Ast::Heading(depth, content.trim().to_string()))
    }

    /// TODO: math

    /// Parse math
    fn math(&mut self) -> Option<Ast> {
        self.pat("$")?;
        let block = self.pat("$").is_some();

        todo!();

        self.pat("$")?;
        if block {
            self.pat("$")?;
        }
        Some(Ast::Math(String::new(), false))
    }

    /// Parse italic
    fn italic(&mut self) -> Option<Ast> {
        self.pat("_")?;

        let mut content = Vec::new();
        while !self.input.starts_with("_") && !self.input.is_empty() {
            // until the next special
            let text = self.take_until_pred(|x| "%_*`\\".contains(x));
            content.push(Ast::Text(text.to_string()));

            // skip any comment
            self.comment();

            // add any escaped text
            if let Some(x) = self.escaped() {
                content.push(Ast::Text(x.to_string()));
            }

            // add any bold text
            if let Some(x) = self.bold() {
                content.push(x);
            }

            // add any monospace text
            if let Some(x) = self.monospace() {
                content.push(x);
            }
        }

        self.pat("_")?;
        Some(Ast::Italic(content))
    }

    // Parse bold
    fn bold(&mut self) -> Option<Ast> {
        self.pat("*")?;

        let mut content = Vec::new();
        while !self.input.starts_with("*") && !self.input.is_empty() {
            // until the next special
            let text = self.take_until_pred(|x| "%_*`\\".contains(x));
            content.push(Ast::Text(text.to_string()));

            // skip any comment
            self.comment();

            // add any escaped text
            if let Some(x) = self.escaped() {
                content.push(Ast::Text(x.to_string()));
            }

            // add any italic text
            if let Some(x) = self.italic() {
                content.push(x);
            }

            // add any monospace text
            if let Some(x) = self.monospace() {
                content.push(x);
            }
        }

        self.pat("*")?;
        Some(Ast::Bold(content))
    }

    // Parse monospace
    fn monospace(&mut self) -> Option<Ast> {
        self.pat("`")?;

        let mut content = Vec::new();
        while !self.input.starts_with("`") && !self.input.is_empty() {
            // until the next special
            let text = self.take_until_pred(|x| "%_*`\\".contains(x));
            content.push(Ast::Text(text.to_string()));

            // skip any comment
            self.comment();

            // add any escaped text
            if let Some(x) = self.escaped() {
                content.push(Ast::Text(x.to_string()));
            }

            // add any italic text
            if let Some(x) = self.italic() {
                content.push(x);
            }

            // add any bold text
            if let Some(x) = self.bold() {
                content.push(x);
            }
        }

        self.pat("`")?;
        Some(Ast::Mono(content))
    }

    /// Parse a paragraph
    fn paragraph(&mut self, also_stop: &str) -> Option<Ast> {
        let mut content = Vec::new();

        // while we have not hit an empty line, meta, any of the stop characters, or end of input:
        while !self.clone().empty_line().is_some()
            && !self.clone().meta().is_some()
            && !also_stop.chars().any(|x| self.input.starts_with(x))
            && !self.input.is_empty()
        {
            // parse comments
            self.comment();

            // Parse up to any of the special characters
            // ensure we don't skip any empty lines here
            let mut text = String::new();
            while !self
                .input
                .starts_with(|x| "%_*`@\\$".contains(x) || also_stop.contains(x))
                && !self.input.is_empty()
            {
                // any non-escaped non-newline text
                let x = self.take_until_pred(|x| "%_*`@\\$\n".contains(x) || also_stop.contains(x));

                // add it
                for word in x.split_whitespace() {
                    // add a whitespace if we don't have one already
                    // TODO: ensure this works correctly
                    if !text
                        .chars()
                        .rev()
                        .next()
                        .map(char::is_whitespace)
                        .unwrap_or(true)
                    {
                        text.push(' ');
                    }

                    // add the word
                    text.push_str(word.trim());
                }

                // stop if we hit the empty line
                if self.clone().empty_line().is_some() {
                    break;
                }

                // else, skip empty
                if !self.take_pred(char::is_whitespace).is_empty()
                    && !text
                        .chars()
                        .rev()
                        .next()
                        .map(char::is_whitespace)
                        .unwrap_or(true)
                {
                    text.push(' ');
                }
            }
            if !text.is_empty() {
                content.push(Ast::Text(text));
            }

            // parse escaped
            if let Some(x) = self.escaped() {
                content.push(Ast::Text(x.to_string()));
            }

            // parse math
            if let Some(x) = self.math() {
                content.push(x);
            }
            
            // parse italic
            if let Some(x) = self.italic() {
                content.push(x);
            }

            // parse bold
            if let Some(x) = self.bold() {
                content.push(x);
            }

            // parse monospace
            if let Some(x) = self.monospace() {
                content.push(x);
            }

            // parse macro
            if let Some(x) = self.call() {
                content.push(x);
            }

            // parse block macro
            if let Some(x) = self.block() {
                content.push(x);
            }
        }

        Some(Ast::Paragraph(content))
    }

    /// Parse a macro name
    fn name(&mut self) -> &str {
        // alphanum or _
        self.take_pred(|x| x.is_alphanumeric() || x == '_')
    }

    /// Parse a meta variable
    fn meta(&mut self) -> Option<(String, String)> {
        // @ and name
        let mut cx = self.clone();
        cx.pat("@")?;

        // meta name
        let name = cx.name().to_string();

        // trim any whitespace
        cx.take_pred(char::is_whitespace);
        cx.pat("=")?;
        cx.take_pred(char::is_whitespace);

        // split based on how the string is escaped
        // content
        let mut content = String::new();
        if cx.pat("\"").is_some() {
            while !cx.input.starts_with('"') && !self.input.is_empty() {
                // text
                let x = cx.take_until_pred(|x| "\"\\".contains(x));
                content.push_str(x);

                // escape
                if let Some(x) = cx.escaped() {
                    content.push_str(x);
                }
            }

            // close
            cx.pat("\"")?;
        } else if cx.pat("'").is_some() {
            while !cx.input.starts_with('\'') && !self.input.is_empty() {
                // text
                let x = cx.take_until_pred(|x| "'\\".contains(x));
                content.push_str(x);

                // escape
                if let Some(x) = cx.escaped() {
                    content.push_str(x);
                }
            }

            // close
            cx.pat("'")?;
        } else {
            while !cx.input.starts_with(['\n', '%']) && !self.input.is_empty() {
                // text
                let x = cx.take_until_pred(|x| "%\n\\".contains(x));
                content.push_str(x);

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
    fn args(&mut self) -> Option<Vec<Vec<Ast>>> {
        let mut cx = self.clone();
        // TODO: block (and or {) for single argument
        cx.pat("(")?;

        // while there's no closing ), parse arguments
        let mut args = Vec::new();
        while !cx.input.starts_with(")") && !cx.input.is_empty() {
            // parse paragraphs and headings
            let mut content = Vec::new();

            while !cx.input.starts_with(";") && !cx.input.starts_with(")") && !cx.input.is_empty() {
                // skip comment
                cx.comment();

                // skip empty
                cx.take_pred(char::is_whitespace);

                // parse a heading
                if let Some(x) = cx.heading() {
                    content.push(x);
                }

                // paragraph, stop on headings as we parse those here
                if let Some(x) = cx.paragraph("=;)") {
                    // TODO: ignore if empty
                    content.push(x);
                }
            }

            // push the argument
            args.push(content);

            // close
            cx.pat(";");
        }

        // close
        cx.pat(")")?;

        // advance own state
        *self = cx;
        Some(args)
    }

    /// Parse a macro call
    fn call(&mut self) -> Option<Ast> {
        let mut cx = self.clone();
        cx.pat("@")?;

        // macro name
        let name = cx.name().to_string();

        // arguments
        cx.take_pred(char::is_whitespace);
        let args = cx.args()?;

        // advance own state
        *self = cx;
        Some(Ast::Macro(name, args))
    }

    /// Parse a block macro
    fn block(&mut self) -> Option<Ast> {
        let mut cx = self.clone();
        cx.pat("@begin")?;

        cx.take_pred(char::is_whitespace);

        // macro name
        let name = cx.name().to_string();

        // args
        cx.take_pred(char::is_whitespace);
        let args = cx.args()?;

        // read until the @end
        // escape only escapes the @end here
        let mut content = String::new();
        while !cx.input.ends_with("@end") && !self.input.is_empty() {
            // up till the escape or @ character
            let x = cx.take_until_pred(|x| "@\\".contains(x));
            content.push_str(x);

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

    /// Parse an empty line, up to the next non-whitespace character
    fn empty_line(&mut self) -> Option<&str> {
        Some(self.take_pred(char::is_whitespace))
            // empty line is a newline, followed by whitespace and another newline
            .filter(|x| x.chars().filter(|x| *x == '\n').count() >= 2)
    }

    /// take until we match a predicate
    fn take_until_pred<F: FnMut(char) -> bool>(&mut self, pred: F) -> &'a str {
        // where it is
        // consume the entire input if we can't find it
        let mid = self.input.find(pred).unwrap_or(self.input.len());

        // split
        let (content, rest) = self.input.split_at(mid);

        // advance cursor
        self.input = rest;
        self.col = content
            .rsplit_once('\n')
            .map(|x| 1 + x.1.width())
            .unwrap_or(self.col + content.width());
        self.row += content.chars().filter(|x| *x == '\n').count();

        content
    }

    /// Take while we match a predicate
    fn take_pred<F: FnMut(char) -> bool>(&mut self, mut pred: F) -> &'a str {
        self.take_until_pred(|x| !pred(x))
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
