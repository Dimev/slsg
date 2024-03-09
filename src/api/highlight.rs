use std::{collections::HashMap, fs, path::Path};

use anyhow::anyhow;
use regex::{Regex, RegexSet};
use serde::Deserialize;

/// Single set of regex rules
#[derive(Deserialize)]
struct Language {
    /// file extentions
    extentions: Vec<String>,

    /// Regex rules
    #[serde(flatten)]
    rules: Vec<(String, String)>,
}

/// Set of highlighter
#[derive(Deserialize)]
pub(crate) struct Languages(HashMap<String, Language>);

/// Highlighting range
pub(crate) struct HighlightRange {
    /// Part of the code
    text: String,

    /// Style to use
    style: String,
}

impl Languages {
    /// Parse a language set from a string
    pub(crate) fn from_str(string: &str) -> Result<Self, anyhow::Error> {
        toml::from_str::<Self>(string).map_err(|x| x.into())
    }

    /// Load all languages
    pub(crate) fn load(path: &impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let mut languages = HashMap::new();

        // load the included languages
        let included = include_str!("languages.toml");
        languages.extend(Self::from_str(included)?.0.into_iter());

        // load the on-disk languages
        for file in fs::read_dir(path)? {
            let text = fs::read_to_string(file?.path())?;
            languages.extend(Self::from_str(&text)?.0.into_iter());
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
                    if x.extentions
                        .iter()
                        .find(|x| x.as_str() == language)
                        .is_some()
                    {
                        Some(x)
                    } else {
                        None
                    }
                })
            })
            .ok_or(anyhow!("Could not find language {language}"))?;

        // highlight
        let mut rules: Vec<(String, Regex)> = Vec::with_capacity(language.rules.len());
        for (name, rule) in language.rules.iter() {
            rules.push((rule.clone(), Regex::new(&rule)?));
        }

        // highlight the code

        todo!()
    }

    /// Highlight one language as html, if it exists
    pub(crate) fn highlight_html(&self, language: &str) -> Option<String> {
        todo!()
    }
}
