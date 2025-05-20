use axum::{Router, routing::get};

pub(crate) fn serve() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { serve_async() });
}

async fn serve_async() {
    let app = Router::new().route("/", get(|| async { "hello" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:1111")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
