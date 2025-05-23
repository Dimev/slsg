use std::{
    path::PathBuf,
    sync::{Arc, Mutex, RwLock, atomic::AtomicU64},
    time::{Duration, Instant},
};

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, header::CONTENT_TYPE},
    response::{Response, Sse, sse::Event},
    routing::get,
};
use mlua::{ErrorContext, ExternalResult, Result};

use notify_debouncer_full::{DebounceEventResult, notify::RecursiveMode};
use tokio::stream;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, decompression::RequestDecompressionLayer};

use crate::generate::{self, Site, generate};

const NOTIFY_PATH: &str = "/change-notify-path-for-slsg-do-not-use.rs";

/// Serve the website on the given address
pub(crate) fn serve(addr: &str) -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .into_lua_err()
        .context("Failed to start async runtime")?
        .block_on(serve_async(addr))
}

fn on_change(res: DebounceEventResult, site_ref: &Arc<RwLock<Site>>) {
    // only changes
    if res
        .map(|x| {
            x.iter()
                .any(|x| x.kind.is_create() || x.kind.is_modify() || x.kind.is_remove())
        })
        .unwrap_or(false)
    {
        // check if the time now is beyond our elapsed time
        // if so, update the site
        // update

        // make site again
        // TODO: better error messages
        // TODO: this fails sometimes
        *site_ref.write().unwrap() = generate().unwrap();

        // notify the clients it got updated
        println!("le update {:?}", std::time::Instant::now());
    }
}

async fn serve_async(addr: &str) -> Result<()> {
    // generate site at startup
    let site = Arc::new(RwLock::new(generate()?));

    // watch changes
    let site_2 = site.clone();
    let debouncer = notify_debouncer_full::new_debouncer(
        Duration::from_millis(100),
        None,
        move |res: DebounceEventResult| {
            on_change(res, &site_2);
        },
    )
    .and_then(|mut debouncer| {
        debouncer
            .watch(&PathBuf::from("."), RecursiveMode::Recursive)
            .map(|_| debouncer)
    });

    // TODO: notify if we fail to watch

    // TODO: poll in the sse thing, then update if there's a version mismatch between the sites

    // actual server
    let app = Router::new()
        .route(
            NOTIFY_PATH,
            get(|| async {

                /*let stream = empty()                    .map(Ok)
                    .throttle(Duration::from_secs(1));
                Sse::new(stream)*/
            }),
        )
        .route(
            "/",
            get(|State(site): State<Arc<RwLock<Site>>>| async move {
                let bytes = site
                    .read()
                    .unwrap()
                    .files
                    .get(&PathBuf::from("index.html"))
                    .cloned()
                    .unwrap_or(b"<!doctype html>oop".into());

                Response::builder()
                    .header(CONTENT_TYPE, HeaderValue::from_static("text/html"))
                    .body(Body::from(bytes))
                    .unwrap()
            }),
        )
        .with_state(site.clone())
        .route(
            "/{*key}",
            get(
                |Path(path): Path<String>, State(site): State<Arc<RwLock<Site>>>| async move {
                    let bytes = site
                        .read()
                        .unwrap()
                        .files
                        .get(&PathBuf::from(path))
                        .cloned()
                        .unwrap_or(b"<!doctype html>oop".into());

                    // TODO: proper mime type
                    Response::builder()
                        .header(CONTENT_TYPE, HeaderValue::from_static("text/html"))
                        .body(Body::from(bytes))
                        .unwrap()
                },
            ),
        )
        .with_state(site.clone())
        .layer(
            ServiceBuilder::new()
                .layer(RequestDecompressionLayer::new())
                .layer(CompressionLayer::new()),
        );

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .into_lua_err()
        .context("Failed to start web server")?;

    println!("Serving on http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .into_lua_err()
        .context("Failed to serve")?;

    // stop watching
    std::mem::drop(debouncer);

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
