use std::iter::{FilterMap, Peekable};

use fancy_regex::{Match, Matches, Regex};

// rule for the highlighter
pub enum HighlightRule {
    Raw(String, String),
    Compiled(String, Regex),
}

pub struct HighlightRange {
    pub style: String,
    pub text: String,
}

pub fn highlight(
    rules: &mut [HighlightRule],
    code: &str,
) -> Result<Vec<HighlightRange>, anyhow::Error> {
    let mut matches = Vec::with_capacity(rules.len());
    for rule in rules.iter_mut() {
        // compile rules
        if let HighlightRule::Raw(style, regex) = rule {
            *rule = HighlightRule::Compiled(style.to_owned(), Regex::new(regex)?);
        }

        // find
        if let HighlightRule::Compiled(style, regex) = rule {
            matches.push((
                style,
                regex.find_iter(code).filter_map(|x| x.ok()).peekable(),
            ));
        }
    }

    // highlight the code
    let mut cur_chunk = String::new();
    let mut cur_style = None;
    let mut highlights = Vec::new();

    for (start, character) in code.char_indices() {
        // advance all styles no longer in the range
        for range in matches.iter_mut() {
            if range.1.peek().map(|x| start >= x.end()).unwrap_or(false) {
                range.1.next();
            }
        }

        // find the index of the last style that matches
        // keep the current item if it still matches
        let style = if cur_style
            .map(|i| {
                (&mut matches[i] as &mut (&mut String, Peekable<FilterMap<Matches, _>>))
                    .1
                    .peek()
                    .map(|x: &Match| start >= x.start() && start < x.end())
                    .unwrap_or(false)
            })
            .unwrap_or(false)
        {
            cur_style
        } else {
            matches.iter_mut().enumerate().find_map(|(i, x)| {
                x.1.peek().and_then(|x| {
                    if start >= x.start() && start < x.end() {
                        Some(i)
                    } else {
                        None
                    }
                })
            })
        };

        // if they are different, push the current string
        if cur_style != style {
            // only add the chunk if it was highlighted
            if !cur_chunk.is_empty() {
                highlights.push(HighlightRange {
                    text: cur_chunk,
                    style: cur_style
                        .and_then(|i| matches.get(i).map(|x| x.0.clone()))
                        .unwrap_or(String::new()),
                });
            }

            // reset the rest
            cur_chunk = String::from(character);
            cur_style = style;
        } else {
            // otherwise, push character
            cur_chunk.push(character);
        }
    }

    // push the last, if any
    if !cur_chunk.is_empty() {
        highlights.push(HighlightRange {
            text: cur_chunk,
            style: cur_style
                .and_then(|i| matches.get(i).map(|x| x.0.clone()))
                .unwrap_or(String::new()),
        });
    }

    Ok(highlights)
}

/// Highlight one language as html, if it exists
pub fn highlight_html(
    rules: &mut [HighlightRule],
    code: &str,
    class_prefix: Option<String>,
) -> Result<String, anyhow::Error> {
    Ok(highlight(rules, code)?
        .into_iter()
        .fold(String::new(), |acc, x| {
            format!(
                r#"{acc}<span class="{}{}">{}</span>"#,
                class_prefix.as_ref().unwrap_or(&String::new()),
                escape_html(&x.style),
                escape_html(&x.text)
            )
        }))
}

fn escape_html(string: &str) -> String {
    string
        .replace('&', "&amp;")
        .replace('\"', "&quot;")
        .replace('\'', "&apos;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
