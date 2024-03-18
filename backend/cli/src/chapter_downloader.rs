use std::{
    io::Seek,
    io::{self, Write},
    path::Path,
};

use anyhow::{anyhow, Result};
use zip::{write::FileOptions, ZipWriter};

use crate::source::model::Page;

pub fn download_chapter_pages_as_cbz<W>(output: W, pages: Vec<Page>) -> Result<()>
where
    W: Write + Seek,
{
    let mut writer = ZipWriter::new(output);
    let client = reqwest::blocking::Client::new();

    pages.into_iter().try_for_each(|page| {
        let image_url = page.image_url.ok_or(anyhow!("page has no image URL"))?;
        let extension = Path::new(image_url.path())
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("jpg")
            .to_owned();

        // TODO we could stream the data from the client into the file
        // would save a bit of memory but i dont think its a big deal
        let response = client.get(image_url).send()?;

        // FIXME we should left pad this number with zeroes up to the maximum
        // amount of pages needed, but for now we pad 4 digits
        // stop reading the bible if this ever becomes an issue
        let filename = format!("{:0>4}.{}", page.index, extension);

        writer.start_file(filename, FileOptions::default())?;
        writer.write_all(response.bytes()?.as_ref())?;

        Ok(())
    })
}
