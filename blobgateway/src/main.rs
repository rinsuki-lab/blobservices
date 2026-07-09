#[tokio::main]
async fn main() {
    blobservices_core::init_tracing_registry();
    tracing::info!("Hello, world!");
}
