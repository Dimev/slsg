use std::{
    collections::HashMap,
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};


use notify::Watcher;

use crate::{
    generate::{generate, Output},
    message::{html_error, print_error, print_success},
};

const VERY_LONG_PATH: &str = "/very-long-path-name-intentionally-used-to-get-update-notifications-please-do-not-name-your-files-like-this.rs";
const UPDATE_NOTIFY_SCRIPT: &str = include_str!("update_notify.html");

pub(crate) fn serve(addr: String) {
    // run the server
    let listener = TcpListener::bind(&addr)
        .unwrap_or_else(|e| panic!("Failed to serve site on {}: {}", addr, e));

    // set nonblocking, to avoid the server freezing
    listener
        .set_nonblocking(true)
        .unwrap_or_else(|e| panic!("Failed to set listener to nonblocking: {}", e));

    // we are live
    println!(
        "Serving on {} - change a file to reload the site",
        listener.local_addr().map(|x| x.to_string()).unwrap_or(addr)
    );

    // detect changes, we only care when it's changed
    let changed = Arc::new(AtomicBool::new(false));
    let changed_clone = changed.clone();

    // watch for changes
    let watcher =
        // we only care about updates, so set the atomic to true if anything happened
        notify::recommended_watcher(move |_| changed_clone.store(true, Ordering::Relaxed))
        // wrap the result ok with the watcher because we don't want it to drop out of scope
        .and_then(|mut watcher| {
            watcher.watch(&PathBuf::from("."), notify::RecursiveMode::Recursive).map(|_| watcher)
        });

    // notify for an error, borrow to let it live
    if let Err(e) = &watcher {
        println!("Failed to watch for changes: {:?}", e)
    };

    // generate the initial site
    let mut site = generate(true);

    // notify if it went bad
    if let Err(ref e) = site {
        print_error("Failed to build site", e)
    }

    // requests to notify when there's an update
    let mut update_notify = Vec::new();

    for stream in listener.incoming() {
        match stream {
            Ok(s) => respond(s, &site, &mut update_notify),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // reload the server
                reload(&changed, &mut site, &mut update_notify);

                // wait a bit so we don't pin the CPU
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => println!("Error while serving: {}", e),
        }
    }

    // manually drop to ensure it lives until here
    std::mem::drop(watcher);
}

fn respond(
    mut stream: TcpStream,
    site: &Result<HashMap<PathBuf, Output>, mlua::Error>,
    update_notify: &mut Vec<TcpStream>,
) {
    // read the url
    let reader = BufReader::new(&mut stream);
    let request = reader
        .lines()
        .next()
        .map(|x| x.unwrap_or(String::new()))
        .unwrap_or(String::new());

    // get the url
    let file_path = request
        .trim_start_matches("GET")
        .trim_end_matches(|x: char| x.is_numeric() || x == '.')
        .trim_end_matches("HTTP/")
        .trim();

    // get the file
    let (content, status, mime) = if let Some(file) = site
        .as_ref()
        .ok()
        .and_then(|x| x.get(&PathBuf::from(file_path)))
    {
        ("sus", 200, Some("sus"))
    }
    // see if it's on index.html
    else if let Some(file) = site
        .as_ref()
        .ok()
        .and_then(|x| x.get(&PathBuf::from(file_path).join("index.html")))
    {
        ("sus", 200, Some("sus"))
    }
    // if it's the update notifier, set the update stream
    else if file_path == VERY_LONG_PATH {
        // don't wait to send things
        stream
            .set_nodelay(true)
            .unwrap_or_else(|e| println!("Failed to set nodelay on stream: {}", e));

        // send the response
        stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\n\r\n",
            ).unwrap_or_else(|e| println!("Failed to write on stream: {}", e));
        stream
            .write_all(b"data: initial\n\n")
            .unwrap_or_else(|e| println!("Failed to write on stream: {}", e));
        stream
            .flush()
            .unwrap_or_else(|e| println!("Failed to flush stream: {}", e));

        // put it on the update notify list
        update_notify.push(stream);

        // no need to write
        return;

    // if the site is an error, push the error page
    } else if let Err(ref error) = site {
        ("sus", 200, Some("sus"))

    // otherwise, push the 404 page
    } else {
        ("sus", 200, Some("sus"))
    };

    // update notify script
    let update_notify = if mime == Some("text/html") {
        UPDATE_NOTIFY_SCRIPT
    } else {
        ""
    };

    // send the page back
    let length = content.len();
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {length}\r\nCache-Control: no-cache\r\n{}\r\n",
        if let Some(mime) = mime {
            format!("Content-Type: {mime}\r\n")
        } else {
            String::new()
        }
    );

    // write the page back
    stream.write(&response.as_bytes());
    stream.write(&content.as_bytes());
}

fn reload(
    changed: &Arc<AtomicBool>,
    site: &mut Result<std::collections::HashMap<PathBuf, Output>, mlua::Error>,
    update_notify: &mut Vec<TcpStream>,
) {
    // went ok and there is no request, check if the site needs reloading
    if changed.swap(false, Ordering::Relaxed) {
        let start = Instant::now();
        *site = generate(true);

        // notify if it went bad
        if let Err(ref e) = *site {
            print_error("Failed to build site", e);

            // notify the listeners it went bad as well
            // TODO
        } else {
            print_success(&format!("Rebuilt in {}ms", start.elapsed().as_millis()));
        }
    }
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
        // Missing from the list for some reason
        "wasm" => Some("application/wasm"),
        _ => None,
    }
}
