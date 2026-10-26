//! `android-activity` 0.5 compatible API for Android tablet entry points.

#[derive(Clone, Debug, Default)]
pub struct AndroidApp {
    _priv: (),
}

impl AndroidApp {
    pub fn internal_data_path(&self) -> Option<std::path::PathBuf> {
        None
    }
}
