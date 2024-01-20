use anyhow::Context;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtonDBCompatibility {
    pub total: i64,
    pub trending_tier: String,
}

pub async fn get_proton_compatibility(app_id: usize) -> anyhow::Result<ProtonDBCompatibility> {
    let url = format!("https://www.protondb.com/api/v1/reports/summaries/{app_id}.json");

    reqwest::get(url)
        .await?
        .json()
        .await
        .with_context(|| "Failed to obtain ProtonDB compatibility")
}
