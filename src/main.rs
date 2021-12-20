use std::cmp::min;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::process::{Command, exit};

use anyhow::Result;
use eframe::{egui, epi};
use egui::{Color32, Event, Key, Label, Widget};
use fuzzy_matcher::FuzzyMatcher;
use itertools::Itertools;

#[derive(Clone, Eq, PartialEq)]
struct Item {
    label: String,
    value: String,

    // the field that should be used for searching.
    haystack: String,
}

trait Matcher {
    /// Applies the query against the list of items and returns a list of matching items.
    /// The resulting list should be ordered by match score
    /// with the best match in the first place.
    fn matches<'a>(&self, query: &str, items: &'a [Item]) -> Vec<&'a Item>;
}

impl<T> Matcher for T where T: FuzzyMatcher {
    fn matches<'a>(&self, query: &str, items: &'a [Item]) -> Vec<&'a Item> {
        items.iter()
            .flat_map(|item| {
                self
                    .fuzzy_match(&item.haystack, query)
                    .map(|score| (score, item))
            })

            .sorted_by_key(|(score, _item)| -score)
            .map(|(_score, item)| item)
            .collect()
    }
}


impl Item {
    pub fn parse(value: impl Into<String>) -> Result<Item> {
        let value = value.into();

        let label_base = value.trim_end_matches('/');

        let label = match label_base.rfind('/') {
            Some(idx) => label_base[idx + 1..].to_string(),
            None => value.clone(),
        };

        let haystack = value.to_string()
            .replace('_', " ")
            .replace('/', " ")
            .to_lowercase();

        Ok(Item { value, label, haystack })
    }
}

struct ShuttleApp {
    query: String,

    all: Vec<Item>,
    filtered: Vec<Item>,
    selected: usize,

    matcher: Box<dyn Matcher>,
}

impl ShuttleApp {
    pub fn new(matcher: Box<dyn Matcher>, items: Vec<Item>) -> Self {
        Self {
            query: String::new(),
            all: items.clone(),
            filtered: items,
            selected: 0,
            matcher,
        }
    }

    fn update_filtered_reset(&mut self) {
        self.filtered = self.all.clone();

        if !self.query.is_empty() {
            self.update_filtered();
        }
    }

    fn update_filtered(&mut self) {
        let query = self.query.to_lowercase();

        let selected_value = self.filtered.get(self.selected);

        let filtered_new = self.matcher
            .matches(query.as_str(), &self.filtered)
            .into_iter()
            .cloned()
            .collect_vec();

        self.selected = selected_value
            .and_then(|val| self.filtered.iter().position(|item| item == val))
            .unwrap_or_default();

        self.filtered = filtered_new;
    }

    pub fn launch(&self, url: &str) {
        Command::new("xdg-open")
            .arg(url)
            .exec();
    }
}

impl epi::App for ShuttleApp {
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        for event in &ctx.input().events {
            match event {
                Event::Text(t) => {
                    self.query += t;
                    self.update_filtered();
                }

                Event::Key { key: Key::Backspace, pressed: true, .. } => {
                    if let Some((pos, _)) = self.query.char_indices().last() {
                        self.query.remove(pos);
                        self.update_filtered_reset();
                    }
                }

                Event::Key { key: Key::W, pressed: true, modifiers } if modifiers.ctrl => {
                    if let Some(pos) = self.query.trim_end().rfind(' ') {
                        self.query.truncate(pos + 1);
                        self.update_filtered_reset();
                    } else {
                        self.query.truncate(0);
                        self.update_filtered_reset();
                    }
                }

                Event::Key { key: Key::Enter, pressed: true, .. } => {
                    if let Some(selected) = self.filtered.get(self.selected) {
                        println!("launching {:?}", selected.value);
                        self.launch(&selected.value);
                    }
                }

                Event::Key { key: Key::ArrowUp, pressed: true, .. } => {
                    self.selected = self.selected.saturating_sub(1);
                }

                Event::Key { key: Key::ArrowDown, pressed: true, .. } => {
                    let max_value = self.filtered.len() - 1;
                    self.selected = min(max_value, self.selected + 1);
                }

                Event::Key { key: Key::Escape, pressed: true, .. } => {
                    exit(1);
                }

                _ => ()
            };
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_height(480.0);
            ui.set_width(640.0);

            // make it all monospaced
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

            // and let no text wrap
            ui.style_mut().wrap = Some(false);

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.set_height(32.0);
                    let query_str = String::from("> ") + &self.query;
                    Label::new(query_str)
                        .text_color(Color32::GOLD)
                        .ui(ui);
                });

                ui.separator();

                let items_iter = self.filtered.iter()
                    .enumerate()
                    .skip(self.selected.saturating_sub(8));

                for (idx, item) in items_iter {
                    let selected = self.selected == idx;
                    let color: Color32 = if selected { Color32::WHITE } else { Color32::GRAY };

                    let response = ui.horizontal(|ui| {
                        ui.set_height(24.0);

                        let label = Label::new(&item.label)
                            .text_color(color)
                            .ui(ui);

                        label.rect
                    });

                    if !ui.clip_rect().intersects(response.inner) {
                        break;
                    }
                }
            })
        });
    }

    fn name(&self) -> &str {
        "Shuttle"
    }
}

fn main() -> anyhow::Result<()> {
    let mut items = BufReader::new(File::open("/tmp/urls")?).lines()
        .flatten()
        .filter(|line| !line.trim().is_empty())
        .map(Item::parse)
        .collect::<Result<Vec<_>, _>>()?;

    items.sort_by(|lhs, rhs| lhs.haystack.cmp(&rhs.haystack));

    let native_options = eframe::NativeOptions {
        always_on_top: true,
        resizable: false,
        transparent: true,
        initial_window_size: Some(egui::Vec2::new(640.0, 480.0)),
        ..Default::default()
    };

    let matcher = Box::new(fuzzy_matcher::skim::SkimMatcherV2::default().ignore_case());
    let app = ShuttleApp::new(matcher, items);
    eframe::run_native(Box::new(app), native_options);
}
