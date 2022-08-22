use itertools::Itertools;
use crate::{Item, Matcher};

pub struct FuzzyMatcher<T>(T);

impl<T> Matcher for FuzzyMatcher<T>
    where T: fuzzy_matcher::FuzzyMatcher,
{
    fn matches<'a>(&self, query: &str, items: &'a [Item]) -> Vec<&'a Item> {
        items.iter()
            .flat_map(|item| {
                self.0
                    .fuzzy_match(&item.haystack, query)
                    .map(|score| (score, item))
            })

            .sorted_by_key(|(score, _item)| -score)
            .map(|(_score, item)| item)
            .collect()
    }
}
