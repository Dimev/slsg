use std::{
    env::current_dir,
    ffi::OsString,
    fs::{self, create_dir_all, read_dir, remove_dir_all},
    path::{Path, PathBuf},
    time::Instant,
};

use generate::generate;
use mlua::{ErrorContext, ExternalResult, Lua, Result, chunk};
use print::print_error;

mod font;
mod generate;
mod highlight;
mod markdown;
mod path;
mod print;
mod serve;
mod templates;

const HELP: &str = "\
SLSG - Scriptable Lua Site Generator

Usage:
  slsg dev [path] [--address]   Serve the site in path (default ./)
  slsg build [path] [--output]  Build the site in path (default ./)
  slsg new <template> [path]    Create a new site in path
  slsg docs                     Show the documentation
  slsg help                     Show this screen

Options:
  -h --help     Show this screen
  -v --version  Print SLSG and luaJIT version
     --verbose  Print out extra information when building

  -a --address  Where to bind the dev server to (default 127.0.0.1:1111)
  -o --output   Where to output the files to (default dist/)
";

fn main() {
    let mut pargs = pico_args::Arguments::from_env();

    // print help
    if pargs.contains(["-h", "--help"]) {
        println!("{}", HELP);
        return;
    }

    // print version
    if pargs.contains(["-v", "--version"]) {
        println!("SLSG {}", env!("CARGO_PKG_VERSION"));

        // luajit version
        let lua = unsafe { Lua::unsafe_new() };
        let version: String = lua
            .load("jit.version")
            .eval::<String>()
            .expect("Failed to get LuaJIT version");
        println!("{}", version);

        // install fennel to get it's version
        let fennel = include_str!("fennel.lua");
        let fennel = lua
            .load(fennel)
            .into_function()
            .expect("Failed to load fennel");
        let version = lua
            .load(chunk! {
                package.preload["fennel"] = $fennel
                return require("fennel").version
            })
            .eval::<String>()
            .expect("failed to install fennel");
        println!("Fennel {}", version);

        return;
    }

    let sub = pargs.subcommand().expect("Failed to parse arguments");
    let err = match sub.as_deref() {
        Some("dev") => dev(pargs),
        Some("build") => build(pargs),
        Some("new") => new(pargs),
        Some("docs") => print_docs(),
        _ => Ok(println!("{}", HELP)),
    };

    // report error
    if let Err(e) = err {
        print_error("Failed", &e);
    }
}

/// Create a new site
fn new(mut pargs: pico_args::Arguments) -> Result<()> {
    let path = pargs
        .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
        .into_lua_err()
        .context("Failed to parse arguments")?
        .unwrap_or(PathBuf::from("."));

    // ensure the path does not exist yet
    if let Ok(mut dir) = path.read_dir() {
        if dir.next().is_some() {
            println!(
                "Failed to create new site: target directory {:?} is not empty!",
                path
            );
            //return;
        }
    }

    todo!();

    // report success
    println!("Created new site in {:?}", path);
    println!("Run `slsg dev` in the directory to start making your site!");
}

/// Find the site.conf file
fn find_working_dir(path: &Path) -> Result<&Path> {
    if path.file_name() == Some(&OsString::from("site.lua")) {
        path.parent().ok_or(mlua::Error::external(
            "`site.lua` does not have a parent directory",
        ))
    } else if path.file_name() == Some(&OsString::from("site.fnl")) {
        path.parent().ok_or(mlua::Error::external(
            "`site.fnl` does not have a parent directory",
        ))
    } else {
        for ancestor in path.ancestors() {
            if ancestor.join("site.lua").exists() || ancestor.join("site.fnl").exists() {
                return Ok(ancestor);
            }
        }

        Err(mlua::Error::external(format!(
            "`site.lua` or `site.fnl` does not exist in `{}` or any of it's ancestors",
            path.to_string_lossy()
        )))
    }
}

/// Build an existing site
fn build(mut pargs: pico_args::Arguments) -> Result<()> {
    let current_dir = current_dir()
        .into_lua_err()
        .context("could not open current directory")?;

    // parse these first to not get confused with the positional arg
    let output_path = pargs
        .opt_value_from_os_str::<_, PathBuf, String>(["-o", "--output"], |x| Ok(PathBuf::from(x)))
        .into_lua_err()
        .context("Failed to parse arguments")?;

    // verbose output?
    let verbose = pargs.contains("--verbose");

    // force clear the directory, only if we are building the current site's ./dist folder
    // or are passed the --force argument
    let force_clear = pargs.contains(["-f", "--force"]);

    let path = if let Some(path) = pargs
        .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
        .into_lua_err()
        .context("Failed to parse arguments")?
    {
        path
    } else {
        find_working_dir(&current_dir)
            .map(|x| x.to_path_buf())
            .context("Failed to find working directory")?
    };

    let output_path = output_path.unwrap_or(path.join(".dist"));

    // clear the output
    // only clear if it's allowed, or it's the output path
    if force_clear && output_path.is_dir()
        || output_path.is_dir() && output_path == path.join(".dist")
    {
        remove_dir_all(&output_path)
            .into_lua_err()
            .with_context(|_| {
                format!(
                    "Failed to remove content of output directory `{}`",
                    output_path.to_string_lossy()
                )
            })?;

    // else, crash if it's not empty
    } else if read_dir(&output_path)
        .map(|mut x| x.next().is_some())
        .unwrap_or(false)
    {
        return Err(mlua::Error::external(format!(
            "Output directory `{}` is not empty, use --force to overwrite",
            output_path.to_string_lossy()
        )));
    }

    // start timing
    let start = Instant::now();

    // make sure the path exists
    create_dir_all(&output_path)
        .into_lua_err()
        .with_context(|_| {
            format!(
                "Failed to create output directory `{}`",
                output_path.to_string_lossy()
            )
        })?;

    // make it canonical
    let output_path = output_path
        .canonicalize()
        .into_lua_err()
        .with_context(|_| {
            format!(
                "Failed to canonicalize output directory path `{}`",
                output_path.to_string_lossy()
            )
        })?;

    // move to where the main.lua file is
    std::env::set_current_dir(&path)
        .into_lua_err()
        .with_context(|_| format!("Failed to change path to `{}`", path.to_string_lossy()))?;

    // generate the site,
    let site = generate(false)?;
    let mut count = 0;
    let mut size = 0;
    for (file_path, contents) in site.files.into_iter() {
        count += 1;
        size += contents.len();

        // create the directory for it
        create_dir_all(
            &file_path
                .to_path(&output_path)
                .parent()
                .ok_or(mlua::Error::external(format!(
                    "output path `{}` could not be created",
                    file_path.to_path(&output_path).to_string_lossy()
                )))?,
        )
        .into_lua_err()
        .with_context(|_| {
            format!(
                "output path `{}` could not be created",
                file_path.to_path(&output_path).to_string_lossy()
            )
        })?;

        // write the file
        fs::write(file_path.to_path(&output_path), contents)
            .into_lua_err()
            .with_context(|_| {
                format!(
                    "Failed to write file `{}`",
                    file_path.to_path(&output_path).to_string_lossy()
                )
            })?;
    }

    // report info, if verbose
    if verbose {
        let size = size as f64 / 1000.0;

        // pick largest size to use for representation
        // if bigger than one mb, scale down
        let megabytes = if size > 1000.0 { true } else { false };
        let size = if megabytes { size / 1000.0 } else { size };

        // and if bigger than a gb, scale down
        let gigabytes = if size > 1000.0 { true } else { false };
        let size = if gigabytes { size / 1000.0 } else { size };

        // pick unit
        let unit = if gigabytes {
            "gb"
        } else if megabytes {
            "mb"
        } else {
            "kb"
        };

        println!(
            "took {}ms - {count} file{} - {size:.2}{unit}",
            start.elapsed().as_millis(),
            if count > 1 { "s" } else { "" },
        );
    }

    Ok(())
}

/// Serve an existing site with the development server
fn dev(mut pargs: pico_args::Arguments) -> Result<()> {
    let addr = pargs
        .opt_value_from_str(["-a", "--address"])
        .into_lua_err()
        .context("Failed to parse arguments")?
        .unwrap_or(String::from("127.0.0.1:1111"));

    let current_dir = current_dir()
        .into_lua_err()
        .context("could not open current directory")?;

    let path = if let Some(path) = pargs
        .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
        .into_lua_err()
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
        .into_lua_err()
        .with_context(|_| format!("Failed to change path to `{}`", path.to_string_lossy()))?;

    // run the development server
    serve::serve(&addr)?;
    println!("Stopped (ctrl-c)");
    Ok(())
}

fn print_docs() -> Result<()> {
    todo!();
}
