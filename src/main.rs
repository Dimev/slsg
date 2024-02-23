mod api;
mod cmd;
mod pretty_print;
use clap::Parser;
use cmd::{generate::Site, serve::serve};
use pretty_print::print_error;
use std::path::PathBuf;

use crate::pretty_print::print_warning;

#[derive(Parser)]
enum Args {
    /// Build the site, from the site.toml file in the current or given directory
    Build {
        /// directory to the site.toml file of the site to build, current working directory by default
        #[clap(short, long)]
        dir: Option<PathBuf>,

        /// directory to write the resulting site to, public/ by default
        #[clap(short, long)]
        output: Option<PathBuf>,
    },

    /// Serve the site locally, for development
    /// Automatically reloads the site preview when a file in either the current directory, or given directory changes
    Dev {
        /// directory to the site.toml file of the site to build, current working directory by default
        #[clap(short, long)]
        dir: Option<PathBuf>,

        /// Adress to listen on for connections, defaults to 127.0.0.1:1111
        #[clap(short, long)]
        address: Option<String>,
    },

    /// List example scripts
    Cookbook {},

    /// Create a new site
    New {},

    /// Init a site in the current directory
    Init {},
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    // run the command
    match args {
        Args::Build { dir, output } => match Site::generate(dir) {
            Ok(site) => {
                // success, print any warnings
                for warning in site.warnings.iter() {
                    print_warning(&warning);
                }

                site.write_to_directory(output)?;
            }
            // Fail, print the errors
            Err(e) => print_error(&format!("{:?}", e)),
        },
        Args::Dev { dir, address } => {
            serve(dir, address)?;
        }
        Args::Cookbook {} => todo!(),
        Args::New {} => todo!(),
        Args::Init {} => todo!(),
    }

    Ok(())
}
