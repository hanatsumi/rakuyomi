use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{chapter_downloader::download_chapter_pages_as_cbz, model::ChapterId, source::Source};

pub async fn fetch_manga_chapter(
    source: &Source,
    downloads_folder_path: &Path,
    chapter_id: &ChapterId,
) -> Result<PathBuf, Error> {
    let output_filename = format!(
        "{}-{}.cbz",
        &chapter_id.manga_id.source_id.0, &chapter_id.chapter_id
    );
    let output_path = downloads_folder_path.join(output_filename);

    if output_path.exists() {
        return Ok(output_path);
    }

    let pages = source
        .get_page_list(
            chapter_id.manga_id.manga_id.clone(),
            chapter_id.chapter_id.clone(),
        )
        .await?;

    let output_file = fs::File::create(&output_path).map_err(|e| anyhow::Error::from(e))?;
    download_chapter_pages_as_cbz(output_file, pages)
        .await
        .map_err(Error::DownloadError)?;

    Ok(output_path)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("an error occurred while downloading the chapter pages")]
    DownloadError(#[source] anyhow::Error),
    #[error("unknown error")]
    Other(#[from] anyhow::Error),
}
