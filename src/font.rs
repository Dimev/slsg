use hb_subset::Blob;
use mlua::{ErrorContext, ExternalResult, Result};
use std::collections::BTreeSet;

use crate::html::text_from_html;

/// Subset a font
pub(crate) fn subset_font(file: &[u8], chars: &BTreeSet<char>) -> Result<Vec<u8>> {
    let font = Blob::from_bytes(file)
        .into_lua_err()
        .context("Failed to read font bytes")?;
    let subsetted = hb_subset::subset(&font, chars.iter().map(|x| *x))
        .into_lua_err()
        .context("Failed to subset font")?;
    Ok(subsetted)
}

// TODO: compression?: https://github.com/odemiral/woff2sfnt-sfnt2woff/blob/master/index.js

/// Get the characters from a html file
pub(crate) fn chars_from_html(html: &str) -> Result<BTreeSet<char>> {
    // get the text
    let text = text_from_html(html)?;

    // and make the charset
    Ok(BTreeSet::from_iter(text.chars()))
}
