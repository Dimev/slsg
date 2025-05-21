use std::{path::PathBuf, sync::Arc};

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, header::CONTENT_TYPE},
    response::Response,
    routing::get,
};
use mlua::{ErrorContext, ExternalResult, Result};
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, decompression::RequestDecompressionLayer};

use crate::generate::{self, Site};

/// Serve the website on the given address
pub(crate) fn serve(addr: &str) -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .into_lua_err()
        .context("Failed to start async runtime")?
        .block_on(serve_async(addr))
}

async fn serve_async(addr: &str) -> Result<()> {
    let site = Arc::new(generate::generate()?);

    let app = Router::new()
        .route(
            "/",
            get(|State(site): State<Arc<Site>>| async move {
                let bytes = site
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
                |Path(path): Path<String>, State(site): State<Arc<Site>>| async move {
                    let bytes = site
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
        .context("Failed to serve")
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
