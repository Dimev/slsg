use crossterm::{
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};
use std::io::stdout;

use crate::{api::highlight::Languages, cookbook::Entry};

fn escape_html(string: &str) -> String {
    string
        .replace('&', "&amp;")
        .replace('\"', "&quot;")
        .replace('\'', "&apos;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Print a warning to the terminal
pub(crate) fn print_warning(str: &str) {
    let mut stdout = stdout();
    execute!(
        stdout,
        SetForegroundColor(Color::Yellow),
        SetAttribute(Attribute::Bold),
        Print("[WARN] ".to_string()),
        Print(str),
        ResetColor,
        SetAttribute(Attribute::Reset),
        Print("\n".to_string()),
    )
    .expect("failed to warn");
}

/// Print an error to the terminal
pub(crate) fn print_error(str: &str) {
    let mut stdout = stdout();
    execute!(
        stdout,
        SetForegroundColor(Color::Red),
        SetAttribute(Attribute::Bold),
        Print("[ERR] ".to_string()),
        Print(str),
        ResetColor,
        SetAttribute(Attribute::Reset),
        Print("\n".to_string()),
    )
    .expect("failed to warn");
}

/// generate html to preview the errors
pub(crate) fn warning_and_error_html(warnings: &[String], errors: &[String]) -> String {
    // styles to use
    let warn_div = "font: 16px monospace; color: #F5871F";
    let err_div = "font: 16px monospace; color: #C82829";
    let button_style = "float: right; font: 16px monospace; border: none; padding: 8px";
    let close_button = format!(
        r#"<button onclick="document.getElementById(&quot;long-warning-box-remove-name&quot;).remove()" style="{button_style}">Close</button>"#
    );

    let center_div =
        "display: flex; justify-content: center; align-items: center; width: 100vw; height: 100vh; border: 0px; margin: 0px; position: fixed; top: 0px; left: 0px; pointer-events: none";
    let inner_div =
        "border-left: #4271AE 5px solid; background: white; max-width: 60%; max-height: 80%; padding: 10px; overflow: scroll; box-shadow: 2px 2px 60px #0005; pointer-events: auto";

    // format the warnings
    let warns: String = warnings.iter().fold(String::new(), |acc, x| {
        let lines = escape_html(x);
        format!(r#"{acc}<pre style="{warn_div}">{lines}</pre>"#)
    });

    // only add them if there are warnings
    let warns = if warns.is_empty() {
        String::new()
    } else {
        format!(r#"<p style="font: 16px monospace">Warnings:</p>{warns}"#)
    };

    // format the errors
    let errs: String = errors.iter().fold(String::new(), |acc, x| {
        let lines = escape_html(x);
        format!(r#"{acc}<pre style="{err_div}">{lines}</pre>"#)
    });

    // only add them if there are errors
    let errs = if errs.is_empty() {
        String::new()
    } else {
        format!(r#"<p style="font: 16px monospace">Errors:</p>{errs}"#)
    };

    // only output a string if there were errors or warnings
    if warnings.is_empty() && errors.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div id="long-warning-box-remove-name" style="{center_div}"><div style="{inner_div}">{close_button}{errs}{warns}</div></div>"#
        )
    }
}

pub(crate) fn print_markdown(md: &str) {
    let mut stdout = stdout();
    for line in md.lines() {
        if line.starts_with('#') {
            queue!(
                stdout,
                SetAttribute(Attribute::Bold),
                Print(line.trim_start_matches('#').trim()),
                Print("\n"),
                SetAttribute(Attribute::Reset),
            )
            .expect("Failed to print entry");
        } else {
            queue!(
                stdout,
                Print(line.trim_start_matches('#').trim()),
                Print("\n"),
            )
            .expect("Failed to print entry");
        }
    }

    execute!(stdout, Print("\n\n")).expect("Failed to write entry");
}

pub(crate) fn print_entry(entry: Entry) {
    let mut stdout = stdout();
    queue!(
        stdout,
        SetAttribute(Attribute::Bold),
        Print(entry.description),
        Print("\n\n"),
        SetAttribute(Attribute::Reset),
    )
    .expect("Failed to print entry");

    for line in entry.tutorial.lines() {
        if line.starts_with('#') {
            queue!(
                stdout,
                SetAttribute(Attribute::Bold),
                Print(line.trim_start_matches('#').trim()),
                Print("\n"),
                SetAttribute(Attribute::Reset),
            )
            .expect("Failed to print entry");
        } else {
            queue!(
                stdout,
                Print(line.trim_start_matches('#').trim()),
                Print("\n"),
            )
            .expect("Failed to print entry");
        }
    }

    queue!(stdout, Print("\n")).expect("Failed to print entry");

    let languages =
        Languages::from_str(include_str!("api/languages.toml")).expect("Failed to parse languages");

    for range in languages
        .highlight(&entry.code.replace("\t", "  "), "lua")
        .expect("Failed to highlight language")
    {
        let color = match range.style.as_str() {
            "comment" => Color::Cyan,
            "keyword" => Color::Yellow,
            "string" => Color::Green,
            _ => Color::White,
        };

        queue!(
            stdout,
            SetForegroundColor(color),
            Print(range.text),
            ResetColor
        )
        .expect("Failed to write entry");
    }

    execute!(stdout, Print("\n\n")).expect("Failed to write entry");
}
