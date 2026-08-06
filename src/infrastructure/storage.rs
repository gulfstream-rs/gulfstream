use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, bail};

use crate::config::StorageConfig;

#[derive(Clone, Debug)]
pub struct Storage {
    config: Arc<StorageConfig>,
}

impl Storage {
    pub async fn initialize(config: StorageConfig) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(&config.root)
            .await
            .with_context(|| format!("create storage root {}", config.root.display()))?;
        tokio::fs::create_dir_all(&config.temporary_root)
            .await
            .with_context(|| {
                format!("create temporary root {}", config.temporary_root.display())
            })?;
        Ok(Self {
            config: Arc::new(config),
        })
    }

    #[must_use]
    pub fn temporary_file(&self, identifier: &str) -> PathBuf {
        self.config.temporary_root.join(identifier)
    }

    #[must_use]
    pub fn media_root(&self, account_id: &str, media_id: &str) -> PathBuf {
        self.config.root.join(account_id).join(media_id)
    }

    #[must_use]
    pub fn original_directory(&self, account_id: &str, media_id: &str) -> PathBuf {
        self.media_root(account_id, media_id)
            .join(&self.config.original_directory_name)
    }

    #[must_use]
    pub fn stream_directory(&self, account_id: &str, media_id: &str) -> PathBuf {
        self.media_root(account_id, media_id)
            .join(&self.config.stream_directory_name)
    }

    #[must_use]
    pub fn stream_staging_directory(&self, job_id: &str) -> PathBuf {
        self.config.temporary_root.join(format!("stream-{job_id}"))
    }

    pub fn object_key(&self, absolute_path: &Path) -> anyhow::Result<String> {
        let relative = absolute_path
            .strip_prefix(&self.config.root)
            .context("object is outside the storage root")?;
        validate_relative_path(relative)?;
        Ok(relative.to_string_lossy().replace('\\', "/"))
    }

    pub fn object_path(&self, object_key: &str) -> anyhow::Result<PathBuf> {
        let relative = Path::new(object_key);
        validate_relative_path(relative)?;
        Ok(self.config.root.join(relative))
    }

    pub async fn move_to_trash(&self, account_id: &str, media_id: &str) -> anyhow::Result<()> {
        let source = self.media_root(account_id, media_id);
        if tokio::fs::metadata(&source).await.is_err() {
            return Ok(());
        }
        let trash = self
            .config
            .root
            .join(&self.config.trash_directory_name)
            .join(account_id);
        tokio::fs::create_dir_all(&trash).await?;
        let destination = trash.join(media_id);
        if tokio::fs::metadata(&destination).await.is_ok() {
            tokio::fs::remove_dir_all(&destination).await?;
        }
        tokio::fs::rename(source, destination).await?;
        Ok(())
    }
}

pub async fn directory_size(path: &Path) -> anyhow::Result<u64> {
    let mut total = 0_u64;
    let mut pending = vec![path.to_owned()];
    while let Some(directory) = pending.pop() {
        let mut entries = tokio::fs::read_dir(&directory).await?;
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_dir() {
                pending.push(entry.path());
            } else if metadata.is_file() {
                total = total.saturating_add(metadata.len());
            }
        }
    }
    Ok(total)
}

pub fn validate_relative_path(path: &Path) -> anyhow::Result<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("invalid relative storage path");
    }
    Ok(())
}
