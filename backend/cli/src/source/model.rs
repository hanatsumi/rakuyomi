use std::default;

use chrono::DateTime;
use num_enum::FromPrimitive;
use url::Url;

#[derive(Debug, Clone, Default, FromPrimitive)]
#[repr(u8)]
pub enum PublishingStatus {
    #[default]
    Unknown = 0,
    Ongoing = 1,
    Completed = 2,
    Cancelled = 3,
    Hiatus = 4,
    NotPublished = 5,
}

#[derive(Debug, Clone, Default, FromPrimitive)]
#[repr(u8)]
pub enum MangaContentRating {
    #[default]
    Safe = 0,
    Suggestive = 1,
    Nsfw = 2,
}

#[derive(Debug, Clone, Default, FromPrimitive)]
#[repr(u8)]
pub enum MangaViewer {
    #[default]
    DefaultViewer = 0,
    Rtl = 1,
    Ltr = 2,
    Vertical = 3,
    Scroll = 4,
}

#[derive(Debug, Clone, Default)]
pub struct Manga {
    pub source_id: String,
    pub id: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub artist: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub cover_url: Option<Url>,
    pub url: Option<Url>,
    pub status: PublishingStatus,
    pub nsfw: MangaContentRating,
    pub viewer: MangaViewer,
    // FIXME i dont think those are needed, the sources have no way of creating them
    pub last_updated: Option<DateTime<chrono_tz::Tz>>,
    pub last_opened: Option<DateTime<chrono_tz::Tz>>,
    pub last_read: Option<DateTime<chrono_tz::Tz>>,
    pub date_added: Option<DateTime<chrono_tz::Tz>>,
}

#[derive(Debug, Clone, Default)]
pub struct MangaPageResult {
    // FIXME should not this be `mangas` instead?
    pub manga: Vec<Manga>,
    pub has_next_page: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Chapter {
    pub source_id: String,
    pub id: String,
    pub manga_id: String,
    pub title: Option<String>,
    pub scanlator: Option<String>,
    pub url: Option<Url>,
    pub lang: String,
    pub chapter_num: Option<f32>,
    pub volume_num: Option<f32>,
    pub date_uploaded: Option<DateTime<chrono_tz::Tz>>,
    // FIXME do we like really need this? aidoku only uses it to order stuff
    // on the display page, but we already return an array on the get chapter list
    // call, so there's already an ordering there
    pub source_order: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Page {
    pub source_id: String,
    pub chapter_id: String,
    pub index: usize,
    pub image_url: Option<Url>,
    pub base64: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DeepLink {
    // FIXME should we store references here?
    pub manga: Option<Manga>,
    pub chapter: Option<Chapter>,
}

#[derive(Debug, Copy, Clone, Default, FromPrimitive)]
#[repr(u8)]
pub enum FilterType {
    #[default]
    Base = 0,
    Group = 1,
    Text = 2,
    Check = 3,
    Select = 4,
    Sort = 5,
    SortSelection = 6,
    Title = 7,
    Author = 8,
    Genre = 9,
}

#[derive(Debug, Clone)]
pub enum Filter {
    Title(String),
}

impl From<&Filter> for FilterType {
    fn from(value: &Filter) -> Self {
        match &value {
            Filter::Title(_) => FilterType::Title,
        }
    }
}

impl Filter {
    pub fn name(&self) -> String {
        match &self {
            Filter::Title(_) => "Title".into(),
        }
    }
}
