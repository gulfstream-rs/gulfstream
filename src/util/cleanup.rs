#[derive(Debug)]
pub struct CleanupPath {
    path: std::path::PathBuf,
    kind: CleanupKind,
    armed: bool,
}

#[derive(Clone, Copy, Debug)]
enum CleanupKind {
    File,
    Directory,
}

impl CleanupPath {
    #[must_use]
    pub fn file(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: CleanupKind::File,
            armed: true,
        }
    }

    #[must_use]
    pub fn directory(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: CleanupKind::Directory,
            armed: true,
        }
    }

    pub fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for CleanupPath {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        match self.kind {
            CleanupKind::File => {
                let _ = std::fs::remove_file(&self.path);
            }
            CleanupKind::Directory => {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }
    }
}
