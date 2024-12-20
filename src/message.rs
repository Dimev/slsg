use std::{fmt::Display, io::stdout};

use mlua::Error;

use crossterm::{
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};

/// Print an error to the terminal
pub(crate) fn print_error<E: Display>(context: &str, error: &E) {
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

/// Print a warning to the terminal
pub(crate) fn print_warning<E: Display>(context: &str, error: &E) {
    let text = error.to_string();
    execute!(
        stdout(),
        SetAttribute(Attribute::Bold),
        SetForegroundColor(Color::Yellow),
        Print(context),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(Color::Yellow),
        Print(":\n"),
        Print(text),
        Print("\n"),
        ResetColor,
    )
    .expect("Failed to print error");
}

/// Print a notification to the terminal
pub(crate) fn notify<E: Display>(context: &str, error: &E) {
    let text = error.to_string();
    execute!(
        stdout(),
        SetAttribute(Attribute::Bold),
        Print(context),
        SetAttribute(Attribute::Reset),
        Print(":\n"),
        Print(text),
        Print("\n"),
    )
    .expect("Failed to print error");
}

/// Produce an error page for html
pub(crate) fn html_error(error: &Error) -> String {
    format!(include_str!("error_template.html"), error)
}
