use anyhow::{anyhow, Context};
use clap::Parser;
use generate::generate;
use pretty_print::{print_error, print_warning};
use serve::serve;
use std::{env, fs, path::PathBuf};

use crate::path::resolve_path;

mod file;
mod generate;
mod highlight;
mod texparse;
mod path;
mod pretty_print;
mod serve;

#[derive(Parser)]
enum Args {
    /// Build the site, and run a development server that reloads when a change is detected
    Dev {
        /// Optional path to either a directory containing `site.lua`, or a lua file that builds the site
        path: Option<PathBuf>,

        /// Address to serve on, defaults to 127.0.0.1:1111
        #[clap(short, long)]
        address: Option<String>,
    },

    /// Build the site, and output the resulting files to a directory
    Build {
        /// Optional path to either a directory containing `site.lua`, or a lua file that builds the site
        path: Option<PathBuf>,

        /// Optional path to the directory to put the resulting files in, defaults to public/
        #[clap(short, long)]
        output: Option<PathBuf>,
    },

    /// Create a new site on the given path
    New { path: PathBuf },

    /// Init the current directory as a site
    Init,

    /// Show the API documentation (README)
    Api,
}

fn main() -> Result<(), anyhow::Error> {
    match Args::parse() {
        Args::Dev { path, address } => serve(path, address),
        Args::Build { path, output } => {
            match generate(&path.clone().unwrap_or(PathBuf::new()), false) {
                Ok(x) => {
                    // path to write all files to
                    let root = output.unwrap_or(path.unwrap_or(PathBuf::new()).join("public"));

                    // show all warnings
                    for warning in x.warnings {
                        print_warning(&warning);
                    }

                    // remove all existing files there
                    if root.exists() {
                        fs::remove_dir_all(&root)?;
                    }

                    for (path, file) in x.files {
                        // resolve the path relative to the root
                        let path = root.join(resolve_path(&path).ok_or(anyhow!("Invalid path!"))?);
                        
                        // create all dirs
                        fs::create_dir_all(path.parent().expect("No parent for the file path!")).context("adawhidawiuhd")?;

                        // write out the path
                        file.write(path).context("no file")?;
                    }

                    println!("Site written to {root:?}");
                }
                Err(x) => {
                    print_error(&format!("{:?}", x.error));
                    for warning in x.warnings {
                        print_warning(&warning);
                    }
                }
            }

            Ok(())
        }
        Args::New { path } => {
            init_folder(&path)?;
            println!("Created new site at {:?}", path);
            Ok(())
        }
        Args::Init => {
            init_folder(&env::current_dir()?)?;
            println!("Created new site in current directory");
            Ok(())
        }
        // TODO
        Args::Api => Ok(println!("Still needs to be implemented!")),
    }
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

    fs::write(path.join("index.lua"), EX_LUA)?;
    fs::write(
        path.join("logo.svg"),
        include_str!("../example/static/logo.svg"),
    )?;
    fs::write(path.join("style.scss"), EX_CSS)?;

    // gitignore to ignore public directory used for builds
    fs::write(path.join(".gitignore"), "public/\n")?;

    Ok(())
}

const EX_LUA: &'static str = "-- parse the sass and make the CSS
local css = site.parseSass(site.loadFile('style.scss'))

-- Render the HTML
local html = fragment(
    -- inline CSS as style
    h.style(css),
    h.title('My website'),
    h.div():sub(
        h.h1('Hello, world!'),
        -- logo
        h.img():attrs({ class = 'logo', alt = 'logo', src = 'logo.svg' })
    )
):renderHtml()

-- render all files
return filetree()
    :withFile('index.html', html)
    :withFile('logo.svg', site.loadFile('logo.svg'))
";

const EX_CSS: &'static str = "html {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100vh;
    font-family: sans-serif;
}
";
