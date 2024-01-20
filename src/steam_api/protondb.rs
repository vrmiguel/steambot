use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtonDBCompatibility {
    pub total: i64,
    pub trending_tier: String,
}
