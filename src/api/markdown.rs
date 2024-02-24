use mlua::{UserData, UserDataFields};
use pulldown_cmark::{html::push_html, Parser};

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
            let mut out = String::new();
            push_html(&mut out, Parser::new(&this.raw));
            Ok(out)
        });
        //fields.add_field_method_get("front", |_, this| Ok(markdown::to_html(&this.raw)));
    }
}
