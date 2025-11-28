use anyhow::{Context, Result, anyhow};
use mlua::{Function, Lua, Table};

const OPEN: &str = "<?";
const CLOSE: &str = "?>";

// TODO: mimic etlua here
// provide a bit better error messages
/// Compile a script that templates
pub(crate) fn compile(lua: &Lua, mut template: &str, name: Option<String>) -> Result<Function> {
    // generated code for the template
    let mut function = String::new();

    // TODO line numbers

    // while there is a <?, find it
    while let Some(open) = template.find(OPEN) {
        // skip to the start of this open
        let rest = &template[open + OPEN.len()..];
        template = rest;

        // find the closing ?>, that's not inside a string
        // TODO: deal with reading a string here?
        // TODO: this will probably cause some amount of weirdness in the markdown parser
        // TODO: but that's a sacrifice I'm willing to make, for now
        // TODO: maybe use the html template there anyway
        // TODO: or expose compiling markdown as a lua function too?
        // TODO: just make everything lua at this point
        // TODO: lua function to parse the markdown, patch in ability to pass a highlighter and math renderer
        let closing = template
            .find(CLOSE)
            .ok_or_else(|| anyhow!("No closing '{CLOSE}' found"))?;

        // see https://github.com/leafo/etlua/blob/master/etlua.moon
        // TODO
        // TODO: keep lines in the lua file the same, that should help with error reporting
        dbg!(template);
    }

    // compile the generated lua code
    if let Some(n) = name {
        // set the name, if any
        lua.load(function).set_name(n)
    } else {
        lua.load(function)
    }
    .into_function()
    .context("Failed to compile template")
}
