use crate::{
    database::Database,
    model::{ChapterId, ChapterState},
};

pub async fn mark_chapter_as_read(db: &Database, id: ChapterId) {
    let chapter_state = db.find_chapter_state(&id).await.unwrap_or_default();
    #[allow(clippy::needless_update)]
    let updated_chapter_state = ChapterState {
        read: true,
        ..chapter_state
    };

    db.upsert_chapter_state(&id, updated_chapter_state).await;
}
