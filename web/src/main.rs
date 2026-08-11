mod protocol;
mod server;

#[tokio::main]
async fn main() {
    let port =
        if let Some(port) = std::env::args().nth(1) { port.parse().unwrap_or(0u16) } else { 0 };

    let listener =
        tokio::net::TcpListener::bind(("127.0.0.1", port)).await.expect("failed to bind");
    let addr = listener.local_addr().expect("failed to get local address");
    let url = format!("http://{addr}");

    println!("Serving at {url}");
    let server = axum::serve(listener, server::build_app()).into_future();
    tokio::pin!(server);
    let waker = std::task::Waker::noop();
    let poll = server.as_mut().poll(&mut std::task::Context::from_waker(waker));
    assert!(matches!(poll, std::task::Poll::Pending), "server failed to start: {poll:?}");

    let _ = webbrowser::open(&url);

    server.await.expect("server error");
}
