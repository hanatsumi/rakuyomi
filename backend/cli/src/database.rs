use std::path::Path;

use anyhow::Result;
use sqlx::{sqlite::SqliteConnectOptions, Pool, Sqlite};

use crate::model::{
    ChapterId, ChapterInformation, ChapterState, MangaId, MangaInformation, MangaState, SourceId,
};

pub struct Database {
    pool: Pool<Sqlite>,
}

// FIXME add proper error handling
impl Database {
    pub async fn new(filename: &Path) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(filename)
            .create_if_missing(true);
        let pool = Pool::connect_with(options).await?;

        sqlx::migrate!().run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn get_manga_library(&self) -> Vec<MangaId> {
        let rows = sqlx::query_as!(
            MangaLibraryRow,
            r#"
                SELECT * FROM manga_library;
            "#
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        rows.into_iter().map(|row| row.manga_id()).collect()
    }

    pub async fn add_manga_to_library(&self, id: MangaId) {
        sqlx::query!(
            r#"
                INSERT INTO manga_library (source_id, manga_id)
                VALUES (?1, ?2)
                ON CONFLICT DO NOTHING
            "#,
            id.source_id.0,
            id.manga_id
        )
        .execute(&self.pool)
        .await
        .unwrap();
    }

    pub async fn find_cached_manga_information(
        &self,
        manga_id: &MangaId,
    ) -> Option<MangaInformation> {
        let maybe_row = sqlx::query_as!(
            MangaInformationsRow,
            r#"
                SELECT * FROM manga_informations
                    WHERE source_id = ?1 AND manga_id = ?2;
            "#,
            manga_id.source_id.0,
            manga_id.manga_id
        )
        .fetch_optional(&self.pool)
        .await
        .unwrap();

        maybe_row.map(|row| row.into())
    }

    pub async fn find_cached_chapter_informations(
        &self,
        manga_id: &MangaId,
    ) -> Vec<ChapterInformation> {
        let rows = sqlx::query_as!(
            ChapterInformationsRow,
            r#"
                SELECT * FROM chapter_informations
                WHERE source_id = ?1 AND manga_id = ?2;
            "#,
            manga_id.source_id.0,
            manga_id.manga_id
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        rows.into_iter().map(|row| row.into()).collect()
    }

    pub async fn upsert_cached_manga_information(&self, information: MangaInformation) {
        let cover_url = information.cover_url.map(|url| url.to_string());

        sqlx::query!(
            r#"
                INSERT INTO manga_informations (source_id, manga_id, title, author, artist, cover_url)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT DO UPDATE SET
                    title = excluded.title,
                    author = excluded.author,
                    artist = excluded.artist,
                    cover_url = excluded.cover_url
            "#,
            information.id.source_id.0,
            information.id.manga_id,
            information.title,
            information.author,
            information.artist,
            cover_url
        ).execute(&self.pool).await.unwrap();
    }

    pub async fn upsert_cached_chapter_information(&self, information: ChapterInformation) {
        let chapter_number = information
            .chapter_number
            .map(|dec| f64::try_from(dec).unwrap());
        let volume_number = information
            .volume_number
            .map(|dec| f64::try_from(dec).unwrap());

        sqlx::query!(
            r#"
                INSERT INTO chapter_informations (source_id, manga_id, chapter_id, title, scanlator, chapter_number, volume_number)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT DO UPDATE SET
                    title = excluded.title,
                    scanlator = excluded.scanlator,
                    chapter_number = excluded.chapter_number,
                    volume_number = excluded.volume_number
            "#,
            information.id.manga_id.source_id.0,
            information.id.manga_id.manga_id,
            information.id.chapter_id,
            information.title,
            information.scanlator,
            chapter_number,
            volume_number
        ).execute(&self.pool).await.unwrap();
    }

    pub async fn find_manga_state(&self, id: &MangaId) -> Option<MangaState> {
        todo!()
    }

    pub async fn find_chapter_state(&self, id: &ChapterId) -> Option<ChapterState> {
        // FIXME we should be able to just specify a override for the `read` field here,
        // but there's a bug in sqlx preventing us: https://github.com/launchbadge/sqlx/issues/2295
        let maybe_row = sqlx::query_as!(
            ChapterStateRow,
            r#"
                SELECT source_id, manga_id, chapter_id, read AS "read: bool" FROM chapter_state
                WHERE source_id = ?1 AND manga_id = ?2 AND chapter_id = ?3;
            "#,
            id.manga_id.source_id.0,
            id.manga_id.manga_id,
            id.chapter_id
        )
        .fetch_optional(&self.pool)
        .await
        .unwrap();

        maybe_row.map(|row| row.into())
    }
}

#[derive(sqlx::FromRow)]
struct MangaInformationsRow {
    source_id: String,
    manga_id: String,
    title: Option<String>,
    author: Option<String>,
    artist: Option<String>,
    cover_url: Option<String>,
}

impl From<MangaInformationsRow> for MangaInformation {
    fn from(value: MangaInformationsRow) -> Self {
        Self {
            id: MangaId {
                source_id: SourceId(value.source_id),
                manga_id: value.manga_id,
            },
            title: value.title,
            author: value.author,
            artist: value.artist,
            cover_url: value
                .cover_url
                .map(|url_string| url_string.as_str().try_into().unwrap()),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ChapterInformationsRow {
    source_id: String,
    manga_id: String,
    chapter_id: String,
    title: Option<String>,
    scanlator: Option<String>,
    chapter_number: Option<f64>,
    volume_number: Option<f64>,
}

impl From<ChapterInformationsRow> for ChapterInformation {
    fn from(value: ChapterInformationsRow) -> Self {
        Self {
            id: ChapterId {
                manga_id: MangaId {
                    source_id: SourceId(value.source_id),
                    manga_id: value.manga_id,
                },
                chapter_id: value.chapter_id,
            },
            title: value.title,
            scanlator: value.scanlator,
            chapter_number: value
                .chapter_number
                .map(|decimal_as_f64| decimal_as_f64.try_into().unwrap()),
            volume_number: value
                .volume_number
                .map(|decimal_as_f64| decimal_as_f64.try_into().unwrap()),
        }
    }
}

#[derive(sqlx::FromRow)]
struct MangaLibraryRow {
    source_id: String,
    manga_id: String,
}

impl MangaLibraryRow {
    pub fn manga_id(self) -> MangaId {
        MangaId {
            source_id: SourceId(self.source_id),
            manga_id: self.manga_id,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ChapterStateRow {
    source_id: String,
    manga_id: String,
    chapter_id: String,
    read: bool,
}

impl From<ChapterStateRow> for ChapterState {
    fn from(value: ChapterStateRow) -> Self {
        Self { read: value.read }
    }
}
