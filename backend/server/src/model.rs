use cli::{
    model::{
        Chapter as DomainChapter, MangaInformation, SourceInformation as DomainSourceInformation,
    },
    usecases::search_mangas::SourceMangaSearchResults as DomainSourceMangaSearchResults,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct SourceInformation {
    id: String,
    name: String,
}

impl From<DomainSourceInformation> for SourceInformation {
    fn from(value: DomainSourceInformation) -> Self {
        Self {
            id: value.id.value().clone(),
            name: value.name,
        }
    }
}

#[derive(Serialize)]
pub struct SourceMangaSearchResults {
    source_information: SourceInformation,
    mangas: Vec<Manga>,
}

impl From<DomainSourceMangaSearchResults> for SourceMangaSearchResults {
    fn from(value: DomainSourceMangaSearchResults) -> Self {
        Self {
            source_information: value.source_information.into(),
            mangas: value.mangas.into_iter().map(Manga::from).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct Manga {
    // FIXME maybe both `id` and `source_id` should be encoded into a single field
    // imo it makes more sense from the frontend perspective
    id: String,
    source_id: String,
    title: String,
}

impl From<MangaInformation> for Manga {
    fn from(manga_information: MangaInformation) -> Self {
        Self {
            id: manga_information.id.value().clone(),
            source_id: manga_information.id.source_id().value().clone(),
            // FIXME what the fuck
            title: manga_information.title.unwrap_or("Unknown title".into()),
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
            information: chapter_information,
            state,
            downloaded,
        }: DomainChapter,
    ) -> Self {
        Self {
            // FIXME what the fuck why
            source_id: chapter_information.id.source_id().value().clone(),
            manga_id: chapter_information.id.manga_id().value().clone(),
            id: chapter_information.id.value().clone(),
            title: chapter_information.title.unwrap_or("Unknown title".into()),
            scanlator: chapter_information.scanlator,
            chapter_num: chapter_information
                .chapter_number
                .map(|decimal| decimal.try_into().unwrap()),
            volume_num: chapter_information
                .volume_number
                .map(|decimal| decimal.try_into().unwrap()),
            read: state.read,
            downloaded,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "type")]
pub enum DownloadAllChaptersProgress {
    Initializing,
    Progressing { downloaded: usize, total: usize },
    Finished,
    Cancelled,
}
