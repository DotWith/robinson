use std::{fs, path::Path, env};

use reqwest::Url;

use crate::Error;

#[derive(Default)]
pub struct Client {
    client: reqwest::Client,
}

impl Client {
    pub fn get_url(&self, path: &str) -> Result<Url, Error> {
        if path.starts_with("http") {
            let url = Url::parse(path).unwrap();
            Ok(url)
        } else {
            let current_dir = env::current_dir()?;
            let absolute_path = current_dir.join(Path::new(path));
            let url = Url::from_file_path(absolute_path).unwrap();
            Ok(url)
        }
    }

    pub async fn get_to_string(&self, url: Url) -> Result<String, Error> {
        match url.scheme() {
            "file" => {
                let path = url.to_file_path().unwrap();
                let text = fs::read_to_string(path)?;
                Ok(text)
            }
            _ => {
                let response = self.client
                    .get(url)
                    .send()
                    .await?;
                let text = response.text().await?;
                Ok(text)
            }
        }
    }
}
