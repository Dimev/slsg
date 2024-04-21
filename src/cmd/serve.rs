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
const DEV_SERVER_404: &str = "/very-long-path-name-intentionally-used-to-get-the-dev-404-page-please-do-not-name-your-files-like-this.rs";
const UPDATE_NOTIFY_SCRIPT: &str = include_str!("update_notify.html");

/// Serve the files
pub(crate) fn serve(
    path: Option<PathBuf>,
    addr: Option<String>,
    standalone: bool,
    spa: bool,
) -> Result<(), anyhow::Error> {
    // load the site
    let warnings = Arc::new(RwLock::new(Vec::new()));
    let errors = Arc::new(RwLock::new(Vec::new()));
    let site = Arc::new(RwLock::new(
        match if standalone {
            Site::generate_standalone(path.clone(), false)
        } else {
            Site::generate(path.clone(), false)
        } {
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
                        pages.insert(PathBuf::from(DEV_SERVER_404), file);
                    }
                } else if spa {
                    if let Some(file) = pages.get(&PathBuf::from("/index.html")).cloned() {
                        pages.insert(PathBuf::from(DEV_SERVER_404), file);
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
        },
    ));

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
            *site = match if standalone {
                Site::generate_standalone(path_cloned.clone(), false)
            } else {
                Site::generate(path_cloned.clone(), false)
            } {
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
                            pages.insert(PathBuf::from(DEV_SERVER_404), file);
                        }
                    } else if spa {
                        if let Some(file) = pages.get(&PathBuf::from("/index.html")).cloned() {
                            pages.insert(PathBuf::from(DEV_SERVER_404), file);
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
    let addr = addr.unwrap_or("127.0.0.1:1111".to_string());
    let listener = TcpListener::bind(&addr)?;
    println!("listening on {}", addr);

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
    let (content, status, mime_type) = if let Some(file) =
        site.read().expect(RW_ERR).get(&PathBuf::from(file_path))
    {
        // get the file content
        (file.get_bytes()?, "200 OK", get_mime_type(&file_path))
    }
    // try to see if this was an index.html file
    else if let Some(file) = site
        .read()
        .expect(RW_ERR)
        .get(&PathBuf::from(file_path).join("index.html"))
    {
        (file.get_bytes()?, "200 OK", Some("text/html"))
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
        .get(&PathBuf::from(DEV_SERVER_404))
    {
        (file.get_bytes()?, "404 NOT FOUND", Some("text/html"))
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
            Some("text/html"),
        )
    };

    // update notify script
    let update_notify = if mime_type == Some("text/html") {
        UPDATE_NOTIFY_SCRIPT
    } else {
        ""
    };

    // get the warnings and errors sheet
    let warns_and_errors = {
        let warnings = warnings.read().expect(RW_ERR);
        let errors = errors.read().expect(RW_ERR);

        warning_and_error_html(&warnings, &errors)
    };

    // send the page back
    let length = content.len() + update_notify.len() + warns_and_errors.len();
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {length}\r\nCache-Control: no-cache\r\n{}\r\n",
        if let Some(mime) = mime_type {
            format!("Content-Type: {mime}\r\n")
        } else {
            String::new()
        }
    );

    // write response and page content
    stream.write_all(response.as_bytes())?;
    stream.write_all(&content)?;

    // write optional warnings and auto-update script
    stream.write_all(warns_and_errors.as_bytes())?;
    stream.write_all(UPDATE_NOTIFY_SCRIPT.as_bytes())?;

    Ok(())
}

/// Get a mime type from a file path
fn get_mime_type<P: AsRef<Path>>(path: &P) -> Option<&str> {
    // see https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Common_types
    match path.as_ref().extension()?.to_str()? {
        "aac" => Some("audio/aac"),
        "abw" => Some("application/x-abiword"),
        "apng" => Some("image/apng"),
        "arc" => Some("application/x-freearc"),
        "avif" => Some("image/avif"),
        "avi" => Some("video/x-msvideo"),
        "azw" => Some("application/vnd.amazon.ebook"),
        "bin" => Some("application/octet-stream"),
        "bmp" => Some("image/bmp"),
        "bz" => Some("application/x-bzip"),
        "bz2" => Some("application/x-bzip2"),
        "cda" => Some("application/x-cdf"),
        "csh" => Some("application/x-csh"),
        "css" => Some("text/css"),
        "csv" => Some("text/csv"),
        "doc" => Some("application/msword"),
        "docx" => Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        "eot" => Some("application/vnd.ms-fontobject"),
        "epub" => Some("application/epub+zip"),
        "gz" => Some("application/gzip"),
        "gif" => Some("application/gif"),
        "htm" | "html" => Some("text/html"),
        "ico" => Some("image/vnd.microsoft.icon"),
        "ics" => Some("text/calendar"),
        "jar" => Some("application/java-archive"),
        "jpeg" | "jpg" => Some("image/jpeg"),
        "js" => Some("text/javascript"),
        "json" => Some("application/json"),
        "jsonld" => Some("application/ld+json"),
        "mid" | "midi" => Some("audio/midi"),
        "mjs" => Some("text/javascript"),
        "mp3" => Some("audio/mpeg"),
        "mp4" => Some("video/mpeg"),
        "mpeg" => Some("video/mpeg"),
        "mpkg" => Some("application/vnd.apple.installer+xml"),
        "odp" => Some("application/vnd.oasis.opendocument.presentation"),
        "ods" => Some("application/vnd.oasis.opendocument.spreadsheet"),
        "odt" => Some("application/vnd.oasis.opendocument.text"),
        "oga" => Some("audio/ogg"),
        "ogv" => Some("video/ogg"),
        "ogx" => Some("application/ogg"),
        "opus" => Some("audio/opus"),
        "otf" => Some("font/otf"),
        "png" => Some("image/png"),
        "pdf" => Some("application/pdf"),
        "php" => Some("application/x-httpd-php"),
        "ppt" => Some("application/vnd.ms-powerpoint"),
        "pptx" => Some("application/vnd/openxmlformats-officedocument.presentationml.presentation"),
        "rar" => Some("application/vnd.rar"),
        "rtf" => Some("application/rtf"),
        "sh" => Some("application/x-sh"),
        "svg" => Some("image/svg+xml"),
        "tar" => Some("application/x-tar"),
        "tif" | "tiff" => Some("image/tiff"),
        "ts" => Some("video/mp2t"),
        "ttf" => Some("font/ttf"),
        "txt" => Some("text/plain"),
        "vsd" => Some("application/vnd.visio"),
        "wav" => Some("audio/wav"),
        "weba" => Some("audio/webm"),
        "webm" => Some("video/webm"),
        "webp" => Some("image/webp"),
        "woff" => Some("font/woff"),
        "woff2" => Some("font/woff2"),
        "xhtml" => Some("application/xhtml+xml"),
        "xls" => Some("application/vnd.ms-exel"),
        "xlsx" => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        "xml" => Some("application/xml"),
        "xul" => Some("application/vnd.mozilla.xul+xml"),
        "zip" => Some("application/zip"),
        "3pg" => Some("video/3gpp"),
        "3g2" => Some("video/3ggp2"),
        "7z" => Some("application/x-7z-compressed"),
        // Missing for some reason
        "wasm" => Some("application/wasm"),
        _ => None,
    }
}
