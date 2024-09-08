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
    version: usize,
}

impl From<DomainSourceInformation> for SourceInformation {
    fn from(value: DomainSourceInformation) -> Self {
        Self {
            id: value.id.value().clone(),
            name: value.name,
            version: value.version,
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
            // FIXME mangas already contain the source information, this is kinda redundant rn
            source_information: value.source_information.clone().into(),
            mangas: value
                .mangas
                .into_iter()
                .map(|manga| Manga::from((value.source_information.clone(), manga)))
                .collect(),
        }
    }
}

#[derive(Serialize)]
pub struct Manga {
    // FIXME maybe both `id` and `source_id` should be encoded into a single field
    // imo it makes more sense from the frontend perspective
    id: String,
    source: SourceInformation,
    title: String,
}

impl From<(DomainSourceInformation, MangaInformation)> for Manga {
    fn from(
        (source_information, manga_information): (DomainSourceInformation, MangaInformation),
    ) -> Self {
        Self {
            id: manga_information.id.value().clone(),
            source: source_information.into(),
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
