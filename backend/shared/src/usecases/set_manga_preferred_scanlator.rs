use crate::{
    database::Database,
    model::{MangaId, MangaState},
};

pub async fn set_manga_preferred_scanlator(
    db: &Database, 
    manga_id: MangaId, 
    preferred_scanlator: Option<String>
) {
    let library_mangas = db.get_manga_library().await;
    if !library_mangas.contains(&manga_id) {
        return;
    }

    let manga_state = db.find_manga_state(&manga_id).await.unwrap_or_default();
    
    let updated_manga_state = MangaState {
        preferred_scanlator,
        ..manga_state
    };
    
    db.upsert_manga_state(&manga_id, updated_manga_state).await;
}

pub async fn get_manga_preferred_scanlator(
    db: &Database, 
    manga_id: &MangaId
) -> Option<String> {
    db.find_manga_state(manga_id)
        .await
        .and_then(|state| state.preferred_scanlator)
}