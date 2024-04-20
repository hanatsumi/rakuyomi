use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use log::debug;
use size::Size;
use tempfile::NamedTempFile;
use walkdir::{DirEntry, WalkDir};

use crate::model::ChapterId;

const CHAPTER_FILE_EXTENSION: &str = "cbz";

#[derive(Clone)]
pub struct ChapterStorage {
    downloads_folder_path: PathBuf,
    storage_size_limit: Size,
}

impl ChapterStorage {
    pub fn new(downloads_folder_path: PathBuf, storage_size_limit: Size) -> Self {
        Self {
            downloads_folder_path,
            storage_size_limit,
        }
    }

    pub fn get_stored_chapter(&self, id: &ChapterId) -> Option<PathBuf> {
        let path = self.path_for_chapter(id);

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn get_path_to_store_chapter(&self, id: &ChapterId) -> PathBuf {
        self.path_for_chapter(id)
    }

    // FIXME depending on `NamedTempFile` here is pretty ugly
    pub fn persist_chapter(
        &self,
        id: &ChapterId,
        temporary_file: NamedTempFile,
    ) -> Result<PathBuf> {
        let mut current_size = self.calculate_storage_size();
        let persisted_chapter_size = Size::from_bytes(temporary_file.as_file().metadata()?.size());

        while current_size + persisted_chapter_size > self.storage_size_limit {
            debug!(
                "persist_chapter: current storage is {current_size}/{}, new persisted chapter is \
                {persisted_chapter_size}, attempting to evict",
                self.storage_size_limit
            );

            self.evict_least_recently_modified_chapter()
                .with_context(|| format!(
                    "while attempting to bring the storage size under the {} limit (current size: {}, persisted chapter size: {})",
                    self.storage_size_limit,
                    current_size,
                    persisted_chapter_size,
                ))?;

            current_size = self.calculate_storage_size();
        }

        let path = self.path_for_chapter(id);
        temporary_file.persist(&path)?;

        Ok(path)
    }

    fn calculate_storage_size(&self) -> Size {
        let size_in_bytes: u64 = self
            .chapter_files_iterator()
            .filter_map(|entry| entry.metadata().ok().map(|metadata| metadata.size()))
            .sum();

        Size::from_bytes(size_in_bytes)
    }

    fn evict_least_recently_modified_chapter(&self) -> Result<()> {
        let chapter_to_evict = self
            .find_least_recently_modified_chapter()?
            .ok_or_else(|| anyhow!("couldn't find any chapters to evict from storage"))?;

        debug!(
            "evict_least_recently_modified_chapter: evicting {}",
            chapter_to_evict.display()
        );

        fs::remove_file(chapter_to_evict)?;

        Ok(())
    }

    fn find_least_recently_modified_chapter(&self) -> Result<Option<PathBuf>> {
        let chapter_path = self
            .chapter_files_iterator()
            .filter_map(|entry| {
                let path = entry.path().to_owned();
                let modified = entry.metadata().ok()?.modified().ok()?;

                Some((path, modified))
            })
            // FIXME i dont think we need to clone here
            .min_by_key(|(_, modified)| *modified)
            .map(|(path, _)| path.to_owned());

        Ok(chapter_path)
    }

    fn chapter_files_iterator(&self) -> impl Iterator<Item = DirEntry> {
        WalkDir::new(&self.downloads_folder_path)
            .into_iter()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let extension = entry.path().extension()?;
                let metadata = entry.metadata().ok()?;

                if !metadata.is_file() || extension != CHAPTER_FILE_EXTENSION {
                    return None;
                }

                Some(entry)
            })
    }

    fn path_for_chapter(&self, chapter_id: &ChapterId) -> PathBuf {
        let output_filename = format!(
            "{}-{}.cbz",
            chapter_id.source_id().value(),
            chapter_id.value()
        );

        self.downloads_folder_path.join(output_filename)
    }
}
