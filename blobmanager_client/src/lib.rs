//! HTTP client for the blobmanager reference API.

mod client;
mod config;
mod error;

pub use client::Client;
pub use config::Config;
pub use error::Error;
