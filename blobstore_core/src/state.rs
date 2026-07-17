use std::sync::Arc;

use crate::BlobProvider;

pub struct AppStateInner<P: BlobProvider> {
    pub provider: P,
}

pub type AppState<P> = Arc<AppStateInner<P>>;

impl<P: BlobProvider> AppStateInner<P> {
    pub async fn new(provider: P) -> AppState<P> {
        AppState::new(AppStateInner { provider })
    }
}
