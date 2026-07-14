use std::{path::PathBuf, str::FromStr, sync::Arc};

pub struct AppStateInner {
    pub root_dir: String,
    pub wip_dir: PathBuf,
    pub done_dir: PathBuf,
}

pub type AppState = Arc<AppStateInner>;

impl AppStateInner {
    pub async fn new() -> AppState {
        let root_dir = std::env::var("BLOBSTORE_LOCAL_ROOT_DIR")
            .expect("BLOBSTORE_LOCAL_ROOT_DIR is required");
        let root_dir_buf = PathBuf::from_str(&root_dir)
            .expect("BLOBSTORE_LOCAL_ROOT_DIR is not valid for PathBuf");
        let wip_dir = {
            let mut p = root_dir_buf.clone();
            p.push("wip");
            p
        };
        let done_dir = {
            let mut p = root_dir_buf;
            p.push("done");
            p
        };
        AppState::new(AppStateInner {
            root_dir,
            wip_dir,
            done_dir,
        })
    }
}
