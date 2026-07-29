mod config;
mod handlers;
mod provider;
mod signer;
mod utils;

#[tokio::main]
async fn main() {
    blobservices_core::init_tracing_registry();
    tracing::info!("Hello, world!");

    let provider = provider::S3StoreProvider::new().await;
    blobstore_core::run(provider).await;
}
