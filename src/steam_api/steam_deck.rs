use anyhow::Context;
use serde::Deserialize;

use crate::HTTP_CLIENT;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeckCompat {
    success: u8,
    results: Results,
}

#[derive(Debug, Deserialize)]
struct Results {
    resolved_category: i64,
}

pub async fn get_steam_deck_compatibility(app_id: usize) -> anyhow::Result<Option<&'static str>> {
    let url = format!(
        "https://store.steampowered.com/saleaction/ajaxgetdeckappcompatibilityreport?nAppID={app_id}&l=english&cc=US"
    );

    let compat: DeckCompat = HTTP_CLIENT
        .get(url)
        .send()
        .await?
        .json()
        .await
        .with_context(|| "Failed to deserialize Deck compat")?;

    if compat.success != 1 {
        Ok(None)
    } else {
        let compatibility_summary = match compat.results.resolved_category {
            1 => "🚫 Não suportado",
            2 => "ℹ️ Jogável",
            3 => "✅ Verificado",
            other => {
                tracing::error!("Got unknown Deck compatibility category: {other}");

                return Ok(None);
            }
        };

        Ok(Some(compatibility_summary))
    }
}
