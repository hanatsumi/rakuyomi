use axum::{
    async_trait,
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use cli::{model::SourceId, source::Source};
use serde::Deserialize;

use crate::{AppError, State};

pub struct SourceExtractor(pub Source);

#[derive(Deserialize)]
struct SourceParams {
    source_id: String,
}

#[async_trait]
impl FromRequestParts<State> for SourceExtractor {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state @ State {
            source_collection, ..
        }: &State,
    ) -> Result<Self, Self::Rejection> {
        let Path(SourceParams { source_id }) = Path::from_request_parts(parts, state).await?;
        let source = source_collection
            .get_by_id(&SourceId::new(source_id))
            .ok_or(AppError::SourceNotFound)?;

        // FIXME this relies upon the `Source` being an `Arc` internally in
        // order to keep things performant.
        // If it wasn't, this would be pretty wasteful... and overall this is
        // pretty bad
        Ok(Self(source.clone()))
    }
}
