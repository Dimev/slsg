/// Print a warning, in yellow
#[macro_export]
macro_rules! printwarn {
    ($($arg:tt)*) => {{
        use std::io::stdout;
        use crossterm::execute;
        use crossterm::style::{Print, SetForegroundColor, ResetColor, SetAttribute, Attribute, Color};

        let string = format!($($arg)*);
        let mut stdout = stdout();
        execute!(
            stdout,
            SetForegroundColor(Color::Yellow),
            SetAttribute(Attribute::Bold),
            Print("[WARN] ".to_string()),
            Print(string),
            ResetColor,
            SetAttribute(Attribute::Reset),
            Print("\n".to_string()),

        ).expect("failed to warn");
    }};
}

/// print an error, in red
#[macro_export]
macro_rules! printerr {
    ($($arg:tt)*) => {{
        use std::io::stdout;
        use crossterm::execute;
        use crossterm::style::{Print, SetForegroundColor, ResetColor, SetAttribute, Attribute, Color};

        let string = format!($($arg)*);
        let mut stdout = stdout();
        execute!(
            stdout,
            SetForegroundColor(Color::Red),
            SetAttribute(Attribute::Bold),
            Print("[ERR] ".to_string()),
            Print(string),
            ResetColor,
            SetAttribute(Attribute::Reset),
            Print("\n".to_string()),

        ).expect("failed to warn");
    }};
}

/// generate html to preview the errors
pub(crate) fn warning_and_error_html(warnings: &Vec<String>, errors: &Vec<String>) -> String {
    // styles to use
    let warn_line = "font: monospace; color: yellow";
    let warn_div = "";

    let center_div = "display: flex; justify-content: center; align-items: center";

    let warns: String = warnings
        .iter()
        .map(|x| {
            let lines: String = x
                .lines()
                .map(|x| format!(r#"<p style="{warn_line}">{x}</p>"#))
                .collect();

            format!(r#"<div style={warn_div}>{lines}</div>"#)
        })
        .collect();

    let errs: String = errors
        .iter()
        .map(|x| {
            let lines: String = x
                .lines()
                .map(|x| format!(r#"<p style="{warn_line}">{x}</p>"#))
                .collect();

            format!(r#"<div style={warn_div}>{lines}</div>"#)
        })
        .collect();

    if warnings.is_empty() && errors.is_empty() {
        String::new()
    } else {
        format!(r#"<div style="{center_div}">{errs}{warns}</div>"#)
    }
}
