use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::PathBuf,
};

use super::generate::Site;

/// Serve the files
pub(crate) fn serve(path: Option<PathBuf>) -> Result<(), anyhow::Error> {
    // TODO: proper reloading
    let site = Site::generate(path)?.page.to_hashmap("/");

    // run the server

    // how zola does it:
    // one thread for file watching
    // one thread for the server
    // channels for sending stuff
    let listener = TcpListener::bind("127.0.0.1:8000")?;
    println!("listening on http://127.0.0.1:8000");

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
        let (content, status) = if let Some(file) = site.get(&PathBuf::from(file_path)) {
            // get the file content
            (file.get_bytes()?, "200 OK")
        }
        // try to see if this was an index.html file
        else if let Some(file) = site.get(&PathBuf::from(file_path).join("index.html")) {
            (file.get_bytes()?, "200 OK")
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

        // TODO: cram content type in here somewhere?
        let response = format!("HTTP/1.1 {status}\r\nContent-Length: {length}\r\n\r\n");

        stream.write_all(response.as_bytes())?;
        stream.write_all(&content)?;
    }

    Ok(())
}
