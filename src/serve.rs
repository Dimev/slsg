use std::{
    collections::HashMap,
    io::{self, BufRead, BufReader, Cursor, Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::channel,
        Arc,
    },
    thread::spawn,
    time::{Duration, Instant},
};

use notify::Watcher;

use crate::{
    generate::{generate, Output},
    message::{html_error, print_error, print_warning},
};

const VERY_LONG_PATH: &str = "very-long-path-name-intentionally-used-to-get-update-notifications-please-do-not-name-your-files-like-this.rs";
pub(crate) const VERY_LONG_404: &str = "very-long-path-name-intentionally-used-to-mark-the-404-file-please-do-not-name-your-files-like-this.rs";

pub(crate) fn serve(addr: String) {
    // run the server
    let listener = TcpListener::bind(&addr)
        .unwrap_or_else(|e| panic!("Failed to serve site on {}: {}", addr, e));

    // we are live
    println!(
        "Serving on {} - change a file to reload the site",
        listener.local_addr().map(|x| x.to_string()).unwrap_or(addr)
    );

    // start the listening thread
    let (listen_sender, incoming) = channel();
    spawn(move || {
        for stream in listener.incoming().filter_map(|x| x.ok()) {
            // directly send them back
            listen_sender
                .send(stream)
                .unwrap_or_else(|e| print_warning("Error while serving", &e));
        }
    });

    // detect changes, we only care when it's changed, and what version it was changed to
    // version allows the other side to detect what version we are on since serving started
    // this means it can reload if the server stops and then starts again, for whatever reason
    let version = Arc::new(AtomicUsize::new(0));
    let changed = Arc::new(AtomicBool::new(false));
    let changed_clone = changed.clone();
    let version_clone = version.clone();

    // watch for changes
    let watcher =
        // we only care about updates, so set the atomic to true if anything happened
        notify::recommended_watcher(move |e: Result<notify::Event, notify::Error>|
            // and make sure that said update is not just file access, otherwise we can trigger ourselves
            if  e.map(|e| !e.kind.is_access()).unwrap_or(false) {
                changed_clone.store(true, Ordering::Relaxed);
                version_clone.fetch_add(1, Ordering::Relaxed);
            })
        // wrap the result ok with the watcher because we don't want it to drop out of scope
        .and_then(|mut watcher| {
            watcher.watch(&PathBuf::from("."), notify::RecursiveMode::Recursive).map(|_| watcher)
        });

    // notify for an error, borrow to not drop it, as doing so would stop the watcher
    if let Err(e) = &watcher {
        print_warning("Failed to watch for changes", e)
    };

    // generate the initial site
    let mut site = generate(true);

    // notify if it went bad
    if let Err(ref e) = site {
        print_error("Failed to build site", e)
    }

    // requests to notify when there's an update
    let mut update_notify = Vec::new();

    loop {
        let stream = incoming.recv_timeout(Duration::from_millis(200));
        match stream {
            Ok(s) => respond(s, &site, &version, &mut update_notify),
            Err(_) => reload(&changed, &mut site, &version, &mut update_notify),
        }
    }
}

fn respond(
    mut stream: TcpStream,
    site: &Result<HashMap<PathBuf, Output>, mlua::Error>,
    version: &Arc<AtomicUsize>,
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
        .trim_start_matches(char::is_alphabetic) // trim GET/POST or similar
        .trim() // trim the space
        .trim_end_matches(|x: char| x.is_numeric() || x == '.') // trim any of the numbers indicating the http version
        .trim_end_matches("HTTP/") // trim the http itself
        .trim() // remove any extra spaces
        .trim_start_matches('/'); // trim starting /, as all paths are relative in the vec we use

    // trim any queries
    let file_path = file_path.split_once('?').map(|x| x.0).unwrap_or(file_path);
    let file_path = file_path.split_once('#').map(|x| x.0).unwrap_or(file_path);

    // get the file
    let (mut content, status, mime): (Box<dyn Read>, u16, Option<&str>) = if let Some(file) = site
        .as_ref()
        .ok()
        .and_then(|x| x.get(&PathBuf::from(&file_path)))
    {
        match file.as_stream() {
            Ok(stream) => (stream, 200, get_mime_type(PathBuf::from(&file_path))),
            Err(e) => {
                print_warning(&format!("Failed to serve file {}", file_path), &e);
                (Box::new("Error!".as_bytes()), 500, Some("text/html"))
            }
        }
    }
    // see if it's on index.html
    else if let Some(file) = site
        .as_ref()
        .ok()
        .and_then(|x| x.get(&PathBuf::from(file_path).join("index.html")))
    {
        match file.as_stream() {
            Ok(stream) => (stream, 200, Some("text/html")),
            Err(e) => {
                print_warning(&format!("Failed to serve file {}", file_path), &e);
                (Box::new("Error!".as_bytes()), 500, Some("text/html"))
            }
        }
    }
    // if it's the update notifier, set the update stream
    else if file_path == VERY_LONG_PATH {
        // don't wait to send things
        stream
            .set_nodelay(true)
            .unwrap_or_else(|e| print_warning("Failed to set nodelay on stream", &e));

        // send the response
        stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\n\r\n",
            ).unwrap_or_else(|e| print_warning("Failed to write on stream", &e));
        stream
            .write_all(b"data: ")
            .unwrap_or_else(|e| print_warning("Failed to write on stream", &e));
        stream
            .write_all(format!("{}\n\n", version.load(Ordering::Relaxed)).as_bytes())
            .unwrap_or_else(|e| print_warning("Failed to write on stream", &e));
        stream
            .flush()
            .unwrap_or_else(|e| print_warning("Failed to flush stream", &e));

        // put it on the update notify list
        update_notify.push(stream);

        // no need to write anything else
        return;

    // if the site is an error, push the error page
    } else if let Err(ref error) = site {
        let error_page = html_error(error);
        (Box::new(Cursor::new(error_page)), 500, Some("text/html"))

    // otherwise, push the 404 page
    } else if let Some(file) = site
        .as_ref()
        .ok()
        .and_then(|x| x.get(&PathBuf::from(VERY_LONG_404)))
    {
        // warn that we serve the 404 page
        print_warning(
            &format!("Failed to serve file (404) {}", file_path),
            &"Forgot to emit the file?",
        );

        // 404, return the not found page if we can get it
        match file.as_stream() {
            Ok(stream) => (stream, 404, Some("text/html")),
            Err(e) => {
                print_warning(&format!("Failed to serve file {}", file_path), &e);
                (Box::new("Error!".as_bytes()), 500, Some("text/html"))
            }
        }
    } else {
        // warn that we serve the 404 page
        print_warning(
            &format!("Failed to serve file (404) {}", file_path),
            &"Forgot to emit the file?",
        );

        // 404, return the not found page
        (
            Box::new(Cursor::new(
                format!(include_str!("not_found_template.html"), file_path).into_bytes(),
            )),
            404,
            Some("text/html"),
        )
    };

    // update notify script, allows reloading the page when we send a message
    let update_notify = if mime == Some("text/html") {
        format!(
            include_str!("update_notify.html"),
            version = version.load(Ordering::Relaxed),
            path = VERY_LONG_PATH
        )
    } else {
        String::new()
    };

    // send the page back
    let response = format!(
        "HTTP/1.1 {status}\r\nCache-Control: no-store\r\n{}\r\n",
        if let Some(mime) = mime {
            format!("Content-Type: {mime}\r\n")
        } else {
            String::new()
        }
    );

    // write the page back
    stream
        .write_all(&response.as_bytes())
        .unwrap_or_else(|e| print_warning("Error while writing response", &e));

    // copy the stream
    io::copy(&mut content, &mut stream)
        .map(|_| ())
        .unwrap_or_else(|e| print_warning("Error while writing response", &e));

    // add the update notify script
    stream
        .write_all(update_notify.as_bytes())
        .unwrap_or_else(|e| print_warning("Error while writing response", &e));

    stream
        .flush()
        .unwrap_or_else(|e| print_warning("Failed to flush stream", &e));
}

fn reload(
    changed: &Arc<AtomicBool>,
    site: &mut Result<std::collections::HashMap<PathBuf, Output>, mlua::Error>,
    version: &Arc<AtomicUsize>,
    update_notify: &mut Vec<TcpStream>,
) {
    // went ok and there is no request, check if the site needs reloading
    if changed.swap(false, Ordering::Relaxed) {
        println!("Files changed, regenerating ...");

        let start = Instant::now();
        *site = generate(true);

        // notify if it went bad
        if let Err(ref e) = *site {
            print_error("Failed to build site", e);
        }

        println!("... Done ({}ms)", start.elapsed().as_millis());

        // notify the listeners we got updated as well
        // only retain the ones that haven't errored out due to likely not being connected anymore
        update_notify.retain_mut(|s| {
            s.write_all(b"data: ")
                .and_then(|_| {
                    s.write_all(format!("{}\n\n", version.load(Ordering::Relaxed)).as_bytes())
                })
                .and_then(|_| s.flush())
                .is_ok()
        });
    }
}

/// Get a mime type from a file path
fn get_mime_type(path: PathBuf) -> Option<&'static str> {
    // see https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Common_types
    match path.extension()?.to_str()? {
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
