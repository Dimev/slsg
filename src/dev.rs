use anyhow::{Context, Result};
use axum::{Router, response::Html, routing::get};

use crate::{print::html_error, site::SiteCache};

/// Serve the site in the current working directory on `addr`
pub(crate) fn serve_dev_site(addr: &str) -> Result<()> {
    // run async runtime
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(serve(addr))
}

/// inner function, running the async runtime
async fn serve(addr: &str) -> Result<()> {
    // site
    let mut site = SiteCache::new();

    // files generated from the site
    let files = site.generate(true)?;

    // file server
    let app = Router::new().route(
        "/",
        get(|| async {
            // for now
            // TODO actually implement fetching files here
            Html(html_error(&"\x02Oops\x03: Not yet implemented!"))
        }),
    );

    // serve
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Serving on 'http://{addr}'");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .with_context(|| format!("Failed to serve dev site at '{addr}'"))
}

/// Shutdown signal
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
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
