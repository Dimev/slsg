use mlua::Result;

/// Parse minimark to html
pub(crate) fn minimark() -> Result<String> {
    // TODO
    // basic syntax:
    // = header
    // *bold*
    // _italic_
    // `mono`
    // % comment
    // <? html ?>
    // <?fnl fennel ?>
    // <?lua lua ?>
    // $ math $
    // $$ math $$
    // ```lang code block```
    // \x escape the next character (works for `, *, _, %, <? needs to be escaped with <??)
    todo!()
}
