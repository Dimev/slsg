use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};

use super::generate::Site;

const RW_ERR: &str = "Cronch: lock was poissoned";
const VERY_LONG_PATH: &str = "/very-long-path-name-intentionally-used-to-get-update-notifications-please-do-not-name-your-files-like-this.rs";
const UPDATE_NOTIFY_SCRIPT: &str = include_str!("update_notify.html");

/// Serve the files
pub(crate) fn serve(path: Option<PathBuf>) -> Result<(), anyhow::Error> {
    // load the site
    let warnings = Arc::new(RwLock::new(Vec::new()));
    let errors = Arc::new(RwLock::new(String::new()));
    let site = Arc::new(RwLock::new(match Site::generate(path.clone()) {
        Ok(res) => {
            // throw all warnings
            for warning in res.warnings.iter() {
                println!("[WARN] {}", warning);
            }

            // give the warnings to the page
            let mut warnings = warnings.write().expect(RW_ERR);
            *warnings = res.warnings;

            res.page.to_hashmap("/")
        }
        Err(e) => {
            // failed to generate, so thow the error
            println!("[ERR] {:?}", e);

            // give the errors to the page
            let mut errors = errors.write().expect(RW_ERR);
            *errors = format!("{:?}", e);

            HashMap::new()
        }
    }));

    // stream to notify when an update happens
    let update_notify = Arc::new(Mutex::new(Vec::<TcpStream>::new()));

    // update the site if any file changed TODO
    let site_cloned = site.clone();
    let path_cloned = path.clone();
    let update_notify_cloned = update_notify.clone();
    let mut debouncer = new_debouncer(Duration::from_millis(500), move |res| match res {
        Ok(_) => {
            println!("Files changed, regenerating site...");

            // regenerate site
            let mut site = site_cloned.write().expect(RW_ERR);
            *site = match Site::generate(path_cloned.clone()) {
                Ok(res) => {
                    // throw all warnings
                    for warning in res.warnings.iter() {
                        println!("[WARN] {}", warning);
                    }

                    // give the warnings to the page
                    let mut warnings = warnings.write().expect(RW_ERR);
                    *warnings = res.warnings;

                    res.page.to_hashmap("/")
                }
                Err(e) => {
                    // failed to generate, so thow the error
                    println!("[ERR] {:?}", e);

                    // give the errors to the page
                    let mut errors = errors.write().expect(RW_ERR);
                    *errors = format!("{:?}", e);

                    HashMap::new()
                }
            };

            // notify the upate
            let mut stream = update_notify_cloned.lock().expect(RW_ERR);
            stream.retain_mut(
                |s| match s.write_all(b"data: update\n\n").and_then(|_| s.flush()) {
                    Ok(()) => true,
                    Err(_) => false,
                },
            );

            println!("... Done!")
        }
        Err(e) => println!("Error watching files: {:?}", e),
    })?;

    // watch the current dir
    if let Some(path) = &path {
        debouncer.watcher().watch(path, RecursiveMode::Recursive)?;
    } else {
        debouncer
            .watcher()
            .watch(Path::new("."), RecursiveMode::Recursive)?;
    }

    // listen to incoming requests
    // TODO: use hyper instead?
    let listener = TcpListener::bind("127.0.0.1:1111")?;
    println!("listening on http://127.0.0.1:1111");

    // TODO: in this loop, don't crash if things go wrong (move to sep function?)

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
        let (content, status) = if let Some(file) = site
            .read()
            .expect("Cronch: rwlock was poissoned")
            .get(&PathBuf::from(file_path))
        {
            // get the file content
            (file.get_bytes()?, "200 OK")
        }
        // try to see if this was an index.html file
        else if let Some(file) = site
            .read()
            .expect("Cronch: rwlock was poissoned")
            .get(&PathBuf::from(file_path).join("index.html"))
        {
            (file.get_bytes()?, "200 OK")
        }
        // if it's the update notifier, set the update stream
        else if file_path == VERY_LONG_PATH {
            // we don't want to wait
            stream.set_nodelay(true)?;

            // send the response
            stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\n\r\n",
            )?;
            stream.write_all(b"data: initial\n\n")?;
            stream.flush()?;

            // set the event stream, as we have one now
            update_notify.lock().expect(RW_ERR).push(stream);

            // don't need to send more
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
        let length = content.len() + UPDATE_NOTIFY_SCRIPT.len();

        // TODO: for html files, append the auto-update script
        // TODO: for html files, append the errors/warnings page
        // TODO: mime type

        // TODO: cram content type in here somewhere?
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {length}\r\nCache-Control: no-cache\r\n\r\n"
        );

        stream.write_all(response.as_bytes())?;
        stream.write_all(&content)?;
        stream.write_all(UPDATE_NOTIFY_SCRIPT.as_bytes())?;
    }

    Ok(())
}
