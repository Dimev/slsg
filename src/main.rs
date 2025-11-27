use std::{
    env::current_dir,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use mlua::Lua;
use print::print_error;

use anyhow::{Context, Result, anyhow};

use crate::{dev::serve_dev_site, print::print_success, site::SiteCache};

mod template;
mod dev;
mod font;
mod print;
mod site;

const HELP: &str = "\
SLSG - Scriptable Lua Site Generator

Usage:
  slsg dev [path] [--address]             Serve the site in path
  slsg build [path] [--output] [--force]  Build the site in path
  slsg new <language> [path]              Create a new site at path
  slsg docs                               Show the documentation
  slsg help                               Show this screen

Options:
  [path]        Where to load the site from, defaults to ./
  -a --address  Where to bind the dev server to (defaults to 127.0.0.1:1111)
  -o --output   Where to output the files to (defaults to .dist/)
  -f --force    Force overwrite the output directory.

  -h --help     Show this screen, and the extra help screen
  -v --version  Print SLSG, LuaJIT, and etlua versions
";
const MANUAL: &str = "\
Lua API:
";

fn main() {
    let mut pargs = pico_args::Arguments::from_env();

    // print help
    if pargs.contains(["-h", "--help"]) {
        println!("{}", HELP);
        println!("{}", MANUAL);
        return;
    }

    // print version
    if pargs.contains(["-v", "--version"]) {
        println!("SLSG v{}", env!("CARGO_PKG_VERSION"));

        // luajit version
        let lua = unsafe { Lua::unsafe_new() };
        let version: String = lua
            .load("jit.version")
            .eval::<String>()
            .expect("Failed to get LuaJIT version");
        println!("{}", version);

        // TODO library versions?

        return;
    }

    let sub = pargs.subcommand().expect("Failed to parse arguments");
    let err = match sub.as_deref() {
        Some("dev") => dev(pargs).context("Could not run dev server"),
        Some("build") => build(pargs).context("Could not build site"),
        Some("new") => new(pargs).context("Could not create new site"),
        Some("docs") => print_docs(),
        _ => Ok(println!("{}", HELP)),
    };

    // report error
    if let Err(e) = err {
        print_error("Failed", &format!("{:?}", e));
    }
}

/// Create a new site
fn new(mut pargs: pico_args::Arguments) -> Result<()> {
    // read where we make the site, or the current directory if none are given
    let path = pargs
        .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
        .context("Failed to parse arguments")?
        .unwrap_or(PathBuf::from("."));

    // ensure the path does not exist yet
    if let Ok(mut dir) = path.read_dir() {
        if dir.next().is_some() {
            return Err(anyhow!(
                "Failed to create new site: target directory '{}' is not empty!",
                path.display()
            ));
        }
    } else {
        fs::create_dir_all(&path).context("Failed to create new site directory")?;
    }

    // make the template
    todo!();

    // report success
    print_success(
        &format!("Created a new site in '{}'", path.display()),
        &"Run 'slsg dev' in the directory to run the dev server",
    );

    Ok(())
}

/// Find the site.conf file
fn find_working_dir(path: &Path) -> Result<&Path> {
    if path.file_name() == Some(&OsString::from("site.lua")) {
        path.parent()
            .ok_or_else(|| anyhow!("'site.lua' does not have a parent directory",))
    } else {
        for ancestor in path.ancestors() {
            if ancestor.join("site.lua").exists() {
                return Ok(ancestor);
            }
        }

        Err(anyhow!(
            "'site.lua' does not exist in '{}' or any of it's ancestors",
            path.display()
        ))
    }
}

/// Build an existing site
fn build(mut pargs: pico_args::Arguments) -> Result<()> {
    let current_dir = current_dir().context("could not open current directory")?;

    // parse these first to not get confused with the positional arg
    let output_path = pargs
        .opt_value_from_os_str::<_, PathBuf, String>(["-o", "--output"], |x| Ok(PathBuf::from(x)))
        .context("Failed to parse arguments")?;

    // force clear the directory, only if we are building the current site's ./dist folder
    // or are passed the --force argument
    let force_clear = pargs.contains(["-f", "--force"]);

    let path = if let Some(path) = pargs
        .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
        .context("Failed to parse arguments")?
    {
        path
    } else {
        find_working_dir(&current_dir)
            .map(|x| x.to_path_buf())
            .context("Failed to find working directory")?
    };

    let (output_path, force_clear) = output_path
        .map(|x| (x, force_clear))
        // force clear if we write to .dist in the project directory
        .unwrap_or((path.join(".dist"), true));

    // make it canonical
    let output_path = std::env::current_dir()
        .context("Failed to get current directory")?
        .join(output_path);

    // move to where the main.lua file is
    std::env::set_current_dir(&path)
        .with_context(|| format!("Failed to change path to '{}'", path.display()))?;

    // generate the site, and write out the files
    SiteCache::new_site()?.write_to_path(&output_path, force_clear)
}

/// Serve an existing site with the development server
fn dev(mut pargs: pico_args::Arguments) -> Result<()> {
    let addr = pargs
        .opt_value_from_str(["-a", "--address"])
        .context("Failed to parse arguments")?
        .unwrap_or(String::from("127.0.0.1:1111"));

    let current_dir = current_dir().context("Could not open current directory")?;

    let path = if let Some(path) = pargs
        .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
        .context("Failed to parse arguments")?
    {
        path
    } else {
        find_working_dir(&current_dir)
            .map(|x| x.to_path_buf())
            .context("Failed to find working directory")?
    };

    // move to where the main.lua file is
    std::env::set_current_dir(&path)
        .with_context(|| format!("Failed to change path to '{}'", path.display()))?;

    // run the development server
    serve_dev_site(&addr)?;
    println!("Stopped (ctrl-c)");
    Ok(())
}

fn print_docs() -> Result<()> {
    todo!();
}
