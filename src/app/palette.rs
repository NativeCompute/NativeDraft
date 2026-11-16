use std::path::PathBuf;

use eframe::egui::{self, Align2, Key, Pos2, Rect, Sense, Vec2};
use web_time::Instant;

use super::{App, Mode};
use crate::anim;
use crate::editor::Caret;
use crate::icons::{self, Icon};
use crate::keys::Command;
use crate::theme::{self, pal};
use crate::ui;

pub struct Palette {
    pub query: String,
    pub sel: usize,
    born: Instant,
    focus: bool,
    back: Option<(u64, Caret)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pick {
    Script(PathBuf),
    Scene(u64),
    Command(Command),
    Line(PathBuf, usize),
}

#[derive(Clone, Debug)]
pub struct Row {
    pub section: &'static str,
    pub icon: Icon,
    pub title: String,
    pub sub: String,
    pub chord: String,
    pub pick: Pick,
}

const ROW_H: f32 = 46.0;
const MAX_ROWS: usize = 40;

pub fn score(hay: &str, needle: &str) -> Option<i32> {
    let hay = hay.to_lowercase();
    let needle = needle.trim().to_lowercase();
    if needle.is_empty() {
        return Some(0);
    }
    if let Some(at) = hay.find(&needle) {
        let word = at == 0 || !hay[..at].chars().last().map(|c| c.is_alphanumeric()).unwrap_or(false);
        return Some(1000 - at.min(500) as i32 + if word { 300 } else { 0 });
    }
    let mut it = hay.chars();
    let mut gaps = 0;
    for n in needle.chars().filter(|c| !c.is_whitespace()) {
        let mut found = false;
        for h in it.by_ref() {
            if h == n {
                found = true;
                break;
            }
            gaps += 1;
        }
        if !found {
            return None;
        }
    }
    Some(200 - gaps.min(190))
}

impl App {
    pub(super) fn open_palette(&mut self) {
        let back = self.ed.focus_block.filter(|_| !self.home && self.mode == Mode::Write).map(|id| (id, self.ed.caret.map(Caret::At).unwrap_or(Caret::End)));
        self.palette = Some(Palette { query: String::new(), sel: 0, born: Instant::now(), focus: true, back });
        self.popover = None;
        self.popover_frame = self.frame_no;
    }

    fn command_allowed(&self, c: Command) -> bool {
        if c == Command::Palette {
            return false;
        }
        if self.home {
            return matches!(c, Command::NewScript | Command::FindScripts | Command::Settings | Command::ToggleLibrary);
        }
        let write = self.can_write();
        match c {
            Command::NewScript | Command::NewScene | Command::NewAct | Command::DualDialogue | Command::AddNote | Command::Element(_) => write,
            _ => true,
        }
    }

    pub(super) fn palette_rows(&mut self, query: &str) -> Vec<Row> {
        let q = query.trim();
        let mut rows: Vec<(i32, Row)> = Vec::new();
        let mut entries = self.entries.clone();
        entries.sort_by_key(|e| std::cmp::Reverse(e.modified));
        for (k, e) in entries.iter().enumerate() {
            if q.is_empty() && k >= 5 {
                break;
            }
            if let Some(s) = score(&e.title, q) {
                let sub = format!("{} · {}", if e.pages == 1 { "1 page".to_string() } else { format!("{} pages", e.pages) }, super::relative_time(e.modified));
                rows.push((s + 40 - k as i32, Row { section: "Scripts", icon: Icon::File, title: e.title.clone(), sub, chord: String::new(), pick: Pick::Script(e.path.clone()) }));
            }
        }
        if !self.home && !q.is_empty() {
            for (i, sc) in self.doc.scenes().iter().enumerate() {
                let name = if sc.heading.trim().is_empty() { format!("Scene {}", i + 1) } else { sc.heading.clone() };
                if let Some(s) = score(&name, q) {
                    rows.push((s, Row { section: "Scenes", icon: Icon::List, title: name, sub: format!("Scene {}", i + 1), chord: String::new(), pick: Pick::Scene(sc.id) }));
                }
            }
        }
        let common = [Command::NewScript, Command::FindScripts, Command::QuickExport, Command::Reports, Command::FocusMode, Command::Settings];
        for c in Command::ALL {
            if !self.command_allowed(c) || (q.is_empty() && !common.contains(&c)) {
                continue;
            }
            let title = match c {
                Command::Element(_) => format!("Element: {}", c.label()),
                _ => c.label(),
            };
            if let Some(s) = score(&title, q) {
                rows.push((s - 20, Row { section: "Commands", icon: Icon::Sparkle, title, sub: c.group().to_string(), chord: self.keys_for(c), pick: Pick::Command(c) }));
            }
        }
        if q.chars().count() >= 2 {
            let found = self.found_for(&q.to_lowercase());
            let scene_rows: Vec<String> = rows.iter().filter(|r| r.1.section == "Scenes").map(|r| r.1.title.clone()).collect();
            for f in found.iter() {
                let here = !self.home && self.path.as_ref() == Some(&f.path);
                for h in f.hits.iter().filter(|h| !(here && scene_rows.iter().any(|t| t == h.line.trim()))).take(3) {
                    rows.push((-100, Row { section: "In the pages", icon: Icon::Search, title: h.line.trim().to_string(), sub: f.title.clone(), chord: String::new(), pick: Pick::Line(f.path.clone(), h.block) }));
                }
            }
        }
        if !q.is_empty() {
            let order = |s: &str| match s {
                "Scripts" => 0,
                "Scenes" => 1,
                "Commands" => 2,
                _ => 3,
            };
            let best = |s: &str| rows.iter().filter(|r| r.1.section == s).map(|r| r.0).max().unwrap_or(i32::MIN);
            let rank: Vec<(&'static str, i32)> = ["Scripts", "Scenes", "Commands", "In the pages"].iter().map(|s| (*s, best(s))).collect();
            let rank_of = |s: &str| rank.iter().find(|r| r.0 == s).map(|r| r.1).unwrap_or(i32::MIN);
            rows.sort_by(|a, b| rank_of(b.1.section).cmp(&rank_of(a.1.section)).then(order(a.1.section).cmp(&order(b.1.section))).then(b.0.cmp(&a.0)));
        }
        let mut out: Vec<Row> = Vec::new();
        for (_, r) in rows {
            let in_section = out.iter().filter(|x| x.section == r.section).count();
            if in_section < 8 && out.len() < MAX_ROWS {
                out.push(r);
            }
        }
        out
    }

    pub(super) fn close_palette(&mut self) {
        if let Some(back) = self.palette.take().and_then(|p| p.back) {
            if !self.home && self.mode == Mode::Write {
                self.ed.focus(back.0, back.1);
            }
        }
    }

    fn show_block(&mut self, id: u64) {
        if self.access.can_edit() {
            self.mode = Mode::Write;
            self.ed.focus_block = Some(id);
            self.ed.jump_to(id);
        } else {
            self.mode = Mode::Read;
            self.pages.show_block = Some(id);
        }
    }

    pub(super) fn run_pick(&mut self, pick: Pick) {
        let open_here = |app: &Self, path: &PathBuf| !app.home && app.path.as_ref() == Some(path);
        match pick {
            Pick::Script(path) => {
                self.palette = None;
                if open_here(self, &path) {
                    if self.access.can_edit() {
                        self.mode = Mode::Write;
                    }
                } else {
                    self.open_from_home(path);
                }
            }
            Pick::Scene(id) => {
                self.palette = None;
                self.show_block(id);
            }
            Pick::Command(c) => {
                self.close_palette();
                self.run(c);
            }
            Pick::Line(path, k) => {
                self.palette = None;
                if open_here(self, &path) {
                    if let Some(id) = self.doc.blocks.get(k).map(|b| b.id) {
                        self.show_block(id);
                    }
                } else {
                    self.pending_jump = Some((path.clone(), k));
                    self.open_from_home(path);
                }
            }
        }
    }

    pub(super) fn palette_keys(&mut self, ctx: &egui::Context) {
        for (_, chord) in self.settings.keys.listen(&[Command::Palette], crate::WEB) {
            if ctx.input_mut(|i| i.consume_shortcut(&chord)) {
                self.close_palette();
                return;
            }
        }
    }

    // palette here---
    pub(super) fn palette_overlay(&mut self, ctx: &egui::Context) {
        let Some((query, sel, born, focus)) = self.palette.as_ref().map(|p| (p.query.clone(), p.sel, p.born, p.focus)) else {
            return;
        };
        let rows = self.palette_rows(&query);
        let p = pal();
        let screen = ctx.screen_rect();
        let t = if anim::enabled() { (born.elapsed().as_secs_f32() / 0.18).clamp(0.0, 1.0) } else { 1.0 };
        if t < 1.0 {
            ctx.request_repaint();
        }
        let (down, up, enter, esc) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, Key::Enter),
                i.consume_key(egui::Modifiers::NONE, Key::Escape),
            )
        });
        let n = rows.len();
        let mut sel = if n == 0 { 0 } else { sel.min(n - 1) };
        if down && n > 0 {
            sel = (sel + 1) % n;
        }
        if up && n > 0 {
            sel = (sel + n - 1) % n;
        }
        let mut picked: Option<Pick> = if enter { rows.get(sel).map(|r| r.pick.clone()) } else { None };
        let mut close = esc;
        let mut query_now = query.clone();

        egui::Area::new(egui::Id::new("ns-palette-scrim"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .interactable(true)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(rect, egui::Rounding::same(crate::chrome::window_radius(ctx, theme::R_WINDOW)), theme::scrim(0.7 * t));
                if resp.clicked() && self.frame_no > self.popover_frame + 1 {
                    close = true;
                }
            });
        let w = (screen.width() - 40.0).min(600.0);
        let list_h = (n as f32 * ROW_H + rows.iter().map(|r| r.section).collect::<std::collections::BTreeSet<_>>().len() as f32 * 26.0).min(screen.height() - 220.0).max(60.0);
        let rect = Rect::from_min_size(Pos2::new(screen.center().x - w * 0.5, screen.top() + 64.0 - (1.0 - t) * 8.0), Vec2::new(w, 64.0 + list_h + 16.0));
        egui::Area::new(egui::Id::new("ns-palette"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.set_opacity(0.3 + 0.7 * t);
                let (win, _) = ui.allocate_exact_size(rect.size(), Sense::click());
                self.palette_rect = win;
                ncf::theme::surface(ui.painter(), win, ncf::theme::R_LG, ncf::theme::Lift::Lift);
                let field = Rect::from_min_size(win.min + Vec2::new(16.0, 14.0), Vec2::new(win.width() - 32.0, 40.0));
                ui.painter().rect_filled(field, egui::Rounding::same(20.0), p.raised);
                icons::draw(ui.painter(), Rect::from_center_size(Pos2::new(field.left() + 20.0, field.center().y), Vec2::splat(16.0)), Icon::Search, p.text_dim);
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(Pos2::new(field.left() + 36.0, field.top()), Pos2::new(field.right() - 12.0, field.bottom()))).layout(egui::Layout::left_to_right(egui::Align::Center)));
                let edit = child.add(
                    egui::TextEdit::singleline(&mut query_now)
                        .id(egui::Id::new("ns-palette-input"))
                        .frame(false)
                        .font(theme::font(theme::T_BODY + 1.0))
                        .desired_width(f32::INFINITY)
                        .hint_text(if self.home { "Search scripts, lines and commands" } else { "Search scripts, scenes, lines and commands" }),
                );
                if focus {
                    edit.request_focus();
                }
                let list = Rect::from_min_max(Pos2::new(win.left() + 8.0, field.bottom() + 10.0), Pos2::new(win.right() - 8.0, win.bottom() - 8.0));
                let mut lu = ui.new_child(egui::UiBuilder::new().max_rect(list).layout(egui::Layout::top_down(egui::Align::Min)));
                lu.set_clip_rect(list);
                egui::ScrollArea::vertical().id_salt("ns-palette-list").auto_shrink([false, false]).show(&mut lu, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    if rows.is_empty() {
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            ui.add_space(14.0);
                            ui.label(egui::RichText::new("Nothing matches.").font(theme::font(theme::T_SM)).color(p.text_faint));
                        });
                    }
                    let mut last = "";
                    self.palette_rows_drawn.clear();
                    for (k, r) in rows.iter().enumerate() {
                        if r.section != last {
                            last = r.section;
                            let (head, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), Sense::hover());
                            ui::eyebrow_text(ui.painter(), Pos2::new(head.left() + 12.0, head.center().y + 2.0), r.section, p.text_faint);
                        }
                        let (row, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H), Sense::click());
                        self.palette_rows_drawn.push(row);
                        let on = k == sel;
                        let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
                        if on || hot > 0.01 {
                            ui.painter().rect_filled(row, egui::Rounding::same(12.0), theme::wash(p.raised, if on { 255 } else { (255.0 * hot) as u8 }));
                        }
                        if on {
                            resp.scroll_to_me(None);
                        }
                        let chip = Rect::from_center_size(Pos2::new(row.left() + 26.0, row.center().y), Vec2::splat(30.0));
                        ui.painter().rect_filled(chip, egui::Rounding::same(9.0), if on { p.primary } else { p.sunken });
                        icons::draw(ui.painter(), chip.shrink(8.0), r.icon, if on { p.ink } else { p.text_dim });
                        let chord_w = if r.chord.is_empty() { 0.0 } else { theme::tracked_width(ui.ctx(), &r.chord, theme::font_hud(theme::T_MICRO), 0.0) + 24.0 };
                        let room = row.right() - chip.right() - 16.0 - chord_w;
                        let tf = theme::font_semi(theme::T_SM);
                        let clip = ui.painter().with_clip_rect(row);
                        clip.text(Pos2::new(chip.right() + 12.0, row.center().y - 8.0), Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &r.title, tf.clone(), 0.0, room), tf, p.text);
                        let sf = theme::font(theme::T_CAP);
                        clip.text(Pos2::new(chip.right() + 12.0, row.center().y + 10.0), Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &r.sub, sf.clone(), 0.0, room), sf, p.text_faint);
                        if !r.chord.is_empty() {
                            let kf = theme::font_hud(theme::T_MICRO);
                            let kw = chord_w - 12.0;
                            let k = Rect::from_center_size(Pos2::new(row.right() - 12.0 - kw * 0.5, row.center().y), Vec2::new(kw, 22.0));
                            clip.rect_filled(k, egui::Rounding::same(7.0), p.sunken);
                            clip.text(k.center(), Align2::CENTER_CENTER, &r.chord, kf, p.text_dim);
                        }
                        if resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if resp.clicked() {
                            picked = Some(r.pick.clone());
                        }
                    }
                });
            });

        if let Some(pal_state) = self.palette.as_mut() {
            if pal_state.query != query_now {
                pal_state.sel = 0;
            } else {
                pal_state.sel = sel;
            }
            pal_state.query = query_now;
            pal_state.focus = false;
        }
        if close {
            self.close_palette();
        } else if let Some(pick) = picked {
            self.run_pick(pick);
        }
    }
}
