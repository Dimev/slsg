use std::{
    fs::{create_dir_all, read_dir, remove_dir_all},
    path::PathBuf,
};

use generate::generate;
use message::print_error;
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

            // move to where the main.lua file is
            if !path.is_dir() {
                panic!("Expected a directory for the path, not a file!");
            } else {
                std::env::set_current_dir(path)
                    .unwrap_or_else(|e| panic!("Failed to change path: {}", e));
            }

            // run the development server
            serve(addr);
        }
        Some("build") => {
            let output_path = pargs
                .opt_value_from_os_str::<_, PathBuf, String>(["-o", "--output"], |x| {
                    Ok(PathBuf::from(x))
                })
                .expect("Failed to parse arguments");

            let path = pargs
                .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
                .expect("Failed to parse arguments")
                .unwrap_or(PathBuf::from("."));

            // force clear the directory, only if we are building the current site's ./public folder
            // or are passed the --force argument
            let force_clear = pargs.contains(["-f", "--force"]) || output_path.is_some();

            // clear the output
            if let Some(ref output_path) = output_path {
                // only clear if it's allowed
                if force_clear {
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
            } else {
                // remove if it's relative to the input, as we are in our own directory now
                remove_dir_all(path.join("public")).unwrap_or_else(|e| {
                    panic!(
                        "Failed to remove the content of output directory {:?}: {}",
                        output_path, e
                    )
                });
            }

            // make sure the path exists
            create_dir_all(&output_path.as_ref().unwrap_or(&path.join("public"))).unwrap_or_else(
                |e| panic!("Failed to create output directory {:?}: {}", output_path, e),
            );

            // make it canonical
            let output_path = output_path
                .as_ref()
                .unwrap_or(&path.join("public"))
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
                std::env::set_current_dir(path)
                    .unwrap_or_else(|e| panic!("Failed to change path: {}", e));
            }

            match generate(false) {
                Ok(files) => {
                    for (path, file) in files {
                        file.to_file(&output_path.join(path)).expect("balls");
                    }
                }
                Err(err) => print_error("Failed to build site", &err),
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
