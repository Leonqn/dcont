use anyhow::{anyhow, Result};
use reqwest::{multipart::Form, Url};

pub struct Qb {
    client: reqwest::Client,
    url: Url,
}

impl Qb {
    pub fn new(url: Url) -> Self {
        Self {
            client: reqwest::Client::default(),
            url,
        }
    }

    pub async fn upload_torrent(&self, url: String, category: String) -> Result<()> {
        let response = self
            .client
            .post(format!("{}/torrents/add", self.url))
            .multipart(Form::new().text("urls", url).text("category", category))
            .send()
            .await?;
        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "unexpected status {}, {}",
                status,
                response.text().await?
            ))
        }
    }
}
