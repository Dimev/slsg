use std::{
    fs,
    path::{Path, PathBuf},
};

/// File for the file tree
#[derive(Clone, Debug)]
pub(crate) enum File {
    /// relative path to the original
    RelPath(PathBuf),

    /// Content of a new file
    New(String),
}

impl File {
    pub fn from_path<P: AsRef<Path>>(path: &P) -> Self {
        Self::RelPath(path.as_ref().into())
    }
    
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), anyhow::Error> {
        match self {
            Self::RelPath(p) => fs::copy(p, path).map(|_| ()),
            Self::New(content) => fs::write(path, content).map(|_| ()),
        }
        .map_err(|x| x.into())
    }
}
