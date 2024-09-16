use std::{ffi::OsString, path::PathBuf};

use generate::{generate, Output};
use message::print_error;
use mlua::ErrorContext;
use serve::serve;

mod generate;
mod message;
mod serve;
mod stdlib;

// TODO: https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap12.html#tag_12_01
const HELP: &str = "\
Scriptable Lua Site Generator

Usage:
  slsg dev [path] [--address]
  slsg build [path] [--output]
  slsg new [path]
  slsg api

Options:
  -h --help     Show this screen
  -v --version  Print version and quit
  -a --address  Address and port to use when hosting the dev server
  -o --output   Output directory to use when building the site
";

const NEW_LUA: &str = include_str!("new.lua");
const NEW_META: &str = include_str!("meta.lua");

const NEW_GITIGNORE: &str = "\
public
";

const API_DOCS: &[(&str, &str)] = &[];

fn main() {
    let mut pargs = pico_args::Arguments::from_env();

    // print help
    if pargs.contains(["-h", "--help"]) {
        println!("{}", HELP);
        return;
    }

    // print version
    if pargs.contains(["-v", "--version"]) {
        println!("slsg {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let sub = pargs.subcommand().expect("Failed to parse arguments");
    match sub.as_deref() {
        Some("dev") => {
            let addr = pargs
                .opt_value_from_str(["-a", "--address"])
                .expect("Failed to parse arguments")
                .unwrap_or(String::from("127.0.0.1:1111"));

            let path = pargs
                .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
                .expect("Failed to parse arguments")
                .unwrap_or(PathBuf::from("."));

            serve(&path, addr);
        }
        Some("build") => {
            let output_path = pargs
                .opt_value_from_os_str::<_, OsString, String>(["-o", "--output"], |x| {
                    Ok(OsString::from(x))
                })
                .expect("Failed to parse arguments")
                .unwrap_or(OsString::from("./public"));

            let path = pargs
                .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
                .expect("Failed to parse arguments")
                .unwrap_or(PathBuf::from("."));

            match generate(path.as_path(), false) {
                Ok(files) => {
                    todo!()
                }
                Err(err) => print_error(&err.context("Failed to build site")),
            }
        }
        Some("new") => {
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

            // make the directories
            std::fs::create_dir_all(&path)
                .unwrap_or_else(|_| panic!("Failed to create directory {:?}", path));

            // example file
            std::fs::write(path.join("main.lua"), NEW_LUA)
                .unwrap_or_else(|_| panic!("Failed to create file {:?}", path.join("main.lua")));
            std::fs::write(path.join("stdlib.meta"), NEW_META).unwrap_or_else(|_| {
                panic!("Failed to create directory {:?}", path.join("stdlib.meta"))
            });
            std::fs::write(path.join(".gitignore"), NEW_GITIGNORE).unwrap_or_else(|_| {
                panic!("Failed to create directory {:?}", path.join(".gitignore"))
            });

            println!("Created new site in {:?}", path);
            println!("Run `slsl dev` in the directory to start making your site!");
        }
        Some("api") => {}
        _ => println!("{}", HELP),
    }
}
