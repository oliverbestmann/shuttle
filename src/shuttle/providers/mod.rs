use std::error::Error;

pub use github::Github;
pub use jenkins::Jenkins;

use crate::Item;

mod github;
mod jenkins;

pub trait Provider: Send + Sync {
    fn title(&self) -> String {
        "Unknown".into()
    }

    /// Loads all items that this provider can provide.
    fn load(&self) -> Result<Vec<Item>, Box<dyn Error + Send + Sync>>;
}
