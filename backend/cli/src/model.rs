use rust_decimal::Decimal;
use url::Url;

use crate::source::model::{Chapter as SourceChapter, Manga as SourceManga};

#[derive(Clone)]
pub struct SourceId(pub String);

#[derive(Clone)]
pub struct MangaId {
    pub source_id: SourceId,
    pub manga_id: String,
}

#[derive(Clone)]
pub struct ChapterId {
    pub manga_id: MangaId,
    pub chapter_id: String,
}

#[derive(Clone)]
pub struct MangaInformation {
    pub id: MangaId,
    pub title: Option<String>,
    pub author: Option<String>,
    pub artist: Option<String>,
    pub cover_url: Option<Url>,
}

#[derive(Clone)]
pub struct ChapterInformation {
    pub id: ChapterId,
    pub title: Option<String>,
    pub scanlator: Option<String>,
    pub chapter_number: Option<Decimal>,
    pub volume_number: Option<Decimal>,
}

pub struct MangaState;

#[derive(Default)]
pub struct ChapterState {
    pub read: bool,
}

impl From<SourceManga> for MangaInformation {
    fn from(value: SourceManga) -> Self {
        Self {
            id: MangaId {
                source_id: SourceId(value.source_id),
                manga_id: value.id,
            },
            title: value.title,
            author: value.author,
            artist: value.artist,
            cover_url: value.cover_url,
        }
    }
}

impl From<SourceChapter> for ChapterInformation {
    fn from(value: SourceChapter) -> Self {
        Self {
            id: ChapterId {
                manga_id: MangaId {
                    source_id: SourceId(value.source_id),
                    manga_id: value.manga_id,
                },
                chapter_id: value.id,
            },
            title: value.title,
            scanlator: value.scanlator,
            // FIXME is this ever fallible?
            chapter_number: value.chapter_num.map(|num| num.try_into().unwrap()),
            volume_number: value.volume_num.map(|num| num.try_into().unwrap()),
        }
    }
}
