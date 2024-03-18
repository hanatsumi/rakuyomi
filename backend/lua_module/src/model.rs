use cli::source::model::{Chapter as SourceChapter, Manga as SourceManga};
use serde::Serialize;

#[derive(Serialize)]
pub struct Manga {
    // FIXME maybe both `id` and `source_id` should be encoded into a single field
    // imo it makes more sense from the frontend perspective
    id: String,
    source_id: String,
    title: String,
}

impl From<SourceManga> for Manga {
    fn from(value: SourceManga) -> Self {
        Self {
            id: value.id,
            source_id: value.source_id,
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
}

impl From<SourceChapter> for Chapter {
    fn from(value: SourceChapter) -> Self {
        Self {
            source_id: value.source_id,
            manga_id: value.manga_id,
            id: value.id,
            title: value.title.unwrap_or("Unknown title".into()),
            scanlator: value.scanlator,
            chapter_num: value.chapter_num,
            volume_num: value.volume_num,
        }
    }
}
