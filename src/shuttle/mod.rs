

use serde::{Deserialize, Serialize};

pub use providers::*;
pub use matchers::*;

mod providers;
mod matchers;

#[derive(Clone, Serialize, Deserialize)]
pub struct Item {
    /// The label will be used to display the item in the UI.
    pub label: String,

    /// The URL that will be opened on selection.
    pub value: String,

    /// The haystack field will be used for actual querying.
    pub haystack: String,
}

pub trait Matcher {
    /// Applies the query against the list of items and returns a list of matching items.
    /// The resulting list should be ordered by match score
    /// with the best match in the first place.
    fn matches<'a>(&self, query: &str, items: &'a [Item]) -> Vec<&'a Item>;
}
