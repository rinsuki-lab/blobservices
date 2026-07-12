pub mod manager {
    include!(concat!(env!("OUT_DIR"), "/blobservices.manager.rs"));
    include!(concat!(env!("OUT_DIR"), "/blobservices.manager.serde.rs"));
}
