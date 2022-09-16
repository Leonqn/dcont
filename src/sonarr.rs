use std::convert::TryInto;

use anyhow::{anyhow, Result};
use derive_more::Display;
use reqwest::{header::HeaderMap, Url};
use serde::Deserialize;

pub struct Sonarr {
    client: reqwest::Client,
    url: Url,
}

impl Sonarr {
    pub fn new(url: Url, api_key: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .default_headers({
                    let mut header = HeaderMap::default();
                    header.insert("X-Api-Key", api_key.try_into().unwrap());
                    header
                })
                .build()
                .unwrap(),
            url,
        }
    }

    pub async fn series(&self) -> Result<Vec<Series>> {
        Ok(self
            .client
            .get(self.url("series"))
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn episode(&self, series_id: SeriesId) -> Result<Vec<Episode>> {
        Ok(self
            .client
            .get(self.url("episode"))
            .query(&[("seriesId", series_id.0)])
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn release(&self, episode_id: EpisodeId) -> Result<Vec<Release>> {
        Ok(self
            .client
            .get(self.url("release"))
            .query(&[("episodeId", episode_id.0)])
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn history(&self, episode_id: EpisodeId) -> Result<History> {
        Ok(self
            .client
            .get(self.url("history"))
            .query(&[("sortDir", "desc"), ("sortKey", "data")])
            .query(&[("episodeId", episode_id.0), ("pageSize", 1000), ("page", 1)])
            .send()
            .await?
            .json()
            .await?)
    }

    fn url(&self, s: &str) -> String {
        format!("{}/{}", self.url, s)
    }
}

#[derive(Deserialize, PartialEq, Eq, Debug, Clone, Copy, Display, Hash)]
pub struct SeriesId(isize);

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub id: SeriesId,
    pub title: String,
    pub seasons: Vec<Season>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    pub season_number: usize,
    pub monitored: bool,
    pub statistics: Statistics,
}

impl Season {
    pub fn needs_update(&self) -> bool {
        self.monitored && self.statistics.percent_of_episodes < 100.0
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Statistics {
    pub percent_of_episodes: f64,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone, Copy, Display)]
pub struct EpisodeId(isize);

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub id: EpisodeId,
    pub series_id: SeriesId,
    pub season_number: usize,
    pub episode_number: usize,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub guid: String,
    pub download_url: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub records: Vec<HistoryEvent>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEvent {
    pub event_type: String,
    pub data: HistoryData,
}

impl HistoryEvent {
    pub fn grabbed(&self) -> Result<Option<Grabbed>> {
        match self.event_type.as_str() {
            "grabbed" => Ok(Some(Grabbed {
                guid: self
                    .data
                    .guid
                    .clone()
                    .ok_or_else(|| anyhow!("missing guid"))?,
            })),
            _ => Ok(None),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryData {
    pub guid: Option<String>,
    pub published_date: Option<String>,
}

#[derive(Debug)]
pub struct Grabbed {
    pub guid: String,
}
