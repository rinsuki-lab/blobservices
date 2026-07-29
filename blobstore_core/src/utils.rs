use std::path::{Component, PathBuf};

pub fn sanitize_address(address: &str) -> Option<String> {
    let raw_address = std::path::Path::new(&address);
    let mut address = PathBuf::new();

    for component in raw_address.components() {
        match component {
            Component::CurDir => return None,
            Component::Normal(c) => address.push(c),
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => return None,
        }
    }

    address.to_str().map(|x| x.to_string())
}
