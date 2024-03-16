/// Cookbook entry
pub(crate) struct Entry {
    /// name of the entry
    pub(crate) name: &'static str,

    /// Short description
    pub(crate) description: &'static str,

    /// elaborate description
    pub(crate) tutorial: &'static str,

    /// Code
    pub(crate) code: &'static str,
}

pub(crate) fn entries() -> Vec<Entry> {
    vec![Entry {
        name: "markdown",
        description: "Custom markdown rendering",
        tutorial: "By default, markdown is rendered straight to HTML. The following script allows custom rendering of markdown",
        code: include_str!("manual_markdown.lua")
    },
    Entry {
        name: "highlighting",
        description: "Syntax highlighting",
        tutorial: "lssg can do syntax highlighting",
        code: r#"local x = "None yet""#,    
    }]
}

/// All cookbook entries
pub(crate) fn lookup(name: &str) -> Option<Entry> {
    entries().into_iter().find(|x| x.name == name)
}
