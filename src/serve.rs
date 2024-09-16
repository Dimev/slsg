use std::{
    collections::HashMap,
    io::BufReader,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use mlua::ErrorContext;
use notify::Watcher;
use tiny_http::{Request, Response, StatusCode};

use crate::{
    generate::{generate, Output},
    message::{html_error, print_error},
};

pub(crate) fn serve(path: &Path, addr: String) {
    // run the server
    let server = tiny_http::Server::http(&addr)
        .expect(&format!("Failed to serve site {:?} on `{}`", path, addr));

    // we are live
    println!("Serving {:?} on `{:?}`", path, server.server_addr());

    // detect changes, we only care when it's changed
    let changed = Arc::new(AtomicBool::new(false));
    let changed_clone = changed.clone();

    // watch for changes
    let watcher =
        // we only care about updates, so set the atomic to true if anything happened
        notify::recommended_watcher(move |_| changed_clone.store(true, Ordering::Relaxed))
        // wrap the result ok with the watcher because we don't want it to drop out of scope
        .and_then(|mut watcher| {
            watcher.watch(dbg!(path), notify::RecursiveMode::Recursive).map(|_| watcher)
        });

    // notify for an error
    if let Err(e) = &watcher {
        println!("Failed to watch for changes: {:?}", e)
    };

    // generate the initial site
    let mut site = generate(path, true).map_err(|e| e.context("Failed to build site"));

    // notify if it went bad
    if let Err(ref e) = site {
        print_error(e)
    }

    // requests to notify when there's an update
    let mut update_notify = Vec::<()>::new();

    loop {
        // timeout, so we can check for changes
        match server.recv_timeout(Duration::from_millis(300)) {
            // normal request
            Ok(Some(rq)) => respond(rq, &site, path),
            // timeout, so no request, see if we need to reload
            Ok(None) => reload(&changed, &mut site, path),
            // something went wrong, we'll report it and ignore
            // TODO: colors
            Err(e) => println!("[ERR] While serving: {}", e),
        }
    }
}

fn respond(rq: Request, site: &Result<HashMap<PathBuf, Output>, mlua::Error>, path: &Path) {
    let url = rq.url();
    /*match site {
        Ok(pages) => {
            if let Some(x) = pages.get(&PathBuf::from(url.trim_matches('/'))) {
                match x {
                    Output::Data(data) => rq.respond(Response::from_data(data.clone())),
                    Output::File(original) => rq.respond(Response::from_file(
                        std::fs::File::open(path.join(original)).unwrap(),
                    )),
                    Output::Command {
                        original,
                        command,
                        placeholder,
                    } => rq.respond(Response::from_string("empty")),
                };
            }
        }
        Err(e) => {
            rq.respond(Response::from_string(html_error(e)));
        }
    }*/
    let response = Response::new(
        StatusCode(200),
        Vec::new(),
        "sus mogus".as_bytes(),
        Some("sus mogus".len()),
        None,
    );

    if let Err(e) = rq.respond(response) {
        println!("Failed while responding: {}", e);
    }
}

fn reload(
    changed: &Arc<AtomicBool>,
    site: &mut Result<std::collections::HashMap<PathBuf, Output>, mlua::Error>,
    path: &Path,
) {
    // went ok and there is no request, check if the site needs reloading
    if changed.swap(false, Ordering::Relaxed) {
        *site = generate(path, true).map_err(|e| e.context("Failed to build site"));

        // notify if it went bad
        if let Err(ref e) = *site {
            print_error(e);

            // notify the listeners it went bad as well
            // TODO
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
