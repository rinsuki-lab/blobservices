mod handlers;
mod provider;
mod utils;

#[tokio::main]
async fn main() {
    blobservices_core::init_tracing_registry();
    tracing::info!("Hello, world!");

    let provider = provider::LocalStoreProvider::new().await;
    blobstore_core::run(provider).await;
}
