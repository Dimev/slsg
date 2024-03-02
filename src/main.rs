mod api;
mod cmd;
mod pretty_print;
use clap::Parser;
use cmd::{generate::Site, serve::serve};
use pretty_print::print_error;
use std::{env, fs, path::PathBuf};

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

    /// Lookup an included example script
    Cookbook {},

    /// Create a new site
    New { path: PathBuf },

    /// Init a site in the current directory
    Init {},
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    // run the command
    match args {
        Args::Build { dir, output } => match Site::generate(dir, false) {
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
        Args::New { path } => {
            // check if the directory is empty
            if !path.read_dir()?.next().is_none() {
                println!("{:?} is not empty!", path);
            }

            // make the directory
            fs::create_dir_all(&path)?;

            // create the site directories
            fs::File::create(path.join("site.toml"))?;

            // TODO: create example site (single hello world page)
            fs::create_dir(path.join("site"))?;

            // TODO: create example logo (single yassg logo)
            fs::create_dir(path.join("static"))?;

            // TODO: create example style (center the hello world text)
            fs::create_dir(path.join("styles"))?;

            // TODO: dump cookbook scripts here
            fs::create_dir(path.join("scripts"))?;

            println!("site created at {:?}", path);
        }
        Args::Init {} => {
            // check if the directory is empty
            if !env::current_dir()?.read_dir()?.next().is_none() {
                println!("Current directory is not empty!");
            }
            // create the site directories
            fs::File::create(PathBuf::from("site.toml"))?;
            fs::create_dir(PathBuf::from("site"))?;
            fs::create_dir(PathBuf::from("static"))?;
            fs::create_dir(PathBuf::from("styles"))?;
            fs::create_dir(PathBuf::from("scripts"))?;

            println!("site created in current directory");
        }
    }

    Ok(())
}
