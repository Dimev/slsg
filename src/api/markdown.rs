use markdown::{
    mdast::{AlignKind, Node, ReferenceKind},
    to_html, to_mdast, ParseOptions,
};
use mlua::{Error, Lua, LuaSerdeExt, Table, UserData, UserDataFields};

/// Parsed markdown
pub(crate) struct Markdown {
    /// raw file content
    raw: String,
}

impl Markdown {
    pub(crate) fn from_str(string: &str) -> Self {
        Self { raw: string.into() }
    }
}

impl UserData for Markdown {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("raw", |_, this| Ok(this.raw.clone()));
        fields.add_field_method_get("html", |_, this| {
            let html = to_html(&this.raw);
            Ok(html)
        });
        fields.add_field_method_get("ast", |lua, this| {
            let md =
                to_mdast(&this.raw, &ParseOptions::default()).map_err(|x| Error::external(x))?;
            let table = ast_to_lua(lua, md)?;
            Ok(table)
        });
    }
}

// Convert a markdown ast to lua tables
fn ast_to_lua(lua: &Lua, ast: Node) -> Result<Table, Error> {
    let table = lua.create_table()?;
    match ast {
        Node::Root(x) => {
            table.set("type", "root")?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
        }
        Node::BlockQuote(x) => {
            table.set("type", "blockquote")?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
        }
        Node::FootnoteDefinition(x) => {
            table.set("type", "footnotedefinition")?;
            table.set("identifier", x.identifier)?;
            table.set("label", x.label)?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
        }
        /*Node::MdxJsxFlowElement(x) => {
            table.set("type", "mdxjsxflowelement")?;
            table.set("name", x.name)?;
            table.set("attributes", x.attributes)?;
        }*/
        Node::List(x) => {
            table.set("type", "list")?;
            table.set("ordered", x.ordered)?;
            table.set("start", x.start)?;
            table.set("spread", x.spread)?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
        }
        Node::Toml(x) => {
            table.set("type", "toml")?;
            let toml: toml::Value =
                toml::from_str(&x.value).map_err(|x| mlua::Error::external(x))?;
            table.set("data", lua.to_value(&toml)?)?;
            table.set("value", x.value)?;
        }
        Node::Yaml(x) => {
            table.set("type", "yaml")?;
            let yaml: serde_yaml::Value =
                serde_yaml::from_str(&x.value).map_err(|x| mlua::Error::external(x))?;
            table.set("data", lua.to_value(&yaml)?)?;
            table.set("value", x.value)?;
        }
        Node::Break(_) => {
            table.set("type", "break")?;
        }
        Node::InlineCode(x) => {
            table.set("type", "inlinecode")?;
            table.set("value", x.value)?;
        }
        Node::InlineMath(x) => {
            table.set("type", "inlinemath")?;
            table.set("value", x.value)?;
        }
        Node::Delete(x) => {
            table.set("type", "delete")?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
        }
        Node::Emphasis(x) => {
            table.set("type", "emphasis")?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
        }
        /* Node::MdxTextExpression */
        Node::FootnoteReference(x) => {
            table.set("type", "footnotereference")?;
            table.set("identifier", x.identifier)?;
            table.set("label", x.label)?;
        }
        Node::Html(x) => {
            table.set("type", "html")?;
            table.set("value", x.value)?;
        }
        Node::Image(x) => {
            table.set("type", "image")?;
            table.set("alt", x.alt)?;
            table.set("url", x.url)?;
            table.set("title", x.title)?;
        }
        Node::ImageReference(x) => {
            table.set("type", "imagereference")?;
            table.set("alt", x.alt)?;
            table.set("identifier", x.identifier)?;
            table.set("label", x.label)?;
            table.set(
                "referencekind",
                match x.reference_kind {
                    ReferenceKind::Shortcut => "shortcut",
                    ReferenceKind::Collapsed => "collapsed",
                    ReferenceKind::Full => "full",
                },
            )?;
        }
        /* Node::MdxJsxTextElement */
        Node::Link(x) => {
            table.set("type", "link")?;
            table.set("url", x.url)?;
            table.set("title", x.title)?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
        }
        Node::LinkReference(x) => {
            table.set("type", "linkreference")?;
            table.set("identifier", x.identifier)?;
            table.set("label", x.label)?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
            table.set(
                "referencekind",
                match x.reference_kind {
                    ReferenceKind::Shortcut => "shortcut",
                    ReferenceKind::Collapsed => "collapsed",
                    ReferenceKind::Full => "full",
                },
            )?;
        }
        Node::Strong(x) => {
            table.set("type", "strong")?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
        }
        Node::Text(x) => {
            table.set("type", "text")?;
            table.set("value", x.value)?;
        }
        Node::Code(x) => {
            table.set("type", "code")?;
            table.set("value", x.value)?;
            table.set("language", x.lang)?;
            table.set("meta", x.meta)?;
        }
        Node::Math(x) => {
            table.set("type", "math")?;
            table.set("vale", x.value)?;
            table.set("meta", x.meta)?;
        }
        /*Node::MdxFlowExpression()*/
        Node::Heading(x) => {
            table.set("type", "heading")?;
            table.set("depth", x.depth)?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
        }
        Node::Table(x) => {
            table.set("type", "table")?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
            table.set(
                "align",
                x.align
                    .into_iter()
                    .map(|x| match x {
                        AlignKind::Left => "left",
                        AlignKind::Right => "right",
                        AlignKind::Center => "center",
                        AlignKind::None => "none",
                    })
                    .collect::<Vec<&str>>(),
            )?;
        }
        Node::ThematicBreak(_) => {
            table.set("type", "thematicbreak")?;
        }
        Node::TableRow(x) => {
            table.set("type", "tablerow")?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
        }
        Node::TableCell(x) => {
            table.set("type", "tablecell")?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
        }
        Node::ListItem(x) => {
            table.set("type", "listitem")?;
            table.set("spread", x.spread)?;
            table.set("checked", x.checked)?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
        }
        Node::Paragraph(x) => {
            table.set("type", "paragraph")?;
            table.set("children", many_ast_to_lua(&lua, x.children)?)?;
        }
        Node::Definition(x) => {
            table.set("type", "definition")?;
            table.set("url", x.url)?;
            table.set("title", x.title)?;
            table.set("identifier", x.identifier)?;
            table.set("label", x.label)?;
        }
        x => todo!("Still need to implement {:?}", x),
    }

    // return the built table
    Ok(table)
}

// Convert children to lua tables
fn many_ast_to_lua(lua: &Lua, ast: Vec<Node>) -> Result<Table, Error> {
    let table = lua.create_table()?;
    for node in ast {
        table.push(ast_to_lua(&lua, node)?)?;
    }

    Ok(table)
}
