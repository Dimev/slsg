use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::sitetree::SiteNode;

impl<'lua> SiteNode<'lua> {
    pub(crate) fn write_to_files(&self, path: PathBuf) {
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
                    value.write_to_files(path.join(key));
                }

                // write out the page
                fs::write(path.join("index.html"), html).expect("Failed to write file");
            }
            SiteNode::Dir { subs } => {
                // create the directory
                fs::create_dir_all(&path).expect("Failed to make directory!");

                // write out all files
                for (key, value) in subs.iter() {
                    value.write_to_files(path.join(key));
                }
            }
            SiteNode::Table { .. } => {}
        }
    }

    pub(crate) fn read_file_content<P: AsRef<Path>>(&self, path: P) -> Vec<u8> {
        todo!()
    }
}
