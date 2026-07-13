use std::sync::Arc;

pub struct AppStateInner {
    pub root_dir: String,
}

pub type AppState = Arc<AppStateInner>;

impl AppStateInner {
    pub async fn new() -> AppState {
        let root_dir = std::env::var("BLOBSTORE_LOCAL_ROOT_DIR")
            .expect("BLOBSTORE_LOCAL_ROOT_DIR is required");
        AppState::new(AppStateInner { root_dir })
    }
}
