use std::path::{Component, Path, PathBuf};

pub fn sanitize_path(base_path: &Path, path: &str) -> PathBuf {
    let mut address = base_path.to_path_buf();
    let raw_address = std::path::Path::new(&path);

    for component in raw_address.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(c) => address.push(c),
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {}
        }
    }

    address
}
