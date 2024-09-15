use std::{
    borrow::Borrow,
    path::{Path, PathBuf},
    time::Duration,
};

use tiny_http::Response;

use crate::generate::{self, generate, Output};

pub(crate) fn serve(path: &Path, addr: String) {
    let server = tiny_http::Server::http(&addr)
        .expect(&format!("Failed to serve site {:?} on `{}`", path, addr));

    println!("Serving {:?} on `{:?}`", path, server.server_addr());

    let site = generate(path, true);

    loop {
        match server.recv_timeout(Duration::from_millis(300)) {
            Ok(Some(rq)) => {
                let url = rq.url();

                match site.borrow() {
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
                    _ => (),
                }
            }
            Ok(None) => {
                // TODO: check for updates
            }
            // TODO: error text here
            Err(e) => println!("[ERR] While serving: {}", e),
        }
    }
}
