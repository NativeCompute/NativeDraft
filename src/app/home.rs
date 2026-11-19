use std::path::PathBuf;

use eframe::egui::{self, Align, Color32, Layout, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use web_time::Instant;

use super::{new_script_button, relative_time, youtrack_button, App};
use crate::anim;
use crate::icons::{self, Icon};
use crate::people;
use crate::storage::{Entry, Found};
use crate::theme::{self, pal};
use crate::ui;

const ROW1_H: f32 = 232.0;
const ROW2_H: f32 = 196.0;
const HERO_NARROW_H: f32 = 340.0;
const ROW_H: f32 = 66.0;
const LIST_MAX: usize = 6;
const SIDE_MIN_H: f32 = 280.0;
const T_TILE_TITLE: f32 = 38.0;
const T_BIG: f32 = 56.0;
const HERO_T: f32 = 56.0;
const HERO_T_MIN: f32 = 28.0;
const SCRIPTS_PAD: f32 = 20.0;
const ROW1_MIN: f32 = 184.0;
const ROW2_MIN: f32 = 160.0;
const LIST_MIN: f32 = 236.0;
const NOTE_STEP: f32 = 50.0;
const PERSON_STEP: f32 = 46.0;

#[derive(Clone, Copy)]
struct Fit {
    row1: f32,
    row2: f32,
    bottom: f32,
}

fn plan_fit(w: f32, h: f32, foot: f32) -> Option<Fit> {
    let gap = theme::GAP;
    if w < 860.0 {
        return None;
    }
    let room = h - foot - gap;
    let mut row1 = ROW1_H;
    let mut row2 = ROW2_H;
    let short = LIST_MIN - (room - row1 - gap - row2);
    if short > 0.0 {
        let give = (ROW1_H - ROW1_MIN) + (ROW2_H - ROW2_MIN);
        if short > give {
            return None;
        }
        let k = short / give;
        row1 -= (ROW1_H - ROW1_MIN) * k;
        row2 -= (ROW2_H - ROW2_MIN) * k;
    }
    Some(Fit { row1, row2, bottom: room - row1 - gap - row2 })
}

impl App {
    pub(super) fn go_home(&mut self) {
        if self.dirty {
            self.save(false);
        }
        if let Some(path) = self.path.clone() {
            if let Some(id) = self.ed.focus_block {
                let page = crate::export::page_of_block(&self.doc, id).unwrap_or(1);
                let k = self.doc.scene_of(id);
                let heading = k.and_then(|k| self.doc.scenes().get(k).map(|s| s.heading.clone())).unwrap_or_default();
                let block = self.doc.index_of(id).unwrap_or(0);
                crate::stats::remember(
                    &mut self.settings.places,
                    crate::stats::Place { path: path.display().to_string(), page, scene: k.map(|k| k + 1).unwrap_or(0), block, heading },
                );
                self.save_settings();
            }
            self.store.close(&path);
        }
        self.home = true;
        self.home_born = Instant::now();
        self.popover = None;
        self.find.open = false;
        self.focus_mode = false;
        self.refresh_entries();
    }

    pub(super) fn place_of(&self, path: &std::path::Path) -> Option<crate::stats::Place> {
        let key = path.display().to_string();
        self.settings.places.iter().find(|p| p.path == key).cloned()
    }

    pub(super) fn home_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin { left: theme::GAP, right: theme::GAP, top: 6.0, bottom: theme::GAP }))
            .show(ctx, |ui| {
                let full = ui.available_rect_before_wrap();
                let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(full).layout(Layout::top_down(Align::Min)));
                let t = (self.home_born.elapsed().as_secs_f32() / 0.32).clamp(0.0, 1.0);
                if t < 1.0 && anim::enabled() {
                    inner.set_opacity(0.25 + 0.75 * anim::smootherstep(t));
                    anim::keep_going(ctx);
                }
                let fits = !self.home_search.trim().is_empty() || self.home_starred || self.entries.is_empty();
                let foot = self.foot_height(&inner);
                let plan = plan_fit(full.width(), full.height(), foot).map(|f| (f, true)).or_else(|| plan_fit(full.width(), full.height(), 0.0).map(|f| (f, false)));
                match plan.filter(|_| !fits) {
                    Some((fit, with_foot)) => {
                        inner.set_width(full.width());
                        self.home_body(&mut inner, Some(fit));
                        if with_foot {
                            let at = Rect::from_min_max(Pos2::new(full.left(), full.bottom() - foot), full.max);
                            let mut foot_ui = inner.new_child(egui::UiBuilder::new().max_rect(at).layout(Layout::top_down(Align::Min)));
                            self.home_foot(&mut foot_ui);
                        }
                    }
                    None => {
                        egui::ScrollArea::vertical()
                            .id_salt("ns-home")
                            .auto_shrink([false, false])
                            .show(&mut inner, |ui| {
                                ui.set_width(full.width() - 10.0);
                                self.home_body(ui, None);
                                self.home_foot(ui);
                            });
                    }
                }
            });
    }

    fn foot_height(&self, ui: &egui::Ui) -> f32 {
        let s = ui.spacing().item_spacing.y;
        let youtrack = !self.settings.youtrack_url.trim().is_empty();
        let buttons = if youtrack { 2.0 } else { 1.0 };
        20.0 + buttons * (ui::BUTTON_H + s) + 10.0 + s + 14.0 + s + 12.0
    }

    pub fn debug_home_fits(&self, ctx: &egui::Context) -> bool {
        ctx.data(|d| d.get_temp::<bool>(egui::Id::new("ns-home-fit"))).unwrap_or(false)
    }

    fn home_body(&mut self, ui: &mut egui::Ui, fit: Option<Fit>) {
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("ns-home-fit"), fit.is_some()));
        let (row1_h, row2_h) = fit.map(|f| (f.row1, f.row2)).unwrap_or((ROW1_H, ROW2_H));
        let w = ui.available_width();
        let gap = theme::GAP;
        let cols: usize = if w >= 860.0 { 4 } else if w >= 520.0 { 2 } else { 1 };
        let unit = (w - gap * (cols - 1) as f32) / cols as f32;

        let needle = self.home_search.trim().to_lowercase();
        let searching = !needle.is_empty();
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
        if self.home_starred {
            list.retain(|e| e.starred);
        }

        // top tiles here---
        if !searching && !self.home_starred {
            let team = self.store.team().cloned();
            let me = team.as_ref().map(|t| t.me.clone());
            let mine = list
                .iter()
                .find(|e| match (&me, &e.edited_by) {
                    (Some(me), Some(by)) => by.id == me.id,
                    _ => false,
                })
                .or_else(|| list.first())
                .cloned();
            let top = ui.cursor().min;
            let at = |col: usize, row_y: f32, span: usize, h: f32| {
                Rect::from_min_size(Pos2::new(top.x + col as f32 * (unit + gap), top.y + row_y), Vec2::new(unit * span as f32 + gap * (span - 1) as f32, h))
            };
            let (hero, cont, week, due, bottom) = match cols {
                4 => {
                    let hero = at(0, 0.0, 2, row1_h + gap + row2_h);
                    (hero, at(2, 0.0, 2, row1_h), at(2, row1_h + gap, 1, row2_h), at(3, row1_h + gap, 1, row2_h), hero.bottom())
                }
                2 => {
                    let hero = at(0, 0.0, 2, HERO_NARROW_H);
                    let cont = at(0, hero.height() + gap, 2, ROW1_H);
                    let y = cont.bottom() - top.y + gap;
                    (hero, cont, at(0, y, 1, ROW2_H), at(1, y, 1, ROW2_H), top.y + y + ROW2_H)
                }
                _ => {
                    let hero = at(0, 0.0, 1, HERO_NARROW_H);
                    let cont = at(0, hero.height() + gap, 1, ROW1_H);
                    let week = at(0, cont.bottom() - top.y + gap, 1, ROW2_H);
                    let due = at(0, week.bottom() - top.y + gap, 1, ROW2_H);
                    (hero, cont, week, due, due.bottom())
                }
            };
            ui.allocate_rect(Rect::from_min_max(top, Pos2::new(top.x + w, bottom)), Sense::hover());
            self.hero_tile(ui, hero, team.as_ref());
            match &mine {
                Some(e) => self.continue_tile(ui, cont, e),
                None => self.start_tile(ui, cont),
            }
            self.week_tile(ui, week);
            self.deadline_tile(ui, due);
            ui.add_space(gap);
        }

        if self.entries.is_empty() {
            self.home_empty(ui);
            return;
        }

        // scripts and side here---
        let side = !searching && cols == 4;
        let list_w = if side { unit * 3.0 + gap * 2.0 } else { w };
        let mut order: Vec<PathBuf> = Vec::new();
        let mut open = None;
        let mut star = None;
        let mut menu = None;
        let mut jump = None;
        let top = ui.cursor().min;
        let mut list_ui = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_size(top, Vec2::new(list_w, f32::INFINITY))).layout(Layout::top_down(Align::Min)));
        let shown = list.len();
        let limit = if self.home_all || searching || self.home_starred || fit.is_some() { usize::MAX } else { LIST_MAX };
        let r = ncf::theme::surface_frame(ncf::theme::R_LG, ncf::theme::Lift::Rest)
            .inner_margin(egui::Margin::same(SCRIPTS_PAD))
            .show(&mut list_ui, |ui| {
                let p = pal();
                let iw = list_w - SCRIPTS_PAD * 2.0;
                ui.set_width(iw);
                let enter = self.scripts_head(ui, iw, shown, searching, if fit.is_some() { LIST_MAX } else { limit }, fit.is_none());
                ui.add_space(10.0);
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("ns-home-cols"), 1usize));
                let mut rows = |ui: &mut egui::Ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for e in list.iter().take(limit) {
                        let selected = self.home_keyed && self.home_sel == order.len();
                        order.push(e.path.clone());
                        let before = ui.cursor().min;
                        match self.script_row(ui, e, iw, selected) {
                            CardAction::Open => open = Some(e.path.clone()),
                            CardAction::Star => star = Some(e.path.clone()),
                            CardAction::Menu(at) => menu = Some((e.path.clone(), at)),
                            CardAction::None => {}
                        }
                        if selected && fit.is_some() {
                            ui.scroll_to_rect(Rect::from_min_size(before, Vec2::new(iw, ROW_H)), None);
                        }
                    }
                    let found = self.found_for(&needle);
                    self.hit_rows.clear();
                    if !found.is_empty() {
                        ui.add_space(16.0);
                        jump = self.found_section(ui, &found, iw);
                    }
                    if enter && open.is_none() && jump.is_none() && searching {
                        match (order.first(), found.first().and_then(|f| f.hits.first().map(|h| (f.path.clone(), h.block)))) {
                            (Some(path), _) => open = Some(path.clone()),
                            (None, Some(hit)) => jump = Some(hit),
                            (None, None) => {}
                        }
                    }
                    if order.is_empty() && found.is_empty() {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(if self.home_starred { "Nothing starred yet. Star a script from its menu." } else { "Nothing matches." }).font(theme::font(theme::T_SM)).color(p.text_faint));
                    }
                };
                match fit {
                    Some(f) => {
                        let room = (f.bottom - SCRIPTS_PAD * 2.0 - ui.min_rect().height()).max(ROW_H);
                        egui::ScrollArea::vertical()
                            .id_salt("ns-home-list")
                            .auto_shrink([false, false])
                            .max_height(room)
                            .show(ui, |ui| {
                                ui.set_width(iw);
                                rows(ui);
                            });
                    }
                    None => rows(ui),
                }
            });
        let list_rect = r.response.rect;
        if side {
            let side_h = fit.map(|f| f.bottom).unwrap_or(list_rect.height().max(SIDE_MIN_H));
            let rect = Rect::from_min_size(Pos2::new(top.x + list_w + gap, top.y), Vec2::new(unit, side_h));
            self.side_tile(ui, rect);
            ui.allocate_rect(Rect::from_min_max(top, Pos2::new(top.x + w, top.y + rect.height().max(list_rect.height()))), Sense::hover());
        } else {
            ui.allocate_rect(list_rect, Sense::hover());
            if !searching && !self.home_starred {
                ui.add_space(gap);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(w, SIDE_MIN_H), Sense::hover());
                self.side_tile(ui, rect);
            }
        }
        self.home_order = order;

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

    fn scripts_head(&mut self, ui: &mut egui::Ui, w: f32, shown: usize, searching: bool, limit: usize, toggle: bool) -> bool {
        let p = pal();
        let team = self.store.team().is_some();
        let mut enter = false;
        ui.allocate_ui_with_layout(Vec2::new(w, 34.0), Layout::left_to_right(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let heading = if searching {
                "Search"
            } else if self.home_starred {
                "Starred"
            } else if team {
                "Team scripts"
            } else {
                "Your scripts"
            };
            ui.label(egui::RichText::new(heading).font(ncf::theme::font_heading(theme::T_H + 1.0)).color(p.text));
            if !searching {
                ui::hud(ui, &shown.to_string(), Some(p.text_faint));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if toggle && !searching && !self.home_starred && shown > LIST_MAX && ui::link(ui, if limit == usize::MAX { "Show fewer" } else { "View all" }) {
                    self.home_all = !self.home_all;
                }
                let search = ui.add(
                    egui::TextEdit::singleline(&mut self.home_search)
                        .id(egui::Id::new("ns-home-search"))
                        .hint_text(if team {
                            format!("Search the team's scripts · {}+F", crate::keys::command_key())
                        } else {
                            format!("Search scripts · {}+F", crate::keys::command_key())
                        })
                        .desired_width((w * 0.42).clamp(160.0, 300.0))
                        .margin(egui::Margin::symmetric(12.0, 7.0)),
                );
                if self.focus_home_search {
                    search.request_focus();
                    self.focus_home_search = false;
                }
                enter = search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
        });
        enter
    }

    // hero tile here---
    fn hero_tile(&mut self, ui: &mut egui::Ui, rect: Rect, team: Option<&crate::backend::Team>) {
        let p = pal();
        ncf::theme::surface(ui.painter(), rect, ncf::theme::R_LG, ncf::theme::Lift::Rest);
        let x = rect.left() + 30.0;
        let room = rect.width() - 60.0;
        let clip = ui.painter().with_clip_rect(rect.shrink(1.0));
        let date = chrono::Local::now().format("%A, %-d %B").to_string();
        ui::eyebrow_text(&clip, Pos2::new(x, rect.top() + 36.0), &date, p.text_dim);
        let hello = match team.map(|t| t.me.first_name().to_string()).or_else(local_name) {
            Some(name) => format!("{}, {name}", greeting()),
            None => greeting().to_string(),
        };
        let fit = |size: f32| theme::tracked_width(ui.ctx(), &hello, theme::font_display(size), ncf::theme::display_tracking(size));
        let size = if fit(HERO_T) <= room { HERO_T } else { (HERO_T * room / fit(HERO_T) * 0.97).max(HERO_T_MIN) };
        let shown = ui::elide_to_width(ui.ctx(), &hello, theme::font_display(size), ncf::theme::display_tracking(size), room);
        theme::tracked_text(&clip, Pos2::new(x, rect.top() + 56.0 + size * 0.5), &shown, theme::font_display(size), p.text, ncf::theme::display_tracking(size));
        let today = crate::stats::today();
        let week = crate::stats::week(&self.settings.words, today);
        let streak = crate::stats::streak(&self.settings.words, today);
        let line = if week > 0 {
            if streak >= 2 {
                format!("You wrote {} words this week, {streak} days running.", grouped(week))
            } else {
                format!("You wrote {} words this week. Keep the streak going.", grouped(week))
            }
        } else {
            "A fresh week. Pick up wherever you like.".to_string()
        };
        let lf = theme::font(theme::T_BODY);
        let ly = rect.top() + 66.0 + size + 8.0;
        clip.text(Pos2::new(x, ly), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &line, lf.clone(), 0.0, room), lf, p.text_dim);
        if self.can_write() {
            let row = Rect::from_min_size(Pos2::new(x, ly + 20.0), Vec2::new(room, 40.0));
            let mut b = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(Layout::left_to_right(Align::Center)));
            b.spacing_mut().item_spacing.x = 8.0;
            if new_script_button(&mut b, 140.0, &self.keys_for(crate::keys::Command::NewScript)) {
                self.new_script();
                self.home = false;
            }
            if ui::button(&mut b, "Import", Some(Icon::Upload), false) {
                self.start_import();
            }
        }
        let art_top = ly + 74.0;
        if rect.bottom() - art_top > 90.0 {
            crate::art::paint_hero(ui, Rect::from_min_max(Pos2::new(rect.left() + 20.0, art_top), Pos2::new(rect.right() - 20.0, rect.bottom() - 16.0)));
        }
    }

    // start tile here---
    fn start_tile(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = pal();
        bento_tile(ui.painter(), rect, p.tile(0), 0.0);
        let x = rect.left() + 26.0;
        let clip = ui.painter().with_clip_rect(rect.shrink(1.0));
        ui::eyebrow_text(&clip, Pos2::new(x, rect.top() + 32.0), "Continue writing", theme::wash(p.on_tile, 190));
        theme::tracked_text(&clip, Pos2::new(x, rect.top() + 76.0), "Nothing yet", theme::font_display(T_TILE_TITLE), p.on_tile, ncf::theme::display_tracking(T_TILE_TITLE));
        clip.text(Pos2::new(x, rect.bottom() - 32.0), egui::Align2::LEFT_CENTER, "Your scripts show up here once you start one.", theme::font_med(theme::T_SM), theme::wash(p.on_tile, 200));
    }

    // continue writing tile here---
    fn continue_tile(&mut self, ui: &mut egui::Ui, rect: Rect, e: &Entry) {
        let p = pal();
        let resp = ui.interact(rect, egui::Id::new("ns-home-continue"), Sense::click());
        let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
        bento_tile(ui.painter(), rect, p.tile(0), hot);
        let ink = p.on_tile;
        let soft = theme::wash(ink, 200);
        let x = rect.left() + 26.0;
        let room = rect.width() - 52.0;
        let clip = ui.painter().with_clip_rect(rect.shrink(1.0));
        ui::eyebrow_text(&clip, Pos2::new(x, rect.top() + 32.0), "Continue writing", soft);
        let track = ncf::theme::display_tracking(T_TILE_TITLE);
        let font = theme::font_display(T_TILE_TITLE);
        let title = ui::elide_to_width(ui.ctx(), &e.title, font.clone(), track, room);
        theme::tracked_text(&clip, Pos2::new(x, rect.top() + 76.0), &title, font, ink, track);

        let place = self.place_of(&e.path);
        let line = match &place {
            Some(pl) if pl.scene > 0 && !pl.heading.trim().is_empty() => format!("Page {} · Scene {} · {}", pl.page, pl.scene, pl.heading.trim()),
            Some(pl) => format!("Page {}", pl.page),
            None => format!("{} · {}", plural(e.pages, "page", "pages"), plural(e.scenes, "scene", "scenes")),
        };
        let mf = theme::font_med(theme::T_SM);
        clip.text(Pos2::new(x, rect.top() + 114.0), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &line, mf.clone(), 0.0, room), mf, soft);
        if let Some(pl) = &place {
            let t = (pl.page as f32 / e.pages.max(1) as f32).clamp(0.02, 1.0);
            let bar = Rect::from_min_size(Pos2::new(x, rect.top() + 136.0), Vec2::new(room, 6.0));
            clip.rect_filled(bar, Rounding::same(3.0), theme::wash(ink, 46));
            clip.rect_filled(Rect::from_min_size(bar.min, Vec2::new(bar.width() * t, 6.0)), Rounding::same(3.0), ink);
        }
        let action = if self.is_opening(&e.path) {
            "Opening…".to_string()
        } else if let Some(who) = &e.editing {
            format!("{} is in it now", who.first_name())
        } else if place.is_some() {
            "Open where you left off".to_string()
        } else {
            "Open".to_string()
        };
        let af = theme::font_bold(theme::T_SM);
        let aw = clip.text(Pos2::new(x, rect.bottom() - 30.0), egui::Align2::LEFT_CENTER, &action, af, ink).width();
        if !self.is_opening(&e.path) && e.editing.is_none() {
            icons::draw(&clip, Rect::from_center_size(Pos2::new(x + aw + 12.0 + 3.0 * hot, rect.bottom() - 30.0), Vec2::splat(14.0)), Icon::ArrowRight, ink);
        }
        if let Some(by) = &e.edited_by {
            let me = self.me();
            let by = people::by_line(by, me.as_ref(), &relative_time(e.modified));
            let bf = theme::font(theme::T_CAP);
            clip.text(Pos2::new(rect.right() - 26.0, rect.bottom() - 30.0), egui::Align2::RIGHT_CENTER, ui::elide_to_width(ui.ctx(), &by, bf.clone(), 0.0, room * 0.4), bf, soft);
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            if let Some(pl) = place {
                self.pending_jump = Some((e.path.clone(), pl.block));
            }
            self.open_from_home(e.path.clone());
        }
    }

    // week tile here---
    fn week_tile(&self, ui: &egui::Ui, rect: Rect) {
        let p = pal();
        bento_tile(ui.painter(), rect, p.tile(1), 0.0);
        let ink = p.on_tile;
        let soft = theme::wash(ink, 200);
        let clip = ui.painter().with_clip_rect(rect.shrink(1.0));
        let x = rect.left() + 22.0;
        ui::eyebrow_text(&clip, Pos2::new(x, rect.top() + 32.0), "This week", soft);
        let today = crate::stats::today();
        let words = crate::stats::week(&self.settings.words, today);
        big_number(ui, &clip, Pos2::new(x, rect.top() + 82.0), &grouped(words), ink, rect.right() - 22.0);
        let streak = crate::stats::streak(&self.settings.words, today);
        let goal = self.settings.session_goal;
        let line = if goal > 0 && self.session_words > 0 {
            let t = (self.session_words.max(0) as f32 / goal as f32).clamp(0.0, 1.0);
            format!("words written · {}% of today's goal", (t * 100.0).floor() as u32)
        } else if streak >= 2 {
            format!("words written · {streak}-day streak")
        } else {
            "words written".to_string()
        };
        let f = theme::font_med(theme::T_SM);
        clip.text(Pos2::new(x, rect.top() + 128.0), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &line, f.clone(), 0.0, rect.width() - 44.0), f, soft);
    }

    // deadline tile here---
    fn deadline_tile(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = pal();
        let today = crate::stats::today();
        let next = self
            .entries
            .iter()
            .filter_map(|e| e.deadline.as_ref().map(|(d, l)| (*d, l.clone(), e.title.clone(), e.path.clone())))
            .filter(|(d, _, _, _)| crate::stats::days_left(*d, today) >= -14)
            .min_by_key(|(d, _, _, _)| *d);
        let resp = ui.interact(rect, egui::Id::new("ns-home-deadline"), if next.is_some() { Sense::click() } else { Sense::hover() });
        let hot = anim::ease(ui.ctx(), resp.id, resp.hovered() && next.is_some(), anim::HOVER);
        bento_tile(ui.painter(), rect, p.tile(2), hot);
        let ink = p.on_tile;
        let soft = theme::wash(ink, 200);
        let clip = ui.painter().with_clip_rect(rect.shrink(1.0));
        let x = rect.left() + 22.0;
        ui::eyebrow_text(&clip, Pos2::new(x, rect.top() + 32.0), "Next deadline", soft);
        let room = rect.width() - 44.0;
        let f = theme::font_med(theme::T_SM);
        match &next {
            Some((d, label, title, _)) => {
                big_number(ui, &clip, Pos2::new(x, rect.top() + 82.0), &crate::stats::when(crate::stats::days_left(*d, today)), ink, rect.right() - 22.0);
                let line = if label.is_empty() { title.clone() } else { format!("{label} · {title}") };
                clip.text(Pos2::new(x, rect.top() + 128.0), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &line, f.clone(), 0.0, room), f, soft);
            }
            None => {
                theme::tracked_text(&clip, Pos2::new(x, rect.top() + 82.0), "None set", theme::font_display(34.0), ink, ncf::theme::display_tracking(34.0));
                clip.text(Pos2::new(x, rect.top() + 128.0), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), "Add one on a script's title page", f.clone(), 0.0, room), f, soft);
            }
        }
        if next.is_some() && resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            if let Some((_, _, _, path)) = next {
                self.open_from_home(path);
            }
        }
    }

    // side tile here---
    fn side_tile(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let p = pal();
        bento_tile(ui.painter(), rect, p.tile(3), 0.0);
        let ink = p.on_tile;
        let soft = theme::wash(ink, 200);
        let x = rect.left() + 22.0;
        let room = rect.width() - 44.0;
        let clip = ui.painter().with_clip_rect(rect.shrink(1.0));
        let hf = theme::font_bold(theme::T_BODY);
        let live: Vec<(String, String)> = self.entries.iter().filter_map(|e| e.editing.as_ref().map(|who| (who.name.clone(), e.title.clone()))).collect();
        let mut open = None;
        let mut y = rect.top() + 36.0;
        if let Some(team) = self.store.team().cloned() {
            let room_name = team.name.split(" · ").next().unwrap_or(&team.name).to_string();
            clip.text(Pos2::new(x, y), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &room_name, hf.clone(), 0.0, room - 70.0), hf, ink);
            if !live.is_empty() {
                let text = format!("{} live", live.len());
                let size = ui::tag_size(ui.ctx(), &text, true);
                ui::tag_at(ui.painter(), Rect::from_min_size(Pos2::new(rect.right() - 18.0 - size.x, y - size.y * 0.5), size), &text, Some(4), true);
            }
            y += 36.0;
            let rows: Vec<(crate::backend::Person, String)> = team
                .members
                .iter()
                .filter(|m| !m.pending)
                .map(|m| {
                    let doing = live.iter().find(|(n, _)| *n == m.person.name).map(|(_, t)| format!("Writing {t}")).unwrap_or_else(|| m.role.label().to_string());
                    (m.person.clone(), doing)
                })
                .collect();
            let total = rows.len();
            let show = fitting(y, PERSON_STEP, rect.bottom(), total);
            for (person, doing) in rows.into_iter().take(show) {
                people::avatar(ui, Rect::from_center_size(Pos2::new(x + 15.0, y + 6.0), Vec2::splat(30.0)), &person);
                let nf = theme::font_semi(theme::T_SM);
                clip.text(Pos2::new(x + 40.0, y - 2.0), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &person.name, nf.clone(), 0.0, room - 40.0), nf, ink);
                let sf = theme::font(theme::T_CAP);
                clip.text(Pos2::new(x + 40.0, y + 15.0), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &doing, sf.clone(), 0.0, room - 40.0), sf, soft);
                y += PERSON_STEP;
            }
            if total > show {
                more_line(&clip, Pos2::new(x, y - 10.0), total - show, "more in the team", soft);
            }
        } else {
            let notes: Vec<(String, String, String, usize, PathBuf)> = self
                .entries
                .iter()
                .flat_map(|e| e.notes.iter().map(move |(n, at)| (n.clone(), e.title.clone(), at.clone(), crate::storage::cover_of(e), e.path.clone())))
                .collect();
            clip.text(Pos2::new(x, y), egui::Align2::LEFT_CENTER, "Notes", hf, ink);
            if !notes.is_empty() {
                let text = format!("{} open", notes.len());
                let tf = theme::font_bold(theme::T_CAP);
                let tw = theme::tracked_width(ui.ctx(), &text, tf.clone(), 0.0) + 20.0;
                let chip = Rect::from_min_size(Pos2::new(rect.right() - 18.0 - tw, y - 12.0), Vec2::new(tw, 24.0));
                clip.rect_filled(chip, Rounding::same(12.0), ink);
                clip.text(chip.center(), egui::Align2::CENTER_CENTER, text, tf, p.tile(3));
            }
            y += 38.0;
            if notes.is_empty() {
                let f = theme::font(theme::T_SM);
                let g = ui.painter().layout("Notes you leave on a line gather here, from every script.".to_string(), f, soft, room);
                clip.galley(Pos2::new(x, y - 8.0), g, soft);
            }
            let total = notes.len();
            let show = fitting(y, NOTE_STEP, rect.bottom(), total);
            for (k, (text, title, at, cover, path)) in notes.into_iter().take(show).enumerate() {
                let row = Rect::from_min_size(Pos2::new(x - 8.0, y - 18.0), Vec2::new(room + 16.0, 44.0));
                let resp = ui.interact(row, egui::Id::new(("ns-home-note", k)), Sense::click());
                let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
                if hot > 0.01 {
                    clip.rect_filled(row, Rounding::same(12.0), theme::wash(ink, (22.0 * hot) as u8));
                }
                let dot = Rect::from_center_size(Pos2::new(x + 14.0, y + 4.0), Vec2::splat(28.0));
                clip.circle_filled(dot.center(), 14.0, p.bright(if cover == 3 { 0 } else { cover }));
                icons::draw(&clip, dot.shrink(8.0), Icon::Note, ink);
                let nf = theme::font_semi(theme::T_SM);
                clip.text(Pos2::new(x + 38.0, y - 4.0), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &text, nf.clone(), 0.0, room - 40.0), nf, ink);
                let where_ = if at.is_empty() { title } else { format!("{title} · {at}") };
                let sf = theme::font(theme::T_CAP);
                clip.text(Pos2::new(x + 38.0, y + 13.0), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &where_, sf.clone(), 0.0, room - 40.0), sf, soft);
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    open = Some(path);
                }
                y += NOTE_STEP;
            }
            if total > show {
                let n = total - show;
                more_line(&clip, Pos2::new(x, y - 10.0), n, if n == 1 { "more note" } else { "more notes" }, soft);
            }
        }
        if let Some(path) = open {
            self.open_from_home(path);
        }
    }

    fn script_row(&self, ui: &mut egui::Ui, e: &Entry, w: f32, selected: bool) -> CardAction {
        let p = pal();
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, ROW_H), Sense::click());
        let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
        let round = Rounding::same(ncf::theme::R_SM + 2.0);
        if hot > 0.01 {
            ui.painter().rect_filled(rect, round, theme::wash(p.raised, (255.0 * hot) as u8));
        }
        if selected {
            ui.painter().rect_stroke(rect.shrink(1.0), round, Stroke::new(2.0_f32, p.focus));
        }
        let cp = ui.painter().with_clip_rect(rect.shrink(1.0));
        let chip = Rect::from_center_size(Pos2::new(rect.left() + 34.0, rect.center().y), Vec2::splat(44.0));
        cover_chip(&cp, chip, &e.title, crate::storage::cover_of(e));

        let (tag, tone, dot) = if self.is_opening(&e.path) {
            ("Opening…".to_string(), None, false)
        } else if let Some(who) = &e.editing {
            (format!("{} is in it", who.first_name()), Some(4), true)
        } else if e.waiting {
            ("Waiting to sync".to_string(), Some(2), false)
        } else if let Some(r) = e.revision {
            (format!("{} pages", crate::revision::name(r)), Some(3), false)
        } else if e.timelines > 0 {
            (format!("{} timelines", e.timelines + 1), Some(1), false)
        } else if !e.draft.is_empty() {
            (ui::elide(&e.draft, 22), None, false)
        } else {
            (String::new(), None, false)
        };
        let star_w = 34.0;
        let tag_w = if tag.is_empty() { 0.0 } else { ui::tag_size(ui.ctx(), &tag, dot).x + 12.0 };
        let tx = chip.right() + 14.0;
        let room = rect.right() - tx - tag_w - star_w - 8.0;
        let tf = theme::font_semi(theme::T_BODY);
        cp.text(Pos2::new(tx, rect.center().y - 9.0), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &e.title, tf.clone(), 0.0, room), tf, p.text);
        let me = self.me();
        let when = match &e.edited_by {
            Some(by) => people::by_line(by, me.as_ref(), &relative_time(e.modified)),
            None => format!("edited {}", relative_time(e.modified)),
        };
        let mut meta = format!("{} · {} · {when}", plural(e.pages, "page", "pages"), plural(e.scenes, "scene", "scenes"));
        if let Some((d, _)) = &e.deadline {
            meta.push_str(&format!(" · due {}", d.format("%-d %b")));
        }
        let mf = theme::font(theme::T_CAP);
        cp.text(Pos2::new(tx, rect.center().y + 11.0), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &meta, mf.clone(), 0.0, room), mf, p.text_faint);
        if !tag.is_empty() {
            let size = ui::tag_size(ui.ctx(), &tag, dot);
            let at = Rect::from_min_size(Pos2::new(rect.right() - star_w - 6.0 - size.x, rect.center().y - size.y * 0.5), size);
            ui::tag_at(ui.painter(), at, &tag, tone, dot);
        }
        if e.cloud.is_some() && tag.is_empty() {
            icons::draw(ui.painter(), Rect::from_center_size(Pos2::new(rect.right() - star_w - 16.0, rect.center().y), Vec2::splat(13.0)), Icon::Cloud, p.text_faint);
        }

        let star_rect = Rect::from_center_size(Pos2::new(rect.right() - star_w * 0.5 - 2.0, rect.center().y), Vec2::splat(26.0));
        let star_resp = ui.interact(star_rect, resp.id.with("star"), Sense::click());
        let star_hot = anim::ease(ui.ctx(), star_resp.id, star_resp.hovered(), anim::HOVER);
        if e.starred || hot > 0.02 {
            icons::draw(
                ui.painter(),
                star_rect.shrink(6.0),
                if e.starred { Icon::StarFilled } else { Icon::Star },
                if e.starred { theme::lighten(p.warn, 0.2 * star_hot) } else { theme::mix(theme::wash(p.text_faint, (200.0 * hot) as u8), p.warn, star_hot) },
            );
        }
        let star_clicked = star_resp.on_hover_text(if e.starred { "Unstar — stars are yours alone" } else { "Star it, for you" }).clicked();
        let resp = if e.preview.is_empty() { resp } else { resp.on_hover_text(format!("{}\nright click for more", e.preview)) };
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if star_clicked {
            CardAction::Star
        } else if resp.secondary_clicked() {
            ui.ctx().pointer_latest_pos().map(CardAction::Menu).unwrap_or(CardAction::None)
        } else if resp.clicked() {
            CardAction::Open
        } else {
            CardAction::None
        }
    }

    pub(super) fn found_for(&mut self, needle: &str) -> Vec<Found> {
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
                    job.append("Note  ", 0.0, egui::TextFormat { font_id: theme::font_semi(theme::T_CAP), color: p.primary_light, ..Default::default() });
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

    fn home_empty(&mut self, ui: &mut egui::Ui) {
        let (title, body, action) = if self.can_write() {
            ("No scripts here yet", "Start one, or bring one in: Fountain, Final Draft and NativeDraft's own files all import.", Some("New script"))
        } else {
            ("No scripts in this space yet", "When someone on the team starts a script, it shows up here.", None)
        };
        if ui::empty_state(ui, Icon::File, title, body, action) {
            self.new_script();
            self.home = false;
        }
        ui.add_space(18.0)
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

fn greeting() -> &'static str {
    use chrono::Timelike;
    match chrono::Local::now().hour() {
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        18..=22 => "Good evening",
        _ => "Up late",
    }
}

pub(super) fn local_name() -> Option<String> {
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

fn big_number(ui: &egui::Ui, painter: &egui::Painter, left_centre: Pos2, figure: &str, ink: Color32, right: f32) {
    let mut size = T_BIG;
    let room = (right - left_centre.x).max(20.0);
    let w = theme::tracked_width(ui.ctx(), figure, theme::font_display(size), ncf::theme::display_tracking(size));
    if w > room {
        size = (size * room / w).max(20.0);
    }
    theme::tracked_text(painter, left_centre, figure, theme::font_display(size), ink, ncf::theme::display_tracking(size));
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

pub(crate) fn fitting(y: f32, step: f32, bottom: f32, total: usize) -> usize {
    let mut cap = 0;
    while cap < total && y + cap as f32 * step + 40.0 <= bottom {
        cap += 1;
    }
    if cap == total {
        return total;
    }
    while cap > 0 && y + cap as f32 * step + 2.0 > bottom - 22.0 {
        cap -= 1;
    }
    cap
}

fn more_line(painter: &egui::Painter, at: Pos2, n: usize, what: &str, color: Color32) {
    let f = theme::font_semi(theme::T_SM);
    painter.text(at, egui::Align2::LEFT_CENTER, format!("+{n} {what}"), f, color);
}
