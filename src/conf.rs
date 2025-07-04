use mlua::Result;

#[derive(Clone, Debug)]
pub(crate) struct Config {
    /// What files to ignore
    pub ignore: Vec<String>,

    /// File to use for 404
    pub not_found: Option<String>,

    /// Output directory, defaults to dist (all files are automatically ignored there)
    pub output_dir: String,

    /// Whether to subset fonts
    pub subset: bool,

    /// extra characters to subset
    pub extra: String,

    /// Setup script to run first
    pub setup: Option<String>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Mode {
    Global,
    Build,
    Ignore,
    Font,
    Dev,
}

impl Config {
    pub(crate) fn parse(conf: &str) -> Result<Config> {
        let mut cfg = Config {
            ignore: Vec::new(),
            output_dir: "dist/".into(),
            not_found: None,
            subset: true,
            extra: String::new(),
            setup: None,
        };

        let mut mode = Mode::Global;

        for (num, line) in conf.lines().enumerate().map(|(n, l)| (n + 1, l)) {
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
            } else if line == "[ignore]" {
                mode = Mode::Ignore
            } else if line == "[dev]" {
                mode = Mode::Dev
            } else if line == "[font]" {
                mode = Mode::Font
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

                if key == "output" {
                    cfg.output_dir = value.into()
                } else if key == "setup" {
                    cfg.setup = Some(value.into());
                } else {
                    return Err(mlua::Error::external(format!(
                        "site.conf:{num}:1: Unrecognized key `{key}`"
                    )));
                }
            } else if mode == Mode::Ignore {
                // no key-values, just add directly
                cfg.ignore.push(line.into());
            } else if mode == Mode::Dev {
                // try and parse a line
                let (key, value) = line.split_once('=').ok_or(mlua::Error::external(format!(
                    "site.conf:{num}:1: Expected a `key = value` pair"
                )))?;

                // trim again
                let key = key.trim();
                let value = value.trim();

                if key == "not-found" {
                    cfg.not_found = Some(value.into());
                } else {
                    return Err(mlua::Error::external(format!(
                        "site.conf:{num}:1: Unrecognized key `{key}`"
                    )));
                }
            } else if mode == Mode::Font {
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

                if key == "subset" {
                    cfg.subset = as_bool?;
                } else if key == "extra" {
                    cfg.extra.push_str(value);
                } else {
                    return Err(mlua::Error::external(format!(
                        "site.conf:{num}:1: Unrecognized key `{key}`"
                    )));
                }
            } else if mode == Mode::Global {
                return Err(mlua::Error::external(format!(
                    "site.conf:{num}:1: Unexpected `key = value` pair outside of a section"
                )));
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
            output = out/ # we want another output

            [ignore]
            # ignore our fennel scripts and templates
            scripts/*
            templates/*
        ";

        let cfg = Config::parse(config).expect("Failed to parse");

        assert_eq!(cfg.output_dir, "out/");
        assert_eq!(
            cfg.ignore,
            vec!["scripts/*".to_string(), "templates/*".to_string()]
        );
    }

    #[test]
    fn parse_config_all() {
        let config = "
            [build]
            output = out/
            setup = script.lua

            [ignore]
            scripts/* # ignore our fennel scripts and templates
            
            [dev]
            not-found = 404.html

            [font]
            subset = false
            extra = abc
            extra = def
        ";

        let cfg = Config::parse(config).expect("Failed to parse");

        assert_eq!(cfg.output_dir, "out/".to_string());
        assert_eq!(cfg.not_found, Some("404.html".to_string()));
        assert_eq!(cfg.ignore, vec!["scripts/*".to_string()]);
        assert_eq!(cfg.subset, false);
        assert_eq!(cfg.extra, "abcdef".to_string());
        assert_eq!(cfg.setup, Some("script.lua".to_string()));
    }

    #[test]
    fn unknown_mode() {
        let config = "
            [weird!]            
        ";

        let err = Config::parse(config).expect_err("this is supposed to fail");

        assert_eq!(
            err.to_string(),
            "site.conf:2:1: Unrecognized section `weird!`".to_string()
        );
    }

    #[test]
    fn unknown_key_val() {
        let config = "
            global-val = true          
        ";

        let err = Config::parse(config).expect_err("this is supposed to fail");

        assert_eq!(
            err.to_string(),
            "site.conf:2:1: Unexpected `key = value` pair outside of a section".to_string()
        );

        let config = "
            [dev]
            unknown-val = true 
        ";

        let err = Config::parse(config).expect_err("this is supposed to fail");

        assert_eq!(
            err.to_string(),
            "site.conf:3:1: Unrecognized key `unknown-val`".to_string()
        );
    }
}
