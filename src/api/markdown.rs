use markdown::{
    mdast::{AlignKind, AttributeContent, AttributeValue, Node, ReferenceKind, Root},
    to_html_with_options, to_mdast, Constructs, Options, ParseOptions,
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

const OPTIONS: ParseOptions = ParseOptions {
    constructs: Constructs {
        attention: true,
        autolink: true,
        block_quote: true,
        character_escape: true,
        character_reference: true,
        code_indented: true,
        code_fenced: true,
        code_text: true,
        definition: true,
        frontmatter: true,
        gfm_autolink_literal: true,
        gfm_label_start_footnote: true,
        gfm_footnote_definition: true,
        gfm_strikethrough: true,
        gfm_table: true,
        gfm_task_list_item: true,
        hard_break_escape: true,
        hard_break_trailing: true,
        heading_atx: true,
        heading_setext: true,
        html_flow: true,
        html_text: true,
        label_start_image: true,
        label_start_link: true,
        label_end: true,
        list_item: true,
        math_flow: true,
        math_text: true,
        mdx_esm: false,
        mdx_expression_flow: false,
        mdx_expression_text: false,
        mdx_jsx_flow: false,
        mdx_jsx_text: false,
        thematic_break: true,
    },
    mdx_expression_parse: None,
    mdx_esm_parse: None,
    gfm_strikethrough_single_tilde: false,
    math_text_single_dollar: true,
};

impl UserData for Markdown {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("raw", |_, this| Ok(this.raw.clone()));
        fields.add_field_method_get("html", |_, this| {
            // convert to html
            to_html_with_options(
                &this.raw,
                &Options {
                    parse: OPTIONS,
                    compile: Default::default(),
                },
            )
            .map_err(Error::external)
        });
        fields.add_field_method_get("ast", |lua, this| {
            // convert to the abstract syntax tree
            let md = to_mdast(&this.raw, &OPTIONS).map_err(Error::external)?;
            let table = ast_to_lua(lua, md)?;
            Ok(table)
        });
        fields.add_field_method_get("front", |lua, this| {
            let md = to_mdast(&this.raw, &OPTIONS).map_err(Error::external)?;
            // first toml or yaml is the front matter
            if let Node::Root(Root { children, .. }) = md {
                if let [Node::Toml(x), ..] = children.as_slice() {
                    let toml: toml::Value = toml::from_str(&x.value).map_err(Error::external)?;
                    Ok(Some(lua.to_value(&toml)?))
                } else if let [Node::Yaml(x), ..] = children.as_slice() {
                    let yaml: serde_yaml::Value =
                        serde_yaml::from_str(&x.value).map_err(Error::external)?;
                    Ok(Some(lua.to_value(&yaml)?))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        });
    }
}

// Convert a markdown ast to lua tables
fn ast_to_lua(lua: &Lua, ast: Node) -> Result<Table, Error> {
    let table = lua.create_table()?;
    match ast {
        Node::Root(x) => {
            table.set("type", "root")?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::BlockQuote(x) => {
            table.set("type", "blockquote")?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::FootnoteDefinition(x) => {
            table.set("type", "footnoteDefinition")?;
            table.set("identifier", x.identifier)?;
            table.set("label", x.label)?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::MdxJsxFlowElement(x) => {
            table.set("type", "mdxJsxFlowElement")?;
            table.set("name", x.name)?;
            table.set("attributes", mdx_jsx_attrs_to_lua(lua, x.attributes)?)?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::List(x) => {
            table.set("type", "list")?;
            table.set("ordered", x.ordered)?;
            table.set("start", x.start)?;
            table.set("spread", x.spread)?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::Toml(x) => {
            table.set("type", "toml")?;
            let toml: toml::Value =
                toml::from_str(&x.value).map_err(mlua::Error::external)?;
            table.set("data", lua.to_value(&toml)?)?;
            table.set("value", x.value)?;
        }
        Node::Yaml(x) => {
            table.set("type", "yaml")?;
            let yaml: serde_yaml::Value =
                serde_yaml::from_str(&x.value).map_err(mlua::Error::external)?;
            table.set("data", lua.to_value(&yaml)?)?;
            table.set("value", x.value)?;
        }
        Node::Break(_) => {
            table.set("type", "break")?;
        }
        Node::InlineCode(x) => {
            table.set("type", "inlineCode")?;
            table.set("value", x.value)?;
        }
        Node::InlineMath(x) => {
            table.set("type", "inlineMath")?;
            table.set("value", x.value)?;
        }
        Node::Delete(x) => {
            table.set("type", "delete")?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::Emphasis(x) => {
            table.set("type", "emphasis")?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::MdxTextExpression(x) => {
            table.set("type", "mdxTextExpression")?;
            table.set("value", x.value)?;
        }
        Node::MdxjsEsm(x) => {
            table.set("type", "mxdJsEsm")?;
            table.set("value", x.value)?;
        }
        Node::FootnoteReference(x) => {
            table.set("type", "footnoteReference")?;
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
            table.set("type", "imageReference")?;
            table.set("alt", x.alt)?;
            table.set("identifier", x.identifier)?;
            table.set("label", x.label)?;
            table.set(
                "referenceKind",
                match x.reference_kind {
                    ReferenceKind::Shortcut => "shortcut",
                    ReferenceKind::Collapsed => "collapsed",
                    ReferenceKind::Full => "full",
                },
            )?;
        }
        Node::MdxJsxTextElement(x) => {
            table.set("type", "mdxJsxTextElement")?;
            table.set("name", x.name)?;
            table.set("attributes", mdx_jsx_attrs_to_lua(lua, x.attributes)?)?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::Link(x) => {
            table.set("type", "link")?;
            table.set("url", x.url)?;
            table.set("title", x.title)?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::LinkReference(x) => {
            table.set("type", "linkReference")?;
            table.set("identifier", x.identifier)?;
            table.set("label", x.label)?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
            table.set(
                "referenceKind",
                match x.reference_kind {
                    ReferenceKind::Shortcut => "shortcut",
                    ReferenceKind::Collapsed => "collapsed",
                    ReferenceKind::Full => "full",
                },
            )?;
        }
        Node::Strong(x) => {
            table.set("type", "strong")?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
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
        Node::MdxFlowExpression(x) => {
            table.set("type", "mdxFlowExpression")?;
            table.set("value", x.value)?;
        }
        Node::Heading(x) => {
            table.set("type", "heading")?;
            table.set("depth", x.depth)?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::Table(x) => {
            table.set("type", "table")?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
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
            table.set("type", "thematicBreak")?;
        }
        Node::TableRow(x) => {
            table.set("type", "tableRow")?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::TableCell(x) => {
            table.set("type", "tableCell")?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::ListItem(x) => {
            table.set("type", "listItem")?;
            table.set("spread", x.spread)?;
            table.set("checked", x.checked)?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::Paragraph(x) => {
            table.set("type", "paragraph")?;
            table.set("children", many_ast_to_lua(lua, x.children)?)?;
        }
        Node::Definition(x) => {
            table.set("type", "definition")?;
            table.set("url", x.url)?;
            table.set("title", x.title)?;
            table.set("identifier", x.identifier)?;
            table.set("label", x.label)?;
        }
    }

    // return the built table
    Ok(table)
}

// Convert children to lua tables
fn many_ast_to_lua(lua: &Lua, ast: Vec<Node>) -> Result<Table, Error> {
    let table = lua.create_table()?;
    for node in ast {
        table.push(ast_to_lua(lua, node)?)?;
    }

    Ok(table)
}

// create an attributes table
fn mdx_jsx_attrs_to_lua(lua: &Lua, ast: Vec<AttributeContent>) -> Result<Table, Error> {
    let attrs = lua.create_table()?;
    for attr in ast {
        let att = lua.create_table()?;
        match attr {
            AttributeContent::Expression { value, .. } => {
                att.set("type", "expression")?;
                att.set("value", value)?;
            }
            AttributeContent::Property(x) => {
                att.set("type", "attribute")?;
                att.set("name", x.name)?;
                match x.value {
                    Some(AttributeValue::Expression(x)) => {
                        att.set("valueType", "expression")?;
                        att.set("value", x.value)?;
                    }
                    Some(AttributeValue::Literal(x)) => {
                        att.set("valueType", "literal")?;
                        att.set("value", x)?;
                    }
                    _ => {}
                }
            }
        }
        attrs.push(att)?;
    }

    Ok(attrs)
}
