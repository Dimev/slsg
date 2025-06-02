use std::collections::VecDeque;

use latex2mathml::latex_to_mathml;
use mlua::{ErrorContext, ExternalResult, Lua, Result, Value, chunk};
use pulldown_cmark::{
    CodeBlockKind, Event, Options, Parser, Tag, TagEnd,
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
    let mut code = None;
    let mut lang = None;

    // functions to run after processing
    let mut functions = VecDeque::new();

    // parse
    // TODO: correct positions with errors
    // TODO: only do lua if apply templating is enabled
    for (event, offset) in Parser::new_ext(content, Options::ENABLE_MATH).into_offset_iter() {
        match event {
            // inline html, parse and run TODO check if we need to run it
            Event::InlineHtml(html) if html.starts_with("<?lua") => {
                // strip
                let code = html
                    .strip_prefix("<?lua")
                    .unwrap()
                    .strip_suffix("?>")
                    .unwrap();

                // run code
                let result: Value = lua.load(code).set_name(format!("={name}")).eval()?;

                // string, numbers or booleans can be embedded directly
                if result.is_string() || result.is_number() || result.is_boolean() {
                    events.push(Event::InlineHtml(result.to_string()?.into()));
                }

                // functions and tables can be called, so run them later
                if result.is_function() || result.is_table() {
                    functions.push_back(result.clone());
                }
            }
            Event::InlineHtml(html) if html.starts_with("<?fnl") => {
                // strip
                let code = html
                    .strip_prefix("<?fnl")
                    .unwrap()
                    .strip_suffix("?>")
                    .unwrap();

                // run code
                let result: Value = lua
                .load(chunk!(require("fennel").eval($code, { ["error-pinpoint"] = false, filename = $name })))
                .set_name(format!("={name}"))
                .eval()?;

                // string, numbers or booleans can be embedded directly
                if result.is_string() || result.is_number() || result.is_boolean() {
                    events.push(Event::InlineHtml(result.to_string()?.into()));
                }

                // functions and tables can be called, so run them later
                if result.is_function() || result.is_table() {
                    functions.push_back(result.clone());
                }
            }
            // block html, parse
            Event::Start(Tag::HtmlBlock) => {
                html = Some(String::new());
                events.push(Event::Start(Tag::HtmlBlock))
            }
            Event::Html(x) => html.as_mut().unwrap().push_str(&x),
            // run lua
            Event::End(TagEnd::HtmlBlock)
                if html
                    .as_ref()
                    .map(|x| x.starts_with("<?lua"))
                    .unwrap_or(false) =>
            {
                // strip
                let code = html
                    .as_mut()
                    .take()
                    .unwrap()
                    .strip_prefix("<?lua")
                    .unwrap()
                    .trim_end() // needed, as the end is only on a paragraph end
                    .strip_suffix("?>")
                    .unwrap();

                // run code
                let result: Value = lua.load(code).set_name(format!("={name}")).eval()?;

                // string, numbers or booleans can be embedded directly
                if result.is_string() || result.is_number() || result.is_boolean() {
                    events.push(Event::Html(result.to_string()?.into()));
                }

                // functions and tables can be called, so run them later
                if result.is_function() || result.is_table() {
                    functions.push_back(result.clone());
                }

                events.push(Event::End(TagEnd::HtmlBlock))
            }
            // run fennel
            Event::End(TagEnd::HtmlBlock)
                if html
                    .as_ref()
                    .map(|x| x.starts_with("<?fnl"))
                    .unwrap_or(false) =>
            {
                // strip
                let code = html
                    .as_mut()
                    .take()
                    .unwrap()
                    .strip_prefix("<?fnl")
                    .unwrap()
                    .trim_end() // needed, as the end is only on a paragraph end
                    .strip_suffix("?>")
                    .unwrap();
                
                // run code
                let result: Value = lua
                .load(chunk!(require("fennel").eval($code, { ["error-pinpoint"] = false, filename = $name })))
                .set_name(format!("={name}"))
                .eval()?;

                // string, numbers or booleans can be embedded directly
                if result.is_string() || result.is_number() || result.is_boolean() {
                    events.push(Event::Html(result.to_string()?.into()));
                }

                // functions and tables can be called, so run them later
                if result.is_function() || result.is_table() {
                    functions.push_back(result.clone());
                }

                events.push(Event::End(TagEnd::HtmlBlock))
            } // else, not
            Event::End(TagEnd::HtmlBlock) => {
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
            Event::DisplayMath(mathml) => {
                events.push(Event::Start(Tag::HtmlBlock));
                events.push(Event::Html(
                    latex_to_mathml(&mathml, latex2mathml::DisplayStyle::Block)
                        .into_lua_err()
                        .with_context(|_| format!("{name}: Failed to compile math"))?
                        .into(),
                ));
                events.push(Event::End(TagEnd::HtmlBlock));
            }
            // code, highlight
            Event::Start(Tag::CodeBlock(l)) => {
                lang = match l {
                    CodeBlockKind::Indented => None,
                    CodeBlockKind::Fenced(l) => Some(l.to_string()),
                };
                code = Some(String::new());
            }
            Event::Text(x) if code.is_some() => code.as_mut().unwrap().push_str(&x),
            Event::End(TagEnd::CodeBlock) => {
                // highlight
                let lang = lang.take().unwrap_or("plain".into());
                let (lang, prefix) = lang
                    .split_once(" ")
                    .map(|x| (x.0, Some(x.1).filter(|x| !x.is_empty())))
                    .unwrap_or((&lang, None));
                let code = code.take().unwrap();
                // easiest to just use the lua function
                // also works nicely in case it is overridden
                let highlighted: String = lua
                    .load(chunk! {highlight($lang, $code, $prefix)})
                    .set_name(format!("={name}"))
                    .eval()?;

                //
                events.push(Event::Start(Tag::HtmlBlock));
                events.push(Event::Html(
                    format!("<pre><code>{}</code></pre>", highlighted).into(),
                ));
                events.push(Event::End(TagEnd::HtmlBlock));
            }
            // rest, just push
            e => events.push(e),
        }
    }

    // push out all events
    let mut out = String::with_capacity(content.len());
    push_html(&mut out, events.into_iter());
    Ok((out, functions))
}
