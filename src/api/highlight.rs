use std::{collections::HashMap, fs, iter::Peekable, path::Path};

use anyhow::anyhow;
use regex::{Matches, Regex, RegexBuilder};
use serde::Deserialize;

/// Single set of regex rules, as strings
#[derive(Deserialize)]
struct LanguageRaw {
    /// file extentions
    extentions: Vec<String>,

    /// Regex rules
    #[serde(flatten, with = "tuple_vec_map")]
    rules: Vec<(String, String)>,
}

/// Single set of regex rules
#[derive(Clone)]
struct Language {
    /// file extentions
    extentions: Vec<String>,

    /// Regex rules
    rules: Vec<(String, Regex)>,
}

/// Set of highlighter
#[derive(Clone)]
pub(crate) struct Languages(HashMap<String, Language>);

/// Highlighting range
#[derive(Debug)]
pub(crate) struct HighlightRange {
    /// Part of the code
    pub(crate) text: String,

    /// Style to use
    pub(crate) style: String,
}

impl Languages {
    /// Parse a language set from a string
    pub(crate) fn from_str(string: &str) -> Result<Self, anyhow::Error> {
        let raw = toml::from_str::<HashMap<String, LanguageRaw>>(string)?;
        let mut languages = HashMap::with_capacity(raw.len());

        for (name, language) in raw {
            let mut rules = Vec::with_capacity(language.rules.len());
            for (style, rule) in language.rules {
                rules.push((style, RegexBuilder::new(&rule).multi_line(true).build()?));
            }
            languages.insert(
                name,
                Language {
                    extentions: language.extentions,
                    rules,
                },
            );
        }

        Ok(Languages(languages))
    }

    /// Load all languages
    pub(crate) fn load(path: &impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let mut languages = HashMap::new();

        // load the included languages
        let included = include_str!("languages.toml");
        languages.extend(Self::from_str(included)?.0);

        // load the on-disk languages
        if let Ok(dir) = fs::read_dir(path) {
            for file in dir {
                let text = fs::read_to_string(file?.path())?;
                languages.extend(Self::from_str(&text)?.0.into_iter());
            }
        }

        Ok(Languages(languages))
    }

    /// Highlight one language, from the language name or extention, if it exists
    pub(crate) fn highlight(
        &self,
        code: &str,
        language: &str,
    ) -> Result<Vec<HighlightRange>, anyhow::Error> {
        // find the language
        let language = self
            .0
            .get(language)
            .or_else(|| {
                self.0.iter().find_map(|(_, x)| {
                    if x.extentions.iter().any(|x| x.as_str() == language) {
                        Some(x)
                    } else {
                        None
                    }
                })
            })
            .ok_or(anyhow!("Could not find language {language}"))?;

        let mut matches: Vec<Peekable<Matches>> = language
            .rules
            .iter()
            .map(|x| x.1.find_iter(code).peekable())
            .collect();

        // highlight the code
        let mut cur_chunk = String::new();
        let mut cur_style = None;
        let mut highlights = Vec::new();

        for (start, character) in code.char_indices() {
            // advance all styles no longer in the range
            for range in matches.iter_mut() {
                if range.peek().map(|x| start >= x.end()).unwrap_or(false) {
                    range.next();
                }
            }

            // find the index of the last style that matches
            // keep the current item if it still matches
            let style = if cur_style
                .map(|i| {
                    (&mut matches[i] as &mut Peekable<Matches>)
                        .peek()
                        .map(|x| start >= x.start() && start < x.end())
                        .unwrap_or(false)
                })
                .unwrap_or(false)
            {
                cur_style
            } else {
                matches.iter_mut().enumerate().find_map(|(i, x)| {
                    x.peek().and_then(|x| {
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
                            .and_then(|i| language.rules.get(i).map(|x| x.0.clone()))
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
                    .and_then(|i| language.rules.get(i).map(|x| x.0.clone()))
                    .unwrap_or(String::new()),
            });
        }

        Ok(highlights)
    }

    /// Highlight one language as html, if it exists
    pub(crate) fn highlight_html(
        &self,
        code: &str,
        language: &str,
        class_prefix: Option<String>,
    ) -> Result<String, anyhow::Error> {
        Ok(self
            .highlight(code, language)?
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
}

fn escape_html(string: &str) -> String {
    string
        .replace('&', "&amp;")
        .replace('\"', "&quot;")
        .replace('\'', "&apos;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
