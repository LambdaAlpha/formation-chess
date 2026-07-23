mod protocol;
mod server;

#[tokio::main]
async fn main() {
    let port = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0u16);

    let listener =
        tokio::net::TcpListener::bind(("127.0.0.1", port)).await.expect("failed to bind");
    let addr = listener.local_addr().expect("failed to get local address");
    let url = format!("http://{addr}");

    let _ = webbrowser::open(&url);
    println!("Serving at {url}");

    axum::serve(listener, server::build_app()).await.expect("server error");
}
