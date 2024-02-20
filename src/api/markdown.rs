use mlua::{UserData, UserDataFields};

/// Parsed markdown
pub(crate) struct Markdown {
    /// raw file content
    raw: String,
}

impl Markdown {
    pub(crate) fn from_str(string: &str) -> Result<Self, anyhow::Error> {
        Ok(Self { raw: string.into() })
    }
}

impl UserData for Markdown {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("raw", |_, this| Ok(this.raw.clone()));
    }
}
