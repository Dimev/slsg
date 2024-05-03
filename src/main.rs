use clap::Parser;
use std::path::PathBuf;

mod file;
mod generate;

#[derive(Parser)]
enum Args {
    /// Build the site, and run a development server that reloads when a change is detected
    Dev {
        /// Optional path to either a directory containing `site.lua`, or a lua file that builds the site
        path: Option<PathBuf>,
    },

    /// Build the site, and output the resulting files to a directory
    Build {
        /// Optional path to either a directory containing `site.lua`, or a lua file that builds the site
        path: Option<PathBuf>,

        /// Optional path to the directory to put the resulting files in, defaults to public/
        output: Option<PathBuf>,
    },

    /// Create a new site with the given name
    New { name: String },

    /// Init the current directory as a site
    Init,

    /// Show the API documentation (README)
    Api,
}

fn main() {
    match Args::parse() {
        Args::Dev { path } => todo!("Not done yet!"),
        Args::Build { path, output } => todo!(),
        Args::New { name } => todo!(),
        Args::Init => todo!(),
        Args::Api => todo!("Not done yet!"),
    }
}
