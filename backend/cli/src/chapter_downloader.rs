use futures::{stream, StreamExt, TryStreamExt};
use std::{io::Seek, io::Write, path::Path};

use anyhow::{anyhow, Ok, Result};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::source::model::Page;

const CONCURRENT_REQUESTS: usize = 4;

pub async fn download_chapter_pages_as_cbz<W>(output: W, pages: Vec<Page>) -> Result<()>
where
    W: Write + Seek,
{
    let mut writer = ZipWriter::new(output);
    let client = reqwest::Client::new();
    let file_options = FileOptions::default().compression_method(CompressionMethod::Stored);

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
                let response_bytes = client.get(image_url).send().await?.bytes().await?;

                Ok((filename, response_bytes))
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
