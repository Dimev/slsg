use clap::Parser;
use serve::serve;
use std::path::PathBuf;

mod file;
mod generate;
mod serve;
mod pretty_print;

#[derive(Parser)]
enum Args {
    /// Build the site, and run a development server that reloads when a change is detected
    Dev {
        /// Optional path to either a directory containing `site.lua`, or a lua file that builds the site
        path: Option<PathBuf>,

        /// Address to serve on, defaults to 127.0.0.1:1111
        address: Option<String>
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

fn main() -> Result<(), anyhow::Error>{
    match Args::parse() {
        Args::Dev { path, address } => serve(path, address),
        Args::Build { path, output } => todo!(),
        Args::New { name } => todo!(),
        Args::Init => todo!(),
        Args::Api => todo!("Not done yet!"),
    }
}
