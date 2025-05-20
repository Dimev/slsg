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

        Ok(cfg)
    }
}
