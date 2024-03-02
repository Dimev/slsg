use markdown::{mdast::Node, to_html, to_mdast, ParseOptions};
use mlua::{Error, Lua, Table, UserData, UserDataFields};

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
        fields.add_field_method_get("front", |lua, this| {
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
            
        }
        _ => todo!(),
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
