//! Reference module binary: serves [`reference_module::router`] on `PORT`
//! (default `8080`), matching the port the module image's `Dockerfile`/OCI
//! manifest exposes for the core's `DockerRuntime` to map (TR-05-001,
//! TR-09-009).

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("bind module port");
    println!("reference module listening on :{port}");
    axum::serve(listener, reference_module::router())
        .await
        .expect("serve reference module");
}
