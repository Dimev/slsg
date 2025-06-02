use std::collections::VecDeque;

use latex2mathml::latex_to_mathml;
use mlua::{ErrorContext, ExternalResult, Lua, Result, Value};
use pulldown_cmark::{
    Event, Options, Parser, Tag, TagEnd,
    html::{self, push_html},
};

use crate::Config;

/// Parse minimark to html
pub(crate) fn markdown(
    lua: &Lua,
    content: &str,
    name: &str,
    conf: &Config,
    apply_template: bool,
) -> Result<(String, VecDeque<Value>)> {
    // events to parse
    let mut events = Vec::new();

    // inline html we might encounter
    let mut html = None;

    // inline code we might encounter
    let mut code = String::new();

    // functions to run after processing
    let mut functions = VecDeque::new();

    // parse
    for (event, offset) in Parser::new_ext(content, Options::ENABLE_MATH).into_offset_iter() {
        match event {
            // inline html, parse and run TODO check if we need to run it
            Event::InlineHtml(html) => events.push(Event::InlineHtml(html)),
            // block html, parse
            Event::Start(Tag::HtmlBlock) => {
                html = Some(String::new());
                events.push(Event::Start(Tag::HtmlBlock))
            }
            Event::Html(x) => html.as_mut().unwrap().push_str(&x),
            Event::End(TagEnd::HtmlBlock) => {
                // TODO: check if it's code we need to run
                events.push(Event::Html(html.take().unwrap().into()));
                events.push(Event::End(TagEnd::HtmlBlock))
            }
            // inline math, compile
            Event::InlineMath(mathml) => events.push(Event::InlineHtml(
                latex_to_mathml(&mathml, latex2mathml::DisplayStyle::Inline)
                    .into_lua_err()
                    .with_context(|_| format!("{name}: Failed to compile math"))?
                    .into(),
            )),
            // display math, compile
            Event::DisplayMath(mathml) => events.push(Event::InlineHtml(
                latex_to_mathml(&mathml, latex2mathml::DisplayStyle::Block)
                    .into_lua_err()
                    .with_context(|_| format!("{name}: Failed to compile math"))?
                    .into(),
            )),
            // code, highlight
            // TODO
            Event::Start(Tag::CodeBlock(lang)) => events.push(Event::Start(Tag::CodeBlock(lang))),
            Event::End(TagEnd::CodeBlock) => events.push(Event::End(TagEnd::CodeBlock)),
            // rest, just push
            e => events.push(e),
        }
    }

    // push out all events
    let mut out = String::with_capacity(content.len());
    push_html(&mut out, events.into_iter());
    Ok((out, functions))
}
