use crate::{model::SourceId, source::Source};

pub trait SourceCollection {
    fn get_by_id(&self, id: &SourceId) -> Option<&Source>;
    fn sources(&self) -> Vec<&Source>;
}
