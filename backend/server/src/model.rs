use cli::model::{Chapter as DomainChapter, MangaInformation};
use serde::Serialize;

#[derive(Serialize)]
pub struct Manga {
    // FIXME maybe both `id` and `source_id` should be encoded into a single field
    // imo it makes more sense from the frontend perspective
    id: String,
    source_id: String,
    title: String,
}

impl From<MangaInformation> for Manga {
    fn from(value: MangaInformation) -> Self {
        Self {
            id: value.id.manga_id,
            source_id: value.id.source_id.0,
            // FIXME what the fuck
            title: value.title.unwrap_or("Unknown title".into()),
        }
    }
}

#[derive(Serialize)]
pub struct Chapter {
    source_id: String,
    manga_id: String,
    id: String,
    title: String,
    scanlator: Option<String>,
    chapter_num: Option<f32>,
    volume_num: Option<f32>,
    read: bool,
    downloaded: bool,
}

impl From<DomainChapter> for Chapter {
    fn from(
        DomainChapter {
            information,
            state,
            downloaded,
        }: DomainChapter,
    ) -> Self {
        Self {
            // FIXME what the fuck why
            source_id: information.id.manga_id.source_id.0,
            manga_id: information.id.manga_id.manga_id,
            id: information.id.chapter_id,
            title: information.title.unwrap_or("Unknown title".into()),
            scanlator: information.scanlator,
            chapter_num: information
                .chapter_number
                .map(|decimal| decimal.try_into().unwrap()),
            volume_num: information
                .chapter_number
                .map(|decimal| decimal.try_into().unwrap()),
            read: state.read,
            downloaded,
        }
    }
}
