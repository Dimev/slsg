use std::{fmt::Display, io::stdout};

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

/// Print a success
pub(crate) fn print_success<E: Display>(context: &str, error: &E) {
    let text = error.to_string();
    execute!(
        stdout(),
        SetAttribute(Attribute::Bold),
        SetForegroundColor(Color::Green),
        Print(context),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(Color::Green),
        Print(":\n"),
        Print(text),
        Print("\n"),
        ResetColor
    )
    .expect("Failed to print error");
}

/// Produce an error page for html
pub(crate) fn html_error<E: Display>(error: &E) -> String {
    let error = error.to_string();
    let mut err = String::with_capacity(error.len());
    for c in error.chars() {
        match c {
            '&' => err.push_str("&amp;"),
            '<' => err.push_str("&lt;"),
            '>' => err.push_str("&gt;"),
            '"' => err.push_str("&quot;"),
            '\'' => err.push_str("&#39;"),
            _ => err.push(c),
        }
    }
    format!(include_str!("error_template.html"), err)
}
