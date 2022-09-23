use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use anyhow::Result;
use eframe::{App, AppCreator, CreationContext, egui, Storage};
use eframe::egui::{Color32, Event, Key, Label, Widget};
use itertools::Itertools;
use serde::Deserialize;
use serde::Serialize;

use crate::egui::{Context, Frame, RichText, Spinner, Visuals};
use crate::shuttle::{Github, Item, Jenkins, Matcher, Provider};

mod shuttle;

enum ShuttleState {
    Loading,
    Loaded(LoadedState),
}

struct LoadedState {
    all: Vec<Item>,
    filtered: Option<Vec<Item>>,
    selected: usize,
}

impl LoadedState {
    fn update_filtered_reset(&mut self, matcher: &dyn Matcher, query: &str) {
        self.filtered = None;

        if query.is_empty() {
            self.filtered = Some(self.all.clone())
        } else {
            self.update_filtered(matcher, query);
        }
    }

    fn update_filtered(&mut self, matcher: &dyn Matcher, query: &str) {
        let query = query.to_lowercase();

        let selected_value = self.filtered
            .as_ref()
            .and_then(|values| values.get(self.selected));

        let values_to_filter = self.filtered.as_ref().unwrap_or(&self.all);

        let filtered_new = matcher
            .matches(query.as_str(), values_to_filter)
            .into_iter()
            .cloned()
            .collect_vec();

        self.selected = selected_value
            .and_then(|val| self.filtered.iter().flatten().position(|item| item.value == val.value))
            .unwrap_or_default();

        self.filtered = Some(filtered_new);
    }
}

struct ShuttleApp {
    query: String,
    state: Arc<Mutex<ShuttleState>>,
    matcher: Box<dyn Matcher>,
    providers: Vec<Arc<dyn Provider>>,
}

impl ShuttleApp {
    pub fn new(providers: Vec<Arc<dyn Provider>>, matcher: Box<dyn Matcher>) -> Self {
        Self {
            query: String::new(),
            state: Arc::new(ShuttleState::Loading.into()),
            providers,
            matcher,
        }
    }

    pub fn launch(&self, url: &str) {
        Command::new("xdg-open")
            .arg(url)
            .exec();
    }

    fn handle_events(&mut self, ctx: &&Context, frame: &mut eframe::Frame) {
        let state = &mut *self.state.lock().unwrap();

        let mut require_update = false;
        let mut require_reset = false;

        let mut move_steps: i32 = 0;
        let mut launch = false;


        for event in &ctx.input().events {
            match event {
                Event::Text(t) => {
                    self.query += t;
                    require_update = true;
                }

                Event::Key { key: Key::Backspace, pressed: true, .. } => {
                    if let Some((pos, _)) = self.query.char_indices().last() {
                        self.query.remove(pos);
                        require_reset = true;
                    }
                }

                Event::Key { key: Key::W, pressed: true, modifiers } if modifiers.ctrl => {
                    if let Some(pos) = self.query.trim_end().rfind(' ') {
                        self.query.truncate(pos + 1);
                        require_reset = true;
                    } else {
                        self.query.truncate(0);
                        require_reset = true;
                    }
                }

                Event::Key { key: Key::Escape, pressed: true, .. } => {
                    frame.quit();
                }

                Event::Key { key: Key::Enter, pressed: true, .. } => {
                    launch = true;
                }

                Event::Key { key: Key::ArrowUp, pressed: true, .. } => {
                    move_steps -= 1;
                }

                Event::Key { key: Key::ArrowDown, pressed: true, .. } => {
                    move_steps += 1;
                }

                _ => ()
            }
        }

        match state {
            ShuttleState::Loading => {}

            ShuttleState::Loaded(state) => {
                if state.filtered.is_none() {
                    require_reset = true;
                }

                if require_reset {
                    state.update_filtered_reset(self.matcher.as_ref(), &self.query);
                } else if require_update {
                    state.update_filtered(self.matcher.as_ref(), &self.query);
                }

                if let Some(filtered) = state.filtered.as_ref() {
                    if !filtered.is_empty() {
                        state.selected = (state.selected as i32 + move_steps).rem_euclid(filtered.len() as _) as _;
                    }

                    if launch {
                        if let Some(selected) = filtered.get(state.selected) {
                            //println!("launching {:?}", selected.value);
                            self.launch(&selected.value);
                            return frame.quit();
                        }
                    }
                }
            }
        }
    }

    fn paint(&mut self, ctx: &Context) {
        let state = &mut *self.state.lock().unwrap();

        egui::CentralPanel::default().show(ctx, |ui| {
            // make it all monospaced
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

            // and let no text wrap
            ui.style_mut().wrap = Some(false);

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.set_height(32.0);
                    let query_str = String::from("> ") + &self.query;
                    Label::new(RichText::new(query_str).color(Color32::GOLD)).ui(ui);
                });

                ui.separator();

                let rows = (ui.available_height() / 28.0).floor() as usize;

                if let ShuttleState::Loaded(state) = state {
                    let items_count = state.filtered.iter()
                        .flatten()
                        .count();

                    let items_iter = state.filtered.iter()
                        .flatten()
                        .enumerate()
                        .skip(state.selected.saturating_sub(rows/2).min(items_count.saturating_sub(rows)));

                    for (idx, item) in items_iter {
                        let selected = state.selected == idx;
                        let color: Color32 = if selected { Color32::WHITE } else { Color32::GRAY };

                        ui.horizontal(|ui| {
                            ui.set_height(24.0);

                            let label = Label::new(RichText::new(&item.label).color(color)).ui(ui);

                            label.rect
                        });

                        // if !ui.clip_rect().intersects(response.inner) {
                        //     break;
                        // }
                    }
                }

                if let ShuttleState::Loading = state {
                    ui.centered_and_justified(|ui| {
                        ui.add(Spinner::new().size(32.0));
                    });
                }
            })
        });
    }
}

impl App for ShuttleApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        self.handle_events(&ctx, frame);
        self.paint(ctx);
    }
}

fn create_app(cc: &CreationContext<'_>, app: ShuttleApp) -> Box<dyn App> {
    cc.egui_ctx.set_visuals(Visuals::dark());

    // cc.frame.set_window_size(egui::Vec2::new(800.0, 600.0));

    let ctx = cc.egui_ctx.clone();
    let state_arc = Arc::clone(&app.state);

    let providers = app.providers.clone();

    spawn(move || {
        let items = load_items(&providers).unwrap();

        let mut state = state_arc.lock().unwrap();

        *state = ShuttleState::Loaded(
            LoadedState {
                all: items,
                filtered: None,
                selected: 0,
            }
        );

        drop(state);

        ctx.request_repaint();
    });

    Box::new(app)
}

fn load_items_from_providers(providers: &[Arc<dyn Provider>]) -> Result<Vec<Item>> {
    use rayon::prelude::*;

    let items: Vec<_> = providers.par_iter()
        .map(|prov| prov.load())
        .collect();

    let items = items.into_iter()
        .flatten_ok()
        .try_collect()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    Ok(items)
}

fn load_items_from_cache(r: impl Read) -> Result<Vec<Item>> {
    let cache: ItemCache = serde_json::from_reader(BufReader::new(r))?;
    Ok(cache.items)
}

fn load_items(providers: &[Arc<dyn Provider>]) -> Result<Vec<Item>> {
    match File::open("/tmp/shuttle.cache") {
        Ok(fp) => load_items_from_cache(fp),
        Err(_) => {
            let mut items = load_items_from_providers(providers)?;

            // by default we sort all items by display label
            items.sort_by(|lhs, rhs| lhs.label.cmp(&rhs.label));

            // serialize all items into the item cache
            let cache = ItemCache { items: items.clone() };
            let writer = BufWriter::new(File::create("/tmp/shuttle.cache")?);
            serde_json::to_writer(writer, &cache)?;

            Ok(items)
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        always_on_top: true,
        resizable: false,
        transparent: false,
        decorated: false,
        ..Default::default()
    };

    let gh = "https://srv-git-01-hh1.alinghi.tipp24.net/api/v3";

    let providers: Vec<Arc<dyn Provider>> = vec![
        Arc::new(Github::new_with_endpoint("b2b", gh)),
        Arc::new(Github::new_with_endpoint("eSailors", gh)),
        Arc::new(Github::new_with_endpoint("iwg", gh)),
        Arc::new(Github::new_with_endpoint("tipp24", gh)),
        Arc::new(Github::new_with_endpoint("website", gh)),
        Arc::new(Github::new_with_endpoint("zig", gh)),
        Arc::new(Jenkins::new("http://jenkins.iwg.ham.sg-cloud.co.uk")),
        Arc::new(Jenkins::new("http://platform-live.code.ham.sg-cloud.co.uk")),
        Arc::new(Jenkins::new("https://platform-jenkins.test.h.zeal.zone")),
        Arc::new(Jenkins::new("http://zig-jenkins.iwg.ham.sg-cloud.co.uk")),
    ];

    // let matcher = Box::new(fuzzy_matcher::skim::SkimMatcherV2::default().ignore_case());
    let matcher = Box::new(shuttle::SimpleMatcher);
    let app = ShuttleApp::new(providers, matcher);
    let app_name = "shuttle";
    let app_creator: AppCreator = Box::new(|ctx| create_app(ctx, app));
    eframe::run_native(app_name, native_options, app_creator);
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct ItemCache {
    items: Vec<Item>,
}
