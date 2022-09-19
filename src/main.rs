use anyhow::{anyhow, Result};
use qb::Qb;
use serde::Deserialize;
use sonarr::{Release, Season, Series};
use std::time::{Duration, SystemTime};
use tokio::time::interval;
use tracing::{info, instrument};

use crate::sonarr::Sonarr;

mod qb;
mod sonarr;

#[derive(Deserialize)]
struct Config {
    sonarr_api_url: String,
    sonar_api_key: String,
    qb_api_url: String,
    #[serde(with = "humantime_serde")]
    check_interval: Duration,
    #[serde(with = "humantime_serde")]
    new_release_duration: Duration,
}

impl Config {
    pub fn init() -> Result<Self> {
        let config_name = std::env::args()
            .nth(1)
            .expect("Config file should be specified as first argument");
        Ok(config::Config::builder()
            .add_source(config::File::with_name(&config_name))
            .build()?
            .try_deserialize()?)
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let config = Config::init().unwrap();

    let sonarr = Sonarr::new(
        config.sonarr_api_url.parse().unwrap(),
        &config.sonar_api_key,
    );
    let qb = Qb::new(config.qb_api_url.parse().unwrap());

    let mut interval = interval(config.check_interval);

    loop {
        interval.tick().await;
        let _ = download_releases(&sonarr, &qb, config.new_release_duration).await;
    }
}

#[instrument(skip_all, err)]
async fn download_releases(sonarr: &Sonarr, qb: &Qb, new_release_duration: Duration) -> Result<()> {
    let series = sonarr.series().await?;
    for series in series.into_iter() {
        for season in series.seasons.iter().filter(|s| s.needs_update()) {
            if let Ok(Some(release)) = find_release(sonarr, &series, season)
                .await
                .map(|r| r.filter(|r| r.publish_date + new_release_duration > SystemTime::now()))
            {
                info!(?release, "downloading release");
                qb.upload_torrent(release.download_url, "tv-sonarr".into())
                    .await?;
            }
        }
    }
    Ok(())
}

#[instrument(skip_all, fields(series_id = %series.id, series_title = %series.title, season = %season.season_number), err, ret)]
async fn find_release(
    sonarr: &Sonarr,
    series: &Series,
    season: &Season,
) -> Result<Option<Release>> {
    let episodes = sonarr.episode(series.id).await?;
    let first_episode = episodes
        .iter()
        .filter(|e| e.season_number == season.season_number)
        .min_by_key(|e| e.episode_number)
        .ok_or_else(|| anyhow!("episodes missing"))?;
    let history = sonarr.history(first_episode.id).await?;
    let grabbed = history
        .records
        .iter()
        .find_map(|r| r.grabbed().transpose())
        .ok_or_else(|| anyhow!("history is empty"))??;
    Ok(sonarr
        .release(first_episode.id)
        .await?
        .into_iter()
        .find(|r| grabbed.guid == r.guid))
}
