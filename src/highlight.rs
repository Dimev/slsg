use std::collections::BTreeMap;

use fancy_regex::{Regex, RegexBuilder};
use mlua::{Result, Table, UserData, UserDataMethods};

/// Single rule for the parser
pub(crate) struct Rule {
    /// Token name to emit
    token: String,

    /// Regex to match with
    regex: Regex,

    /// New state to enter
    next: Option<String>,
}

/// Highlighted span
struct Span {
    /// Text that was highlighted
    text: String,

    /// Token it's part of
    token: String,
}

/// Highlighter for a language
pub(crate) struct Highlighter {
    /// All parser rules
    rules: BTreeMap<String, Vec<Rule>>,
}

impl Highlighter {
    /// Create a highlighter from a lua ruleset
    pub fn from_rules(rules: Table) -> Result<Self> {
        let mut states = BTreeMap::new();

        // all rule pairs
        for state in rules.pairs() {
            let (name, rules): (String, Table) = state?;
            let mut own_rules = Vec::with_capacity(rules.len()? as usize);
            for rule in rules.sequence_values() {
                // get the rules
                let rule: Table = rule?;
                let token: String = rule.get("token")?;
                let regex: String = rule.get("regex")?;
                let next: Option<String> = rule.get("next")?;

                // push to our own rules
                own_rules.push(Rule {
                    token,
                    regex: RegexBuilder::new(&regex)
                        .build()
                        .map_err(|x| mlua::Error::external(x))?,
                    next,
                });
            }

            states.insert(name, own_rules);
        }

        Ok(Self { rules: states })
    }

    /// Highlight code
    fn highlight(&self, mut text: &str) -> Vec<Span> {
        let mut spans = Vec::new();
        let mut state = "start".to_string();

        // as long as there's text input
        while !text.is_empty() {
            // find the closest match
            if let Some(rules) = self.rules.get(&state) {
                // find thle closest match
                if let Some((first, token, next)) = rules
                    .iter()
                    .filter_map(|x| {
                        x.regex
                            .find(text)
                            .unwrap_or(None)
                            .map(|y| (y, &x.token, &x.next))
                    })
                    .min_by_key(|x| x.0.start())
                {
                    // split to the part we don't know of
                    let no_token = &text[..first.start()];
                    let tokened = &text[first.start()..first.end()];

                    // push the no-token text, if any
                    if !no_token.is_empty() {
                        spans.push(Span {
                            text: no_token.to_string(),
                            token: String::new(),
                        });
                    }

                    // push the tokened text, if any
                    if !tokened.is_empty() {
                        spans.push(Span {
                            text: tokened.to_string(),
                            token: token.to_string(),
                        });
                    }

                    // update the state
                    text = &text[first.end()..];
                    state = next.clone().unwrap_or(state);
                } else {
                    // nothing found, push the rest
                    spans.push(Span {
                        text: text.to_string(),
                        token: String::new(),
                    });

                    // end of input
                    text = "";
                }
            } else {
                // no state, simply push the rest
                spans.push(Span {
                    text: text.to_string(),
                    token: String::new(),
                });

                // end of input
                text = "";
            }
        }

        spans
    }
}

impl UserData for Highlighter {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "highlight_html",
            |_, this, (text, class): (String, Option<String>)| {
                Ok(this
                    .highlight(&text)
                    .into_iter()
                    .map(|x| {
                        // escape the html
                        let escaped = x
                            .text
                            .replace("&", "&amp;")
                            .replace("\"", "&quot;")
                            .replace("'", "&apos;")
                            .replace("<", "&lt;")
                            .replace(">", "&gt;");

                        // add the span html node
                        format!(
                            "<span class=\"{}{}\">{escaped}</span>",
                            class.as_ref().unwrap_or(&String::new()),
                            x.token
                        )
                    })
                    .collect::<String>())
            },
        );
        methods.add_method("highlight_ast", |lua, this, text: String| {
            let out = lua.create_table()?;
            for span in this.highlight(&text).into_iter() {
                out.push(lua.create_table_from(
                    // table of { text = ..., token = ... }
                    [("text", span.text), ("token", span.token)].into_iter(),
                )?)?
            }

            Ok(out)
        });
    }
}

// TODO: tests
