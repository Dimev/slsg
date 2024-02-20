use mlua::UserData;

/// Parsed markdown
pub(crate) struct Markdown {
    /// raw file content
    raw: String,
}

impl Markdown {
    pub(crate) fn from_str(string: &str) -> Result<Self, anyhow::Error> {
        Ok(Self {
            raw: string.into(),
        })
    }
}

impl UserData for Markdown {
    
}
