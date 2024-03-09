use crossterm::style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor};
use std::io::stdout;

fn escape_html(string: &str) -> String {
    string
        .replace("&", "&amp;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
}

/// Print a warning to the terminal
pub(crate) fn print_warning(str: &str) {
    let mut stdout = stdout();
    crossterm::execute!(
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
    crossterm::execute!(
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
pub(crate) fn warning_and_error_html(warnings: &Vec<String>, errors: &Vec<String>) -> String {
    // styles to use
    let warn_div = "font: 16px monospace; color: #F5871F";
    let err_div = "font: 16px monospace; color: #C82829";

    let center_div =
        "display: flex; justify-content: center; align-items: center; width: 100vw; height: 100vh; border: 0px; margin: 0px; position: fixed; top: 0px; left: 0px";
    let inner_div =
        "border-left: #4271AE 5px solid; background: white; max-width: 60%; max-height: 80%; padding: 10px; overflow: scroll";

    // format the warnings
    let warns: String = warnings
        .iter()
        .map(|x| {
            let lines = escape_html(x);
            format!(r#"<pre style="{warn_div}">{lines}</pre>"#)
        })
        .collect();

    // only add them if there are warnings
    let warns = if warns.is_empty() {
        String::new()
    } else {
        format!(r#"<p style="font: 16px monospace">Warnings:</p>{warns}"#)
    };

    // format the errors
    let errs: String = errors
        .iter()
        .map(|x| {
            let lines = escape_html(x);
            format!(r#"<pre style="{err_div}">{lines}</pre>"#)
        })
        .collect();

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
        format!(r#"<div style="{center_div}"><div style="{inner_div}">{errs}{warns}</div></div>"#)
    }
}
