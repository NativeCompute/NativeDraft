use std::path::{Path, PathBuf};

use eframe::egui::{self, Align, Layout, Pos2, Rect, Sense, Stroke, Vec2};

use super::{App, Ask, Mode, Popover};
use crate::alerts::Tone;
use crate::anim;
use crate::editor::Caret;
use crate::icons::{self, Icon};
use crate::keys;
use crate::model::{Document, Element};
use crate::storage;
use crate::theme::{self, pal};
use crate::timelines::{self, Bundle, Tl};
use crate::timelines_view::{self as tv, Act, RowInfo};
use crate::ui;

pub struct Side {
    pub at: usize,
    pub files: Vec<(PathBuf, Option<String>, Option<String>)>,
}

pub struct BranchDialog {
    pub parent: Tl,
    pub from: Option<(Tl, usize, Option<String>)>,
    pub heading: String,
    pub name: String,
    pub choice: String,
    pub stay: String,
    pub focus: bool,
    pub opened: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuKind {
    Dot(usize, usize),
    Chip(usize),
}

pub struct Menu {
    pub kind: MenuKind,
    pub pos: Pos2,
    pub opened: u64,
}

type Todo = Option<Box<dyn FnOnce(&mut App)>>;

pub const COMING: &str = "Timelines are coming to NativeCloud";

fn read(path: &Path) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string(path).ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        None
    }
}

fn write(path: &Path, text: Option<&str>) -> Result<(), String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match text {
            Some(t) => {
                if let Some(dir) = path.parent() {
                    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                }
                let tmp = path.with_extension("md.tmp");
                std::fs::write(&tmp, t).map_err(|e| e.to_string())?;
                std::fs::rename(&tmp, path).map_err(|e| e.to_string())
            }
            None => match std::fs::remove_file(path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
                _ => Ok(()),
            },
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, text);
        Err(COMING.to_string())
    }
}

pub fn color_of(b: &Bundle, t: Tl) -> egui::Color32 {
    let p = pal();
    match b.color(t) {
        None => p.primary,
        Some(c) => p.groups[c % p.groups.len()],
    }
}

impl App {
    // bundle here---

    pub(super) fn timelines_ok(&self) -> bool {
        if cfg!(target_arch = "wasm32") {
            return false;
        }
        let Some(path) = &self.path else { return false };
        self.store.place(path).is_none() && self.store.team().is_none() && path.exists()
    }

    pub(super) fn has_timelines(&self) -> bool {
        self.bundle.as_ref().is_some_and(|b| !b.is_empty())
    }

    pub(super) fn load_bundle(&mut self) {
        self.tgraph = None;
        self.tl = None;
        self.bundle = None;
        self.tl_missing.clear();
        self.tl_whole = Some(self.doc.clone());
        self.tl_menu = None;
        #[cfg(not(target_arch = "wasm32"))]
        {
            if !self.timelines_ok() {
                return;
            }
            let Some(path) = self.path.clone() else { return };
            if let Ok((mut b, t)) = timelines::bundle_of(&path) {
                *b.doc_mut(t) = self.doc.clone();
                self.tl = t;
                self.bundle = Some(b);
            }
        }
    }

    fn reload_bundle(&mut self) {
        let doc = self.doc.clone();
        let whole = self.tl_whole.clone();
        self.load_bundle();
        self.doc = doc;
        if let Some(b) = self.bundle.as_mut() {
            *b.doc_mut(self.tl) = self.doc.clone();
        }
        self.tl_whole = whole;
    }

    pub(super) fn live(&self) -> Option<Bundle> {
        let mut b = self.bundle.clone()?;
        *b.doc_mut(self.tl) = self.doc.clone();
        Some(b)
    }

    pub(super) fn after_save_bundle(&mut self) {
        let doc = self.doc.clone();
        let tl = self.tl;
        if let Some(b) = self.bundle.as_mut() {
            *b.doc_mut(tl) = doc;
        }
    }

    pub(super) fn script_path(&self) -> Option<PathBuf> {
        match (&self.bundle, self.tl) {
            (Some(b), Some(_)) => Some(b.main_path.clone()),
            _ => self.path.clone(),
        }
    }

    pub(super) fn main_title(&self) -> Option<String> {
        match (&self.bundle, self.tl) {
            (Some(b), Some(_)) => Some(b.main.meta.title.clone()),
            _ => None,
        }
    }

    pub(super) fn timeline_name(&self) -> String {
        self.bundle.as_ref().map(|b| b.name(self.tl)).unwrap_or_else(|| "Main".into())
    }

    pub(super) fn timeline_graph(&mut self) -> Option<(timelines::Graph, Vec<timelines::Issue>)> {
        if self.tgraph.is_none() {
            let b = self.live()?;
            self.tgraph = Some((timelines::graph(&b), timelines::issues(&b)));
        }
        self.tgraph.clone()
    }

    pub(super) fn refresh_timelines(&mut self) {
        self.tgraph = None;
        let Some(b) = self.live() else {
            self.tl_whole = Some(self.doc.clone());
            return;
        };
        let Some(whole) = self.tl_whole.clone() else {
            self.tl_whole = Some(self.doc.clone());
            return;
        };
        let mut lost = Vec::new();
        for blk in &whole.blocks {
            if blk.element != Element::SceneHeading || blk.link.is_some() {
                continue;
            }
            let Some(sid) = &blk.sid else { continue };
            if self.doc.scene_by_sid(sid).is_some() {
                continue;
            }
            if !timelines::refs_to(&b, sid).is_empty() {
                lost.push(sid.clone());
            }
        }
        if lost.is_empty() {
            self.tl_whole = Some(self.doc.clone());
        }
        self.tl_missing = lost;
    }

    // commit here---

    pub(super) fn commit(&mut self, new: Bundle, changed: Vec<Tl>, removed: Vec<PathBuf>) -> bool {
        let Some(open) = self.path.clone() else { return false };
        let mut files: Vec<(PathBuf, Option<String>, Option<String>)> = Vec::new();
        let mut open_doc: Option<Document> = None;
        for t in changed {
            let path = new.path(t).to_path_buf();
            if path == open {
                open_doc = Some(new.doc(t).clone());
                continue;
            }
            if files.iter().any(|f| f.0 == path) {
                continue;
            }
            let after = storage::to_markdown(new.doc(t));
            let before = read(&path);
            if before.as_deref() != Some(after.as_str()) {
                files.push((path, before, Some(after)));
            }
        }
        for path in removed {
            if path == open {
                continue;
            }
            let before = read(&path);
            if before.is_some() {
                files.push((path, before, None));
            }
        }
        let mut failed = None;
        for (path, _, after) in &files {
            if let Err(e) = write(path, after.as_deref()) {
                failed = Some(e);
                break;
            }
        }
        if let Some(e) = failed {
            for (path, before, _) in &files {
                let _ = write(path, before.as_deref());
            }
            self.deck.say("Could not change the timelines", &e, Tone::Danger);
            self.reload_bundle();
            return false;
        }
        self.checkpoint();
        if let Some(doc) = open_doc.filter(|d| *d != self.doc) {
            self.doc = doc;
            self.mark_changed();
            self.snapshot_due = false;
            self.baseline = self.doc.clone();
            self.ed.fresh = true;
        }
        if !files.is_empty() {
            self.sides.push(Side { at: self.undo.len(), files });
        }
        self.redo_sides.clear();
        let doc = self.doc.clone();
        self.tl = new.find_path(&open).unwrap_or(None);
        self.bundle = Some(new);
        if let Some(b) = self.bundle.as_mut() {
            *b.doc_mut(self.tl) = doc;
        }
        if self.dirty {
            self.save(false);
        }
        self.tgraph = None;
        self.tl_whole = Some(self.doc.clone());
        self.tl_missing.clear();
        self.layout_stale = true;
        true
    }

    pub(super) fn undo_sides(&mut self) {
        let mut touched = false;
        while self.sides.last().is_some_and(|s| s.at > self.undo.len()) {
            let Some(s) = self.sides.pop() else { break };
            for (path, before, _) in s.files.iter().rev() {
                let _ = write(path, before.as_deref());
            }
            self.redo_sides.push(s);
            touched = true;
        }
        if touched {
            self.reload_bundle();
            self.tl_whole = Some(self.doc.clone());
            self.tl_missing.clear();
        }
    }

    pub(super) fn redo_sides(&mut self) {
        let mut touched = false;
        while self.redo_sides.last().is_some_and(|s| s.at <= self.undo.len()) {
            let Some(s) = self.redo_sides.pop() else { break };
            for (path, _, after) in &s.files {
                let _ = write(path, after.as_deref());
            }
            self.sides.push(s);
            touched = true;
        }
        if touched {
            self.reload_bundle();
        }
    }

    pub(super) fn trim_sides(&mut self) {
        for s in &mut self.sides {
            s.at = s.at.saturating_sub(1);
        }
        self.sides.retain(|s| s.at > 0);
    }

    // actions here---

    pub(super) fn open_timeline(&mut self, t: Tl, at: Option<usize>, write: bool) {
        let Some(b) = &self.bundle else { return };
        if t.is_some_and(|i| i >= b.items.len()) {
            return;
        }
        let path = b.path(t).to_path_buf();
        if write && self.access.can_edit() {
            self.mode = Mode::Write;
        }
        if self.path.as_ref() == Some(&path) {
            if let Some(k) = at {
                if let Some(id) = self.doc.blocks.get(k).map(|b| b.id) {
                    self.ed.focus_block = Some(id);
                    self.ed.jump_to(id);
                }
            }
            return;
        }
        if let Some(k) = at {
            self.pending_jump = Some((path.clone(), k));
        }
        self.open(path);
    }

    pub(super) fn ask_branch_here(&mut self) {
        if !self.timelines_ok() {
            self.deck.say(COMING, "For now, branch a script that lives on this computer.", Tone::Info);
            return;
        }
        if !self.guard_edit() {
            return;
        }
        let scenes = self.doc.scenes();
        let k = self.ed.focus_block.and_then(|id| self.doc.scene_of(id));
        let from = k.and_then(|k| scenes.get(k)).map(|sc| {
            let h = sc.start;
            let sid = self.doc.blocks[h].link.clone().or_else(|| self.doc.blocks[h].sid.clone());
            (self.tl, h, sid)
        });
        let heading = k.and_then(|k| scenes.get(k)).map(|s| s.heading.clone()).unwrap_or_default();
        self.ask_branch(self.tl, from, heading);
    }

    pub(super) fn ask_branch(&mut self, parent: Tl, from: Option<(Tl, usize, Option<String>)>, heading: String) {
        self.branch_dialog = Some(BranchDialog {
            parent,
            from,
            heading,
            name: String::new(),
            choice: String::new(),
            stay: String::new(),
            focus: true,
            opened: self.frame_no,
        });
        self.tl_menu = None;
        self.popover = None;
    }

    fn create_branch(&mut self) {
        let Some(d) = self.branch_dialog.take() else { return };
        let Some(mut b) = self.live() else { return };
        let name = if d.name.trim().is_empty() { "New timeline".to_string() } else { d.name.trim().to_string() };
        let mut changed = Vec::new();
        let sid = match d.from {
            None => None,
            Some((_, _, Some(s))) => Some(s),
            Some((t, h, None)) => {
                changed.push(t);
                Some(b.ensure_sid(t, h))
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        let path = storage::unique_timeline_path(&b.main_path, &name, &b.items.iter().map(|t| t.path.clone()).collect::<Vec<_>>());
        #[cfg(target_arch = "wasm32")]
        let path = storage::timelines_dir(&b.main_path).join("timeline.md");
        let i = b.add_branch(path, d.parent, sid, &name, &d.choice, &d.stay);
        changed.push(Some(i));
        if self.commit(b, changed, Vec::new()) {
            let from = if d.heading.trim().is_empty() { "the start".to_string() } else { d.heading.trim().to_string() };
            self.deck.ok("Timeline created", &format!("{name} branches from {from}"));
            self.open_timeline(Some(i), None, true);
            if let Some(first) = self.doc.blocks.first().map(|b| b.id) {
                self.ed.focus(first, Caret::Start);
            }
        }
    }

    fn timeline_by_id(&self, id: &str) -> Option<usize> {
        self.bundle.as_ref()?.by_id(id)
    }

    pub(super) fn rename_timeline(&mut self, id: &str, name: &str) {
        let (Some(mut b), Some(i)) = (self.live(), self.timeline_by_id(id)) else { return };
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        b.items[i].meta_mut().name = name.to_string();
        b.items[i].doc.meta.title = name.to_string();
        if self.commit(b, vec![Some(i)], Vec::new()) {
            self.deck.ok("Renamed", name);
        }
    }

    pub(super) fn set_choice(&mut self, id: &str, choice: &str) {
        let (Some(mut b), Some(i)) = (self.live(), self.timeline_by_id(id)) else { return };
        b.items[i].meta_mut().choice = choice.trim().to_string();
        self.commit(b, vec![Some(i)], Vec::new());
    }

    fn set_color(&mut self, i: usize, c: usize) {
        let Some(mut b) = self.live() else { return };
        b.items[i].meta_mut().color = Some(c % timelines::COLORS);
        self.commit(b, vec![Some(i)], Vec::new());
    }

    fn duplicate_timeline(&mut self, i: usize) {
        let Some(mut b) = self.live() else { return };
        let mut copy = b.items[i].clone();
        let name = format!("{} (copy)", copy.meta().name);
        #[cfg(not(target_arch = "wasm32"))]
        {
            copy.path = storage::unique_timeline_path(&b.main_path, &name, &b.items.iter().map(|t| t.path.clone()).collect::<Vec<_>>());
        }
        let id = b.new_timeline_id();
        let color = b.next_color();
        let m = copy.meta_mut();
        m.id = id;
        m.name = name.clone();
        m.color = Some(color);
        copy.doc.meta.title = name.clone();
        for blk in &mut copy.doc.blocks {
            blk.sid = None;
        }
        b.items.push(copy);
        let n = b.items.len() - 1;
        if self.commit(b, vec![Some(n)], Vec::new()) {
            self.deck.ok("Timeline duplicated", &name);
        }
    }

    pub(super) fn ask_delete_timeline(&mut self, i: usize) {
        let Some(b) = &self.bundle else { return };
        let name = b.name(Some(i));
        let subs: Vec<String> = b.descendants(Some(i)).into_iter().map(|k| b.name(Some(k))).collect();
        let body = if subs.is_empty() {
            format!("“{name}” and its scenes. A snapshot of it is kept first.")
        } else {
            format!("“{name}” and the timelines that branch from it: {}. A snapshot of each is kept first.", subs.join(", "))
        };
        let id = b.items[i].meta().id.clone();
        self.deck.ask("Delete this timeline?", &body, "Delete", Tone::Danger, Ask::DeleteTimeline(id));
    }

    pub(super) fn delete_timeline(&mut self, id: &str) {
        let (Some(mut b), Some(i)) = (self.live(), self.timeline_by_id(id)) else { return };
        let mut gone = b.descendants(Some(i));
        gone.push(i);
        gone.sort();
        gone.dedup();
        let name = b.name(Some(i));
        let mut removed = Vec::new();
        for &k in gone.iter().rev() {
            let t = b.items.remove(k);
            let _ = self.store.take_snapshot(&t.path, &t.doc, Some("Before the timeline was deleted"));
            removed.push(t.path);
        }
        let open_gone = self.path.as_ref().is_some_and(|p| removed.contains(p));
        if open_gone {
            let main = b.main_path.clone();
            self.dirty = false;
            for p in &removed {
                let _ = write(p, None);
            }
            self.pending_jump = None;
            self.open(main);
            self.deck.say("Timeline deleted", &name, Tone::Warn);
            return;
        }
        if self.commit(b, Vec::new(), removed) {
            let undo = self.keys_for(keys::Command::Undo);
            self.deck.say("Timeline deleted", &format!("{name} · {undo} brings it back"), Tone::Warn);
        }
    }

    fn merge_into(&mut self, row: usize, dot: usize) {
        let Some((g, _)) = self.timeline_graph() else { return };
        let Some(open) = self.tl else { return };
        let (Some(mut b), Some(r)) = (self.live(), g.rows.get(row)) else { return };
        let Some(d) = r.dots.get(dot) else { return };
        let mut changed = vec![Some(open)];
        let sid = match (&d.sid, d.same, d.head) {
            (Some(s), Some(_), _) => s.clone(),
            (_, None, Some(h)) => {
                changed.push(r.tl);
                b.ensure_sid(r.tl, h)
            }
            (Some(s), _, _) => s.clone(),
            _ => return,
        };
        b.items[open].meta_mut().merges_into = Some(sid);
        if self.commit(b, changed, Vec::new()) {
            self.deck.ok("Merged", &format!("{} now flows back into {}", self.timeline_name(), d.heading.trim()));
        }
    }

    fn remove_merge(&mut self, i: usize) {
        let Some(mut b) = self.live() else { return };
        b.items[i].meta_mut().merges_into = None;
        self.commit(b, vec![Some(i)], Vec::new());
    }

    fn make_own(&mut self, row: usize, dot: usize) {
        let Some((g, _)) = self.timeline_graph() else { return };
        let Some(r) = g.rows.get(row) else { return };
        let Some(h) = r.dots.get(dot).and_then(|d| d.head) else { return };
        let Some(mut b) = self.live() else { return };
        if timelines::make_own(&mut b, r.tl, h) && self.commit(b, vec![r.tl], Vec::new()) {
            self.deck.ok("It is its own scene now", "Edit it here; the original stays as it was.");
        }
    }

    fn use_as_same(&mut self, row: usize, dot: usize) {
        let Some((g, _)) = self.timeline_graph() else { return };
        let Some(r) = g.rows.get(row) else { return };
        let Some(d) = r.dots.get(dot) else { return };
        let Some(mut b) = self.live() else { return };
        let mut changed = vec![self.tl];
        let sid = match (&d.sid, d.same, d.head) {
            (Some(s), Some(_), _) => s.clone(),
            (_, None, Some(h)) => {
                changed.push(r.tl);
                b.ensure_sid(r.tl, h)
            }
            _ => return,
        };
        let doc = b.doc_mut(self.tl);
        let at = self
            .ed
            .focus_block
            .and_then(|id| doc.scene_of(id))
            .and_then(|k| doc.scenes().get(k).map(|s| s.end))
            .unwrap_or(doc.blocks.len());
        let mut blk = doc.new_block(Element::SceneHeading, "");
        blk.link = Some(sid);
        doc.blocks.insert(at.min(doc.blocks.len()), blk);
        if self.commit(b, changed, Vec::new()) {
            self.deck.ok("Same scene added", &format!("{} now shows {}", self.timeline_name(), d.heading.trim()));
        }
    }

    pub(super) fn delete_scene_with_refs(&mut self, n: usize) -> bool {
        let Some(mut b) = self.live() else { return false };
        let Some(sc) = self.doc.scenes().get(n).cloned() else { return false };
        let Some(sid) = sc.sid.clone().filter(|_| sc.link.is_none()) else { return false };
        let refs = timelines::refs_to(&b, &sid);
        if refs.is_empty() {
            return false;
        }
        let mut changed = timelines::reanchor_for_delete(&mut b, self.tl, &sid);
        if !b.doc_mut(self.tl).remove_scene(n) {
            return false;
        }
        changed.push(self.tl);
        if self.commit(b, changed, Vec::new()) {
            let undo = self.keys_for(keys::Command::Undo);
            let heading = if sc.heading.trim().is_empty() { "Untitled scene".to_string() } else { sc.heading.trim().to_string() };
            self.deck.say("Scene deleted", &format!("{heading} · the timelines that used it were re-anchored · {undo} brings it all back"), Tone::Warn);
        }
        true
    }

    pub(super) fn scene_refs_text(&self, n: usize) -> Option<String> {
        let b = self.live()?;
        let sc = self.doc.scenes().get(n).cloned()?;
        let sid = sc.sid.filter(|_| sc.link.is_none())?;
        let refs = timelines::refs_to(&b, &sid);
        (!refs.is_empty()).then(|| refs.describe())
    }

    pub(super) fn reanchor_lost(&mut self) {
        let (Some(mut b), Some(whole)) = (self.live(), self.tl_whole.clone()) else { return };
        let mut old = b.clone();
        *old.doc_mut(self.tl) = whole;
        let mut changed: Vec<Tl> = Vec::new();
        for sid in self.tl_missing.clone() {
            for t in timelines::reanchor_for_delete(&mut old, self.tl, &sid) {
                if !changed.contains(&t) {
                    changed.push(t);
                }
            }
        }
        for t in &changed {
            if *t == self.tl {
                let fresh: Vec<(u64, String)> = old.doc(self.tl).blocks.iter().filter_map(|x| x.sid.clone().map(|s| (x.id, s))).collect();
                let doc = b.doc_mut(self.tl);
                for (id, s) in fresh {
                    if let Some(k) = doc.index_of(id) {
                        if doc.blocks[k].sid.is_none() && doc.blocks[k].element == Element::SceneHeading {
                            doc.blocks[k].sid = Some(s);
                        }
                    }
                }
            } else {
                *b.doc_mut(*t) = old.doc(*t).clone();
            }
        }
        if self.commit(b, changed, Vec::new()) {
            self.deck.ok("Re-anchored", "The timelines that used that scene now hold on to its neighbours.");
        }
    }

    // lost strip here---

    pub(super) fn lost_strip(&mut self, ui: &mut egui::Ui, avail: f32, pad: f32) {
        if self.tl_missing.is_empty() {
            return;
        }
        let p = pal();
        let n = self.tl_missing.len();
        let (band, _) = ui.allocate_exact_size(Vec2::new(avail, 52.0), Sense::hover());
        let r = Rect::from_min_max(Pos2::new(band.left() + pad, band.top() + 6.0), Pos2::new(band.right() - pad, band.bottom() - 4.0));
        ui.painter().rect_filled(r, egui::Rounding::same(14.0), theme::wash(p.warn, if p.dark { 40 } else { 34 }));
        ui.painter().rect_stroke(r, egui::Rounding::same(14.0), Stroke::new(1.0_f32, theme::wash(p.warn, 140)));
        icons::draw(ui.painter(), Rect::from_center_size(Pos2::new(r.left() + 20.0, r.center().y), Vec2::splat(15.0)), Icon::Warning, p.warn);
        let text = if n == 1 { "Another timeline used the scene you just removed." } else { "Other timelines used the scenes you just removed." };
        let mut row = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(Vec2::new(40.0, 4.0))).layout(Layout::left_to_right(Align::Center)));
        row.label(egui::RichText::new(text).font(theme::font(theme::T_SM)).color(p.text));
        row.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui::button(ui, "Re-anchor", None, true) {
                self.reanchor_lost();
            }
            if ui::button(ui, "Undo", None, false) {
                self.undo();
            }
        });
    }

    // sidebar here---

    pub(super) fn side_timelines(&mut self, ui: &mut egui::Ui) {
        if !self.has_timelines() || self.home {
            return;
        }
        let Some(b) = self.bundle.clone() else { return };
        let p = pal();
        let mut pick = None;
        for t in b.order() {
            let depth = b.depth(t) as f32;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
            let on = t == self.tl;
            let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
            if on {
                ui.painter().rect_filled(rect, egui::Rounding::same(10.0), theme::wash(p.raised, 230));
            } else if hot > 0.01 {
                ui.painter().rect_filled(rect, egui::Rounding::same(10.0), theme::wash(p.raised, (150.0 * hot) as u8));
            }
            let x = rect.left() + 22.0 + depth.min(4.0) * 10.0;
            ui.painter().circle_filled(Pos2::new(x, rect.center().y), 4.5, color_of(&b, t));
            let f = if on { theme::font_semi(theme::T_SM) } else { theme::font(theme::T_SM) };
            let room = rect.right() - x - 14.0;
            ui.painter().text(
                Pos2::new(x + 12.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                ui::elide_to_width(ui.ctx(), &b.name(t), f.clone(), 0.0, room),
                f,
                if on { p.text } else { p.text_dim },
            );
            if resp.on_hover_text(format!("Open the {} timeline", b.name(t))).clicked() && !on {
                pick = Some(t);
            }
        }
        if let Some(t) = pick {
            self.open_timeline(t, None, false);
        }
    }

    pub(super) fn toolbar_timeline(&mut self, ui: &mut egui::Ui) {
        if !self.has_timelines() {
            return;
        }
        let name = self.timeline_name();
        let resp = ui::select_button(ui, "Timeline", &name);
        self.timeline_button_rect = resp.rect;
        if resp.on_hover_text("Switch timeline").clicked() {
            self.toggle_popover(Popover::Timelines);
        }
    }

    pub(super) fn timelines_popover(&mut self, ctx: &egui::Context) {
        let Some(b) = self.bundle.clone() else {
            self.popover = None;
            return;
        };
        let anchor = self.timeline_button_rect;
        let mut pick = None;
        let names: Vec<(Tl, String)> = b.order().into_iter().map(|t| (t, b.name(t))).collect();
        let area = egui::Area::new(egui::Id::new("ns-timelines-menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(anchor.left(), anchor.bottom() + 8.0))
            .show(ctx, |ui| {
                ui::popover_frame().show(ui, |ui| {
                    let labels: Vec<&str> = names.iter().map(|(_, n)| n.as_str()).collect();
                    ui.set_width(ui::menu_width(ui, &labels).max(200.0));
                    let mut rects = Vec::new();
                    for (t, n) in &names {
                        let label = format!("{}{n}", "   ".repeat(b.depth(*t).min(4)));
                        if ui::menu_item(ui, &mut rects, &label, if *t == self.tl { Icon::Check } else { Icon::Graph }, false) {
                            pick = Some(*t);
                        }
                    }
                    ui::separator(ui);
                    if ui::menu_item(ui, &mut rects, "See them all", Icon::Graph, false) {
                        self.mode = Mode::Timelines;
                        self.popover = None;
                    }
                });
            })
            .response;
        if let Some(t) = pick {
            self.popover = None;
            self.open_timeline(t, None, false);
            return;
        }
        self.close_popover_if_outside(ctx, &[area.rect, anchor]);
    }

    // page here---

    pub(super) fn timelines_tools_right(&mut self, ui: &mut egui::Ui) {
        if !self.timelines_ok() {
            return;
        }
        let sel = self.tview.sel;
        let tip = format!("A new timeline that branches from the chosen scene · {}", self.keys_for(keys::Command::NewTimeline));
        let can = sel.is_some() && self.can_write();
        let resp = ui.add_enabled_ui(can, |ui| ui::button_response(ui, "Branch from scene", Some(Icon::Plus), true, None)).inner;
        if resp.on_hover_text(tip).clicked() {
            if let Some((r, d)) = sel {
                self.branch_from_dot(r, d);
            }
        }
    }

    pub(super) fn branch_from_dot(&mut self, r: usize, d: usize) {
        if !self.guard_edit() {
            return;
        }
        let Some((g, _)) = self.timeline_graph() else { return };
        let Some(row) = g.rows.get(r) else { return };
        let Some(dot) = row.dots.get(d) else { return };
        let from = match (dot.same, dot.head) {
            (Some(_), _) => dot.sid.clone().map(|s| (row.tl, dot.head.unwrap_or(0), Some(s))),
            (None, Some(h)) => Some((row.tl, h, dot.sid.clone())),
            _ => None,
        };
        if from.is_none() {
            return;
        }
        self.ask_branch(row.tl, from, dot.heading.clone());
    }

    pub(super) fn timelines_page(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        if !self.timelines_ok() {
            let body = if cfg!(target_arch = "wasm32") {
                "Branching storylines live in the desktop app for now."
            } else {
                "Branching storylines work on scripts that live on this computer. Move a copy here to branch it."
            };
            ui.add_space(ui.available_height() * 0.22);
            ui::empty_state(ui, Icon::Graph, COMING, body, None);
            return;
        }
        let Some((g, issues)) = self.timeline_graph() else { return };
        let Some(b) = self.live() else { return };
        let inner = ui.max_rect().shrink(theme::STAGE_PAD);
        let mut stage = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(Layout::top_down(Align::Min)));
        let ui = &mut stage;

        let endings = timelines::endings(&g).len();
        let paths = timelines::paths(&g);
        let n = b.items.len() + 1;
        let one = |n: u64, a: &str, b: &str| if n == 1 { format!("1 {a}") } else { format!("{n} {b}") };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.add_space(6.0);
            ui.label(egui::RichText::new(self.main_title().unwrap_or_else(|| self.title_or_untitled())).font(theme::font_semi(theme::T_H)).color(p.text));
            ui.add_space(6.0);
            ui::stat_chip(ui, &n.to_string(), if n == 1 { "timeline" } else { "timelines" });
            ui::stat_chip(ui, &endings.to_string(), if endings == 1 { "ending" } else { "endings" });
            ui::stat_chip(ui, &paths.to_string(), if paths == 1 { "path" } else { "paths" });
            if !issues.is_empty() {
                let text = one(issues.len() as u64, "thing to look at", "things to look at");
                let tip = issues.iter().map(|i| i.text(&b)).collect::<Vec<_>>().join("\n");
                if ui::button_response(ui, &text, Some(Icon::Warning), false, None).on_hover_text(tip).clicked() {
                    if let Some(row) = issues.first().and_then(|i| g.row_of(i.tl())) {
                        self.tview.chip_sel = Some(row);
                        self.tview.sel = g.rows[row].dots.first().map(|_| (row, 0));
                        self.tview.reveal = true;
                    }
                }
            }
        });
        ui.add_space(8.0);
        if b.is_empty() {
            ui.label(
                egui::RichText::new("Right-click a scene to branch the story there, or choose one and press Branch from scene.")
                    .font(theme::font(theme::T_SM))
                    .color(p.text_faint),
            );
            ui.add_space(4.0);
        }

        let infos: Vec<RowInfo> = g
            .rows
            .iter()
            .map(|row| {
                let t = row.tl;
                let sub = match b.meta(t) {
                    None => {
                        let n = row.dots.len();
                        if n == 1 { "Main · 1 scene".to_string() } else { format!("Main · {n} scenes") }
                    }
                    Some(m) if !m.choice.trim().is_empty() => m.choice.trim().to_string(),
                    Some(_) => match row.start_col {
                        0 => "From the start".to_string(),
                        c => format!("After scene {c}"),
                    },
                };
                RowInfo {
                    name: b.name(t),
                    sub,
                    color: color_of(&b, t),
                    open: t == self.tl,
                    issues: issues.iter().filter(|i| i.tl() == t).map(|i| i.text(&b)).collect(),
                }
            })
            .collect();

        let mut acts = Vec::new();
        egui::ScrollArea::both()
            .id_salt("ns-timelines")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                acts = tv::show(ui, &g, &infos, &mut self.tview);
            });
        self.timelines_keys(ui.ctx(), &g);
        for a in acts {
            match a {
                Act::Select(r, d) => {
                    self.tview.sel = Some((r, d));
                    self.tview.chip_sel = None;
                }
                Act::Open(r, d) => {
                    let Some(row) = g.rows.get(r) else { continue };
                    let head = row.dots.get(d).and_then(|x| x.head);
                    self.open_timeline(row.tl, head, true);
                }
                Act::DotMenu(r, d, pos) => self.tl_menu = Some(super::stories::Menu { kind: MenuKind::Dot(r, d), pos, opened: self.frame_no }),
                Act::OpenRow(r) => {
                    if let Some(row) = g.rows.get(r) {
                        self.tview.chip_sel = Some(r);
                        self.open_timeline(row.tl, None, false);
                    }
                }
                Act::ChipMenu(r, pos) => {
                    self.tview.chip_sel = Some(r);
                    self.tl_menu = Some(super::stories::Menu { kind: MenuKind::Chip(r), pos, opened: self.frame_no });
                }
            }
        }
    }

    fn timelines_keys(&mut self, ctx: &egui::Context, g: &timelines::Graph) {
        let busy = ctx.memory(|m| m.focused().is_some()) || self.popover.is_some() || self.tl_menu.is_some() || self.branch_dialog.is_some() || self.palette.is_some() || self.deck.asking();
        if busy {
            return;
        }
        for key in [egui::Key::ArrowLeft, egui::Key::ArrowRight, egui::Key::ArrowUp, egui::Key::ArrowDown] {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                self.tview.sel = tv::step(g, self.tview.sel, key);
                self.tview.reveal = true;
            }
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
            if let Some((r, d)) = self.tview.sel {
                if let Some(row) = g.rows.get(r) {
                    let head = row.dots.get(d).and_then(|x| x.head);
                    self.open_timeline(row.tl, head, true);
                }
            }
        }
    }

    // menus here---

    pub(super) fn timeline_menu(&mut self, ctx: &egui::Context) {
        let Some(menu) = self.tl_menu.as_ref().map(|m| (m.kind, m.pos, m.opened)) else { return };
        let (kind, pos, opened) = menu;
        let Some((g, _)) = self.timeline_graph() else {
            self.tl_menu = None;
            return;
        };
        let Some(b) = self.live() else {
            self.tl_menu = None;
            return;
        };
        let write = self.can_write();
        let mut todo: Todo = None;
        let area = egui::Area::new(egui::Id::new("ns-timeline-menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos + Vec2::new(4.0, 4.0))
            .show(ctx, |ui| {
                ui::popover_frame().show(ui, |ui| {
                    let mut rects = Vec::new();
                    match kind {
                        MenuKind::Dot(r, d) => {
                            let Some(row) = g.rows.get(r) else { return };
                            let Some(dot) = row.dots.get(d) else { return };
                            let open_name = b.name(self.tl);
                            let from_ancestor = self.tl.is_some() && b.ancestors(self.tl).contains(&row.tl) && dot.same.is_none() && !dot.gap;
                            let mut labels = vec!["Open in Write", "Branch from here…"];
                            let same_label = format!("Show in {open_name} as a Same scene");
                            let merge_label = format!("Merge {open_name} into here");
                            if from_ancestor {
                                labels.push(&same_label);
                                labels.push(&merge_label);
                            }
                            ui.set_width(ui::menu_width(ui, &labels).max(220.0));
                            if ui::menu_item(ui, &mut rects, "Open in Write", Icon::Pencil, false) {
                                let (t, h) = (row.tl, dot.head);
                                todo = Some(Box::new(move |app: &mut App| app.open_timeline(t, h, true)));
                            }
                            if write && !dot.gap && ui::menu_item(ui, &mut rects, "Branch from here…", Icon::Graph, false) {
                                todo = Some(Box::new(move |app: &mut App| app.branch_from_dot(r, d)));
                            }
                            if write && from_ancestor {
                                if ui::menu_item(ui, &mut rects, &same_label, Icon::Link, false) {
                                    todo = Some(Box::new(move |app: &mut App| app.use_as_same(r, d)));
                                }
                                if ui::menu_item(ui, &mut rects, &merge_label, Icon::ArrowRight, false) {
                                    todo = Some(Box::new(move |app: &mut App| app.merge_into(r, d)));
                                }
                            }
                            if write && dot.same.is_some() && ui::menu_item(ui, &mut rects, "Make it its own", Icon::Unlink, false) {
                                todo = Some(Box::new(move |app: &mut App| app.make_own(r, d)));
                            }
                            let merges = row.tl.and_then(|i| b.items[i].meta().merges_into.clone()).is_some();
                            if write && merges && d + 1 == row.dots.len() && ui::menu_item(ui, &mut rects, "Remove the merge", Icon::Close, false) {
                                let i = row.tl.unwrap_or(0);
                                todo = Some(Box::new(move |app: &mut App| app.remove_merge(i)));
                            }
                        }
                        MenuKind::Chip(r) => {
                            let Some(row) = g.rows.get(r) else { return };
                            let t = row.tl;
                            ui.set_width(ui::menu_width(ui, &["Open", "Rename…", "Choice text…", "Duplicate timeline", "Remove the merge", "Delete timeline…"]).max(200.0));
                            if ui::menu_item(ui, &mut rects, "Open", Icon::Pencil, false) {
                                todo = Some(Box::new(move |app: &mut App| app.open_timeline(t, None, true)));
                            }
                            let Some(i) = t else { return };
                            if !write {
                                return;
                            }
                            let id = b.items[i].meta().id.clone();
                            if ui::menu_item(ui, &mut rects, "Rename…", Icon::Pencil, false) {
                                let (seed, id) = (b.name(t), id.clone());
                                todo = Some(Box::new(move |app: &mut App| app.deck.prompt("Rename timeline", "Only the name changes; its file stays where it is.", "Timeline name", &seed, "Rename", Ask::RenameTimeline(id))));
                            }
                            if ui::menu_item(ui, &mut rects, "Choice text…", Icon::Note, false) {
                                let (seed, id) = (b.items[i].meta().choice.clone(), id.clone());
                                todo = Some(Box::new(move |app: &mut App| {
                                    app.deck.prompt("The choice", "What the character chooses to take this timeline. Games show it as the option.", "Choice", &seed, "Save", Ask::TimelineChoice(id))
                                }));
                            }
                            ui.horizontal(|ui| {
                                ui.add_space(10.0);
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let p = pal();
                                for c in 0..timelines::COLORS {
                                    let on = b.color(t) == Some(c);
                                    if ui::swatch(ui, p.groups[c % p.groups.len()], "", on, None) {
                                        todo = Some(Box::new(move |app: &mut App| app.set_color(i, c)));
                                    }
                                }
                            });
                            if ui::menu_item(ui, &mut rects, "Duplicate timeline", Icon::Copy, false) {
                                todo = Some(Box::new(move |app: &mut App| app.duplicate_timeline(i)));
                            }
                            if b.items[i].meta().merges_into.is_some() && ui::menu_item(ui, &mut rects, "Remove the merge", Icon::Close, false) {
                                todo = Some(Box::new(move |app: &mut App| app.remove_merge(i)));
                            }
                            ui::separator(ui);
                            if ui::menu_item(ui, &mut rects, "Delete timeline…", Icon::Trash, true) {
                                todo = Some(Box::new(move |app: &mut App| app.ask_delete_timeline(i)));
                            }
                        }
                    }
                });
            })
            .response;
        if let Some(f) = todo {
            self.tl_menu = None;
            f(self);
            return;
        }
        let d = ui::Dismisser { opened_on: opened };
        if d.should_close(ctx, self.frame_no, &[area.rect]) {
            self.tl_menu = None;
        }
    }

    // dialog here---

    pub(super) fn branch_window(&mut self, ctx: &egui::Context) {
        let Some(opened) = self.branch_dialog.as_ref().map(|d| d.opened) else { return };
        let p = pal();
        let screen = ctx.screen_rect();
        let mut close = false;
        let mut go = false;
        let veil = anim::ease(ctx, "ns-branch-veil", true, 0.24);
        egui::Area::new(egui::Id::new("ns-branch-scrim"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .interactable(true)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(rect, egui::Rounding::same(crate::chrome::window_radius(ctx, theme::R_WINDOW)), theme::scrim(veil));
                if resp.clicked() && self.frame_no > opened + 1 {
                    close = true;
                }
            });
        let t = anim::ease(ctx, "ns-branch-rise", true, 0.30);
        let size = Vec2::new(440.0, 372.0);
        let rect = Rect::from_center_size(screen.center(), size * (0.97 + 0.03 * t));
        let parent_name = self.bundle.as_ref().map(|b| b.name(self.branch_dialog.as_ref().and_then(|d| d.parent))).unwrap_or_else(|| "Main".into());
        let Some(d) = self.branch_dialog.as_mut() else { return };
        egui::Area::new(egui::Id::new("ns-branch"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.set_opacity(0.2 + 0.8 * t);
                let (win, _) = ui.allocate_exact_size(rect.size(), Sense::click());
                theme::lift_shadow(ui.painter(), win, theme::R_ISLAND, 1.0);
                ui.painter().rect_filled(win, egui::Rounding::same(theme::R_ISLAND), p.solid);
                ui.painter().rect_stroke(win.shrink(0.5), egui::Rounding::same(theme::R_ISLAND), Stroke::new(1.0_f32, theme::window_edge()));
                let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(win.shrink2(Vec2::new(22.0, 18.0))).layout(Layout::top_down(Align::Min)));
                let ui = &mut ui;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    icons::show(ui, Icon::Graph, 16.0, p.primary_light);
                    ui.label(egui::RichText::new("Branch the story here").font(theme::font_semi(theme::T_H)).color(p.text));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui::icon_button(ui, Icon::Close, "Cancel · Esc", 26.0) {
                            close = true;
                        }
                    });
                });
                let from = if d.heading.trim().is_empty() { "the start".to_string() } else { d.heading.trim().to_string() };
                ui.label(egui::RichText::new(ui::elide(&format!("From {from} in {parent_name}"), 60)).font(theme::font(theme::T_CAP)).color(p.text_faint));
                ui.add_space(12.0);
                let w = ui.available_width();
                ui::section(ui, "Name");
                let r = ui::field(ui, &mut d.name, "Maria says no", w);
                if d.focus {
                    r.request_focus();
                    d.focus = false;
                }
                ui.add_space(8.0);
                ui::section(ui, "The choice that leads here");
                ui::field(ui, &mut d.choice, "She refuses the job", w);
                ui.add_space(8.0);
                ui::section(ui, &format!("Staying on {parent_name} (optional)"));
                ui::field(ui, &mut d.stay, "She takes the job", w);
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Games show these as the options at this scene.").font(theme::font(theme::T_CAP)).color(p.text_faint));
                let mut f = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(Pos2::new(win.left() + 22.0, win.bottom() - 58.0), Pos2::new(win.right() - 22.0, win.bottom() - 18.0))).layout(Layout::right_to_left(Align::Center)));
                if ui::button(&mut f, "Branch", Some(Icon::Plus), true) {
                    go = true;
                }
                if ui::button(&mut f, "Cancel", None, false) {
                    close = true;
                }
            });
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            close = true;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
            go = true;
        }
        if go {
            self.create_branch();
        } else if close {
            self.branch_dialog = None;
        }
    }
}
