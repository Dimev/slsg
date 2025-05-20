use std::collections::BTreeSet;

use allsorts::{
    binary::read::ReadScope,
    error::ParseError,
    font::read_cmap_subtable,
    font_data::FontData,
    gsub::{GlyphOrigin, RawGlyph, RawGlyphFlags},
    tables::{FontTableProvider, cmap::Cmap},
    tag,
    tinyvec::TinyVec,
};
use mlua::{ErrorContext, ExternalResult, Result};

/// Subset a font
pub(crate) fn subset_font(file: &[u8], chars: BTreeSet<char>) -> Result<Vec<u8>> {
    // from https://github.com/yeslogic/allsorts-tools/blob/master/src/subset.rs
    let font_file = ReadScope::new(file).read::<FontData>().into_lua_err()?;
    let font_provider = font_file.table_provider(0).into_lua_err()?;
    let cmap_data = font_provider.read_table_data(tag::CMAP).into_lua_err()?;
    let cmap = ReadScope::new(&cmap_data).read::<Cmap>().into_lua_err()?;
    let (_, cmap_subtable) = read_cmap_subtable(&cmap)
        .into_lua_err()?
        .ok_or(mlua::Error::external("no suitable cmap sub-table found"))
        .context("No suitable cmap subtable found")?;

    // this is inserted at the front
    let glyphs = std::iter::once(Ok(Some(RawGlyph {
        unicodes: TinyVec::new(),
        glyph_index: 0,
        liga_component_pos: 0,
        glyph_origin: GlyphOrigin::Direct,
        flags: RawGlyphFlags::empty(),
        variation: None,
        extra_data: (),
    })))
    // from https://github.com/yeslogic/allsorts-tools/blob/master/src/glyph.rs
    .chain(chars.iter().map(|c| {
        if let Some(glyph_index) = cmap_subtable.map_glyph(*c as u32)? {
            Ok(Some(RawGlyph {
                unicodes: TinyVec::Inline([*c; 1].into()),
                glyph_index,
                liga_component_pos: 0,
                glyph_origin: GlyphOrigin::Char(*c),
                flags: RawGlyphFlags::empty(),
                variation: None,
                extra_data: (),
            }))
        } else {
            Ok(None)
        }
    }))
    .collect::<std::result::Result<Vec<_>, ParseError>>()
    .into_lua_err()?;

    let mut glyphs: Vec<RawGlyph<()>> = glyphs.into_iter().flatten().collect();
    glyphs.sort_by(|a, b| a.glyph_index.cmp(&b.glyph_index));
    let mut glyph_ids = glyphs.iter().map(|g| g.glyph_index).collect::<Vec<_>>();
    glyph_ids.dedup();

    if glyph_ids.is_empty() {
        return Err(mlua::Error::external("empty font after subsetting"));
    }

    // subset the font
    allsorts::subset::subset(&font_provider, &glyph_ids).into_lua_err()
}
