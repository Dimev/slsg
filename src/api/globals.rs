use mlua::{Lua, Table};
use svgbob::to_svg_with_settings;

use super::file::File;

/// Load all program globals into the lua globals
pub(crate) fn load_globals(lua: &Lua, debug: bool) -> Result<(), anyhow::Error> {
    // create a new file
    let file = lua.create_function(|_, text: String| Ok(File::New(text)))?;

    // convert ascii to svg with svgbob
    // TODO: options
    let ascii_to_svg = lua.create_function(|_, (text, options): (String, Option<Table>)| {
        let options = if let Some(options) = options {
            svgbob::Settings {
                font_size: (options.get::<&str, Option<usize>>("fontsize")?).unwrap_or(14),
                font_family: (options.get::<&str, Option<String>>("fontfamily")?)
                    .unwrap_or("monospace".into()),
                fill_color: (options.get::<&str, Option<String>>("fillcolor")?)
                    .unwrap_or("black".into()),
                background: (options.get::<&str, Option<String>>("background")?)
                    .unwrap_or("white".into()),
                stroke_color: (options.get::<&str, Option<String>>("strokecolor")?)
                    .unwrap_or("black".into()),
                stroke_width: (options.get::<&str, Option<f32>>("strokewidth")?).unwrap_or(2.0),
                scale: (options.get::<&str, Option<f32>>("scale")?).unwrap_or(8.0),
                include_backdrop: (options.get::<&str, Option<bool>>("backdrop")?).unwrap_or(true),
                include_styles: (options.get::<&str, Option<bool>>("styles")?).unwrap_or(true),
                include_defs: (options.get::<&str, Option<bool>>("defs")?).unwrap_or(true),
            }
        } else {
            svgbob::Settings::default()
        };

        Ok(to_svg_with_settings(&text, &options))
    })?;

    // escape html
    // TODO let html_escape = lua.create_function(|_, text: String| Ok())?;

    // config
    // TODO

    // load
    let table = lua.create_table()?;
    table.set("file", file)?;
    table.set("svgbob", ascii_to_svg)?;
    table.set("debug", debug)?;
    lua.globals().set("yassg", table)?;

    // standard lib
    lua.load(include_str!("lib.lua"))
        .set_name("builtin://stdlib.lua")
        .exec()?;

    Ok(())
}
