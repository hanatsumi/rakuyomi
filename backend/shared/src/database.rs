use std::{collections::HashSet, path::Path};

use anyhow::Result;
use futures::{stream, StreamExt, TryStreamExt};
use sqlx::{sqlite::SqliteConnectOptions, Pool, QueryBuilder, Sqlite};

use crate::model::{
    ChapterId, ChapterInformation, ChapterState, MangaId, MangaInformation, MangaState,
};

pub struct Database {
    pool: Pool<Sqlite>,
}

const BIND_LIMIT: usize = 32766;

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

    pub async fn add_manga_to_library(&self, manga_id: MangaId) {
        let source_id = manga_id.source_id().value();
        let manga_id = manga_id.value();

        sqlx::query!(
            r#"
                INSERT INTO manga_library (source_id, manga_id)
                VALUES (?1, ?2)
                ON CONFLICT DO NOTHING
            "#,
            source_id,
            manga_id
        )
        .execute(&self.pool)
        .await
        .unwrap();
    }

    pub async fn remove_manga_from_library(&self, manga_id: MangaId) {
        let source_id = manga_id.source_id().value();
        let manga_id = manga_id.value();

        sqlx::query!(
            r#"
                DELETE FROM manga_library
                WHERE source_id = ?1 AND manga_id = ?2
            "#,
            source_id,
            manga_id
        )
        .execute(&self.pool)
        .await
        .unwrap();
    }

    pub async fn find_cached_manga_information(
        &self,
        manga_id: &MangaId,
    ) -> Option<MangaInformation> {
        let source_id = manga_id.source_id().value();
        let manga_id = manga_id.value();

        let maybe_row = sqlx::query_as!(
            MangaInformationsRow,
            r#"
                SELECT * FROM manga_informations
                    WHERE source_id = ?1 AND manga_id = ?2;
            "#,
            source_id,
            manga_id
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
        let source_id = manga_id.source_id().value();
        let manga_id = manga_id.value();

        let rows = sqlx::query_as!(
            ChapterInformationsRow,
            r#"
                SELECT * FROM chapter_informations
                WHERE source_id = ?1 AND manga_id = ?2
                ORDER BY manga_order ASC;
            "#,
            source_id,
            manga_id
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        rows.into_iter().map(|row| row.into()).collect()
    }

    pub async fn upsert_cached_manga_information(&self, manga_information: MangaInformation) {
        let source_id = manga_information.id.source_id().value();
        let manga_id = manga_information.id.value();
        let cover_url = manga_information.cover_url.map(|url| url.to_string());

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
            source_id,
            manga_id,
            manga_information.title,
            manga_information.author,
            manga_information.artist,
            cover_url
        ).execute(&self.pool).await.unwrap();
    }

    pub async fn upsert_cached_chapter_informations(
        &self,
        manga_id: &MangaId,
        chapter_informations: Vec<ChapterInformation>,
    ) {
        // We need to both update the existing information about the chapters, and delete the chapters that are no longer present
        let cached_chapter_ids: HashSet<_> = self
            .find_cached_chapter_informations(manga_id)
            .await
            .into_iter()
            .map(|information| information.id)
            .collect();

        let chapter_ids: HashSet<_> = chapter_informations
            .iter()
            .map(|information| information.id.clone())
            .collect();

        let removed_chapter_ids: Vec<_> =
            (&cached_chapter_ids - &chapter_ids).into_iter().collect();

        // We use 2 binds to place the `source_id` and `manga_id` on the query.
        let remove_chapters_query_available_binds = BIND_LIMIT - 2;

        stream::iter(removed_chapter_ids.chunks(remove_chapters_query_available_binds))
            .then(|chapter_ids| async move {
                let mut builder = QueryBuilder::new("DELETE FROM chapter_informations WHERE ");

                builder
                    .push(" source_id = ")
                    .push_bind(manga_id.source_id().value())
                    .push(" AND manga_id = ")
                    .push_bind(manga_id.value())
                    .push(" AND chapter_id IN ")
                    .push_tuples(chapter_ids, |mut b, chapter_id| {
                        b.push_bind(chapter_id.value());
                    });

                builder.build().execute(&self.pool).await?;

                anyhow::Ok(())
            })
            .try_collect::<()>()
            .await
            .unwrap();

        let insert_field_count = 8;
        stream::iter(chapter_informations.into_iter().enumerate().collect::<Vec<_>>().chunks(BIND_LIMIT / insert_field_count))
            .then(|enumerated_chapter_informations| async move {
                let mut builder = QueryBuilder::new(
                    "INSERT INTO chapter_informations (source_id, manga_id, chapter_id, manga_order, title, scanlator, chapter_number, volume_number) "
                );

                builder
                    .push_values(enumerated_chapter_informations, |mut b, (index, chapter_information)| {
                        let index = *index as i64;
                        let source_id = chapter_information.id.source_id().value();
                        let manga_id = chapter_information.id.manga_id().value();
                        let chapter_id = chapter_information.id.value();

                        let chapter_number = chapter_information
                            .chapter_number
                            .map(|dec| f64::try_from(dec).unwrap());
                        let volume_number = chapter_information
                            .volume_number
                            .map(|dec| f64::try_from(dec).unwrap());

                        b.push_bind(source_id)
                            .push_bind(manga_id)
                            .push_bind(chapter_id)
                            .push_bind(index)
                            .push_bind(chapter_information.title.clone())
                            .push_bind(chapter_information.scanlator.clone())
                            .push_bind(chapter_number)
                            .push_bind(volume_number);
                    })
                    .push(r#"
                        ON CONFLICT DO UPDATE SET
                        manga_order = excluded.manga_order,
                        title = excluded.title,
                        scanlator = excluded.scanlator,
                        chapter_number = excluded.chapter_number,
                        volume_number = excluded.volume_number
                    "#);

                builder.build().execute(&self.pool).await?;

                anyhow::Ok(())
            })
            .try_collect::<()>().await.unwrap();
    }

    pub async fn find_manga_state(&self, _id: &MangaId) -> Option<MangaState> {
        todo!()
    }

    pub async fn find_chapter_state(&self, chapter_id: &ChapterId) -> Option<ChapterState> {
        let source_id = chapter_id.source_id().value();
        let manga_id = chapter_id.manga_id().value();
        let chapter_id = chapter_id.value();

        // FIXME we should be able to just specify a override for the `read` field here,
        // but there's a bug in sqlx preventing us: https://github.com/launchbadge/sqlx/issues/2295
        let maybe_row = sqlx::query_as!(
            ChapterStateRow,
            r#"
                SELECT source_id, manga_id, chapter_id, read AS "read: bool" FROM chapter_state
                WHERE source_id = ?1 AND manga_id = ?2 AND chapter_id = ?3;
            "#,
            source_id,
            manga_id,
            chapter_id,
        )
        .fetch_optional(&self.pool)
        .await
        .unwrap();

        maybe_row.map(|row| row.into())
    }

    pub async fn upsert_chapter_state(&self, chapter_id: &ChapterId, state: ChapterState) {
        let source_id = chapter_id.source_id().value();
        let manga_id = chapter_id.manga_id().value();
        let chapter_id = chapter_id.value();

        sqlx::query!(
            r#"
                INSERT INTO chapter_state (source_id, manga_id, chapter_id, read)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT DO UPDATE SET
                    read = excluded.read
            "#,
            source_id,
            manga_id,
            chapter_id,
            state.read,
        )
        .execute(&self.pool)
        .await
        .unwrap();
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
            id: MangaId::from_strings(value.source_id, value.manga_id),
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
    #[allow(dead_code)]
    manga_order: i64,
    title: Option<String>,
    scanlator: Option<String>,
    chapter_number: Option<f64>,
    volume_number: Option<f64>,
}

impl From<ChapterInformationsRow> for ChapterInformation {
    fn from(value: ChapterInformationsRow) -> Self {
        Self {
            id: ChapterId::from_strings(value.source_id, value.manga_id, value.chapter_id),
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
        MangaId::from_strings(self.source_id, self.manga_id)
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
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
