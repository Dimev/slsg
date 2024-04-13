mod api;
mod cmd;
mod cookbook;
mod pretty_print;

use anyhow::anyhow;
use clap::Parser;
use cmd::{
    generate::{GenerateError, Site},
    serve::serve,
};
use cookbook::lookup;
use pretty_print::{print_entry, print_error, print_markdown};
use std::{env, fs, path::PathBuf};

use crate::{cookbook::entries, pretty_print::print_warning};

#[derive(Parser)]
enum Args {
    /// Build the site, from the site.toml file in the current or given directory
    Build {
        /// directory to the site.toml file of the site to build, current working directory by default
        dir: Option<PathBuf>,

        /// directory to write the resulting site to, public/ by default
        #[clap(short, long)]
        output: Option<PathBuf>,

        /// Whether we want to render a standalone lua file
        /// expects either a path to a directory with an index.lua file, allowing colocated files, or a single lua file, without colocated files
        #[clap(short, long, action)]
        standalone: bool,
    },

    /// Serve the site locally, for development
    /// Automatically reloads the site preview when a file in either the current directory, or given directory changes
    Dev {
        /// directory to the site.toml file of the site to build, current working directory by default
        dir: Option<PathBuf>,

        /// Adress to listen on for connections, defaults to 127.0.0.1:1111
        #[clap(short, long)]
        address: Option<String>,

        /// Whether we want to render a standalone lua file
        /// expects either a path to a directory with an index.lua file, allowing colocated files, or a single lua file, without colocated files
        #[clap(short, long, action)]
        standalone: bool,

        /// Whether to treat the generated site as a single page app, and reroute everything to index.html if the file is not found
        #[clap(long, action)]
        spa: bool,
    },

    /// Lookup an included example script
    Cookbook { name: Option<String> },

    /// Print the readme
    Readme,

    /// Create a new site
    New { path: PathBuf },

    /// Init a site in the current directory
    Init,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    // run the command
    match args {
        Args::Build {
            dir,
            output,
            standalone,
        } => match if standalone {
            Site::generate_standalone(dir, false)
        } else {
            Site::generate(dir, false)
        } {
            Ok(site) => {
                // success, print any warnings
                for warning in site.warnings.iter() {
                    print_warning(warning);
                }

                site.write_to_directory(output)?;
            }
            // Fail, print the errors
            Err(GenerateError { warnings, error }) => {
                print_error(&format!("{:?}", error));
                for warning in warnings {
                    print_warning(&warning);
                }
            }
        },
        Args::Dev {
            dir,
            address,
            standalone,
            spa,
        } => {
            serve(dir, address, standalone, spa)?;
        }
        Args::Cookbook { name } => {
            if let Some(entry) = name.and_then(|x| lookup(&x)) {
                print_entry(entry);
            } else {
                println!("Could not find the given entry. Available entries are:");
                for entry in entries() {
                    println!(" - {}: {}", entry.name, entry.description);
                }
            }
        }
        Args::Readme => print_markdown(include_str!("../README.md")),
        Args::New { path } => {
            init_folder(&path)?;
            println!("site created at {:?}", path);
        }
        Args::Init {} => {
            init_folder(&env::current_dir()?)?;
            println!("site created in current directory");
        }
    }

    Ok(())
}

fn init_folder(path: &PathBuf) -> Result<(), anyhow::Error> {
    // check if the directory is empty
    if let Ok(mut dir) = path.read_dir() {
        if dir.next().is_some() {
            Err(anyhow!("Directory is not empty!"))?
        }
    }

    // make the directory
    fs::create_dir_all(&path)?;

    // create the site directories
    fs::write(
        path.join("site.toml"),
        "# dev-404: \"404.html\"\n\n[config]\n",
    )?;

    fs::create_dir(path.join("site"))?;
    fs::write(
        path.join("site/index.lua"),
        "local html = h.p('Hello, world!'):renderHtml()\n\nreturn page():withHtml(html)",
    )?;

    // TODO: create example logo (single lssg logo)
    fs::create_dir(path.join("static"))?;

    // TODO: create example style (center the hello world text)
    fs::create_dir(path.join("styles"))?;

    // TODO: basic component script here
    fs::create_dir(path.join("scripts"))?;

    // gitignore to ignore public directory used for builds
    fs::write(path.join(".gitignore"), "public/\n")?;

    Ok(())
}
