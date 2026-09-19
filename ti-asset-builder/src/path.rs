use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::Context;

pub trait PathBufExt {
    /// Appends a string directly to the end of the path
    fn append_str(self, suffix: impl AsRef<OsStr>) -> Self;
}

pub trait PathExt {
    /// Makes the relative path relative to the main path's parent with a suffix
    fn relative_parent_suffix(
        &self,
        relative: impl AsRef<Path>,
        suffix: impl AsRef<OsStr>,
    ) -> anyhow::Result<PathBuf>;
}

impl PathBufExt for PathBuf {
    fn append_str(mut self, suffix: impl AsRef<std::ffi::OsStr>) -> Self {
        self.as_mut_os_string().push(suffix);
        self
    }
}

impl PathExt for Path {
    fn relative_parent_suffix(
        &self,
        relative: impl AsRef<Path>,
        suffix: impl AsRef<OsStr>,
    ) -> anyhow::Result<PathBuf> {
        let path = self.join("..").join(relative).append_str(suffix);
        path.normalize_lexically()
            .with_context(|| format!("Failed to normalize path: {path:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_parent_suffix_example() {
        let path = PathBuf::from("this/is/a/test.toml");
        let expected = PathBuf::from("this/is/a/file.png");
        assert_eq!(
            path.relative_parent_suffix("file", ".png").unwrap(),
            expected
        );
    }
}
