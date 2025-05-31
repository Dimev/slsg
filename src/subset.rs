use std::collections::BTreeSet;

use hb_subset::Blob;
use mlua::{ErrorContext, ExternalResult, Result};

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
