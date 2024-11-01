use axum::extract::State as StateExtractor;
use axum::routing::{get, put};
use axum::{Json, Router};
use cli::usecases;
use cli::usecases::update_settings::UpdateableSettings;

use crate::state::State;
use crate::AppError;

pub fn routes() -> Router<State> {
    Router::new()
        .route("/settings", get(get_settings))
        .route("/settings", put(update_settings))
}

async fn get_settings(
    StateExtractor(State { settings, .. }): StateExtractor<State>,
) -> Json<UpdateableSettings> {
    Json(UpdateableSettings::from(&*settings.lock().await))
}

async fn update_settings(
    StateExtractor(State {
        settings,
        settings_path,
        ..
    }): StateExtractor<State>,
    Json(updateable_settings): Json<UpdateableSettings>,
) -> Result<Json<UpdateableSettings>, AppError> {
    let mut settings = settings.lock().await;
    usecases::update_settings(&mut settings, &settings_path, updateable_settings)?;

    Ok(Json(UpdateableSettings::from(&*settings)))
}
