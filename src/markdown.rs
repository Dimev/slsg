use std::{collections::VecDeque, iter::repeat};

use latex2mathml::latex_to_mathml;
use mlua::{
    ErrorContext, ExternalResult, Lua, Result,
    Value::{self, Nil},
    chunk,
};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html::push_html};
use relative_path::RelativePath;
use unicode_width::UnicodeWidthStr;

use crate::path::{DoubleFileExt, HtmlToIndex};

/// Parse minimark to html
pub(crate) fn markdown(
    lua: &Lua,
    content: &str,
    name: &RelativePath,
) -> Result<(String, VecDeque<Value>)> {
    // translated name
    let path = name
        .with_extension("html")
        .without_double_ext()
        .ok_or(mlua::Error::external(format!(
            "Expected path `{name}` to have a second `.lua` or `.fnl` extension"
        )))?;
    let path = path.html_to_index().unwrap_or(path);

    // set up environment
    // current file
    lua.globals().set("curfile", name.as_str())?;

    // current directory
    lua.globals()
        .set("curdir", name.parent().map(RelativePath::as_str))?;

    // where this file will be emitted to
    lua.globals().set("curtarget", path.as_str())?;

    // directory this file will be emitted to
    lua.globals()
        .set("curtargetdir", path.parent().map(RelativePath::as_str))?;

    // events to parse
    let mut events = Vec::new();

    // inline html we might encounter
    let mut html = None;

    // inline code we might encounter
    let mut code = None;
    let mut lang = None;
    let mut pos = None;

    // functions to run after processing
    let mut functions = VecDeque::new();

    // parse
    for (event, offset) in Parser::new_ext(
        content,
        Options::ENABLE_MATH
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_SMART_PUNCTUATION
            | Options::ENABLE_HEADING_ATTRIBUTES,
    )
    .into_offset_iter()
    {
        match event {
            Event::InlineHtml(html) if html.starts_with("<?") && name.has_double_ext("lua") => {
                let position = offset.start;
                let lines = content[..position].chars().filter(|x| *x == '\n').count();
                let width = content[..position]
                    .rsplit_once('\n')
                    .map(|x| x.1)
                    .unwrap_or(&content[..position])
                    .width();

                // strip
                let code = String::from_iter(
                    repeat('\n')
                        .take(lines)
                        .chain(repeat(' ').take(width + 3))
                        .chain(
                            html.strip_prefix("<?")
                                .unwrap()
                                .strip_suffix("?>")
                                .unwrap()
                                .chars(),
                        ),
                );

                // run code
                let result: Value = lua.load(code).set_name(format!("@{name}")).eval()?;

                // string, numbers or booleans can be embedded directly
                if result.is_string() || result.is_number() || result.is_integer() {
                    events.push(Event::Html(
                        lua.coerce_string(result)?
                            .unwrap()
                            .to_str()?
                            .to_owned()
                            .into(),
                    ));
                }
                // boolean
                else if let Some(b) = result.as_boolean() {
                    events.push(Event::Html(if b { "true" } else { "false" }.into()));
                }
                // functions and tables can be called, so run them later
                else if result.is_function() || result.is_table() {
                    functions.push_back(result.clone());
                }
            }
            Event::InlineHtml(html) if html.starts_with("<?") && name.has_double_ext("fnl") => {
                let position = offset.start;
                let lines = content[..position].chars().filter(|x| *x == '\n').count();
                let width = content[..position]
                    .rsplit_once('\n')
                    .map(|x| x.1)
                    .unwrap_or(&content[..position])
                    .width();

                // strip
                let code = String::from_iter(
                    repeat('\n')
                        .take(lines)
                        .chain(repeat(' ').take(width + 3))
                        .chain(
                            html.strip_prefix("<?")
                                .unwrap()
                                .strip_suffix("?>")
                                .unwrap()
                                .chars(),
                        ),
                );

                // run code
                let name = name.as_str();
                let result: Value = lua
                    .load(chunk!(require("fennel").eval($code, { ["error-pinpoint"] = false, filename = $name.as_str })))
                    .set_name(format!("@{name}"))
                    .eval()?;

                // string, numbers or booleans can be embedded directly
                if result.is_string() || result.is_number() || result.is_integer() {
                    events.push(Event::Html(
                        lua.coerce_string(result)?
                            .unwrap()
                            .to_str()?
                            .to_owned()
                            .into(),
                    ));
                }
                // boolean
                else if let Some(b) = result.as_boolean() {
                    events.push(Event::Html(if b { "true" } else { "false" }.into()));
                }
                // functions and tables can be called, so run them later
                else if result.is_function() || result.is_table() {
                    functions.push_back(result.clone());
                }
            }
            // block html, parse
            Event::Start(Tag::HtmlBlock) => {
                html = Some(String::new());
                pos = Some(offset.start);
                events.push(Event::Start(Tag::HtmlBlock))
            }
            Event::Html(x) => html.as_mut().unwrap().push_str(&x),
            // run lua
            Event::End(TagEnd::HtmlBlock)
                if html.as_ref().map(|x| x.starts_with("<?")).unwrap_or(false)
                    && name.has_double_ext("lua") =>
            {
                let position = pos.take().unwrap();
                let lines = content[..position].chars().filter(|x| *x == '\n').count();
                let width = content[..position]
                    .rsplit_once('\n')
                    .map(|x| x.1)
                    .unwrap_or(&content[..position])
                    .width();

                // strip
                let code = String::from_iter(
                    repeat('\n')
                        .take(lines)
                        .chain(repeat(' ').take(width + 3))
                        .chain(
                            html.as_mut()
                                .take()
                                .unwrap()
                                .strip_prefix("<?")
                                .unwrap()
                                .trim_end()
                                .strip_suffix("?>")
                                .unwrap()
                                .chars(),
                        ),
                );

                // run code
                let result: Value = lua.load(code).set_name(format!("@{name}")).eval()?;

                // string, numbers or booleans can be embedded directly
                if result.is_string() || result.is_number() || result.is_integer() {
                    events.push(Event::Html(
                        lua.coerce_string(result)?
                            .unwrap()
                            .to_str()?
                            .to_owned()
                            .into(),
                    ));
                }
                // boolean
                else if let Some(b) = result.as_boolean() {
                    events.push(Event::Html(if b { "true" } else { "false" }.into()));
                }
                // functions and tables can be called, so run them later
                else if result.is_function() || result.is_table() {
                    functions.push_back(result.clone());
                }

                events.push(Event::End(TagEnd::HtmlBlock))
            }
            // run fennel
            Event::End(TagEnd::HtmlBlock)
                if html.as_ref().map(|x| x.starts_with("<?")).unwrap_or(false)
                    && name.has_double_ext("fnl") =>
            {
                let position = pos.take().unwrap();
                let lines = content[..position].chars().filter(|x| *x == '\n').count();
                let width = content[..position]
                    .rsplit_once('\n')
                    .map(|x| x.1)
                    .unwrap_or(&content[..position])
                    .width();

                // strip
                let code = String::from_iter(
                    repeat('\n')
                        .take(lines)
                        .chain(repeat(' ').take(width + 3))
                        .chain(
                            html.as_mut()
                                .take()
                                .unwrap()
                                .strip_prefix("<?")
                                .unwrap()
                                .trim_end()
                                .strip_suffix("?>")
                                .unwrap()
                                .chars(),
                        ),
                );

                // run code
                let name = name.as_str();
                let result: Value = lua
                    .load(chunk!(require("fennel").eval($code, { ["error-pinpoint"] = false, filename = $name })))
                    .set_name(format!("@{name}"))
                    .eval()?;

                // string, numbers or booleans can be embedded directly
                if result.is_string() || result.is_number() || result.is_integer() {
                    events.push(Event::Html(
                        lua.coerce_string(result)?
                            .unwrap()
                            .to_str()?
                            .to_owned()
                            .into(),
                    ));
                }
                // boolean
                else if let Some(b) = result.as_boolean() {
                    events.push(Event::Html(if b { "true" } else { "false" }.into()));
                }
                // functions and tables can be called, so run them later
                else if result.is_function() || result.is_table() {
                    functions.push_back(result.clone());
                }

                events.push(Event::End(TagEnd::HtmlBlock))
            }
            // else, not
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
                let lang = lang.take().unwrap_or("".into());
                let (lang, prefix) = lang
                    .split_once(" ")
                    .map(|x| (x.0, Some(x.1).filter(|x| !x.is_empty())))
                    .unwrap_or((&lang, None));
                let code = code.take().unwrap();
                // easiest to just use the lua function
                // also works nicely in case it is overridden
                let highlighted: String = lua
                    .load(chunk! {highlight($lang, $code, $prefix)})
                    .set_name(format!("@{name}"))
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

    // unset environment
    lua.globals().set("curfile", Nil)?;
    lua.globals().set("curdir", Nil)?;
    lua.globals().set("curtarget", Nil)?;
    lua.globals().set("curtargetdir", Nil)?;

    Ok((out, functions))
}
