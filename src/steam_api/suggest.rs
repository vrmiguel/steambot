use anyhow::Context;
use serde::Deserialize;

use crate::utils::deserialize_string_to_usize;

#[derive(Debug, Deserialize)]
pub struct Suggestion {
    #[serde(deserialize_with = "deserialize_string_to_usize")]
    pub id: usize,
    pub img: String,
    pub name: String,
    pub price: String,
    pub small_cap: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

pub async fn get_suggestions(term: &str) -> anyhow::Result<Vec<Suggestion>> {
    let encoded_term = urlencoding::Encoded::new(term);
    let url = format!("https://store.steampowered.com/search/suggest?cc=BR&l=brazilian&realm=1&origin=https:%2F%2Fstore.steampowered.com&f=jsonfull&term={encoded_term}&require_type=game,software");

    reqwest::get(url)
        .await?
        .json()
        .await
        .with_context(|| "Failed to deserialize response of /suggest")
}

#[tokio::test]
async fn gets_suggestions() {
    get_suggestions("tf2")
        .await
        .unwrap()
        .into_iter()
        .any(|suggestion| suggestion.name == "Team Fortress 2");
}
