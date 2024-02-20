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

        let (content, status) = if request == "GET / HTTP/1.1" {
            let content = site
                .get(&PathBuf::from("/index.html"))
                .unwrap()
                .get_string()?;
            (content, "200 OK")
        } else {
            ("Not found".into(), "404 NOT FOUND")
        };

        let length = content.len();
        let response = format!("HTTP/1.1 {status}\r\nContent-Length: {length}\r\n\r\n{content}");

        stream.write_all(response.as_bytes())?;
    }

    Ok(())
}
