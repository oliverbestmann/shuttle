use itertools::Itertools;
use crate::{Item, Matcher};

pub struct SimpleMatcher;

impl Matcher for SimpleMatcher {
    fn matches<'a>(&self, query: &str, items: &'a [Item]) -> Vec<&'a Item> {
        let query = query.to_lowercase();
        let query_parts = query.split_whitespace().collect_vec();

        items.iter()
            .filter(|item| query_parts
                .iter()
                .all(|part| item.haystack.contains(part)))
            .collect()
    }
}
