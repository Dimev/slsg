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
pub(crate) struct Highlighter {
    language: String,
    regex: Regex,
    rules: Vec<Rule>,
}

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
            let value = val?;

            // parse the pair as a rule
            rules.push(Rule::from_table(lua, value)?);
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
        // TODO
        Ok(code.into())
    }
}
