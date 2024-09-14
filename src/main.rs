use std::path::{Path, PathBuf};

use generate::generate;

mod generate;

fn main() {
    let mut pargs = pico_args::Arguments::from_env();
    let sub = pargs.subcommand().expect("torrstohen");

    match sub.as_deref() {
        Some("dev") => println!("dev mode"),
        Some("build") => {
            generate(PathBuf::from(".").as_path(), true).expect("breh");
        }
        _ => println!("rstohen"),
    }
}
