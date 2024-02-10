use std::{collections::HashMap, fs, path::PathBuf, str::FromStr};

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

impl<'lua> SiteNode<'lua> {
    pub(crate) fn render(&self, path: PathBuf) {
        match self {
            SiteNode::Asset { path: asset_path } => {
                // write out file, directory should already exist
                fs::copy(asset_path, path).expect("Failed to copy file");
            }
            SiteNode::Page { html, subs, .. } => {
                // create the directory
                fs::create_dir_all(&path).expect("Failed to make directory!");

                // write out the rest
                for (key, value) in subs.iter() {
                    value.render(path.join(key));
                }

                // write out the page
                fs::write(path.join("index.html"), html).expect("Failed to write file");
            }
            SiteNode::Dir { subs } => {
                // create the directory
                fs::create_dir_all(&path).expect("Failed to make directory!");

                // write out all files
                for (key, value) in subs.iter() {
                    value.render(path.join(key));
                }
            }
            SiteNode::Table { .. } => {}
        }
    }

    fn to_table(self, lua: &Lua) -> Table<'_> {
        let table = lua.create_table().expect("Failed to create table!");

        match self {
            Self::Asset { path } => {
                table.set("type", "asset").expect("Failed to intert table");
                table
                    .set(
                        "path",
                        path.into_os_string()
                            .into_string()
                            .expect("Failed to convert OsString to string!"),
                    )
                    .expect("Failed to intert table");
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

/// Build the tree
pub(crate) fn render_tree(lua: &Lua, filetree: FileNode) -> SiteNode {
    match filetree {
        FileNode::Lua { code, subs } => {
            // collect siblings
            let subs = subs
                .into_iter()
                .map(|(k, v)| (k, render_tree(lua, v).to_table(lua)));

            // load into a table
            let subs = lua.create_table_from(subs).expect("Failed to make table");

            // set in globals
            lua.globals()
                .set("directories", subs)
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
