use std::{collections::HashMap, path::PathBuf, str::FromStr};

use mlua::{Lua, Table};

use crate::filetree::FileNode;

/// A single item in the page tree
#[derive(Debug)]
pub(crate) enum SiteNode<'lua> {
    /// Asset, any file that can be included or loaded
    Asset { path: PathBuf },

    /// Page, with siblings
    Page {
        html: String,
        meta: Table<'lua>,
        subs: HashMap<String, SiteNode<'lua>>,
    },

    /// Lua table
    Table { table: Table<'lua> },

    /// Subdirectory
    Dir {
        subs: HashMap<String, SiteNode<'lua>>,
    },
}

// TODO: make this a lua uservalue instead, with the option to convert to/from?
impl<'lua> SiteNode<'lua> {
    fn to_table(self, lua: &Lua) -> Table<'_> {
        let table = lua.create_table().expect("Failed to create table!");

        match self {
            Self::Asset { path } => {
                // we are an asset
                table.set("type", "asset").expect("Failed to intert table");

                // add file name, stem and extention
                table
                    .set(
                        "extention",
                        path.extension().map(|x| {
                            x.to_os_string()
                                .into_string()
                                .expect("Failed to convert OsString to string!")
                        }),
                    )
                    .expect("Failed to intert table");
                table
                    .set(
                        "stem",
                        path.file_stem().map(|x| {
                            x.to_os_string()
                                .into_string()
                                .expect("Failed to convert OsString to string!")
                        }),
                    )
                    .expect("Failed to intert table");
                table
                    .set(
                        "name",
                        path.file_name().map(|x| {
                            x.to_os_string()
                                .into_string()
                                .expect("Failed to convert OsString to string!")
                        }),
                    )
                    .expect("Failed to intert table");

                // asset path
                table
                    .set(
                        "path",
                        path.into_os_string()
                            .into_string()
                            .expect("Failed to convert OsString to string!"),
                    )
                    .expect("Failed to intert table");

                // file loading functions
                // TODO
            }
            Self::Page { html, meta, subs } => {
                table.set("type", "page").expect("Failed to intert table");
                table.set("meta", meta).expect("Failed to intert table");
                table.set("html", html).expect("Failed to intert table");
                table
                    .set(
                        "subs",
                        lua.create_table_from(subs.into_iter().map(|(k, v)| (k, v.to_table(lua))))
                            .expect("Failed to make table!"),
                    )
                    .expect("Failed to intert table");
            }
            Self::Table { table: meta } => {
                table.set("type", "meta").expect("Failed to intert table");
                table.set("meta", meta).expect("Failed to intert table");
            }
            Self::Dir { subs } => {
                table.set("type", "dir").expect("Failed to intert table");
                table
                    .set(
                        "subs",
                        lua.create_table_from(subs.into_iter().map(|(k, v)| (k, v.to_table(lua))))
                            .expect("Failed to make table!"),
                    )
                    .expect("Failed to intert table");
            }
        }

        table
    }

    fn from_table(table: Table<'lua>) -> Option<Self> {
        match table
            .get::<&str, String>("type")
            .expect("Failed to get type")
            .as_str()
        {
            "page" => {
                // get the subdirs
                let subs: Table = table.get("subs").expect("Failed to get subs");

                // convert the subdirs from tables
                let subs = subs
                    .pairs()
                    .filter_map(|x| x.ok())
                    .map(|(k, v)| {
                        (
                            k,
                            Self::from_table(v).expect("Failed to convert from table"),
                        )
                    })
                    .collect();

                // get the resulting html and the meta table
                let html: String = table.get("html").expect("Failed to get html");
                let meta: Table = table.get("meta").expect("Failed to get meta");

                // resulting node
                Some(SiteNode::Page { html, meta, subs })
            }
            "table" => {
                // meta table only
                let table: Table = table.get("meta").expect("Failed to get meta");

                // so return a table
                Some(SiteNode::Table { table })
            }
            "asset" => {
                let path: String = table.get("path").expect("failed to get path");

                Some(SiteNode::Asset {
                    path: PathBuf::from_str(path.as_str()).expect("Failed to parse path"),
                })
            }
            "dir" => {
                // get the subdirs
                let subs: Table = table.get("subs").expect("Failed to get subs");

                // convert the subdirs from tables
                let subs = subs
                    .pairs()
                    .filter_map(|x| x.ok())
                    .map(|(k, v)| {
                        (
                            k,
                            Self::from_table(v).expect("Failed to convert from table"),
                        )
                    })
                    .collect();

                Some(SiteNode::Dir { subs })
            }
            _ => None,
        }
    }
}

impl FileNode {
    pub(crate) fn evaluate(self, lua: &Lua) -> SiteNode {
        render_tree(lua, self)
    }
}

/// Build the tree
fn render_tree(lua: &Lua, filetree: FileNode) -> SiteNode {
    match filetree {
        FileNode::Lua { code, subs } => {
            // collect siblings
            let subs = subs
                .into_iter()
                .map(|(k, v)| (k, render_tree(lua, v).to_table(lua)));

            // load into a table
            let subs = lua.create_table_from(subs).expect("Failed to make table");

            // set the colocated files in the globals
            lua.globals()
                .set("colocatedFiles", subs)
                .expect("Failed to set ");

            // run script
            // TODO: figure out a way to duplicate the globals and back to create a clean env
            let result: Table = lua.load(code).eval().expect("Cronch");

            // convert into either a page or table
            SiteNode::from_table(result).expect("Failed to convert from a table")
        }
        FileNode::Dir { subs } => SiteNode::Dir {
            subs: subs
                .into_iter()
                .map(|(k, v)| (k, render_tree(lua, v)))
                .collect(),
        },
        FileNode::Asset { path } => SiteNode::Asset { path },
    }
}
