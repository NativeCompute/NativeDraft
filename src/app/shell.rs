use eframe::egui::{self, Align, Layout, Pos2, Rect, Sense, Stroke, Vec2};

use super::{App, Mode, Popover};
use crate::anim;
use crate::backend::Source;
use crate::icons::{self, Icon};
use crate::keys;
use crate::model::Element;
use crate::theme::{self, pal};
use crate::ui;

pub const SIDE_W: f32 = 232.0;
pub const RAIL_W: f32 = 64.0;
pub const FOLD_BELOW: f32 = 1180.0;
pub const INSPECTOR_W: f32 = 318.0;
pub const TOOLBAR_H: f32 = 58.0;
pub const STATUS_H: f32 = 58.0;
pub const TABS: [&str; 4] = ["Scenes", "Notes", "Changes", "Cast"];

impl App {
    pub(super) fn side_folded(&self, ctx: &egui::Context) -> bool {
        !self.settings.show_library || self.focus_mode || ctx.screen_rect().width() < FOLD_BELOW
    }

    // sidebar here---
    pub(super) fn sidebar(&mut self, ctx: &egui::Context) {
        if self.focus_mode && !self.home {
            self.side_rect = Rect::NOTHING;
            return;
        }
        let folded = self.side_folded(ctx);
        let w = if folded { RAIL_W } else { SIDE_W };
        egui::SidePanel::left("ns-sidebar")
            .exact_width(w + theme::GAP)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::none().inner_margin(egui::Margin { left: theme::GAP, right: 0.0, top: 6.0, bottom: theme::GAP }))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                self.side_rect = rect;
                ncf::theme::glass(ui.painter(), rect, ncf::theme::R_LG);
                let inner = rect.shrink2(Vec2::new(if folded { 10.0 } else { 12.0 }, 14.0));
                let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(Layout::top_down(Align::Min)));
                ui.spacing_mut().item_spacing.y = 2.0;
                if folded {
                    self.side_rail(&mut ui);
                } else {
                    self.side_full(&mut ui);
                }
            });
    }

    fn side_full(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        if self.can_write() && big_primary(ui, "New script", &format!("New script · {}", self.keys_for(keys::Command::NewScript))) {
            self.new_script();
            self.home = false;
        }
        ui.add_space(12.0);
        let starred = self.entries.iter().filter(|e| e.starred).count();
        if ui::side_link(ui, Icon::Home, "Home", None, self.home && !self.home_starred).clicked() {
            self.home_starred = false;
            if !self.home {
                self.go_home();
            }
        }
        if let Some(path) = self.path.clone() {
            let title = self.title_or_untitled();
            if ui::side_link(ui, Icon::File, &title, Some("open"), !self.home).clicked() && self.home {
                self.open_from_home(path);
            }
        }
        if ui::side_link(ui, Icon::Star, "Starred", Some(&starred.to_string()), self.home && self.home_starred).clicked() {
            self.home_starred = !(self.home && self.home_starred);
            if !self.home {
                self.go_home();
            }
        }
        if self.store.team().is_some() {
            let r = ui::side_link(ui, Icon::Trash, "Recently deleted", None, self.popover == Some(Popover::Trash));
            self.account_button_rect = r.rect;
            if r.clicked() {
                self.toggle_popover(Popover::Trash);
            }
        }

        let spaces = self.store.spaces();
        if !spaces.is_empty() {
            ui::side_section(ui, "Cloud");
            let source = self.store.source();
            let mut pick = None;
            if ui::side_link(ui, Icon::Folder, "On this computer", None, source == Source::Local).clicked() {
                pick = Some(Source::Local);
            }
            let orgs: std::collections::BTreeSet<&str> = spaces.iter().filter_map(|s| s.org.as_deref()).collect();
            for s in &spaces {
                let name = if s.personal { "My cloud".to_string() } else if orgs.len() > 1 { s.name.clone() } else { s.short.clone() };
                let icon = if s.personal { Icon::Cloud } else { Icon::Users };
                let here = Source::Space(s.id.clone());
                if ui::side_link(ui, icon, &name, Some(&s.scripts.to_string()), source == here).clicked() {
                    pick = Some(here);
                }
            }
            if let Some(src) = pick {
                if src != source {
                    self.store.set_source(src);
                    self.home_sel = 0;
                    self.refresh_entries();
                }
                if !self.home {
                    self.go_home();
                }
            }
        } else if self.cloud_on() && !self.signed_in() {
            ui::side_section(ui, "Cloud");
            if ui::side_link(ui, Icon::Cloud, "Sign in to sync", None, false).clicked() {
                self.open_settings_tab(super::cloudui::ACCOUNT_TAB);
            }
        }

        let foot_h = 2.0 * ui::SIDE_ROW_H + 48.0;
        let room = ui.available_height() - foot_h;
        if room > 0.0 {
            ui.add_space(room);
        }
        let y = ui.cursor().min.y;
        ui.painter().line_segment([Pos2::new(ui.max_rect().left() + 4.0, y), Pos2::new(ui.max_rect().right() - 4.0, y)], Stroke::new(1.0_f32, p.line));
        ui.add_space(8.0);
        let sopen = self.popover == Some(Popover::Settings);
        let r = ui::side_link(ui, Icon::Settings, "Settings", None, false);
        self.settings_button_rect = r.rect;
        if r.on_hover_text(format!("Settings · {}", self.keys_for(keys::Command::Settings))).clicked() {
            if sopen {
                self.popover = None;
            } else {
                self.toggle_popover(Popover::Settings);
            }
        }
        self.side_person(ui);
    }

    fn side_person(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        let (name, sub, me) = match self.store.team() {
            Some(t) => (t.me.name.clone(), format!("{} · {}", t.name, t.role.label()), Some(t.me.clone())),
            None => (
                super::home::local_name().unwrap_or_else(|| "You".to_string()),
                format!("{} {}", self.entries.len(), if self.entries.len() == 1 { "script" } else { "scripts" }),
                None,
            ),
        };
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
        self.foot_rect = rect;
        let av = Rect::from_center_size(Pos2::new(rect.left() + 20.0, rect.center().y), Vec2::splat(32.0));
        match &me {
            Some(person) => crate::people::avatar(ui, av, person),
            None => {
                ui.painter().circle_filled(av.center(), 16.0, p.text);
                let initial: String = name.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
                ui.painter().text(av.center(), egui::Align2::CENTER_CENTER, initial, theme::font_bold(theme::T_SM), p.backdrop);
            }
        }
        let room = rect.right() - av.right() - 14.0;
        let nf = theme::font_semi(theme::T_SM + 0.5);
        ui.painter().text(Pos2::new(av.right() + 10.0, rect.center().y - 8.0), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &name, nf.clone(), 0.0, room), nf, p.text);
        let sf = theme::font(theme::T_CAP);
        ui.painter().text(Pos2::new(av.right() + 10.0, rect.center().y + 9.0), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &sub, sf.clone(), 0.0, room), sf, p.text_faint);
        if me.is_some() {
            self.account_button_rect = rect;
            if resp.clicked() {
                self.toggle_popover(Popover::Account);
            }
        }
    }

    fn side_rail(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        let w = ui.available_width();
        let row = |ui: &mut egui::Ui, icon: Icon, tip: &str, on: bool| -> egui::Response {
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 42.0), Sense::click());
            let cell = Rect::from_center_size(rect.center(), Vec2::splat(40.0));
            let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
            let (sel_fill, sel_ink) = ncf::theme::pal().selected();
            if on {
                ui.painter().rect_filled(cell, egui::Rounding::same(12.0), sel_fill);
            } else if hot > 0.01 {
                ui.painter().rect_filled(cell, egui::Rounding::same(12.0), theme::wash(p.raised, (255.0 * hot) as u8));
            }
            icons::draw(ui.painter(), Rect::from_center_size(cell.center(), Vec2::splat(17.0)), icon, if on { sel_ink } else { p.text_dim });
            resp.on_hover_text(tip)
        };
        if self.can_write() {
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 46.0), Sense::click());
            let c = Rect::from_center_size(rect.center(), Vec2::splat(42.0));
            let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
            let (fill, _, ink) = ui::button_colors(true, hot);
            ui.painter().rect_filled(c, egui::Rounding::same(14.0), fill);
            icons::draw(ui.painter(), c.shrink(12.0), Icon::Plus, ink);
            if resp.on_hover_text(format!("New script · {}", self.keys_for(keys::Command::NewScript))).clicked() {
                self.new_script();
                self.home = false;
            }
            ui.add_space(8.0);
        }
        if row(ui, Icon::Home, "Home", self.home && !self.home_starred).clicked() {
            self.home_starred = false;
            if !self.home {
                self.go_home();
            }
        }
        if let Some(path) = self.path.clone() {
            if row(ui, Icon::File, &self.title_or_untitled(), !self.home).clicked() && self.home {
                self.open_from_home(path);
            }
        }
        if row(ui, Icon::Star, "Starred", self.home && self.home_starred).clicked() {
            self.home_starred = !(self.home && self.home_starred);
            if !self.home {
                self.go_home();
            }
        }
        let room = ui.available_height() - 2.0 * 42.0 - 8.0;
        if room > 0.0 {
            ui.add_space(room);
        }
        let r = row(ui, Icon::Settings, "Settings", self.popover == Some(Popover::Settings));
        self.settings_button_rect = r.rect;
        if r.clicked() {
            self.toggle_popover(Popover::Settings);
        }
        let r = row(ui, Icon::Sidebar, "Show the library", false);
        self.foot_rect = r.rect;
        if r.clicked() {
            self.settings.show_library = true;
            self.focus_mode = false;
            self.save_settings();
        }
    }

    // toolbar here---
    pub(super) fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("ns-toolbar")
            .exact_height(TOOLBAR_H)
            .show_separator_line(false)
            .frame(egui::Frame::none().inner_margin(egui::Margin { left: theme::GAP, right: theme::GAP, top: 6.0, bottom: 0.0 }))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ncf::theme::glass(ui.painter(), rect, rect.height() * 0.5);
                let inner = rect.shrink2(Vec2::new(8.0, 6.0));
                let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(Layout::left_to_right(Align::Center)));
                self.toolbar_row(&mut ui);
            });
    }

    fn toolbar_row(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        ui.spacing_mut().item_spacing.x = 6.0;
        let tip = format!("Back to your scripts · {}", self.keys_for(keys::Command::FindScripts));
        if ui::ribbon_button(ui, Icon::ChevronLeft, "Library", &tip, false) {
            self.go_home();
        }
        self.library_button = ui.min_rect();
        ui.add_space(2.0);
        let tabs = [
            ui::SegTab { label: "Write", icon: Some(Icon::Pencil), count: None, dot: false },
            ui::SegTab { label: "Cards", icon: Some(Icon::Grid), count: None, dot: false },
            ui::SegTab { label: "Read", icon: Some(Icon::Eye), count: None, dot: false },
        ];
        if let Some(picked) = ui::seg_tabs(ui, &tabs, self.mode.index(), None) {
            let m = Mode::from_index(picked);
            if m == Mode::Read || self.guard_edit() {
                self.mode = m;
            }
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let (rect, menu) = ui.allocate_exact_size(Vec2::new(34.0, 34.0), Sense::click());
            self.menu_button_rect = rect;
            let open = matches!(self.popover, Some(Popover::Menu) | Some(Popover::Snapshots));
            let hot = anim::ease(ui.ctx(), menu.id, menu.hovered() || open, anim::HOVER);
            ui.painter().circle_filled(rect.center(), 17.0, theme::mix(p.raised, p.line_strong, hot * 0.6));
            icons::draw(ui.painter(), rect.shrink(10.0), Icon::More, if open { p.text } else { p.text_dim });
            if menu.on_hover_text("More actions").clicked() {
                self.toggle_popover(Popover::Menu);
            }
            let open_i = self.settings.show_scenes && !self.focus_mode;
            let tip = format!("Scenes, notes, changes and cast · {}", self.keys_for(keys::Command::ToggleScenes));
            if ui::ribbon_button(ui, Icon::Sidebar, "", &tip, open_i) {
                self.settings.show_scenes = !self.settings.show_scenes;
                self.focus_mode = false;
                self.save_settings();
            }
            match self.mode {
                Mode::Write => self.write_tools_right(ui),
                Mode::Cards => self.cards_tools_right(ui),
                Mode::Read => self.read_tools_right(ui),
            }
        });
    }

    fn write_tools_right(&mut self, ui: &mut egui::Ui) {
        let write = self.can_write();
        let tip = format!("Export this script as a PDF · {}", self.keys_for(keys::Command::QuickExport));
        if ui::button_response(ui, "Export", Some(Icon::Download), true, None).on_hover_text(tip).clicked() {
            self.export(crate::export::Format::Pdf, true);
        }
        if write && ui::ribbon_button(ui, Icon::History, "Snapshot", "Keep a copy of the script as it is now · find them under ⋯ → Snapshots", false) {
            self.take_snapshot();
        }
        let tip = format!("A note on the line the caret is in · {}", self.keys_for(keys::Command::AddNote));
        if write && ui::ribbon_button(ui, Icon::Note, "Note", &tip, false) {
            self.run(keys::Command::AddNote);
        }
        let current = self.ed.current_element(&self.doc);
        let label = current.map(|e| e.label()).unwrap_or(Element::Action.label());
        if ui.available_width() > 150.0 {
            let resp = ui::select_button(ui, "Element", label);
            self.element_button_rect = resp.rect;
            if resp.on_hover_text("The element the caret is in · Tab cycles").clicked() {
                self.toggle_popover(Popover::Elements);
            }
        }
    }

    fn cards_tools_right(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        let tip = format!("A new scene after the one you are in · {}", self.keys_for(keys::Command::NewScene));
        if self.can_write() && ui::button_response(ui, "Scene", Some(Icon::Plus), true, None).on_hover_text(tip).clicked() {
            self.new_scene();
            self.mode = Mode::Cards;
        }
        let tip = "Beat guides on the timeline: inciting incident, the act breaks, midpoint and climax";
        if ui::ribbon_button(ui, Icon::Timeline, "Beats", tip, self.settings.beat_guides) {
            self.settings.beat_guides = !self.settings.beat_guides;
            self.save_settings();
        }
        let hint = ui::pick_that_fits(ui, &["Drag cards to reorder scenes", "Drag to reorder"], ui.available_width() - 20.0);
        if !hint.is_empty() {
            ui.label(egui::RichText::new(hint).font(theme::font(theme::T_SM)).color(p.text_faint));
        }
    }

    fn read_tools_right(&mut self, ui: &mut egui::Ui) {
        let tip = format!("Choose pages and colours · {}", self.keys_for(keys::Command::ExportWindow));
        if ui::button_response(ui, "Export…", Some(Icon::Download), true, None).on_hover_text(tip).clicked() {
            self.open_export_window();
        }
        let tip = format!("Export this script as a PDF · {}", self.keys_for(keys::Command::QuickExport));
        if ui::ribbon_button(ui, Icon::Print, "Quick Export", &tip, false) {
            self.export(crate::export::Format::Pdf, true);
        }
        if !self.access.can_edit() {
            self.lock_chip(ui);
        }
    }

    // status here---
    pub(super) fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("ns-status")
            .exact_height(STATUS_H)
            .show_separator_line(false)
            .frame(egui::Frame::none().inner_margin(egui::Margin { left: theme::GAP, right: theme::GAP, top: 10.0, bottom: theme::GAP }))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                self.status_rect = rect;
                ncf::theme::glass(ui.painter(), rect, rect.height() * 0.5);
                let inner = rect.shrink2(Vec2::new(10.0, 0.0));
                let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(Layout::left_to_right(Align::Center)));
                self.status_row(&mut ui);
            });
    }

    fn status_row(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        let l = &self.layout;
        let one = |n: usize, a: &'static str, b: &'static str| if n == 1 { a } else { b };
        let (pages, scenes, words) = (l.pages, l.scenes, l.words);
        ui.spacing_mut().item_spacing.x = 8.0;
        let right_w = 260.0;
        let room = ui.available_width() - right_w;
        let mut used = 0.0;
        let mut chip = |ui: &mut egui::Ui, v: String, u: &str| {
            let w = theme::tracked_width(ui.ctx(), &v, theme::font_hud(theme::T_SM - 1.5), 0.0) + theme::tracked_width(ui.ctx(), u, theme::font(theme::T_SM), 0.0) + 34.0;
            if used + w < room {
                ui::stat_chip(ui, &v, u);
                used += w;
            }
        };
        chip(ui, pages.to_string(), one(pages, "page", "pages"));
        chip(ui, format!("~{pages}"), "min");
        chip(ui, scenes.to_string(), one(scenes, "scene", "scenes"));
        chip(ui, super::home::grouped(words as i64), one(words, "word", "words"));
        if self.session_words > 0 {
            chip(ui, format!("+{}", super::home::grouped(self.session_words)), "this session");
        }
        let goal = self.settings.session_goal;
        if goal > 0 && used + 70.0 < room {
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(60.0, 14.0), Sense::hover());
            let t = (self.session_words.max(0) as f32 / goal as f32).clamp(0.0, 1.0);
            let track = Rect::from_center_size(rect.center(), Vec2::new(56.0, 4.0));
            ui.painter().rect_filled(track, egui::Rounding::same(2.0), p.line_strong);
            if t > 0.0 {
                let fill = Rect::from_min_max(track.min, Pos2::new(track.left() + (track.width() * t).max(4.0), track.bottom()));
                ui.painter().rect_filled(fill, egui::Rounding::same(2.0), p.primary);
            }
            resp.on_hover_text(format!("Session goal: {} of {goal} words", self.session_words.max(0)));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(6.0);
            let (text, col) = self.save_state();
            ui.label(egui::RichText::new(text).font(theme::font(theme::T_SM)).color(p.text_dim));
            let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
            ui.painter().circle_filled(r.center(), 4.0, col);
        });
    }

    pub(super) fn save_state(&self) -> (String, egui::Color32) {
        let p = pal();
        let cloud = self.path.as_ref().and_then(|x| self.entries.iter().find(|e| &e.path == x)).and_then(|e| e.cloud.clone());
        if self.dirty {
            return ("Unsaved".to_string(), p.warn);
        }
        let when = self.saved_at.map(|t| format!(" · {}", super::ago_instant(t))).unwrap_or_default();
        match (cloud, self.store.sync()) {
            (Some(place), Some(s)) if s.warn => (format!("Saved · {} for {place}", s.text.to_lowercase()), p.warn),
            (Some(place), _) => (format!("Saved · synced to {place}{when}"), p.ok),
            (None, _) => (format!("Saved on this computer{when}"), p.ok),
        }
    }

    // inspector here---
    pub(super) fn inspector(&mut self, ctx: &egui::Context) {
        if !self.settings.show_scenes || self.focus_mode {
            self.aside_rect = Rect::NOTHING;
            self.aside_islands.clear();
            return;
        }
        egui::SidePanel::right("ns-inspector")
            .exact_width(INSPECTOR_W + theme::GAP)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::none().inner_margin(egui::Margin { left: theme::GAP, right: theme::GAP, top: theme::GAP, bottom: 0.0 }))
            .show(ctx, |ui| {
                self.aside_rect = ui.max_rect();
                self.aside_islands.clear();
                let notes = self.doc.blocks.iter().filter(|b| !b.remark.trim().is_empty()).count();
                let tabs = [
                    ui::SegTab { label: TABS[0], icon: None, count: None, dot: false },
                    ui::SegTab { label: TABS[1], icon: None, count: (notes > 0).then_some(notes), dot: false },
                    ui::SegTab { label: TABS[2], icon: None, count: None, dot: self.doc.meta.revising() },
                    ui::SegTab { label: TABS[3], icon: None, count: None, dot: false },
                ];
                let top = ui.cursor().min;
                if let Some(k) = ui::seg_tabs(ui, &tabs, self.settings.inspector_tab.min(3), Some(ui.available_width())) {
                    if k != self.settings.inspector_tab {
                        self.settings.inspector_tab = k;
                        self.save_settings();
                    }
                }
                self.aside_islands.push(Rect::from_min_max(top, Pos2::new(ui.max_rect().right(), ui.cursor().min.y)));
                ui.add_space(10.0);
                let r = theme::island_frame(ncf::theme::R_LG)
                    .inner_margin(egui::Margin::symmetric(10.0, 12.0))
                    .show(ui, |ui| {
                        ui.set_min_size(ui.available_size());
                        let room = Rect::from_min_size(ui.cursor().min, ui.available_size());
                        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(room).layout(Layout::top_down(Align::Min)));
                        child.set_clip_rect(room.intersect(ui.clip_rect()));
                        match self.settings.inspector_tab.min(3) {
                            0 => self.scene_outliner(&mut child),
                            1 => {
                                self.left_tab = 0;
                                self.left_rows.clear();
                                self.notes_list(&mut child);
                            }
                            2 => {
                                self.left_tab = 1;
                                self.left_rows.clear();
                                self.changes_list(&mut child);
                            }
                            _ => self.cast_panel(&mut child),
                        }
                        ui.allocate_rect(room, Sense::hover());
                    });
                self.left_rect = r.response.rect;
                self.aside_islands.push(r.response.rect);
            });
    }

    // title page sheet here---
    pub(super) fn title_sheet(&mut self, ctx: &egui::Context) {
        if !self.title_sheet || self.home {
            return;
        }
        let screen = ctx.screen_rect();
        let veil = anim::ease(ctx, "ns-title-veil", true, 0.24);
        let mut close = false;
        egui::Area::new(egui::Id::new("ns-title-scrim"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .interactable(true)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(rect, egui::Rounding::same(crate::chrome::window_radius(ctx, theme::R_WINDOW)), theme::scrim(veil));
                if resp.clicked() && self.frame_no > self.popover_frame + 1 {
                    close = true;
                }
            });
        let size = Vec2::new((screen.width() - 80.0).clamp(360.0, 460.0), (screen.height() - 80.0).clamp(360.0, 640.0));
        let rect = Rect::from_center_size(screen.center(), size);
        egui::Area::new(egui::Id::new("ns-title-sheet"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                let (win, _) = ui.allocate_exact_size(rect.size(), Sense::click());
                self.title_sheet_rect = win;
                ncf::theme::surface(ui.painter(), win, ncf::theme::R_LG, ncf::theme::Lift::Float);
                let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(win.shrink2(Vec2::new(22.0, 18.0))).layout(Layout::top_down(Align::Min)));
                ui.horizontal(|ui| {
                    ui.label(ui::display_text("Title page", theme::T_H + 3.0, pal().text));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui::icon_button(ui, Icon::Close, &format!("Close · {}", self.keys_for(keys::Command::ToggleDetails)), 26.0) {
                            close = true;
                        }
                    });
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical().id_salt("ns-title-sheet-scroll").auto_shrink([false, false]).show(&mut ui, |ui| {
                    ui.set_max_width(ui.available_width() - 6.0);
                    self.details_panel(ui);
                });
            });
        if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.title_sheet = false;
        }
    }
}

fn big_primary(ui: &mut egui::Ui, label: &str, tip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
    let (fill, _, ink) = ui::button_colors(true, hot);
    ui.painter().rect_filled(rect, egui::Rounding::same(16.0), fill);
    let font = theme::font_semi(theme::T_BODY);
    let tw = theme::tracked_width(ui.ctx(), label, font.clone(), 0.0);
    let x = rect.center().x - (tw + 22.0) * 0.5;
    icons::draw(ui.painter(), Rect::from_center_size(Pos2::new(x + 7.0, rect.center().y), Vec2::splat(14.0)), Icon::Plus, ink);
    ui.painter().text(Pos2::new(x + 22.0, rect.center().y), egui::Align2::LEFT_CENTER, label, font, ink);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.on_hover_text(tip).clicked()
}
