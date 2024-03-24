use std::path::PathBuf;

use crate::model::ChapterId;

#[derive(Clone)]
pub struct ChapterStorage {
    downloads_folder_path: PathBuf,
}

impl ChapterStorage {
    pub fn new(downloads_folder_path: PathBuf) -> Self {
        Self {
            downloads_folder_path,
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

    fn path_for_chapter(&self, chapter_id: &ChapterId) -> PathBuf {
        let output_filename = format!(
            "{}-{}.cbz",
            chapter_id.source_id().value(),
            chapter_id.value()
        );

        self.downloads_folder_path.join(output_filename)
    }
}
