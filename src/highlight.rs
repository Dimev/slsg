use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    ops::Range,
};

use mlua::{
    ErrorContext, ExternalResult, Lua, ObjectLike, Result, Table,
    Value::{self, Nil},
};
use regex::Regex;

// Inspired by the highlighter used by `micro`
// this allows reusing the syntax files, after converting them

thread_local! {
    /// Regex cache, to prevent them from compiling all at the start
    static REGEX_CACHE: RefCell<Vec<Regex>> = RefCell::new(Vec::new());

    /// Regex text to id
    static REGEX_RES: RefCell<BTreeMap<String, usize>> = RefCell::new(BTreeMap::new());
}

/// innner cached regex
#[derive(Clone)]
enum InnerRegex {
    New(String),
    Cached(usize),
}

impl InnerRegex {
    fn init(&mut self) -> Result<usize> {
        if let Self::New(re) = self
            && let Some(id) = REGEX_RES.with_borrow(|x| x.get(re).cloned())
        {
            // convert self to use the id
            *self = Self::Cached(id);
        } else if let Self::New(regex) = self {
            // create the regex
            let re = Regex::new(regex)
                .into_lua_err()
                .with_context(|_| format!("Failed to make regex `{regex}`"))?;

            // id is the end of the array
            let id = REGEX_CACHE.with_borrow(Vec::len);

            // add to the cache
            REGEX_CACHE.with_borrow_mut(|x| x.push(re));

            // add to the relations
            REGEX_RES.with_borrow_mut(|x| x.insert(regex.clone(), id));

            // convert self to use the id
            *self = Self::Cached(id);
        }

        match self {
            Self::Cached(id) => Ok(*id),
            _ => unreachable!(),
        }
    }

    fn find(&mut self, haystack: &str) -> Result<Option<Range<usize>>> {
        // build our regex
        let id = self.init()?;

        // use it
        Ok(REGEX_CACHE
            // find
            .with_borrow(|x| x[id].find(haystack))
            // get the range out
            .map(|x| x.range()))
    }

    fn is_match(&mut self, haystack: &str) -> Result<bool> {
        // build our regex
        let id = self.init()?;

        // use it
        Ok(REGEX_CACHE
            // find
            .with_borrow(|x| x[id].is_match(haystack)))
    }
}

/// Cached regex
#[derive(Clone)]
struct CachedRegex(RefCell<InnerRegex>);

impl CachedRegex {
    fn new(re: &str) -> Self {
        // put it in the cell
        Self(RefCell::new(InnerRegex::New(re.to_string())))
    }

    fn find(&self, haystack: &str) -> Result<Option<Range<usize>>> {
        self.0.borrow_mut().find(haystack)
    }

    fn is_match(&self, haystack: &str) -> Result<bool> {
        self.0.borrow_mut().is_match(haystack)
    }
}

/// Highlighter
#[derive(Clone)]
pub(crate) struct Highlighter {
    language: String,
    regex: CachedRegex,
    rules: Vec<Rule>,
}

#[derive(Clone)]
enum Rule {
    Match {
        re: CachedRegex,
        name: String,
    },
    // TODO: no inner rules here
    Complex {
        open: CachedRegex,
        close: CachedRegex,
        skip: Option<CachedRegex>,
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

    fn range(&self, mut text: &str) -> Result<Option<Range<usize>>> {
        match self {
            // simply the range
            Self::Match { re, .. } => re.find(text),
            _ => Ok(None),
            /*Self::Complex {
                open, close, skip, ..
            } => {
                // find the open
                let (open_start, open_end) = open.find(text)?;

                // move so it's relative
                text = &text[open_end..];

                // find the close
                let close = close.find(text)?;

                // start off
                let mut close_start = close.start();
                let mut close_end = close.end();

                // while there is a skip before our close, take that
                while let Some(skip) = skip
                    .as_ref()
                    .and_then(|x| x.find(text)?)
                    .filter(|x| x.start() <= close.start())
                    .filter(|_| false)
                {
                    // TODO
                }

                Some((open.start(), open.start() + close_end))
            }*/
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
                open: CachedRegex::new(&begin),
                close: CachedRegex::new(&end),
                skip: skip.map(|x| CachedRegex::new(&x)),
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
                re: CachedRegex::new(&table.get::<String>(1)?),
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
        let regex = CachedRegex::new(&regex);

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
    pub(crate) fn match_filename(&self, name: &str) -> Result<bool> {
        Ok(self.language.to_lowercase() == name.to_lowercase() || self.regex.is_match(name)?)
    }

    /// Highlight code
    pub(crate) fn highlight(&self, mut code: &str, prefix: &str) -> Result<String> {
        let mut out = String::with_capacity(code.len());
        // TODO see https://github.com/Dimev/slsg/blob/f5d4ec56b868e54e4e65465f73de2256a64052a1/src/highlight.rs
        // TODO: inconsistent with how micro works
        // I think that instead simply goes to the next one except for matches
        // see how the code there works
        // TODO: see https://github.com/zyedidia/micro/blob/master/pkg/highlight/highlighter.go#L113
        //
        while !code.is_empty() {
            // find the closest match
            if let Some((rule, start, end)) = self
                .rules
                .iter()
                .filter_map(|x| {
                    let range = x.range(code).ok()??;
                    Some((x, range.start, range.end))
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
