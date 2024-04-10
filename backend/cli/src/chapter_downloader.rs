use futures::{stream, StreamExt, TryStreamExt};
use std::{
    io::Seek,
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

use anyhow::anyhow;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::{
    chapter_storage::ChapterStorage,
    model::ChapterId,
    source::{model::Page, Source},
};

const CONCURRENT_REQUESTS: usize = 4;

pub async fn ensure_chapter_is_in_storage(
    chapter_storage: &ChapterStorage,
    source: &Source,
    chapter_id: &ChapterId,
) -> Result<PathBuf, Error> {
    if let Some(path) = chapter_storage.get_stored_chapter(chapter_id) {
        return Ok(path);
    }

    // FIXME like downloaderror is a really bad name??
    let pages = source
        .get_page_list(
            chapter_id.manga_id().value().clone(),
            chapter_id.value().clone(),
        )
        .await
        .map_err(Error::DownloadError)?;

    let output_path = chapter_storage.get_path_to_store_chapter(chapter_id);

    // Write chapter pages to a temporary file, so that if things go wrong
    // we do not have a borked .cbz file in the chapter storage.
    let temporary_file =
        NamedTempFile::new_in(output_path.parent().unwrap()).map_err(|e| Error::Other(e.into()))?;
    download_chapter_pages_as_cbz(&temporary_file, source, pages)
        .await
        .map_err(Error::DownloadError)?;

    // If we succeeded downloading all the chapter pages, persist our temporary
    // file into the chapter storage definitively.
    temporary_file
        .persist(&output_path)
        .map_err(|e| Error::Other(e.into()))?;

    Ok(output_path)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("an error occurred while downloading the chapter pages")]
    DownloadError(#[source] anyhow::Error),
    #[error("unknown error")]
    Other(#[from] anyhow::Error),
}

pub async fn download_chapter_pages_as_cbz<W>(
    output: W,
    source: &Source,
    pages: Vec<Page>,
) -> anyhow::Result<()>
where
    W: Write + Seek,
{
    let mut writer = ZipWriter::new(output);
    let client = reqwest::Client::new();
    let file_options = FileOptions::default().compression_method(CompressionMethod::Stored);

    dbg!(&pages);

    stream::iter(pages)
        .map(|page| {
            let client = &client;

            async move {
                let image_url = page.image_url.ok_or(anyhow!("page has no image URL"))?;
                let extension = Path::new(image_url.path())
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("jpg")
                    .to_owned();

                // FIXME we should left pad this number with zeroes up to the maximum
                // amount of pages needed, but for now we pad 4 digits
                // stop reading the bible if this ever becomes an issue
                let filename = format!("{:0>4}.{}", page.index, extension);

                // TODO we could stream the data from the client into the file
                // would save a bit of memory but i dont think its a big deal
                let request = source.get_image_request(image_url).await?;
                let response_bytes = client
                    .execute(request)
                    .await?
                    .error_for_status()?
                    .bytes()
                    .await?;

                anyhow::Ok((filename, response_bytes))
            }
        })
        .buffer_unordered(CONCURRENT_REQUESTS)
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .try_for_each(|(filename, response_bytes)| {
            writer.start_file(filename, file_options)?;
            writer.write_all(response_bytes.as_ref())?;

            Ok(())
        })
}
