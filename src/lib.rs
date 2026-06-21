pub mod app;
pub mod i18n;
pub mod po;
pub mod project;
pub mod ui;
pub mod update;
pub mod util;
pub mod vcs;
pub mod workflow;

#[cfg(test)]
pub(crate) mod test_support;

pub fn app_title_with_version(version: &str) -> String {
    format!("TranslateR v{version}")
}

pub fn app_title() -> String {
    app_title_with_version(env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::{app_title, app_title_with_version};

    #[test]
    fn app_title_includes_version() {
        assert_eq!(app_title_with_version("1.2.3"), "TranslateR v1.2.3");
        assert_eq!(
            app_title(),
            app_title_with_version(env!("CARGO_PKG_VERSION"))
        );
    }
}
