use std::{
    convert::Infallible,
    path::PathBuf,
    sync::{Arc, Mutex, RwLock, atomic::AtomicU64},
    time::{Duration, Instant},
};

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, header::CONTENT_TYPE},
    response::{
        Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use mlua::{ErrorContext, ExternalResult, Result};

use notify_debouncer_full::{DebounceEventResult, notify::RecursiveMode};
use tokio::{
    stream,
    sync::{
        Notify,
        watch::{Sender, channel},
    },
};
use tokio_stream::{StreamExt, wrappers::WatchStream};
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, decompression::RequestDecompressionLayer};

use crate::generate::{self, Site, generate};

const NOTIFY_PATH: &str = "/change-notify-path-for-slsg-do-not-use.rs";
const NOTIFY_SCRIPT: &[u8] = b"<script>
const src = new EventSource('/change-notify-path-for-slsg-do-not-use.rs');
src.onmessage = msg => msg.data === 'update' && location.reload();
window.onbeforeunload = () => src.close();
</script>";

/// Serve the website on the given address
pub(crate) fn serve(addr: &str) -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .into_lua_err()
        .context("Failed to start async runtime")?
        .block_on(serve_async(addr))
}

/// What to serve
enum Present {
    /// Successfully made the site
    Site(Site),

    /// Error
    Error(String),
}

impl Present {
    /// Generate it
    fn generate() -> Self {
        // generate the site
        let site = generate();

        // make ourselves based on the result
        match site {
            Ok(site) => Self::Site(site),
            Err(err) => {
                // convert to string
                let err = err.to_string();

                // TODO: prettify in a template
                Self::Error(err)
            }
        }
    }

    /// Respond to a request
    fn respond(&self, path: String) -> Response {
        match self {
            Self::Error(err) => Response::builder()
                .header(CONTENT_TYPE, HeaderValue::from_static("text/html"))
                .body(Body::from(
                    [err.as_bytes(), NOTIFY_SCRIPT]
                        .into_iter()
                        .flatten()
                        .cloned()
                        .collect::<Vec<u8>>(),
                )) // TODO: pretty
                .expect("Failed to make response"),
            Self::Site(site) => Response::builder()
                .header(CONTENT_TYPE, HeaderValue::from_static("text/html"))
                .body(Body::from(
                    [
                        &site.files[&PathBuf::from("index.lua.html")][..],
                        NOTIFY_SCRIPT,
                    ]
                    .into_iter()
                    .flatten()
                    .cloned()
                    .collect::<Vec<u8>>(), // TODO: path
                ))
                .expect("Failed to make response"),
        }
    }
}

/// type alias for later
type SiteState = Arc<RwLock<Present>>;

/// When a file is changed
fn on_change(res: DebounceEventResult, notify: &Sender<()>, site_ref: &SiteState) {
    // only changes
    if res
        .map(|x| {
            x.iter()
                .any(|x| x.kind.is_create() || x.kind.is_modify() || x.kind.is_remove())
        })
        .unwrap_or(false)
    {
        // make site again
        *site_ref.write().expect("Rwlock poisoned") = Present::generate();

        // notify the clients it got updated
        notify.send(()).expect("Failed to notify");

        // TODO: also notify console we got updated
        println!("le update {:?}", std::time::Instant::now());
    }
}

async fn serve_async(addr: &str) -> Result<()> {
    // generate site at startup
    let site = Arc::new(RwLock::new(Present::generate()));

    // notification for when the site is updated
    let (update_send, update_recv) = channel(());
    let update_recv = Arc::new(update_recv);

    // watch changes
    let site_2 = site.clone();
    let debouncer = notify_debouncer_full::new_debouncer(
        Duration::from_millis(100),
        None,
        move |res: DebounceEventResult| {
            on_change(res, &update_send, &site_2);
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
            get(|| async move {
                let rx = WatchStream::new((*update_recv).clone());

                Sse::new(
                    rx.map(|_| Event::default().data("update"))
                        .map(Ok::<Event, Infallible>),
                )
                .keep_alive(KeepAlive::new().interval(Duration::from_secs(5)))
            }),
        )
        .route(
            "/",
            get(|State(site): State<SiteState>| async move {
                site.read()
                    .expect("rwlock poissoned")
                    .respond("index.html".to_string())
            }),
        )
        .with_state(site.clone())
        .route(
            "/{*key}",
            get(
                |Path(path): Path<String>, State(site): State<SiteState>| async move {
                    site.read().expect("rwlock poissoned").respond(path)
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
