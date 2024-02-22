use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use notify::Watcher;

use super::generate::Site;

/// Serve the files
pub(crate) fn serve(path: Option<PathBuf>) -> Result<(), anyhow::Error> {
    // load the site
    let mut warnings = Vec::new();
    let mut errors = String::new();
    let site = Arc::new(RwLock::new(match Site::generate(path.clone()) {
        Ok(res) => {
            // throw all warnings
            for warning in res.warnings.iter() {
                println!("[WARN] {}", warning);
            }

            warnings = res.warnings;

            res.page.to_hashmap("/")
        }
        Err(e) => {
            // failed to generate, so thow the error
            println!("[ERR] {:?}", e);
            HashMap::new()
        }
    }));

    // stream to notify when an update happens
    let mut update_notify: Option<TcpStream> = None;

    // update the site if any file changed TODO
    let site_cloned = site.clone();
    let path_cloned = path.clone();
    let mut watcher = notify::recommended_watcher(move |res| match res {
        Ok(_) => {
            println!("Files changed, regenerating site...");

            // regenerate site
            let mut site = site_cloned.write().expect("Cronch: rwlock was poissoned");
            *site = match Site::generate(path_cloned.clone()) {
                Ok(res) => {
                    // throw all warnings
                    for warning in res.warnings.iter() {
                        println!("[WARN] {}", warning);
                    }

                    // TODO warnings = res.warnings;

                    res.page.to_hashmap("/")
                }
                Err(e) => {
                    // failed to generate, so thow the error
                    println!("[ERR] {:?}", e);
                    HashMap::new()
                }
            };

            println!("... Done!")
        }
        Err(e) => println!("Error watching files: {:?}", e),
    })?;

    // watch the current dir
    if let Some(path) = &path {
        watcher.watch(path, notify::RecursiveMode::Recursive)?;
    } else {
        watcher.watch(Path::new("."), notify::RecursiveMode::Recursive)?;
    }

    // listen to incoming requests
    let listener = TcpListener::bind("127.0.0.1:1111")?;
    println!("listening on http://127.0.0.1:1111");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let reader = BufReader::new(&mut stream);
        let request = reader.lines().next().unwrap()?;

        // trim the request
        let file_path = request
            .trim_start_matches("GET")
            .trim_end_matches("HTTP/1.1")
            .trim();

        // try and get the file
        let (content, status) = if let Some(file) = site.read().expect("Cronch: rwlock was poissoned").get(&PathBuf::from(file_path)) {
            // get the file content
            (file.get_bytes()?, "200 OK")
        }
        // try to see if this was an index.html file
        else if let Some(file) = site.read().expect("Cronch: rwlock was poissoned").get(&PathBuf::from(file_path).join("index.html")) {
            (file.get_bytes()?, "200 OK")
        }
        // if it's the update notifier, set the update stream
        else if file_path == "/very-long-path-name-intentionally-used-to-get-update-notifications-please-do-not-name-your-files-like-this.rs" {
            println!("TODO");
            continue;
        }
        // see if we can get the 404 file
        else {
            (
                format!("<!DOCTYPE html><p>{} Not found</p>", file_path).into_bytes(),
                "404 NOT FOUND",
            )
        };

        // send the page back
        let length = content.len();

        // TODO: for html files, append the auto-update script
        // TODO: for html files, append the errors/warnings page
        // TODO: mime type

        // TODO: cram content type in here somewhere?
        let response = format!("HTTP/1.1 {status}\r\nContent-Length: {length}\r\n\r\n");

        stream.write_all(response.as_bytes())?;
        stream.write_all(&content)?;
    }

    Ok(())
}
