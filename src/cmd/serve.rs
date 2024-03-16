use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

use super::generate::Site;
use crate::{
    api::file::File,
    cmd::generate::GenerateError,
    pretty_print::{print_error, print_warning, warning_and_error_html},
};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};

const RW_ERR: &str = "Cronch: lock was poissoned";
const VERY_LONG_PATH: &str = "/very-long-path-name-intentionally-used-to-get-update-notifications-please-do-not-name-your-files-like-this.rs";
const UPDATE_NOTIFY_SCRIPT: &str = include_str!("update_notify.html");

/// Serve the files
pub(crate) fn serve(path: Option<PathBuf>, addr: Option<String>) -> Result<(), anyhow::Error> {
    // load the site
    let warnings = Arc::new(RwLock::new(Vec::new()));
    let errors = Arc::new(RwLock::new(Vec::new()));
    let site = Arc::new(RwLock::new(match Site::generate(path.clone(), true) {
        Ok(res) => {
            // throw all warnings
            for warning in res.warnings.iter() {
                print_warning(warning);
            }

            // give the warnings to the page
            let mut warnings = warnings.write().expect(RW_ERR);
            *warnings = res.warnings;

            let mut pages = res.page.into_hashmap("/");

            // set the 404 error page
            if let Some(dev_404) = res.dev_404 {
                if let Some(file) = pages.get(&PathBuf::from("/").join(dev_404)).cloned() {
                    pages.insert(PathBuf::from("dev-server-404-error-page.rs"), file);
                }
            }

            pages
        }
        Err(GenerateError {
            warnings: page_warnings,
            error,
        }) => {
            // failed to generate, so thow the error
            print_error(&format!("{:?}", error));

            // print warnings
            for warning in &page_warnings {
                print_warning(warning);
            }

            // give the errors to the page
            errors.write().expect(RW_ERR).push(format!("{:?}", error));

            // give the warnings to the page
            warnings.write().expect(RW_ERR).extend(page_warnings);

            HashMap::new()
        }
    }));

    // stream to notify when an update happens
    let update_notify = Arc::new(Mutex::new(Vec::<TcpStream>::new()));

    // update the site if any file changed TODO
    let site_cloned = site.clone();
    let path_cloned = path.clone();
    let warnings_cloned = warnings.clone();
    let errors_cloned = errors.clone();
    let update_notify_cloned = update_notify.clone();
    let mut debouncer = new_debouncer(Duration::from_millis(500), move |res| match res {
        Ok(_) => {
            println!("Files changed, regenerating site...");
            let start = std::time::Instant::now();

            // regenerate site
            let mut site = site_cloned.write().expect(RW_ERR);
            *site = match Site::generate(path_cloned.clone(), true) {
                Ok(res) => {
                    // throw all warnings
                    for warning in res.warnings.iter() {
                        print_warning(warning);
                    }

                    // give the warnings to the page
                    let mut warnings = warnings_cloned.write().expect(RW_ERR);
                    *warnings = res.warnings;

                    // clear errors
                    errors_cloned.write().expect(RW_ERR).clear();

                    let mut pages = res.page.into_hashmap("/");

                    // set the 404 error page
                    if let Some(dev_404) = res.dev_404 {
                        if let Some(file) = pages.get(&PathBuf::from("/").join(dev_404)).cloned() {
                            pages.insert(PathBuf::from("dev-server-404-error-page.rs"), file);
                        }
                    }

                    pages
                }

                Err(GenerateError {
                    warnings: page_warnings,
                    error,
                }) => {
                    // failed to generate, so thow the error
                    print_error(&format!("{:?}", error));

                    // print warnings
                    for warning in &page_warnings {
                        print_warning(warning);
                    }

                    // clear errors and warnings
                    let mut errors = errors_cloned.write().expect(RW_ERR);
                    let mut warnings = warnings_cloned.write().expect(RW_ERR);

                    // give them to the page
                    *errors = vec![format!("{:?}", error)];
                    *warnings = page_warnings;

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

            let duration = start.elapsed().as_millis();
            println!("... Done! ({}ms)", duration);
        }
        Err(e) => print_error(&format!("While watching files: {:?}", e)),
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
    let addr = addr.unwrap_or("127.0.0.1:1111".to_string());
    let listener = TcpListener::bind(&addr)?;
    println!("listening on {}", addr);

    // TODO: in this loop, don't crash if things go wrong (move to sep function?)

    for stream in listener.incoming() {
        if let Err(e) = handle_connection(stream?, &site, &warnings, &errors, &update_notify) {
            print_error(&format!("While responding to request: {:?}", e));
        }
    }

    Ok(())
}

fn handle_connection(
    mut stream: TcpStream,
    site: &Arc<RwLock<HashMap<PathBuf, File>>>,
    warnings: &Arc<RwLock<Vec<String>>>,
    errors: &Arc<RwLock<Vec<String>>>,
    update_notify: &Arc<Mutex<Vec<TcpStream>>>,
) -> Result<(), anyhow::Error> {
    let reader = BufReader::new(&mut stream);
    let request = reader.lines().next().unwrap_or(Ok("".to_string()))?;

    // trim the request
    let file_path = request
        .trim_start_matches("GET")
        .trim_end_matches("HTTP/1.1")
        .trim();

    // try and get the file
    let (content, status, is_html) = if let Some(file) =
        site.read().expect(RW_ERR).get(&PathBuf::from(file_path))
    {
        // get the file content
        (file.get_bytes()?, "200 OK", file_path.ends_with(".html"))
    }
    // try to see if this was an index.html file
    else if let Some(file) = site
        .read()
        .expect(RW_ERR)
        .get(&PathBuf::from(file_path).join("index.html"))
    {
        (file.get_bytes()?, "200 OK", true)
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
        return Ok(());
    }
    // see if we can get the 404 file
    else if let Some(file) = site
        .read()
        .expect(RW_ERR)
        .get(&PathBuf::from("dev-server-404-error-page.rs"))
    {
        (file.get_bytes()?, "404 NOT FOUND", true)
    }
    // otherwise use the default 404
    else {
        (
            format!(
                "<!DOCTYPE html><h1>404: Not found</h1><p>page {} not found</p>",
                file_path
            )
            .into_bytes(),
            "404 NOT FOUND",
            true,
        )
    };

    // update notify script
    let update_notify = if is_html { UPDATE_NOTIFY_SCRIPT } else { "" };

    // get the warnings and errors sheet
    let warns_and_errors = {
        let warnings = warnings.read().expect(RW_ERR);
        let errors = errors.read().expect(RW_ERR);

        warning_and_error_html(&warnings, &errors)
    };

    // send the page back
    let length = content.len() + update_notify.len() + warns_and_errors.len();
    let response =
        format!("HTTP/1.1 {status}\r\nContent-Length: {length}\r\nCache-Control: no-cache\r\n\r\n");

    // write response and page content
    stream.write_all(response.as_bytes())?;
    stream.write_all(&content)?;

    // write optional warnings and auto-update script
    stream.write_all(warns_and_errors.as_bytes())?;
    stream.write_all(UPDATE_NOTIFY_SCRIPT.as_bytes())?;

    Ok(())
}
