pub mod core {
    include!(concat!(env!("OUT_DIR"), "/blobservices.core.rs"));
    include!(concat!(env!("OUT_DIR"), "/blobservices.core.serde.rs"));
}

pub mod manager {
    include!(concat!(env!("OUT_DIR"), "/blobservices.manager.rs"));
    include!(concat!(env!("OUT_DIR"), "/blobservices.manager.serde.rs"));
}

pub mod storage {
    include!(concat!(env!("OUT_DIR"), "/blobservices.storage.rs"));
    include!(concat!(env!("OUT_DIR"), "/blobservices.storage.serde.rs"));
}
