use std::error::Error;


use serde::Deserialize;

use crate::shuttle::{Item, Provider};

pub struct Jenkins {
    endpoint: String,
}

impl Jenkins {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self { endpoint: endpoint.into() }
    }
}

impl Provider for Jenkins {
    fn load(&self) -> Result<Vec<Item>, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/api/json", self.endpoint);
        let response: Response = ureq::get(&url).call()?.into_json()?;
        Ok(response.jobs.into_iter().map(Into::into).collect())
    }
}


#[derive(Deserialize)]
struct Response {
    jobs: Vec<Job>,
}

#[derive(Deserialize)]
struct Job {
    name: String,
    url: String,
}

impl From<Job> for Item {
    fn from(job: Job) -> Self {
        // search on a lowercase version of the job name
        let haystack = job.name.to_lowercase();

        Item {
            value: job.url,
            label: job.name,
            haystack,
        }
    }
}
