use std::path::PathBuf;

use serde::Serialize;

use crate::ErrorResponse;

use super::state::Job;

#[derive(Serialize)]
#[serde(untagged)]
pub enum CompletedJobResult {
    FetchChapter(PathBuf),
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "type", content = "data")]
pub enum JobDetail {
    Pending,
    Completed(CompletedJobResult),
    Error(ErrorResponse),
}

impl JobDetail {
    pub async fn from_job(job: Job) -> (Self, Option<Job>) {
        let Job::FetchChapter(handle) = job;
        if !handle.is_finished() {
            return (JobDetail::Pending, Some(Job::FetchChapter(handle)));
        }

        let detail = match handle.await.unwrap() {
            Ok(path) => JobDetail::Completed(CompletedJobResult::FetchChapter(path)),
            Err(e) => JobDetail::Error(ErrorResponse::from(&e)),
        };

        (detail, None)
    }
}
