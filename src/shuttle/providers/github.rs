use std::error::Error;

use serde::Deserialize;

use crate::shuttle::{Item, Provider};

pub struct Github {
    endpoint: String,
    orga: String,
}

impl Github {
    pub fn new(orga: impl Into<String>) -> Self {
        Self {
            endpoint: String::from("https://api.github.com"),
            orga: orga.into(),
        }
    }

    pub fn new_with_endpoint(orga: impl Into<String>, endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            orga: orga.into(),
        }
    }
}

impl Provider for Github {
    fn load(&self) -> Result<Vec<Item>, Box<dyn Error + Send + Sync>> {
        // TODO fetch more than the first page of URLs
        let url = format!(
            "{}/orgs/{}/repos?sort=updated&per_page=100",
            self.endpoint.trim_end_matches('/'),
            self.orga,
        );

        let repositories: Vec<Repository> = ureq::get(&url).call()?.into_json()?;
        Ok(repositories.into_iter().map(Into::into).collect())
    }
}

#[derive(Deserialize)]
struct Repository {
    full_name: String,
    html_url: String,
}

impl From<Repository> for Item {
    fn from(repo: Repository) -> Self {
        Item {
            label: repo.full_name.clone(),
            haystack: repo.full_name,
            value: repo.html_url,
        }
    }
}
