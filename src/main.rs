use std::{
    env::current_dir,
    ffi::OsString,
    fs::{create_dir_all, read_dir, remove_dir_all},
    path::{Path, PathBuf},
};

use mlua::{ErrorContext, ExternalResult, Lua, ObjectLike, Result, Table, chunk};

mod conf;
mod generate;
mod serve;
mod subset;

const HELP: &str = "\
SLSG - Scriptable Lua Site Generator

Usage:
  slsg dev [path] [--address]   Serve the site in path (default ./)
  slsg build [path] [--output]  Build the site in path (default ./)
  slsg new [path]               Create a new site in path
  slsg docs                     Show the documentation
  slsg help                     Show this screen

Options:
  -h --help     Show this screen
  -v --version  Print SLSG and luaJIT version

  -a --address  Address and port to use when hosting the dev server (default 127.0.0.1:1111)
  -o --output   Output directory to use when building the site (default path/dist/)
  --            Pass anything after this as arguments to the lua script (the ... table)
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
    match sub.as_deref() {
        Some("dev") => dev(&mut pargs),
        Some("build") => build(&mut pargs),
        Some("new") => new(pargs),
        Some("docs") => print_docs(),
        _ => println!("{}", HELP),
    }
}

/// Find the site.conf file
fn find_working_dir(path: &Path) -> Result<&Path> {
    if path.file_name() == Some(&OsString::from("site.conf")) {
        path.parent().ok_or(mlua::Error::external(
            "site.conf does not have a parent directory",
        ))
    } else {
        for ancestor in path.ancestors() {
            if ancestor.join("site.conf").exists() {
                return Ok(ancestor);
            }
        }

        Err(mlua::Error::external(format!(
            "site.conf does not exist in `{}` or any of it's ancestors",
            path.to_string_lossy()
        )))
    }
}

/// Create a new site
fn new(mut pargs: pico_args::Arguments) {
    let path = pargs
        .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
        .expect("Failed to parse arguments")
        .unwrap_or(PathBuf::from("."));

    // ensure the path does not exist yet
    if let Ok(mut dir) = path.read_dir() {
        if dir.next().is_some() {
            println!(
                "Failed to create new site: target directory {:?} is not empty!",
                path
            );
            return;
        }
    }

    todo!();

    // report success
    println!("Created new site in {:?}", path);
    println!("Run `slsg dev` in the directory to start making your site!");
}

/// Build an existing site
fn build(pargs: &mut pico_args::Arguments) {
    let output_path = pargs
        .opt_value_from_os_str::<_, PathBuf, String>(["-o", "--output"], |x| Ok(PathBuf::from(x)))
        .expect("Failed to parse arguments");

    let current_dir = current_dir().expect("oop");

    let path = pargs
        .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
        .expect("Failed to parse arguments")
        .unwrap_or_else(|| find_working_dir(&current_dir).expect("oop").to_path_buf());

    // force clear the directory, only if we are building the current site's ./dist folder
    // or are passed the --force argument
    let force_clear = pargs.contains(["-f", "--force"]) || output_path.is_some();

    // clear the output
    if let Some(ref output_path) = output_path {
        // only clear if it's allowed
        if force_clear && output_path.is_dir() {
            remove_dir_all(&output_path).unwrap_or_else(|e| {
                panic!(
                    "Failed to remove the content of output directory {:?}: {}",
                    output_path, e
                )
            });

        // else, crash if it's not empty
        } else if read_dir(&output_path)
            .map(|mut x| x.next().is_some())
            .unwrap_or(false)
        {
            panic!("Output directory has items in it!");
        }
    } else if path.join("dist").is_dir() {
        // remove if it's relative to the input, as we are in our own directory now
        remove_dir_all(path.join("dist")).unwrap_or_else(|e| {
            panic!(
                "Failed to remove the content of output directory {:?}: {}",
                output_path, e
            )
        });
    }

    // make sure the path exists
    create_dir_all(&output_path.as_ref().unwrap_or(&path.join("dist")))
        .unwrap_or_else(|e| panic!("Failed to create output directory {:?}: {}", output_path, e));

    // make it canonical
    todo!("read the config for this");
    let output_path = output_path
        .as_ref()
        .unwrap_or(&path.join("dist"))
        .canonicalize()
        .unwrap_or_else(|e| {
            panic!(
                "Failed to canonicalize output directory path {:?}: {}",
                output_path, e
            )
        });

    // move to where the main.lua file is
    if !path.is_dir() {
        panic!("Expected a directory for the path, not a file!");
    } else {
        std::env::set_current_dir(path).unwrap_or_else(|e| panic!("Failed to change path: {}", e));
    }

    // generate the site,
    /*match generate(false) {
        Ok(files) => {
            // write out all files
            for (path, file) in files {
                file.to_file(&output_path.join(path))
                    .expect("failed to output file");
            }
        }
        Err(err) => print_error("Failed to build site", &err),
    }*/
}

/// Serve an existing site with the development server
fn dev(pargs: &mut pico_args::Arguments) {
    let addr = pargs
        .opt_value_from_str(["-a", "--address"])
        .expect("Failed to parse arguments")
        .unwrap_or(String::from("127.0.0.1:1111"));

    let current_dir = current_dir().expect("oop");

    let path = pargs
        .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
        .expect("Failed to parse arguments")
        .unwrap_or_else(|| find_working_dir(&current_dir).expect("oop").to_path_buf());

    // move to where the main.lua file is
    if !path.is_dir() {
        panic!("Expected a directory for the path, not a file!");
    } else {
        std::env::set_current_dir(path).unwrap_or_else(|e| panic!("Failed to change path: {}", e));
    }

    // run the development server
    serve::serve(&addr).unwrap();
    todo!("result");
}

fn print_docs() {
    todo!();
}
