use mlua::Result;

#[derive(Clone, Debug)]
pub(crate) struct Config {
    /// What files to ignore
    pub ignore: Vec<String>,

    /// What extra syntaxes to load
    pub syntaxes: Vec<String>,

    /// File to use for 404
    pub not_found: Option<String>,

    /// Output directory, defaults to dist (all files are automatically ignored there)
    pub output_dir: String,

    /// Whether to enable fennel (<?fnl ... ?>), true by default
    pub fennel: bool,

    /// Whether to enable minimark (.mmk files, and <?mmk ?>), true by default
    pub minimark: bool,

    /// Whether to enable lua (<?lua ... ?>), true by default
    pub lua: bool,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Mode {
    Global,
    Highlight,
    Build,
    Ignore,
    Dev,
}

impl Config {
    pub(crate) fn parse(conf: &str) -> Result<Config> {
        let mut cfg = Config {
            ignore: Vec::new(),
            syntaxes: Vec::new(),
            output_dir: "dist/".into(),
            not_found: None,
            fennel: true,
            minimark: true,
            lua: true,
        };

        let mut mode = Mode::Global;

        for (num, line) in conf.lines().enumerate() {
            // split out comment
            let (line, _) = line.split_once('#').unwrap_or((line, ""));
            let line = line.trim();

            // skip if line is empty
            if line.is_empty() {
                continue;
            }
            // try and detect mode change
            else if line == "[build]" {
                mode = Mode::Build
            } else if line == "[highlight]" {
                mode = Mode::Highlight
            } else if line == "[ignore]" {
                mode = Mode::Ignore
            } else if line == "[dev]" {
                mode = Mode::Ignore
            } else if let Some(mode) = line.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
                return Err(mlua::Error::external(format!(
                    "site.conf:{num}:1: Unrecognized section `{mode}`"
                )));
            } else if mode == Mode::Build {
                // try and parse a line
                let (key, value) = line.split_once('=').ok_or(mlua::Error::external(format!(
                    "site.conf:{num}:1: Expected a `key = value` pair"
                )))?;

                // trim again
                let key = key.trim();
                let value = value.trim();

                // here we expect key-value pairs
                let as_bool = Some(value == "true")
                    .filter(|_| ["true", "false"].contains(&value))
                    .ok_or(mlua::Error::external(format!(
                        "site.conf:{num}:1: Expected a `key = value` pair"
                    )));

                if key == "dir" {
                    cfg.output_dir = value.into()
                } else if key == "allow-lua" {
                    cfg.lua = dbg!(as_bool?)
                } else if key == "allow-fennel" {
                    cfg.fennel = as_bool?
                } else if key == "allow-minimark" {
                    cfg.minimark = as_bool?
                } else {
                    return Err(mlua::Error::external(format!(
                        "site.conf:{num}:1: Unrecognized key `{key}`"
                    )));
                }
            } else if mode == Mode::Ignore {
                // no key-values, just add directly
                cfg.ignore.push(line.into());
            } else if mode == Mode::Highlight {
                cfg.syntaxes.push(line.into());
            } else if mode == Mode::Dev {
                // try and parse a line
                let (key, value) = line.split_once('=').ok_or(mlua::Error::external(format!(
                    "site.conf:{num}:1: Expected a `key = value` pair"
                )))?;

                // trim again
                let key = key.trim();
                let value = value.trim();

                
            }
        }

        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config() {
        let config = "
            [build]
            allow-lua = false # we don't want lua

            [ignore]
            scripts/* # ignore our fennel scripts and templates
            syntax/* # ignore our syntax files

            [highlight]
            syntax/
            
        ";

        let cfg = Config::parse(config).expect("Failed to parse");

        assert_eq!(cfg.lua, false);
        assert_eq!(
            cfg.ignore,
            vec!["scripts/*".to_string(), "syntax/*".to_string()]
        );
        assert_eq!(cfg.syntaxes, vec!["syntax/".to_string()]);
    }

    #[test]
    fn parse_config_all() {
        let config = "
            [build]
            dir = out/
            allow-fennel = false
            allow-lua = false # we don't want lua
            allow-minimark = false 

            [ignore]
            scripts/* # ignore our fennel scripts and templates
            syntax/* # ignore our syntax files

            [highlight]
            syntax/

            [dev]
            not-found = 404.html
            
        ";

        let cfg = Config::parse(config).expect("Failed to parse");

        assert_eq!(cfg.fennel, false);
        assert_eq!(cfg.lua, false);
        assert_eq!(cfg.minimark, false);
        assert_eq!(cfg.output_dir, "out/".to_string());
        assert_eq!(cfg.not_found, Some("404.html".to_string()));
        assert_eq!(
            cfg.ignore,
            vec!["scripts/*".to_string(), "syntax/*".to_string()]
        );
        assert_eq!(cfg.syntaxes, vec!["syntax/".to_string()]);
    }
}
