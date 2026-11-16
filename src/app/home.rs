use std::path::PathBuf;

use eframe::egui::{self, Align, Color32, Layout, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use web_time::Instant;

use super::{new_script_button, relative_time, youtrack_button, App, Popover};
use crate::anim;
use crate::icons::{self, Icon};
use crate::people;
use crate::storage::{Entry, Found};
use crate::theme::{self, pal};
use crate::ui;

const CARD_W: f32 = 268.0;
const CARD_H: f32 = 132.0;
const TILE_H: f32 = 168.0;
const T_TILE_TITLE: f32 = 30.0;
const T_BIG: f32 = 46.0;
const HERO_T: f32 = 40.0;
const HERO_T_MIN: f32 = 24.0;
const HERO_BUTTONS: f32 = 300.0;
const SCRIPTS_PAD: f32 = 20.0;

impl App {
    pub(super) fn go_home(&mut self) {
        if self.dirty {
            self.save(false);
        }
        if let Some(path) = self.path.clone() {
            self.left_off = self.ed.focus_block.map(|id| {
                let page = crate::export::page_of_block(&self.doc, id).unwrap_or(1);
                let scene = self
                    .doc
                    .scene_of(id)
                    .and_then(|k| self.doc.scenes().get(k).map(|s| s.heading.clone()))
                    .unwrap_or_default();
                (path.clone(), page, scene)
            });
            self.store.close(&path);
        }
        self.home = true;
        self.home_born = Instant::now();
        self.popover = None;
        self.find.open = false;
        self.focus_mode = false;
        self.refresh_entries();
    }

    pub(super) fn home_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: theme::GAP,
                right: theme::GAP,
                top: theme::GAP * 0.5,
                bottom: theme::GAP,
            }))
            .show(ctx, |ui| {
                let full = ui.available_rect_before_wrap();
                let w = (full.width() - 2.0 * theme::GAP).clamp(300.0, 1100.0);
                let col = Rect::from_min_size(
                    Pos2::new(full.center().x - w * 0.5, full.top()),
                    Vec2::new(w, full.height()),
                );
                let mut inner = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(col)
                        .layout(Layout::top_down(Align::Min)),
                );
                let t = (self.home_born.elapsed().as_secs_f32() / 0.32).clamp(0.0, 1.0);
                if t < 1.0 && anim::enabled() {
                    inner.set_opacity(0.25 + 0.75 * anim::smootherstep(t));
                    anim::keep_going(ctx);
                }
                egui::ScrollArea::vertical()
                    .id_salt("ns-home")
                    .auto_shrink([false, false])
                    .show(&mut inner, |ui| {
                        ui.set_width(w - 14.0);
                        self.home_body(ui);
                    });
            });
    }

    fn home_body(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        let team = self.store.team().cloned();
        let w = ui.available_width();

        // greeting straight glass here---
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let hello = match team.as_ref().map(|t| t.me.first_name().to_string()).or_else(local_name) {
                    Some(name) => format!("{}, {name}", greeting()),
                    None => greeting().to_string(),
                };
                let about = match &team {
                    Some(team) => {
                        let people = team.members.iter().filter(|m| !m.pending).count().max(1);
                        format!(
                            "{} · {} · {}",
                            team.name,
                            plural(self.entries.len(), "script", "scripts"),
                            plural(people, "person", "people"),
                        )
                    }
                    None => format!("Your library · {}", plural(self.entries.len(), "script", "scripts")),
                };
                ui::hud(ui, &about.replace(" · ", " / "), None);
                ui.add_space(6.0);
                let buttons = HERO_BUTTONS + if team.is_some() { 38.0 } else { 0.0 };
                let room = (w - buttons).max(120.0);
                let at = |size: f32| theme::tracked_width(ui.ctx(), &hello, theme::font_display(size), ncf::theme::display_tracking(size));
                let full = at(HERO_T);
                let size = if full <= room { HERO_T } else { (HERO_T * room / full * 0.97).max(HERO_T_MIN) };
                let shown = ui::elide_to_width(ui.ctx(), &hello, theme::font_display(size), ncf::theme::display_tracking(size), room);
                ui.add(egui::Label::new(ui::display_text(&shown, size, p.text)).extend());
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.can_write() {
                    if new_script_button(ui, 150.0, &self.keys_for(crate::keys::Command::NewScript)) {
                        self.new_script();
                        self.home = false;
                    }
                    ui.add_space(4.0);
                    if ui::button(ui, "Import", Some(Icon::Upload), false) {
                        self.start_import();
                    }
                }
                if team.is_some() && ui::icon_button(ui, Icon::Trash, "Recently deleted", 30.0) {
                    self.toggle_popover(Popover::Trash);
                }
            });
        });
        ui.add_space(14.0);
        self.sources_bar(ui, w);

        // search here---
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.home_search)
                .id(egui::Id::new("ns-home-search"))
                .hint_text(if team.is_some() {
                    format!("Search the team's scripts  ·  {}+F", crate::keys::command_key())
                } else {
                    format!("Search titles and every line  ·  {}+F", crate::keys::command_key())
                })
                .desired_width(w.min(380.0))
                .margin(egui::Margin::symmetric(12.0, 8.0)),
        );
        if self.focus_home_search {
            search.request_focus();
            self.focus_home_search = false;
        }
        let enter = search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.add_space(16.0);

        let needle = self.home_search.trim().to_lowercase();
        let mut list: Vec<Entry> = self
            .entries
            .iter()
            .filter(|e| {
                needle.is_empty()
                    || e.title.to_lowercase().contains(&needle)
                    || e.author.to_lowercase().contains(&needle)
                    || e.preview.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect();
        list.sort_by_key(|a| std::cmp::Reverse(a.modified));

        if self.entries.is_empty() {
            self.home_empty(ui);
            self.home_foot(ui);
            return;
        }

        // you left off here---
        let me = team.as_ref().map(|t| t.me.clone());
        if needle.is_empty() {
            let mine = list
                .iter()
                .find(|e| match (&me, &e.edited_by) {
                    (Some(me), Some(by)) => by.id == me.id,
                    _ => false,
                })
                .or_else(|| list.first())
                .cloned();
            if let Some(e) = mine {
                self.bento_row(ui, &e, w);
                ui.add_space(theme::GAP);
            }
        }

        // every script cards here---
        let (starred, rest): (Vec<Entry>, Vec<Entry>) = list.into_iter().partition(|e| e.starred);
        let mut order: Vec<PathBuf> = Vec::new();
        let mut open = None;
        let mut star = None;
        let mut menu = None;
        let mut jump = None;
        let shown = starred.len() + rest.len();
        ncf::theme::surface_frame(ncf::theme::R_LG, ncf::theme::Lift::Rest)
            .inner_margin(egui::Margin::same(SCRIPTS_PAD))
            .show(ui, |ui| {
                let w = w - SCRIPTS_PAD * 2.0;
                ui.set_width(w);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    let heading = if !needle.is_empty() {
                        "Search"
                    } else if team.is_some() {
                        "Team scripts"
                    } else {
                        "Your scripts"
                    };
                    ui.label(egui::RichText::new(heading).font(ncf::theme::font_heading(theme::T_H + 1.0)).color(p.text));
                    if needle.is_empty() {
                        ui::hud(ui, &format!("{shown:02}"), Some(p.text_faint));
                    }
                });
                ui.add_space(12.0);
                let both = !starred.is_empty() && !rest.is_empty();
                for (label, icon, items) in [
                    ("Starred", Icon::StarFilled, &starred),
                    (if team.is_some() { "Team scripts" } else { "Scripts" }, Icon::File, &rest),
                ] {
                    if items.is_empty() {
                        continue;
                    }
                    if !order.is_empty() {
                        ui.add_space(18.0);
                    }
                    if both || label == "Starred" {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            icons::show(ui, icon, 12.0, if label == "Starred" { p.warn } else { p.text_faint });
                            ui::hud(ui, &format!("{label} / {:02}", items.len()), Some(p.text_dim));
                        });
                        ui.add_space(8.0);
                    }
                    let gap = theme::GAP;
                    let cols = (((w + gap) / (CARD_W + gap)).floor() as usize).max(1);
                    let card_w = (w - gap * (cols - 1) as f32) / cols as f32;
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("ns-home-cols"), cols));
                    let rows = items.len().div_ceil(cols);
                    for (k, row) in items.chunks(cols).enumerate() {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for e in row {
                                let selected = self.home_keyed && self.home_sel == order.len();
                                order.push(e.path.clone());
                                match self.script_card(ui, e, card_w, selected) {
                                    CardAction::Open => open = Some(e.path.clone()),
                                    CardAction::Star => star = Some(e.path.clone()),
                                    CardAction::Menu(at) => menu = Some((e.path.clone(), at)),
                                    CardAction::None => {}
                                }
                            }
                        });
                        if k + 1 < rows {
                            ui.add_space(gap);
                        }
                    }
                }
                let found = self.found_for(&needle);
                self.hit_rows.clear();
                if !found.is_empty() {
                    if !order.is_empty() {
                        ui.add_space(18.0);
                    }
                    jump = self.found_section(ui, &found, w);
                }
                if enter && open.is_none() && jump.is_none() && !needle.is_empty() {
                    match (order.first(), found.first().and_then(|f| f.hits.first().map(|h| (f.path.clone(), h.block)))) {
                        (Some(path), _) => open = Some(path.clone()),
                        (None, Some(hit)) => jump = Some(hit),
                        (None, None) => {}
                    }
                }
                if order.is_empty() && found.is_empty() {
                    ui.label(
                        egui::RichText::new("Nothing matches.")
                            .font(theme::font(theme::T_SM))
                            .color(p.text_faint),
                    );
                }
            });
        self.home_order = order;

        self.home_foot(ui);

        if let Some(path) = star {
            self.toggle_star(&path);
        }
        if let Some((path, at)) = menu {
            self.rail.menu = Some((path, at, self.frame_no));
        }
        if let Some(path) = open {
            self.open_from_home(path);
        } else if let Some((path, k)) = jump {
            self.pending_jump = Some((path.clone(), k));
            self.open_from_home(path);
        }
    }

    fn found_for(&mut self, needle: &str) -> Vec<Found> {
        if needle.chars().count() < 2 {
            return Vec::new();
        }
        if self.home_hits.as_ref().map(|(n, _)| n.as_str()) != Some(needle) {
            let found = self.store.search(needle).unwrap_or_default();
            self.home_hits = Some((needle.to_string(), found));
        }
        self.home_hits.as_ref().map(|(_, f)| f.clone()).unwrap_or_default()
    }

    fn found_section(&mut self, ui: &mut egui::Ui, found: &[Found], w: f32) -> Option<(PathBuf, usize)> {
        let p = pal();
        let mut picked = None;
        let all: usize = found.iter().map(|f| f.total).sum();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            icons::show(ui, Icon::Search, 12.0, p.text_faint);
            ui::hud(ui, &format!("In the pages / {all:02}"), Some(p.text_dim));
        });
        ui.add_space(8.0);
        let font = theme::font_page(theme::T_SM);
        for f in found {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                ui.label(egui::RichText::new(&f.title).font(theme::font_med(theme::T_SM)).color(p.text));
                let more = if f.total == 1 { "1 match".to_string() } else { format!("{} matches", f.total) };
                ui.label(egui::RichText::new(more).font(theme::font_mono(theme::T_CAP)).color(p.text_faint));
            });
            ui.add_space(4.0);
            for h in &f.hits {
                let chars: Vec<char> = h.line.chars().collect();
                let cut = |a: usize, b: usize| chars[a.min(chars.len())..b.min(chars.len())].iter().collect::<String>();
                let mut job = egui::text::LayoutJob::default();
                let plain = egui::TextFormat { font_id: font.clone(), color: p.text_dim, ..Default::default() };
                if h.note {
                    job.append("NOTE  ", 0.0, egui::TextFormat { font_id: theme::font_mono(theme::T_CAP), color: p.primary_light, ..Default::default() });
                }
                job.append(&cut(0, h.start), 0.0, plain.clone());
                job.append(
                    &cut(h.start, h.start + h.len),
                    0.0,
                    egui::TextFormat { font_id: font.clone(), color: p.text, background: theme::wash(p.primary, 90), ..Default::default() },
                );
                job.append(&cut(h.start + h.len, chars.len()), 0.0, plain);
                job.wrap.max_width = w - 28.0;
                job.wrap.max_rows = 1;
                let galley = ui.painter().layout_job(job);
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, galley.rect.height() + 14.0), Sense::click());
                let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
                theme::hover_surface(ui.painter(), rect, theme::R_SM, p.primary, hot, false);
                ui.painter().galley(Pos2::new(rect.left() + 14.0, rect.center().y - galley.rect.height() * 0.5), galley, p.text);
                self.hit_rows.push((f.path.clone(), h.block, rect));
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    picked = Some((f.path.clone(), h.block));
                }
            }
            if f.total > f.hits.len() {
                ui.label(
                    egui::RichText::new(format!("and {} more in this script", f.total - f.hits.len()))
                        .font(theme::font(theme::T_CAP))
                        .color(p.text_faint),
                );
            }
            ui.add_space(12.0);
        }
        picked
    }

    pub(super) fn open_from_home(&mut self, path: PathBuf) {
        self.mode = super::Mode::Write;
        self.open(path);
    }

    fn bento_row(&mut self, ui: &mut egui::Ui, e: &Entry, w: f32) {
        let gap = theme::GAP;
        let cols: usize = if w >= 820.0 { 4 } else if w >= 470.0 { 2 } else { 1 };
        let unit = (w - gap * (cols - 1) as f32) / cols as f32;
        let top = ui.cursor().min;
        let place = |col: usize, row: usize, span: usize| {
            Rect::from_min_size(
                Pos2::new(top.x + col as f32 * (unit + gap), top.y + row as f32 * (TILE_H + gap)),
                Vec2::new(unit * span as f32 + gap * (span - 1) as f32, TILE_H),
            )
        };
        let (go, session, library, rows) = match cols {
            4 => (place(0, 0, 2), place(2, 0, 1), place(3, 0, 1), 1),
            2 => (place(0, 0, 2), place(0, 1, 1), place(1, 1, 1), 2),
            _ => (place(0, 0, 1), place(0, 1, 1), place(0, 2, 1), 3),
        };
        ui.allocate_rect(
            Rect::from_min_size(top, Vec2::new(w, rows as f32 * TILE_H + (rows - 1) as f32 * gap)),
            Sense::hover(),
        );
        self.continue_tile(ui, go, e);
        self.session_tile(ui, session);
        self.library_tile(ui, library);
    }

    // continue writing tile here---
    fn continue_tile(&mut self, ui: &mut egui::Ui, rect: Rect, e: &Entry) {
        let p = pal();
        let resp = ui.interact(rect, egui::Id::new("ns-home-continue"), Sense::click());
        let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
        bento_tile(ui.painter(), rect, p.tile(0), hot);
        let ink = p.on_tile;
        let soft = theme::wash(ink, 190);
        let x = rect.left() + 24.0;
        let clip = ui.painter().with_clip_rect(rect.shrink(1.0));
        ui::eyebrow_text(&clip, Pos2::new(x, rect.top() + 28.0), "Continue writing", soft);

        let room = rect.width() - 48.0;
        let track = ncf::theme::display_tracking(T_TILE_TITLE);
        let font = theme::font_display(T_TILE_TITLE);
        let title = ui::elide_to_width(ui.ctx(), &e.title, font.clone(), track, room);
        theme::tracked_text(&clip, Pos2::new(x, rect.top() + 66.0), &title, font, ink, track);

        let place = match &self.left_off {
            Some((path, page, scene)) if *path == e.path => {
                if scene.is_empty() {
                    format!("You left off on page {page}")
                } else {
                    format!("Page {page} · {scene}")
                }
            }
            _ => format!("{} · {}", plural(e.pages, "page", "pages"), plural(e.scenes, "scene", "scenes")),
        };
        let mut meta = place;
        if let Some(by) = &e.edited_by {
            let me = self.me();
            meta.push_str(&format!(" · saved by {}", people::by_line(by, me.as_ref(), &relative_time(e.modified))));
        }
        let button_w = 96.0;
        let meta_room = (rect.width() - 48.0 - button_w - 16.0).max(40.0);
        let meta_font = theme::font_med(theme::T_SM);
        clip.text(
            Pos2::new(x, rect.bottom() - 34.0),
            egui::Align2::LEFT_CENTER,
            ui::elide_to_width(ui.ctx(), &meta, meta_font.clone(), 0.0, meta_room),
            meta_font,
            soft,
        );

        let pill = Rect::from_min_size(Pos2::new(rect.right() - 24.0 - button_w, rect.bottom() - 52.0), Vec2::new(button_w, 36.0));
        if self.is_opening(&e.path) {
            clip.text(pill.center(), egui::Align2::CENTER_CENTER, "Opening…", theme::font_bold(theme::T_SM), ink);
        } else if let Some(who) = &e.editing {
            let text = format!("{} is editing", who.first_name());
            clip.text(Pos2::new(pill.right(), pill.center().y), egui::Align2::RIGHT_CENTER, text, theme::font_bold(theme::T_CAP), ink);
        } else {
            clip.rect_filled(pill, Rounding::same(pill.height() * 0.5), ink);
            let label = clip.layout_no_wrap("Open".to_string(), theme::font_bold(theme::T_SM), p.tile(0));
            let total = label.rect.width() + 6.0 + 14.0;
            let lx = pill.center().x - total * 0.5;
            clip.galley(Pos2::new(lx, pill.center().y - label.rect.height() * 0.5), label, p.tile(0));
            let arrow = Rect::from_center_size(Pos2::new(lx + total - 7.0 + 3.0 * hot, pill.center().y), Vec2::splat(14.0));
            icons::draw(&clip, arrow, Icon::ArrowRight, p.tile(0));
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            self.open_from_home(e.path.clone());
        }
    }

    // session tile here---
    fn session_tile(&self, ui: &egui::Ui, rect: Rect) {
        let p = pal();
        bento_tile(ui.painter(), rect, p.tile(1), 0.0);
        let ink = p.on_tile;
        let soft = theme::wash(ink, 190);
        let clip = ui.painter().with_clip_rect(rect.shrink(1.0));
        let x = rect.left() + 22.0;
        ui::eyebrow_text(&clip, Pos2::new(x, rect.top() + 28.0), "This session", soft);
        let words = self.session_words.max(0);
        big_figure(ui, &clip, Pos2::new(x, rect.top() + 76.0), &grouped(words), if words == 1 { "word" } else { "words" }, ink, rect.right() - 22.0);
        let goal = self.settings.session_goal;
        let room = rect.width() - 44.0;
        let line = if goal > 0 {
            let t = (words as f32 / goal as f32).clamp(0.0, 1.0);
            let track = Rect::from_min_size(Pos2::new(x, rect.bottom() - 52.0), Vec2::new(room, 6.0));
            clip.rect_filled(track, Rounding::same(3.0), theme::wash(ink, 40));
            if t > 0.0 {
                let fill = Rect::from_min_size(track.min, Vec2::new((room * t).max(6.0), 6.0));
                clip.rect_filled(fill, Rounding::same(3.0), ink);
            }
            if words as u32 >= goal {
                format!("Goal met · {} words", grouped(goal as i64))
            } else {
                format!("{}% of your {}-word goal", (t * 100.0).floor() as u32, grouped(goal as i64))
            }
        } else {
            "Written since you opened Blackline".to_string()
        };
        let font = theme::font_med(theme::T_CAP);
        clip.text(
            Pos2::new(x, rect.bottom() - 28.0),
            egui::Align2::LEFT_CENTER,
            ui::elide_to_width(ui.ctx(), &line, font.clone(), 0.0, room),
            font,
            soft,
        );
    }

    // library tile here---
    fn library_tile(&self, ui: &egui::Ui, rect: Rect) {
        let p = pal();
        bento_tile(ui.painter(), rect, p.tile(2), 0.0);
        let ink = p.on_tile;
        let soft = theme::wash(ink, 190);
        let clip = ui.painter().with_clip_rect(rect.shrink(1.0));
        let x = rect.left() + 22.0;
        ui::eyebrow_text(&clip, Pos2::new(x, rect.top() + 28.0), "Your library", soft);
        let pages: usize = self.entries.iter().map(|e| e.pages).sum();
        let scenes: usize = self.entries.iter().map(|e| e.scenes).sum();
        let starred = self.entries.iter().filter(|e| e.starred).count();
        big_figure(ui, &clip, Pos2::new(x, rect.top() + 76.0), &grouped(pages as i64), if pages == 1 { "page" } else { "pages" }, ink, rect.right() - 22.0);
        let mut line = format!("{} · {}", plural(self.entries.len(), "script", "scripts"), plural(scenes, "scene", "scenes"));
        if starred > 0 {
            line.push_str(&format!(" · {starred} starred"));
        }
        let font = theme::font_med(theme::T_CAP);
        clip.text(
            Pos2::new(x, rect.bottom() - 28.0),
            egui::Align2::LEFT_CENTER,
            ui::elide_to_width(ui.ctx(), &line, font.clone(), 0.0, rect.width() - 44.0),
            font,
            soft,
        );
    }

    fn script_card(&self, ui: &mut egui::Ui, e: &Entry, w: f32, selected: bool) -> CardAction {
        let p = pal();
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, CARD_H), Sense::click());
        let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
        let round = Rounding::same(theme::R_MD);
        ui.painter().rect_filled(rect, round, theme::mix(p.raised, p.sunken, hot * 0.6));
        if hot > 0.01 {
            ui.painter().rect_stroke(rect.shrink(0.5), round, Stroke::new(1.0_f32, theme::wash(p.line_strong, (255.0 * hot) as u8)));
        }
        if selected {
            ui.painter().rect_stroke(rect.expand(3.0), Rounding::same(theme::R_MD + 3.0), Stroke::new(2.0_f32, p.focus));
        }

        let pad = 16.0;
        let x = rect.left() + pad;
        let cp = ui.painter().with_clip_rect(rect.shrink(1.0));
        let inner = w - pad * 2.0;
        let chip = Rect::from_min_size(Pos2::new(x, rect.top() + 16.0), Vec2::splat(38.0));
        cover_chip(&cp, chip, &e.title, crate::storage::cover_of(e));
        let tx = chip.right() + 12.0;
        let t_room = rect.right() - pad - 28.0 - tx;
        let title_font = theme::font_bold(theme::T_BODY + 0.5);
        cp.text(
            Pos2::new(tx, rect.top() + 26.0),
            egui::Align2::LEFT_CENTER,
            ui::elide_to_width(ui.ctx(), &e.title, title_font.clone(), 0.0, t_room),
            title_font,
            p.text,
        );
        let author = if e.author.trim().is_empty() {
            "No author yet".to_string()
        } else {
            format!("by {}", e.author.trim())
        };
        cp.text(
            Pos2::new(tx, rect.top() + 45.0),
            egui::Align2::LEFT_CENTER,
            ui::elide_to_width(ui.ctx(), &author, theme::font(theme::T_CAP), 0.0, rect.right() - pad - tx),
            theme::font(theme::T_CAP),
            p.text_dim,
        );
        theme::tracked_text(
            ui.painter(),
            Pos2::new(x, rect.top() + 76.0),
            &format!("{:02} pg / {:02} sc", e.pages, e.scenes).to_uppercase(),
            theme::font_hud(theme::T_MICRO + 0.5),
            p.text_faint,
            0.8,
        );
        if e.cloud.is_some() {
            let col = if e.waiting { p.warn } else { p.text_faint };
            let right = rect.right() - pad;
            if e.waiting {
                let label = "WAITING TO SYNC";
                let tw = theme::tracked_width(ui.ctx(), label, theme::font_hud(theme::T_MICRO), 0.8);
                theme::tracked_text(ui.painter(), Pos2::new(right - tw, rect.top() + 76.0), label, theme::font_hud(theme::T_MICRO), col, 0.8);
                icons::draw(ui.painter(), Rect::from_center_size(Pos2::new(right - tw - 10.0, rect.top() + 76.0), Vec2::splat(11.0)), Icon::Cloud, col);
            } else {
                icons::draw(ui.painter(), Rect::from_center_size(Pos2::new(right - 6.0, rect.top() + 76.0), Vec2::splat(12.0)), Icon::Cloud, col);
            }
        }

        let foot_y = rect.bottom() - 22.0;
        let me = self.me();
        if self.is_opening(&e.path) {
            ui.painter().text(
                Pos2::new(x, foot_y),
                egui::Align2::LEFT_CENTER,
                "Opening…",
                theme::font_med(theme::T_CAP),
                p.text_dim,
            );
        } else if let Some(who) = &e.editing {
            editing_badge(ui, Pos2::new(rect.right() - pad, foot_y), who);
            if let Some(by) = &e.edited_by {
                people::avatar(ui, Rect::from_center_size(Pos2::new(x + 9.0, foot_y), Vec2::splat(18.0)), by);
            }
        } else if let Some(by) = &e.edited_by {
            people::avatar(ui, Rect::from_center_size(Pos2::new(x + 9.0, foot_y), Vec2::splat(18.0)), by);
            cp.text(
                Pos2::new(x + 26.0, foot_y),
                egui::Align2::LEFT_CENTER,
                ui::elide_to_width(
                    ui.ctx(),
                    &people::by_line(by, me.as_ref(), &relative_time(e.modified)),
                    theme::font(theme::T_CAP),
                    0.0,
                    inner - 26.0,
                ),
                theme::font(theme::T_CAP),
                p.text_dim,
            );
        } else {
            ui.painter().text(
                Pos2::new(x, foot_y),
                egui::Align2::LEFT_CENTER,
                relative_time(e.modified),
                theme::font(theme::T_CAP),
                p.text_faint,
            );
        }

        let star_rect = Rect::from_center_size(Pos2::new(rect.right() - 22.0, rect.top() + 26.0), Vec2::splat(24.0));
        let star_resp = ui.interact(star_rect, resp.id.with("star"), Sense::click());
        let star_hot = anim::ease(ui.ctx(), star_resp.id, star_resp.hovered(), anim::HOVER);
        if e.starred || hot > 0.02 {
            icons::draw(
                ui.painter(),
                star_rect.shrink(6.0),
                if e.starred { Icon::StarFilled } else { Icon::Star },
                if e.starred {
                    theme::lighten(p.warn, 0.2 * star_hot)
                } else {
                    theme::mix(theme::wash(p.text_faint, (200.0 * hot) as u8), p.warn, star_hot)
                },
            );
        }
        let star_clicked = star_resp
            .on_hover_text(if e.starred { "Unstar — stars are yours alone" } else { "Star it, for you" })
            .clicked();

        let resp = if e.preview.is_empty() {
            resp
        } else {
            resp.on_hover_text(format!("{}\nright click for more", e.preview))
        };
        if star_clicked {
            CardAction::Star
        } else if resp.secondary_clicked() {
            ui.ctx()
                .pointer_latest_pos()
                .map(CardAction::Menu)
                .unwrap_or(CardAction::None)
        } else if resp.clicked() {
            CardAction::Open
        } else {
            CardAction::None
        }
    }

    fn home_empty(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        theme::island_frame(theme::R_ISLAND)
            .inner_margin(egui::Margin::symmetric(28.0, 26.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(ui::display_text("No scripts yet", 28.0, p.text));
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(if self.can_write() {
                        "Start one, or bring one in: Fountain, Final Draft and Blackline's own files all import."
                    } else {
                        "When someone on the team starts a script, it shows up here."
                    })
                    .font(theme::font(theme::T_SM))
                    .color(p.text_dim),
                );
                if self.can_write() {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if new_script_button(ui, 150.0, &self.keys_for(crate::keys::Command::NewScript)) {
                            self.new_script();
                            self.home = false;
                        }
                        ui.add_space(4.0);
                        if ui::button(ui, "Import", Some(Icon::Upload), false) {
                            self.start_import();
                        }
                    });
                }
            });
        ui.add_space(18.0);
    }

    fn home_foot(&mut self, ui: &mut egui::Ui) {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.set_width(260.0);
            if !self.settings.youtrack_url.trim().is_empty() && youtrack_button(ui, &self.settings.youtrack_url) {
                let url = self.settings.youtrack_url.trim().to_string();
                if url.starts_with("http://") || url.starts_with("https://") {
                    self.store.open_url(&url);
                } else {
                    self.deck.warn("That is not a web address", "Set the YouTrack link in Settings.");
                }
            }
            if ui::quiet_button(ui, "Take the tour", Some(Icon::Sparkle)) {
                self.start_tour(super::onboarding::Kind::Full);
            }
            ui.add_space(10.0);
            let (r, _) = ui.allocate_exact_size(Vec2::new(260.0, 14.0), Sense::hover());
            if ui::maker_line(ui, r, true) {
                self.store.open_url(crate::NATIVECOMPUTE_URL);
            }
        });
        ui.add_space(12.0);
    }

    pub(super) fn home_keys(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let typing = ctx.memory(|m| m.focused()).is_some();
        let n = self.home_order.len();
        let cols = ctx.data(|d| d.get_temp::<usize>(egui::Id::new("ns-home-cols"))).unwrap_or(3).max(1);
        if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::F))) {
            self.focus_home_search = true;
        }
        if typing || n == 0 {
            return;
        }
        let mut sel = self.home_sel.min(n - 1);
        let before = sel;
        let mut moved = false;
        ctx.input(|i| {
            moved = [Key::ArrowRight, Key::ArrowLeft, Key::ArrowDown, Key::ArrowUp]
                .iter()
                .any(|k| i.key_pressed(*k));
            if i.key_pressed(Key::ArrowRight) {
                sel = (sel + 1).min(n - 1);
            }
            if i.key_pressed(Key::ArrowLeft) {
                sel = sel.saturating_sub(1);
            }
            if i.key_pressed(Key::ArrowDown) {
                sel = (sel + cols).min(n - 1);
            }
            if i.key_pressed(Key::ArrowUp) {
                sel = sel.saturating_sub(cols);
            }
        });
        if moved && !self.home_keyed {
            self.home_keyed = true;
            sel = before;
        }
        self.home_sel = sel;
        if self.home_keyed && ctx.input(|i| i.key_pressed(Key::Enter)) {
            if let Some(path) = self.home_order.get(sel).cloned() {
                self.open_from_home(path);
            }
        }
    }
}

enum CardAction {
    None,
    Open,
    Star,
    Menu(Pos2),
}

fn editing_badge(ui: &egui::Ui, right: Pos2, who: &crate::backend::Person) {
    let p = pal();
    let text = format!("{} is editing", who.first_name());
    let galley = ui
        .painter()
        .layout_no_wrap(text, theme::font_med(theme::T_CAP), p.text);
    let left = right.x - galley.rect.width();
    ui.painter().circle_filled(Pos2::new(left - 9.0, right.y), 3.2, p.ok);
    ui.painter()
        .galley(Pos2::new(left, right.y - galley.rect.height() * 0.5), galley, p.text);
}

fn greeting() -> &'static str {
    use chrono::Timelike;
    match chrono::Local::now().hour() {
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        18..=22 => "Good evening",
        _ => "Up late",
    }
}

fn local_name() -> Option<String> {
    if crate::WEB {
        return None;
    }
    let raw = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).ok()?;
    let first = raw.split(['.', '_', '-', ' ']).next()?.trim();
    if first.is_empty() || first == "root" || !first.chars().all(|c| c.is_alphabetic()) {
        return None;
    }
    let mut c = first.chars();
    Some(c.next()?.to_uppercase().collect::<String>() + &c.as_str().to_lowercase())
}

fn bento_tile(painter: &egui::Painter, rect: Rect, colour: Color32, hot: f32) {
    let lift = if hot > 0.5 { ncf::theme::Lift::Float } else { ncf::theme::Lift::Rest };
    ncf::theme::shadow(painter, rect, ncf::theme::R_LG, lift);
    painter.rect_filled(rect, Rounding::same(ncf::theme::R_LG), theme::lighten(colour, 0.08 * hot));
}

fn big_figure(ui: &egui::Ui, painter: &egui::Painter, left_centre: Pos2, figure: &str, unit: &str, ink: Color32, right: f32) {
    let track = ncf::theme::display_tracking(T_BIG);
    let mut size = T_BIG;
    let unit_font = theme::font_bold(theme::T_SM);
    let unit_w = theme::tracked_width(ui.ctx(), unit, unit_font.clone(), 0.0);
    let room = (right - left_centre.x - unit_w - 8.0).max(20.0);
    let w = theme::tracked_width(ui.ctx(), figure, theme::font_display(size), track);
    if w > room {
        size = (size * room / w).max(20.0);
    }
    let w = theme::tracked_text(painter, left_centre, figure, theme::font_display(size), ink, ncf::theme::display_tracking(size));
    painter.text(
        Pos2::new(left_centre.x + w + 8.0, left_centre.y + size * 0.22),
        egui::Align2::LEFT_CENTER,
        unit,
        unit_font,
        theme::wash(ink, 190),
    );
}

pub(crate) fn cover_chip(painter: &egui::Painter, rect: Rect, title: &str, cover: usize) {
    let p = pal();
    painter.rect_filled(rect, Rounding::same(theme::R_SM), p.tile(cover));
    let initial: String = title.trim().chars().find(|c| c.is_alphanumeric()).map(|c| c.to_uppercase().collect()).unwrap_or_else(|| "·".to_string());
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, initial, theme::font_display(rect.height() * 0.5), p.on_tile);
}

// cover swatches here---
pub(crate) fn cover_swatches(ui: &mut egui::Ui, current: usize, enabled: bool) -> Option<usize> {
    let p = pal();
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for (i, name) in crate::storage::COVERS.iter().enumerate() {
            let sense = if enabled { Sense::click() } else { Sense::hover() };
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(26.0), sense);
            let hot = anim::ease(ui.ctx(), resp.id, resp.hovered() && enabled, anim::HOVER);
            let c = rect.center();
            let alpha = if enabled { 255 } else { 110 };
            ui.painter().circle_filled(c, 10.0 + 1.0 * hot, theme::wash(p.tile(i), alpha));
            if i == current {
                ui.painter().circle_stroke(c, 12.5, Stroke::new(2.0_f32, p.text));
            }
            let label = format!("{}{}", name[..1].to_uppercase(), &name[1..]);
            let resp = resp.on_hover_text(if enabled { label } else { format!("{label} · {}", crate::backend::COVER_WHEN_OPEN) });
            if enabled && resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() && i != current {
                picked = Some(i);
            }
        }
    });
    picked
}

pub(crate) fn grouped(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}
