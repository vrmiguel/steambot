use std::fmt::Display;

use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DlcForApp {
    #[serde(rename = "dlc")]
    pub dlcs: Vec<Dlc>,
}

#[derive(Debug, Deserialize)]
pub struct Dlc {
    pub name: String,
    pub price_overview: PriceOverview,
    pub platforms: Platforms,
}

#[derive(Debug, Deserialize)]
pub struct Platforms {
    pub windows: bool,
    pub mac: bool,
    pub linux: bool,
}

#[derive(Debug, Deserialize)]
pub struct PriceOverview {
    #[serde(rename = "final")]
    pub final_price: i64,
    pub discount_percent: i64,
}

pub async fn get_dlcs(app_id: usize) -> anyhow::Result<DlcForApp> {
    let url =
        format!("https://store.steampowered.com/api/dlcforapp/?appid={app_id}&cc=BR&l=brazilian");

    reqwest::get(url)
        .await?
        .json()
        .await
        .with_context(|| "Failed to deserialize response of /dlcforapp")
}

impl Display for DlcForApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.dlcs.is_empty() {
            return Ok(());
        }

        writeln!(f, "DLCs\n")?;
        for dlc in &self.dlcs {
            writeln!(f, "{dlc}")?
        }

        Ok(())
    }
}

impl Display for Dlc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name.trim_end(), self.price_overview)?;
        Ok(())
    }
}

impl Display for PriceOverview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "R$ {:.2}", self.final_price as f64 / 100.0)?;
        if self.discount_percent > 0 {
            write!(f, " ({}% off)", self.discount_percent)?;
        }

        Ok(())
    }
}

impl Display for Platforms {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use arrayvec::ArrayVec;
        use slicedisplay::SliceDisplay;

        let mut platforms = ArrayVec::<_, 3>::new();

        if self.windows {
            platforms.push("Windows");
        }
        if self.mac {
            platforms.push("macOS");
        }
        if self.linux {
            platforms.push("Linux");
        }

        write!(f, "{}", (&*platforms).display().terminator(' ', ' '))
    }
}
