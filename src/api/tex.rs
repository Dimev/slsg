use mlua::{Error, Lua, Table};

pub(crate) enum Tex {
    /// Entire document
    Document(Vec<Tex>),

    /// Paragraph, text seperated by one empty line
    Paragraph(String, Vec<Tex>),

    /// Block of nodes (\begin{x} ... \end(x)
    Block(String, String, Vec<String>, Vec<Tex>),

    /// Single command (\x{a, b, c})
    Command(String, Vec<String>),

    /// Raw text
    Text(String),

    /// Math, as raw text inside ($$ ... $$)
    Math(String),

    /// Inline math, as raw text inside ($ ... $)
    InlineMath(String),

    /// Verbatim, (\verb| ... |, where | can be any character)
    Verbatim(String),

    /// Ampersand &
    Ampersand,
}

impl Tex {
    pub(crate) fn to_lua<'a>(&self, lua: &'a Lua) -> Result<Table<'a>, Error> {
        lua.create_table()
    }

    // TODO: open, close, newline, push
}

pub(crate) enum TexError {}

pub(crate) fn parse_tex(mut tex: &str) -> Result<Tex, TexError> {
    // current tex
    let mut res = Tex::Document(Vec::new());
    
    // parse the input
    while !tex.is_empty() {
        // comment
        if tex.starts_with('%') {
            // skip to newline
            if let Some(idx) = tex.find('\n') {
                tex = &tex[idx..];
            } else {
                tex = "";
            }
        }
        // escaped command 
        if tex.starts_with("\\\\") {
        
        }

        // verbatim

        // open
        
        // command
        if tex.starts_with('\\') {
            
        }

        // inline math

        // math
        
    }
    
    Ok(res)
}
