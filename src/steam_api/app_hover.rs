use anyhow::Context;
use serde::Deserialize;

use crate::HTTP_CLIENT;

#[derive(Debug, Deserialize)]
pub struct AppHoverDetails {
    #[serde(rename = "strReleaseDate")]
    pub release_date: String,
    #[serde(rename = "strDescription")]
    pub description: String,
    #[serde(rename = "ReviewSummary")]
    pub review_summary: ReviewSummary,
}

#[derive(Debug, Deserialize)]
pub struct ReviewSummary {
    #[serde(rename = "strReviewSummary")]
    pub review_summary: String,
    #[serde(rename = "cReviews")]
    pub review_count: usize,
}

pub async fn get_app_hover_details(app_id: usize) -> anyhow::Result<AppHoverDetails> {
    let url =
        format!("https://store.steampowered.com/apphoverpublic/{app_id}/?l=brazilian&json=1&cc=br");

    HTTP_CLIENT
        .get(url)
        .send()
        .await?
        .json()
        .await
        .with_context(|| "Failed to deserialize response of /apphoverpublic ")
}
