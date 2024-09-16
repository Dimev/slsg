use std::io::stdout;

use mlua::Error;

use crossterm::{
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};

// TODO: macros for these

/// Print an error to the terminal
pub(crate) fn print_error(context: &str, error: &Error) {
    let text = error.to_string();
    execute!(
        stdout(),
        SetAttribute(Attribute::Bold),
        SetForegroundColor(Color::Red),
        Print(context),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(Color::Red),
        Print(":\n"),
        Print(text),
        Print("\n"),
        ResetColor,
    )
    .expect("Failed to print error");
}

/// Print a success to the terminal
pub(crate) fn print_success(context: &str) {
    execute!(
        stdout(),
        SetAttribute(Attribute::Bold),
        SetForegroundColor(Color::Green),
        Print(context),
        Print("\n"),
        SetAttribute(Attribute::Reset),
        ResetColor,
    )
    .expect("Failed to print error");
}
/// Produce an error page for html
pub(crate) fn html_error(error: &Error) -> String {
    format!(include_str!("error_template.html"), error)
}
