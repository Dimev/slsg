use std::{ffi::OsString, path::PathBuf};

use generate::generate;

mod generate;

const HELP: &str = "\
SLSG [MODE] [OPTION]

sus amogus
";

const NEW_LUA: &str = "\
-- parse our sass
local css = site.sass(site.read(./style.css))

-- make an example page
local html = site.html(
    h.head {
        h.style(style),
        h.title 'My Website',
    },
    h.body {
        h.div {
            h.h1 'Hello, world!',
            h.img { class = 'logo', alt = 'SLSG logo', src = 'logo.svg' }
        }   
    }
)

-- emit it to the generator
site.emit('index.html', html)

-- emit the logo of SLSG to the generator
site.emit('logo.svg', site.logo)
";

const NEW_META: &str = "\

";

const NEW_GITIGNORE: &str = "\
public
";

const API_DOCS: &[(&str, &str)] = &[];

fn main() {
    let mut pargs = pico_args::Arguments::from_env();
    let sub = pargs.subcommand().expect("torrstohen");

    match sub.as_deref() {
        Some("dev") => {
            let path = pargs
                .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
                .unwrap()
                .unwrap_or(PathBuf::from("."));

            let addr = pargs
                .opt_value_from_os_str::<_, OsString, String>(["-a", "--address"], |x| {
                    Ok(OsString::from(x))
                })
                .unwrap()
                .unwrap_or(OsString::from("127.0.0.1:1111"));
        }
        Some("build") => {
            let path = pargs
                .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
                .unwrap()
                .unwrap_or(PathBuf::from("."));

            let output_path = pargs
                .opt_value_from_os_str::<_, OsString, String>(["-o", "--output"], |x| {
                    Ok(OsString::from(x))
                })
                .unwrap()
                .unwrap_or(OsString::from("./public"));

            generate(path.as_path(), true).expect("breh");
        }
        Some("new") => {
            let path = pargs
                .opt_free_from_os_str::<PathBuf, String>(|x| Ok(PathBuf::from(x)))
                .unwrap()
                .unwrap_or(PathBuf::from("."));

            // make the directories
            std::fs::create_dir_all(&path);

            // example file
            std::fs::write(path.join("main.lua"), NEW_LUA);
            std::fs::write(path.join("stdlib.meta"), NEW_META);
            std::fs::write(path.join(".gitignore"), NEW_GITIGNORE);

            println!("Created new site in {:?}", path);
            println!("Run `slsl dev` in the directory to start making your site!");
        }
        Some("api") => {}
        _ => println!("{}", HELP),
    }
}
