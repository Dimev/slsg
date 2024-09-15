use std::io::stdout;

use mlua::Error;

use crossterm::{
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};

/// Print an error to the terminal
pub(crate) fn print_error(error: &Error) {
    let text = error.to_string();
    execute!(
        stdout(),
        SetAttribute(Attribute::Bold),
        SetForegroundColor(Color::Red),
        Print("[ERR] "),
        Print(text),
        Print("\n"),
        ResetColor,
        SetAttribute(Attribute::Reset)
    )
    .expect("Failed to print error");
}

/// Produce an error page for html
pub(crate) fn html_error(error: &Error) -> String {
    format!(include_str!("error_template.html"), error)
}
