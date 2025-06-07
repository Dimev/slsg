use mlua::{
    ErrorContext, ExternalResult, Lua, ObjectLike, Result, Table,
    Value::{self, Nil},
};
use regex::Regex;

// Inspired by the highlighter used by `micro`
// this allows reusing the syntax files, after converting them

thread_local! {
    /// Regex cache, to prevent them from compiling all at the start
    static REGEX_CACHE: () = ();
}

/// Cached regex
enum CachedRegex {
    // TODO use
}

/// Highlighter
#[derive(Clone)]
pub(crate) struct Highlighter {
    language: String,
    regex: Regex,
    rules: Vec<Rule>,
}

#[derive(Clone)]
enum Rule {
    Match {
        re: Regex,
        name: String,
    },
    Complex {
        open: Regex,
        close: Regex,
        skip: Option<Regex>,
        name: String,
        rules: Vec<Rule>,
    },
}

impl Rule {
    fn name(&self) -> &str {
        match self {
            Self::Match { name, .. } => name,
            Self::Complex { name, .. } => name,
        }
    }

    fn range(&self, mut text: &str) -> Option<(usize, usize)> {
        match self {
            // simply the range
            Self::Match { re, .. } => re.find(text).map(|x| (x.start(), x.end())),
            Self::Complex {
                open, close, skip, ..
            } => {
                // find the open
                let open = open.find(text)?;

                // move so it's relative
                text = &text[open.end()..];

                // find the close
                let close = close.find(text)?;

                // start off
                let mut close_start = close.start();
                let mut close_end = close.end();

                // while there is a skip before our close, take that
                while let Some(skip) = skip
                    .as_ref()
                    .and_then(|x| x.find(text))
                    .filter(|x| x.start() <= close.start())
                    .filter(|_| false)
                {
                    // TODO
                }

                Some((open.start(), open.start() + close_end))
            }
        }
    }

    fn from_table(lua: &Lua, table: Table) -> Result<Rule> {
        let token = table.get("token")?;
        if table.contains_key("open")? {
            let begin: String = table.get("open")?;
            let end: String = table.get("close")?;
            let skip: Option<String> = table.get("skip")?;
            let rules: Option<Table> = table.get("rules")?;

            Ok(Rule::Complex {
                open: Regex::new(&begin)
                    .into_lua_err()
                    .context("Failed to compile regex")?,
                close: Regex::new(&end)
                    .into_lua_err()
                    .context("Failed to compile regex")?,
                skip: skip
                    .map(|x| {
                        Regex::new(&x)
                            .into_lua_err()
                            .context("Failed to compile regex")
                    })
                    .transpose()?,
                name: token,
                rules: rules
                    .map(|x| {
                        let mut out = Vec::with_capacity(x.len().map(|x| x as usize)?);
                        for val in x.sequence_values() {
                            let val = val?;
                            out.push(Rule::from_table(&lua, val)?);
                        }
                        Result::Ok(out)
                    })
                    .transpose()?
                    .unwrap_or(Vec::new()),
            })
        } else {
            Ok(Rule::Match {
                re: Regex::new(&table.get::<String>(1)?)
                    .into_lua_err()
                    .context("Failed to compile regex")?,
                name: token,
            })
        }
    }
}

impl Highlighter {
    pub(crate) fn from_table(lua: &Lua, table: Table) -> Result<Highlighter> {
        // read meta
        let name: String = table.get("name")?;
        let regex: String = table.get("regex")?;
        let regex = Regex::new(&regex)
            .into_lua_err()
            .context("Failed to compile regex")?;

        // read rules
        // ignore these, as we just had them
        table.set("name", Nil)?;
        table.set("regex", Nil)?;

        // read the rest
        let mut rules = Vec::new();
        for val in table.sequence_values() {
            // parse the pair as a rule
            rules.push(Rule::from_table(lua, val?)?);
        }
        Ok(Highlighter {
            language: name,
            regex,
            rules,
        })
    }

    /// Check if the name or extension matches this highlighter
    pub(crate) fn match_filename(&self, name: &str) -> bool {
        self.language.to_lowercase() == name.to_lowercase() || self.regex.is_match(name)
    }

    /// Highlight code
    pub(crate) fn highlight(&self, mut code: &str, prefix: &str) -> Result<String> {
        let mut out = String::with_capacity(code.len());
        // TODO see https://github.com/Dimev/slsg/blob/f5d4ec56b868e54e4e65465f73de2256a64052a1/src/highlight.rs
        // TODO: inconsistent with how micro works
        // I think that instead simply goes to the next one except for matches
        // see how the code there works
        while !code.is_empty() {
            // find the closest match
            if let Some((rule, start, end)) = self
                .rules
                .iter()
                .filter_map(|x| {
                    let (start, end) = x.range(code)?;
                    Some((x, start, end))
                })
                .min_by_key(|x| x.1)
            {
                // split by what part we know and don't
                let up_to = &code[..start];
                let include = &code[start..end]; // TODO: proper
                let rest = &code[end..];

                // TODO: match so the end works
                // TODO: also include the inside of include
                // TODO: also escape html
                out.push_str(&format!(
                    "<span>{}</span><span class=\"{prefix}{}\">{}</span>",
                    escape_html(up_to),
                    rule.name(),
                    escape_html(include),
                ));
                code = rest;
            } else {
                // no match, push the rest
                // TODO html escape
                out.push_str(&format!("<span>{}</span>", escape_html(code)));
                code = "";
            }
        }

        // TODO: line numbers
        Ok(out)
    }
}

/// Escape html
fn escape_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    for c in html.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
