use std::collections::VecDeque;

use mlua::{Lua, Result, Value};
use pulldown_cmark::{Parser, html};

use crate::{Config, templates::template};

enum Modifier {
    Bold,
    Italic,
    Mono,
}

/// Parse minimark to html
pub(crate) fn markdown(
    lua: &Lua,
    content: &str,
    name: &str,
    conf: &Config,
    apply_template: bool,
) -> Result<(String, VecDeque<Value>)> {
    // apply template first
    let (res, functions) = if apply_template {
        template(lua, content, name, conf)?
    } else {
        (content.to_string(), VecDeque::new())
    };

    // convert to markdown
    let mut out = String::with_capacity(res.len());
    for event in Parser::new(&res) {
        match event {
            x => html::push_html(&mut out, Some(x).into_iter()),
        }
    }

    Ok((out, functions))
}
