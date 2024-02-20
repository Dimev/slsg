mod api;
mod cmd;

use clap::Parser;
use cmd::{generate::Site, serve::serve};
use std::path::PathBuf;

#[derive(Parser)]
enum Args {
    /// Build the site
    Build {
        /// directory to the site.toml file of the site to build, current working directory by default
        #[clap(short, long)]
        dir: Option<PathBuf>,

        /// directory to write to, public/ by default
        #[clap(short, long)]
        output: Option<PathBuf>,
    },

    /// Serve the site locally, for development
    Dev {
        /// directory to the site.toml file of the site to build, current working directory by default
        #[clap(short, long)]
        dir: Option<PathBuf>,
    },

    /// List example scripts
    Cookbook {},
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    // run the command
    match args {
        Args::Build { dir, output } => {
            let site = Site::generate(dir)?;

            for warning in site.warnings.iter() {
                println!("[WARN] {}", warning);
            }

            site.write_to_directory(output)?;
        }
        Args::Dev { dir } => {
            serve(dir)?;
        }
        Args::Cookbook {} => {}
    }

    Ok(())
}
