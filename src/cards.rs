use eframe::egui::{self, Pos2, Rect, Sense, Vec2};

use crate::anim;
use crate::icons::Icon;
use crate::model::{act_title, eighths_label, ActSpan, Document, Scene};
use crate::theme::{self, pal};
use crate::ui;

pub const CARD_W: f32 = 236.0;
pub const CARD_H: f32 = 156.0;
const GAP: f32 = 18.0;
const HEAD_H: f32 = 36.0;
const RIBBON_H: f32 = 40.0;
const RIBBON_GAP: f32 = 12.0;

#[derive(Default)]
pub struct CardsState {
    pub dragging: Option<usize>,
    pub drop_at: Option<usize>,
    pub menu: Option<(usize, Pos2, u64)>,
    pub last_rects: Vec<Rect>,
    pub new_card_rect: Option<Rect>,
    pub only: Vec<usize>,
    pub naming: Option<[String; STORYLINES]>,
    pub scroll_to: Option<usize>,
    pub segments: Vec<(usize, Rect)>,
    pub chips: Vec<(usize, Rect)>,
    pub lanes: Vec<(Option<usize>, Rect)>,
    pub extra_lanes: Vec<usize>,
    pub beat_chips: Vec<(String, Rect)>,
}

pub const STORYLINES: usize = 6;

const LANE_MIN: f32 = 220.0;
const LANE_MAX: f32 = 440.0;
const LANE_GAP: f32 = 16.0;
const LANE_HEAD: f32 = 44.0;
const LANE_PAD: f32 = 10.0;
const CARD_PAD: f32 = 14.0;
const CARD_GAP: f32 = 10.0;
const GRIP_H: f32 = 52.0;
const SYN_LINE: f32 = 18.0;

pub const BEATS: [(&str, f32); 5] = [
    ("Inciting incident", 0.10),
    ("Break into two", 0.25),
    ("Midpoint", 0.50),
    ("Break into three", 0.75),
    ("Climax", 0.90),
];

pub fn beat_pages(total_eighths: usize) -> Vec<(&'static str, f32, usize)> {
    BEATS
        .iter()
        .map(|(name, f)| (*name, *f, ((f * total_eighths as f32) / 8.0).floor() as usize + 1))
        .collect()
}

pub fn spans(scenes: &[Scene], lengths: &std::collections::HashMap<u64, (usize, usize)>) -> (Vec<(usize, usize)>, usize) {
    let mut at = 0usize;
    let mut out = Vec::with_capacity(scenes.len());
    for sc in scenes {
        let len = lengths.get(&sc.id).map(|l| l.1).unwrap_or(0).max(1);
        out.push((at, len));
        at += len;
    }
    (out, at)
}

pub fn beat_scene(spans: &[(usize, usize)], total: usize, f: f32) -> Option<usize> {
    let mark = f * total as f32;
    spans.iter().position(|(a, l)| (*a as f32) <= mark && mark < (*a + *l) as f32)
}

pub enum Act {
    Open(u64),
    Move(usize, usize),
    Tint(u64, Option<usize>),
    Delete(usize),
    NewScene,
    OpenAct(u64),
}

pub struct Out {
    pub changed: bool,
    pub act: Option<Act>,
    pub rename: Option<(usize, String)>,
}

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    doc: &mut Document,
    st: &mut CardsState,
    lengths: &std::collections::HashMap<u64, (usize, usize)>,
    act_notes: &std::collections::HashMap<u64, String>,
    live: Option<u64>,
    frame_no: u64,
    guides: bool,
) -> Out {
    let p = pal();
    let scenes = doc.scenes();
    let acts = doc.acts();
    let mut out = Out {
        changed: false,
        act: None,
        rename: None,
    };

    let avail = ui.available_width();
    let cols = (((avail - 2.0 * GAP) + GAP) / (CARD_W + GAP)).floor().max(1.0) as usize;
    let grid_w = cols as f32 * CARD_W + (cols as f32 - 1.0) * GAP;
    let left = ui.min_rect().left() + ((avail - grid_w) * 0.5).max(GAP);
    let (spans, total) = spans(&scenes, lengths);
    let bar_top = ui.cursor().top() + 18.0;
    let bar_h = if scenes.is_empty() {
        0.0
    } else {
        beat_bar(ui, doc, st, &scenes, &acts, &spans, total, live, guides, Rect::from_min_size(Pos2::new(left, bar_top), Vec2::new(grid_w, 0.0)), &mut out)
    };
    let top = bar_top + bar_h + 6.0;

    let mut slots: Vec<Rect> = Vec::with_capacity(scenes.len() + 1);
    let mut ribbons: Vec<(Rect, &ActSpan)> = Vec::new();
    let (mut y, mut c) = (top, 0usize);
    let mut next_act = 0usize;
    let mut ribbon = |y: &mut f32, c: &mut usize, a| {
        if *c > 0 {
            *y += CARD_H + GAP;
            *c = 0;
        }
        ribbons.push((Rect::from_min_size(Pos2::new(left, *y), Vec2::new(grid_w, RIBBON_H)), a));
        *y += RIBBON_H + RIBBON_GAP;
    };
    for sc in &scenes {
        while let Some(a) = acts.get(next_act).filter(|a| a.start < sc.start) {
            ribbon(&mut y, &mut c, a);
            next_act += 1;
        }
        slots.push(Rect::from_min_size(
            Pos2::new(left + c as f32 * (CARD_W + GAP), y),
            Vec2::new(CARD_W, CARD_H),
        ));
        c += 1;
        if c == cols {
            c = 0;
            y += CARD_H + GAP;
        }
    }
    for a in &acts[next_act.min(acts.len())..] {
        ribbon(&mut y, &mut c, a);
    }
    slots.push(Rect::from_min_size(
        Pos2::new(left + c as f32 * (CARD_W + GAP), y),
        Vec2::new(CARD_W, CARD_H),
    ));
    let slot = |k: usize| slots[k];
    let bottom = slots.last().map(|r| r.bottom()).unwrap_or(top) + GAP;
    let (whole, _) = ui.allocate_exact_size(
        Vec2::new(avail, (top - bar_top) + 24.0 + (bottom - top) + 60.0),
        Sense::hover(),
    );
    let _ = whole;
    if let Some(k) = st.scroll_to.take() {
        if k < scenes.len() {
            ui.scroll_to_rect(slot(k).expand(GAP), Some(egui::Align::Center));
        }
    }
    let beat_tags: Vec<(usize, &str)> = if guides {
        BEATS.iter().filter_map(|(name, f)| beat_scene(&spans, total, *f).map(|k| (k, *name))).collect()
    } else {
        Vec::new()
    };

    for (rect, a) in &ribbons {
        if act_ribbon(ui, *rect, a, act_notes.get(&a.id).map(String::as_str).unwrap_or("")) {
            out.act = Some(Act::OpenAct(a.id));
        }
    }

    if scenes.is_empty() {
        ui.painter().text(
            Pos2::new(ui.min_rect().center().x, top - 6.0),
            egui::Align2::CENTER_BOTTOM,
            "No scenes yet — every scene heading becomes a card.",
            theme::font(theme::T_BODY),
            p.text_faint,
        );
    }

    st.last_rects.clear();
    let pointer = ui.ctx().pointer_latest_pos();
    let mut drag_started: Option<usize> = None;

    for (k, sc) in scenes.iter().enumerate() {
        let rect = slot(k);
        st.last_rects.push(rect);
        let being_dragged = st.dragging == Some(k);
        let head = Rect::from_min_size(rect.min, Vec2::new(rect.width(), HEAD_H));
        let body_resp = ui.interact(rect, egui::Id::new(("ns-card", sc.id)), Sense::click());
        let head_resp = ui.interact(head, egui::Id::new(("ns-card-head", sc.id)), Sense::click_and_drag());

        let hovered = body_resp.hovered() || head_resp.hovered();
        let selected = live == Some(sc.id);
        let lift = anim::ease(ui.ctx(), ("ns-card-lift", sc.id), hovered || selected, 0.20);
        let r = rect.expand(1.2 * lift);
        let tint = sc.tint.map(|t| p.group(t));
        let accent = tint.unwrap_or(p.primary);
        let outside = !st.only.is_empty() && !sc.tint.map(|t| st.only.contains(&t)).unwrap_or(false);
        let fade: f32 = if being_dragged { 0.35 } else if outside { 0.22 } else { 1.0 };

        draw_card(ui, r, sc, accent, tint, lift, selected, fade, lengths);
        let here: Vec<&str> = beat_tags.iter().filter(|(b, _)| *b == k).map(|(_, n)| *n).collect();
        if let Some(first) = here.first() {
            let font = theme::font_hud(theme::T_MICRO);
            let up = if here.len() > 1 { format!("{first} +{}", here.len() - 1) } else { first.to_string() };
            let w = theme::tracked_width(ui.ctx(), &up, font.clone(), 0.8);
            let at = Pos2::new(r.right() - w - 4.0, r.top() - 8.0);
            ui.painter().rect_filled(Rect::from_center_size(Pos2::new(at.x - 8.0, at.y), Vec2::splat(5.0)), egui::Rounding::same(theme::R_ROW), theme::wash(p.primary_light, (255.0 * fade) as u8));
            theme::tracked_text(ui.painter(), at, &up, font, theme::wash(p.primary_light, (255.0 * fade) as u8), 0.8);
        }

        let syn_rect = Rect::from_min_max(
            Pos2::new(r.left() + 14.0, r.top() + HEAD_H + 2.0),
            Pos2::new(r.right() - 14.0, r.bottom() - 30.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(syn_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(syn_rect.intersect(ui.clip_rect()));
        if let Some(i) = doc.index_of(sc.id) {
            let resp = child.add(
                egui::TextEdit::multiline(&mut doc.blocks[i].note)
                    .id(egui::Id::new(("ns-card-synopsis", sc.id)))
                    .frame(false)
                    .font(theme::font(theme::T_CAP + 1.0))
                    .text_color(theme::wash(p.text_dim, (255.0 * fade) as u8))
                    .desired_width(syn_rect.width())
                    .desired_rows(4)
                    .margin(egui::Margin::ZERO)
                    .hint_text(
                        egui::RichText::new("What happens in this scene?")
                            .color(theme::wash(p.text_faint, 150)),
                    ),
            );
            if resp.changed() {
                let b = &mut doc.blocks[i];
                if b.note.contains('\n') {
                    b.note = b.note.replace('\n', " ");
                }
                out.changed = true;
            }
        }

        if head_resp.drag_started() {
            drag_started = Some(k);
        }
        if head_resp.clicked() {
            out.act = Some(Act::Open(sc.id));
        }
        let heading_tip = if sc.heading.trim().is_empty() {
            "Untitled scene".to_string()
        } else {
            sc.heading.clone()
        };
        let _ = head_resp.on_hover_text(format!("{heading_tip}\nClick to open · drag to move"));
        if body_resp.secondary_clicked() {
            if let Some(q) = pointer {
                st.menu = Some((k, q, frame_no));
            }
        }
    }

    // card makes new here---
    let nk = scenes.len();
    let nr = slot(nk);
    st.new_card_rect = Some(nr);
    let resp = ui.interact(nr, egui::Id::new("ns-card-new"), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
    ui::wipe(ui.painter(), nr, hot, if p.dark { p.raised } else { p.sunken });
    ui.painter().rect_stroke(
        nr.shrink(0.5),
        egui::Rounding::same(theme::R_CARD),
        egui::Stroke::new(1.0_f32, theme::mix(p.line, p.text_faint, hot)),
    );
    let c = nr.center();
    crate::icons::draw(
        ui.painter(),
        Rect::from_center_size(c - Vec2::new(0.0, 12.0), Vec2::splat(16.0)),
        Icon::Plus,
        theme::mix(p.text_dim, p.text, hot),
    );
    let label = "New scene";
    let font = theme::font_hud(theme::T_CAP);
    let lw = theme::tracked_width(ui.ctx(), label, font.clone(), 0.9);
    theme::tracked_text(
        ui.painter(),
        Pos2::new(c.x - lw * 0.5, c.y + 14.0),
        label,
        font,
        theme::mix(p.text_dim, p.text, hot),
        0.9,
    );
    if resp.on_hover_text(format!("A new scene at the end · {}+Enter in the script", crate::keys::command_key())).clicked() {
        out.act = Some(Act::NewScene);
    }

    // dragging card new here---
    if let Some(k) = drag_started {
        st.dragging = Some(k);
    }
    let still_down = ui.input(|i| i.pointer.primary_down());
    if let Some(from) = st.dragging {
        if let Some(q) = pointer {
            let mut best = (f32::MAX, from);
            for k in 0..=scenes.len() {
                let edge = if k < scenes.len() {
                    let r = slot(k);
                    Pos2::new(r.left() - GAP * 0.5, r.center().y)
                } else {
                    let r = slot(scenes.len().saturating_sub(1));
                    Pos2::new(r.right() + GAP * 0.5, r.center().y)
                };
                let d = edge.distance(q);
                if d < best.0 {
                    best = (d, k);
                }
            }
            st.drop_at = Some(best.1);

            let at = best.1;
            let marker_x = if at < scenes.len() {
                slot(at).left() - GAP * 0.5
            } else {
                slot(scenes.len().saturating_sub(1)).right() + GAP * 0.5
            };
            let marker_row = if at < scenes.len() { slot(at) } else { slot(scenes.len().saturating_sub(1)) };
            let bar = Rect::from_center_size(
                Pos2::new(marker_x, marker_row.center().y),
                Vec2::new(3.0, CARD_H - 24.0),
            );
            let moving = at != from && at != from + 1;
            if moving {
                ui.painter().rect_filled(bar, egui::Rounding::same(theme::R_ROW), p.primary_light);
            }

            if let Some(sc) = scenes.get(from) {
                let ghost = Rect::from_min_size(q - Vec2::new(40.0, 18.0), Vec2::new(CARD_W, HEAD_H + 8.0));
                egui::Area::new(egui::Id::new("ns-card-ghost"))
                    .order(egui::Order::Tooltip)
                    .fixed_pos(ghost.min)
                    .interactable(false)
                    .show(ui.ctx(), |ui| {
                        let (r, _) = ui.allocate_exact_size(ghost.size(), Sense::hover());
                        theme::glass_surface(ui.painter(), r, theme::R_CARD, 1.0);
                        ui.painter().rect_stroke(r.shrink(0.5), egui::Rounding::same(theme::R_CARD), egui::Stroke::new(1.0_f32, p.primary_light));
                        ui.painter().rect_filled(
                            Rect::from_center_size(Pos2::new(r.left() + 20.0, r.center().y), Vec2::splat(7.0)),
                            egui::Rounding::same(theme::R_ROW),
                            p.primary_light,
                        );
                        ui.painter().text(
                            Pos2::new(r.left() + 34.0, r.center().y),
                            egui::Align2::LEFT_CENTER,
                            ui::elide_to_width(ui.ctx(), &sc.heading, theme::font_med(theme::T_SM), 0.0, r.width() - 48.0),
                            theme::font_med(theme::T_SM),
                            p.text,
                        );
                    });
            }
            anim::keep_going(ui.ctx());
        }
        if !still_down {
            if let Some(to) = st.drop_at.take() {
                if to != from && to != from + 1 {
                    out.act = Some(Act::Move(from, to));
                }
            }
            st.dragging = None;
        }
    }

    card_menu(ui, doc, st, &scenes, frame_no, &mut out);

    out
}

// card's menu here---
fn card_menu(ui: &mut egui::Ui, doc: &Document, st: &mut CardsState, scenes: &[Scene], frame_no: u64, out: &mut Out) {
    let p = pal();
    if let Some((k, at, opened)) = st.menu {
        if let Some(sc) = scenes.get(k) {
            let mut close = false;
            let area = egui::Area::new(egui::Id::new("ns-card-menu"))
                .order(egui::Order::Foreground)
                .fixed_pos(at)
                .show(ui.ctx(), |ui| {
                    ui::popover_frame().show(ui, |ui| {
                        ui.set_width(ui::menu_width(ui, &["Open in the script", "Delete this scene"]).max(176.0));
                        let mut rects = Vec::new();
                        if ui::menu_item(ui, &mut rects, "Open in the script", Icon::Pencil, false) {
                            out.act = Some(Act::Open(sc.id));
                            close = true;
                        }
                        ui::section(ui, "Storyline");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.0;
                            ui.add_space(6.0);
                            for t in 0..STORYLINES {
                                if ui::swatch(ui, p.group(t), doc.meta.storyline(t).unwrap_or(""), sc.tint == Some(t), None) {
                                    out.act = Some(Act::Tint(sc.id, Some(t)));
                                    close = true;
                                }
                            }
                            if ui::icon_button(ui, Icon::Eraser, "No colour", 22.0) {
                                out.act = Some(Act::Tint(sc.id, None));
                                close = true;
                            }
                        });
                        ui::separator(ui);
                        if ui::menu_item(ui, &mut rects, "Delete this scene", Icon::Trash, true) {
                            out.act = Some(Act::Delete(k));
                            close = true;
                        }
                    });
                })
                .response;
            let d = ui::Dismisser { opened_on: opened };
            if close
                || d.should_close(ui.ctx(), frame_no, &[area.rect])
                || ui.input(|i| i.key_pressed(egui::Key::Escape))
            {
                st.menu = None;
            }
        } else {
            st.menu = None;
        }
    }

}

fn storyline_label(doc: &Document, t: usize) -> String {
    doc.meta.storyline(t).map(str::to_string).unwrap_or_else(|| format!("{} story", (b'A' + t as u8) as char))
}

#[allow(clippy::too_many_arguments)]
fn beat_bar(
    ui: &mut egui::Ui,
    doc: &mut Document,
    st: &mut CardsState,
    scenes: &[Scene],
    acts: &[ActSpan],
    spans: &[(usize, usize)],
    total: usize,
    live: Option<u64>,
    guides: bool,
    frame: Rect,
    out: &mut Out,
) -> f32 {
    let p = pal();
    let left = frame.left();
    let w = frame.width();
    let mut y = frame.top();
    let used: Vec<usize> = (0..STORYLINES)
        .filter(|t| scenes.iter().any(|s| s.tint == Some(*t)) || doc.meta.storyline(*t).is_some())
        .collect();

    // storylines legend here---
    let row = Rect::from_min_size(Pos2::new(left, y), Vec2::new(w, 26.0));
    let hud = theme::font_hud(theme::T_MICRO);
    let mut x = left + theme::tracked_text(ui.painter(), Pos2::new(left, row.center().y), "Storylines", hud.clone(), p.text_faint, 0.8) + 14.0;
    let mut right = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::right_to_left(egui::Align::Center)));
    let naming = st.naming.is_some();
    if ui::quiet_button(&mut right, if naming { "Done" } else { "Name" }, Some(if naming { Icon::Check } else { Icon::Pencil })) {
        st.naming = if naming {
            None
        } else {
            Some(std::array::from_fn(|t| doc.meta.storyline(t).unwrap_or("").to_string()))
        };
    }
    if !st.only.is_empty() && ui::quiet_button(&mut right, "All", None) {
        st.only.clear();
    }
    let stop = right.min_rect().left() - 12.0;
    st.chips.clear();
    if used.is_empty() {
        ui.painter().text(
            Pos2::new(x, row.center().y),
            egui::Align2::LEFT_CENTER,
            "Right click a card to give it a storyline colour.",
            theme::font(theme::T_CAP),
            p.text_faint,
        );
    }
    for t in &used {
        let label = storyline_label(doc, *t);
        let font = theme::font_med(theme::T_CAP + 1.0);
        let tw = ui.painter().layout_no_wrap(label.clone(), font.clone(), p.text).rect.width();
        let chip = Rect::from_min_size(Pos2::new(x, row.top() + 1.0), Vec2::new(tw + 30.0, 24.0));
        if chip.right() > stop {
            break;
        }
        let resp = ui.interact(chip, egui::Id::new(("ns-storyline", *t)), Sense::click());
        let on = st.only.contains(t);
        let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
        theme::hover_surface(ui.painter(), chip, theme::R_SM, p.group(*t), hot, on);
        let dim = !st.only.is_empty() && !on;
        ui.painter().rect_filled(
            Rect::from_center_size(Pos2::new(chip.left() + 12.0, chip.center().y), Vec2::splat(8.0)),
            egui::Rounding::same(theme::R_ROW),
            if dim { theme::wash(p.group(*t), 90) } else { p.group(*t) },
        );
        ui.painter().text(
            Pos2::new(chip.left() + 22.0, chip.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            if dim { p.text_faint } else { p.text },
        );
        st.chips.push((*t, chip));
        if resp.on_hover_text("Show only this storyline · click again to add or remove").clicked() {
            if on {
                st.only.retain(|x| x != t);
            } else {
                st.only.push(*t);
            }
        }
        x = chip.right() + 4.0;
    }
    y += 30.0;

    // naming storylines here---
    if let Some(names) = st.naming.as_mut() {
        let per = 3usize;
        let cell_w = (w / per as f32).floor();
        for (t, name) in names.iter_mut().enumerate() {
            let cell = Rect::from_min_size(Pos2::new(left + (t % per) as f32 * cell_w, y + (t / per) as f32 * 34.0), Vec2::new(cell_w - 12.0, 30.0));
            ui.painter().rect_filled(
                Rect::from_center_size(Pos2::new(cell.left() + 6.0, cell.center().y), Vec2::splat(9.0)),
                egui::Rounding::same(theme::R_ROW),
                p.group(t),
            );
            let mut field = ui.new_child(egui::UiBuilder::new().max_rect(cell.with_min_x(cell.left() + 18.0)).layout(egui::Layout::left_to_right(egui::Align::Center)));
            let resp = field.add(
                egui::TextEdit::singleline(name)
                    .id(egui::Id::new(("ns-storyline-name", t)))
                    .hint_text(format!("{} story", (b'A' + t as u8) as char))
                    .desired_width(cell.width() - 18.0)
                    .margin(egui::Margin::symmetric(8.0, 5.0)),
            );
            if resp.changed() {
                doc.meta.set_storyline(t, name);
                out.changed = true;
            }
        }
        y += 2.0 * 34.0 + 8.0;
    }

    // timeline here---
    let label_rows = if guides { 2.0 * 13.0 } else { 0.0 };
    let track = Rect::from_min_size(Pos2::new(left, y + label_rows + 4.0), Vec2::new(w, 20.0));
    ui.painter().rect_filled(track, egui::Rounding::same(theme::R_SM), if p.dark { p.raised } else { p.sunken });
    let total_f = total.max(1) as f32;
    let px = |e: f32| track.left() + track.width() * (e / total_f);
    st.segments.clear();
    for (k, (sc, (at, len))) in scenes.iter().zip(spans).enumerate() {
        let seg = Rect::from_min_max(Pos2::new(px(*at as f32), track.top()), Pos2::new(px((*at + *len) as f32), track.bottom()));
        let inner = seg.shrink2(Vec2::new(if seg.width() > 3.0 { 0.75 } else { 0.0 }, 0.0));
        let resp = ui.interact(seg, egui::Id::new(("ns-timeline", sc.id)), Sense::click());
        let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
        let base = sc.tint.map(|t| p.group(t)).unwrap_or(if p.dark { p.line_strong } else { p.line });
        let outside = !st.only.is_empty() && !sc.tint.map(|t| st.only.contains(&t)).unwrap_or(false);
        let col = if outside { theme::wash(base, 50) } else { theme::mix(base, p.text, 0.25 * hot) };
        ui.painter().rect_filled(inner, egui::Rounding::same(theme::R_ROW), col);
        if live == Some(sc.id) {
            ui.painter().rect_stroke(inner.expand(1.0), egui::Rounding::same(theme::R_ROW), egui::Stroke::new(1.5_f32, p.text));
        }
        st.segments.push((k, seg));
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let page = (*at / 8) + 1;
        let heading = if sc.heading.trim().is_empty() { "Untitled scene" } else { sc.heading.trim() };
        let tip = match sc.tint {
            Some(t) => format!("{:02} · {heading}\np.{page} · {} pg · {}", sc.number, eighths_label(*len), storyline_label(doc, t)),
            None => format!("{:02} · {heading}\np.{page} · {} pg", sc.number, eighths_label(*len)),
        };
        if resp.on_hover_text(tip).clicked() {
            st.scroll_to = Some(k);
        }
    }

    // act breaks here---
    for a in acts.iter().skip(1) {
        if let Some(k) = scenes.iter().position(|s| s.start > a.start) {
            let x = px(spans[k].0 as f32).round() + 0.5;
            ui.painter().line_segment([Pos2::new(x, track.top() - 3.0), Pos2::new(x, track.bottom() + 3.0)], egui::Stroke::new(2.0_f32, p.text));
        }
    }

    // beat guides here---
    if guides {
        for (i, (name, f, page)) in beat_pages(total).into_iter().enumerate() {
            let x = px(f * total_f).round() + 0.5;
            let mut yy = track.top() - 2.0;
            while yy < track.bottom() + 2.0 {
                ui.painter().line_segment([Pos2::new(x, yy), Pos2::new(x, (yy + 3.0).min(track.bottom() + 2.0))], egui::Stroke::new(1.0_f32, p.primary_light));
                yy += 5.0;
            }
            let label = format!("{name} · p. {page}");
            let lw = theme::tracked_width(ui.ctx(), &label, hud.clone(), 0.7);
            let ly = y + 6.0 + if i % 2 == 0 { 0.0 } else { 13.0 };
            let lx = (x - lw * 0.5).clamp(left, left + w - lw);
            ui.painter().line_segment([Pos2::new(x, ly + 6.0), Pos2::new(x, track.top() - 2.0)], egui::Stroke::new(1.0_f32, theme::wash(p.primary_light, 120)));
            theme::tracked_text(ui.painter(), Pos2::new(lx, ly), &label, hud.clone(), p.primary_light, 0.7);
        }
    }

    // ruler here---
    let pages = (total as f32 / 8.0).ceil().max(1.0) as usize;
    let step = [1usize, 5, 10, 20, 50].into_iter().find(|s| (pages / s) as f32 * 34.0 < w).unwrap_or(100);
    let ry = track.bottom() + 4.0;
    let mut n = 1usize;
    while n <= pages {
        let x = px(((n - 1) * 8) as f32).round() + 0.5;
        ui.painter().line_segment([Pos2::new(x, ry), Pos2::new(x, ry + 4.0)], egui::Stroke::new(1.0_f32, p.text_faint));
        ui.painter().text(Pos2::new(x + 3.0, ry + 2.0), egui::Align2::LEFT_TOP, n.to_string(), theme::font_mono(theme::T_MICRO), p.text_faint);
        n = if n == 1 && step > 1 { step } else { n + step };
    }
    (ry + 18.0) - frame.top()
}

fn act_ribbon(ui: &mut egui::Ui, rect: Rect, a: &ActSpan, note: &str) -> bool {
    let p = pal();
    let resp = ui.interact(rect, egui::Id::new(("ns-act-ribbon", a.id)), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
    let cy = rect.center().y;
    ui.painter().rect_filled(
        Rect::from_center_size(Pos2::new(rect.left() + 10.0, cy), Vec2::splat(7.0)),
        egui::Rounding::same(theme::R_ROW),
        theme::mix(p.primary_light, p.text, hot),
    );
    let title = if a.title.trim().is_empty() { act_title(a.number) } else { a.title.trim().to_string() };
    let title = ui::elide_to_width(
        ui.ctx(),
        &title.to_uppercase(),
        theme::font_display(20.0),
        0.0,
        (rect.width() - 40.0).max(40.0),
    );
    let g = ui.painter().layout_no_wrap(title, theme::font_display(20.0), p.text);
    let w = g.rect.width();
    ui.painter().galley(Pos2::new(rect.left() + 24.0, cy - g.rect.height() * 0.5), g, p.text);
    let mut x = rect.left() + 24.0 + w + 16.0;
    if !note.is_empty() {
        let font = theme::font_hud(theme::T_MICRO + 0.5);
        let up = note.to_string();
        let nw = theme::tracked_width(ui.ctx(), &up, font.clone(), 0.8);
        if x + nw + 40.0 < rect.right() {
            theme::tracked_text(ui.painter(), Pos2::new(x, cy), &up, font, p.text_faint, 0.8);
            x += nw + 16.0;
        }
    }
    let end = rect.right() - 8.0;
    if end - x > 24.0 {
        let y = cy.round() + 0.5;
        ui.painter().line_segment(
            [Pos2::new(x, y), Pos2::new(end, y)],
            egui::Stroke::new(1.0_f32, theme::mix(p.line, p.line_strong, hot)),
        );
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.on_hover_text("Go to this act in the script").clicked()
}

#[allow(clippy::too_many_arguments)]
fn draw_card(
    ui: &egui::Ui,
    r: Rect,
    sc: &Scene,
    accent: egui::Color32,
    tint: Option<egui::Color32>,
    lift: f32,
    selected: bool,
    fade: f32,
    lengths: &std::collections::HashMap<u64, (usize, usize)>,
) {
    let p = pal();
    let painter = ui.painter();
    let radius = theme::R_CARD;

    let _ = accent;
    let w = |c: egui::Color32| theme::wash(c, (c.a() as f32 * fade) as u8);
    painter.rect_filled(r, egui::Rounding::same(radius), w(theme::mix(p.solid_hi, p.raised, 0.6 * lift)));
    painter.rect_stroke(
        r.shrink(0.5),
        egui::Rounding::same(radius),
        egui::Stroke::new(
            1.0_f32,
            w(if selected { p.primary_light } else { theme::mix(p.line, p.text_faint, lift) }),
        ),
    );
    theme::top_edge(painter, r);
    if let Some(t) = tint {
        painter.rect_filled(
            Rect::from_min_size(r.min + Vec2::new(1.0, 1.0), Vec2::new(3.0, r.height() - 2.0)),
            egui::Rounding::same(theme::R_ROW),
            w(t),
        );
    }

    let cy = r.top() + HEAD_H * 0.5 + 2.0;
    let num_col = theme::wash(p.primary_light, (255.0 * fade) as u8);
    let ng = painter.layout_no_wrap(format!("{:02}", sc.number), theme::font_display(18.0), num_col);
    let num_w = ng.rect.width();
    painter.galley(Pos2::new(r.left() + 14.0, cy - ng.rect.height() * 0.5), ng, num_col);
    let heading = if sc.heading.trim().is_empty() {
        "Untitled scene".to_string()
    } else {
        sc.heading.clone()
    };
    let head_x = r.left() + 14.0 + num_w + 10.0;
    let clip = painter.with_clip_rect(r.shrink(1.0));
    clip.text(
        Pos2::new(head_x, cy),
        egui::Align2::LEFT_CENTER,
        ui::elide_to_width(ui.ctx(), &heading, theme::font_semi(theme::T_SM), 0.0, r.right() - head_x - 14.0),
        theme::font_semi(theme::T_SM),
        theme::wash(p.text, (255.0 * fade) as u8),
    );

    let (page, eighths) = lengths.get(&sc.id).copied().unwrap_or((0, 0));
    let mut foot = if page > 0 {
        format!("p. {page} · {} pg", eighths_label(eighths))
    } else {
        "not on the page yet".to_string()
    };
    if !sc.cast.is_empty() {
        foot.push_str(" · ");
        foot.push_str(&sc.cast.join(", "));
    }
    let foot_font = theme::font_hud(theme::T_MICRO);
    theme::tracked_text(
        &clip,
        Pos2::new(r.left() + 14.0, r.bottom() - 16.0),
        &ui::elide_to_width(ui.ctx(), &foot, foot_font.clone(), 0.6, r.width() - 28.0),
        foot_font,
        theme::wash(p.text_faint, (255.0 * fade) as u8),
        0.6,
    );
}

pub fn lanes_of(doc: &Document, scenes: &[Scene], extra: &[usize]) -> Vec<Option<usize>> {
    let mut out: Vec<Option<usize>> = (0..STORYLINES)
        .filter(|t| scenes.iter().any(|s| s.tint == Some(*t)) || doc.meta.storyline(*t).is_some() || extra.contains(t))
        .map(Some)
        .collect();
    if out.is_empty() || scenes.iter().any(|s| s.tint.is_none()) {
        out.push(None);
    }
    out
}

pub fn next_storyline(doc: &Document, scenes: &[Scene], extra: &[usize]) -> Option<usize> {
    let lanes = lanes_of(doc, scenes, extra);
    (0..STORYLINES).find(|t| !lanes.contains(&Some(*t)))
}

fn lane_label(doc: &Document, t: Option<usize>) -> String {
    match t {
        Some(t) => storyline_label(doc, t),
        None => "No storyline".to_string(),
    }
}

fn synopsis_rows(ui: &egui::Ui, text: &str, w: f32) -> usize {
    if text.trim().is_empty() {
        return 1;
    }
    let g = ui.painter().layout(text.to_string(), theme::font(theme::T_SM), pal().text, w);
    g.rows.len().clamp(1, 6)
}

fn card_height(ui: &egui::Ui, doc: &Document, sc: &Scene, w: f32) -> f32 {
    let note = doc.index_of(sc.id).map(|i| doc.blocks[i].note.as_str()).unwrap_or("");
    let rows = synopsis_rows(ui, note, w - 2.0 * CARD_PAD);
    let tag = if time_of(sc).is_some() { 30.0 } else { 0.0 };
    GRIP_H + rows as f32 * SYN_LINE + tag + CARD_PAD
}

fn time_of(sc: &Scene) -> Option<(String, Option<usize>)> {
    let (_, _, time) = crate::report::split_heading(&sc.heading);
    crate::app::time_tone(&time).map(|t| (crate::app::sentence(&time), Some(t)))
}

struct Mark {
    at: f32,
    label: String,
    beat: bool,
    act: Option<u64>,
    scene: Option<usize>,
}

#[allow(clippy::too_many_arguments)]
fn beat_row(ui: &mut egui::Ui, doc: &Document, st: &mut CardsState, scenes: &[Scene], spans: &[(usize, usize)], total: usize, guides: bool, frame: Rect, out: &mut Out) -> f32 {
    let p = pal();
    let acts = doc.acts();
    let mut items: Vec<Mark> = Vec::new();
    let total_f = total.max(1) as f32;
    for a in &acts {
        let at = scenes.iter().position(|s| s.start > a.start).and_then(|k| spans.get(k)).map(|s| s.0 as f32 / total_f).unwrap_or(1.0);
        let title = if a.title.trim().is_empty() { act_title(a.number) } else { a.title.trim().to_string() };
        items.push(Mark { at: at - 0.0001, label: crate::app::sentence(&title), beat: false, act: Some(a.id), scene: None });
    }
    if guides {
        for (name, f) in BEATS {
            items.push(Mark { at: f, label: name.to_string(), beat: true, act: None, scene: beat_scene(spans, total, f) });
        }
    }
    items.sort_by(|a, b| a.at.partial_cmp(&b.at).unwrap_or(std::cmp::Ordering::Equal));
    st.beat_chips.clear();
    let row = Rect::from_min_size(frame.min, Vec2::new(frame.width(), 26.0));
    let mut right = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::right_to_left(egui::Align::Center)));
    let naming = st.naming.is_some();
    if ui::quiet_button(&mut right, if naming { "Done" } else { "Name storylines" }, Some(if naming { Icon::Check } else { Icon::Pencil })) {
        st.naming = if naming { None } else { Some(std::array::from_fn(|t| doc.meta.storyline(t).unwrap_or("").to_string())) };
    }
    let stop = right.min_rect().left() - 12.0;
    let mut x = row.left();
    let font = theme::font_semi(theme::T_CAP);
    if items.is_empty() {
        ui.painter().text(Pos2::new(x, row.center().y), egui::Align2::LEFT_CENTER, "Turn on Beats to see the story's turning points here.", theme::font(theme::T_CAP), p.text_faint);
    }
    for Mark { label, beat, act, scene, .. } in items {
        let tw = if beat { ui::tag_size(ui.ctx(), &label, false).x } else { ui.painter().layout_no_wrap(label.clone(), font.clone(), p.text).rect.width() + 8.0 };
        let chip = Rect::from_min_size(Pos2::new(x, row.top() + 1.0), Vec2::new(tw, 24.0));
        if chip.right() > stop {
            break;
        }
        let resp = ui.interact(chip, egui::Id::new(("ns-beat-chip", label.as_str())), Sense::click());
        let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
        if beat {
            ui::tag_at(ui.painter(), chip.shrink2(Vec2::new(0.0, 1.0 - hot)), &label, Some(0), false);
        } else {
            ui.painter().text(Pos2::new(chip.left() + 4.0, chip.center().y), egui::Align2::LEFT_CENTER, &label, font.clone(), theme::mix(p.text_faint, p.text, hot));
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let tip = if beat { "Show the scene this beat lands in" } else { "Go to this act in the script" };
        if resp.on_hover_text(tip).clicked() {
            if let Some(id) = act {
                out.act = Some(Act::OpenAct(id));
            } else if let Some(k) = scene {
                st.scroll_to = Some(k);
            }
        }
        st.beat_chips.push((label, chip));
        x = chip.right() + 6.0;
    }
    let mut y = row.bottom() + 8.0;
    if let Some(names) = st.naming.as_mut() {
        let per = 3usize;
        let cell_w = (frame.width() / per as f32).floor();
        for (t, name) in names.iter_mut().enumerate() {
            let cell = Rect::from_min_size(Pos2::new(frame.left() + (t % per) as f32 * cell_w, y + (t / per) as f32 * 34.0), Vec2::new(cell_w - 12.0, 30.0));
            ui.painter().rect_filled(Rect::from_center_size(Pos2::new(cell.left() + 6.0, cell.center().y), Vec2::splat(9.0)), egui::Rounding::same(theme::R_ROW), p.group(t));
            let mut field = ui.new_child(egui::UiBuilder::new().max_rect(cell.with_min_x(cell.left() + 18.0)).layout(egui::Layout::left_to_right(egui::Align::Center)));
            let resp = field.add(
                egui::TextEdit::singleline(name)
                    .id(egui::Id::new(("ns-storyline-name", t)))
                    .hint_text(format!("{} story", (b'A' + t as u8) as char))
                    .desired_width(cell.width() - 18.0)
                    .margin(egui::Margin::symmetric(8.0, 5.0)),
            );
            if resp.changed() {
                out.rename = Some((t, name.clone()));
            }
        }
        y += 2.0 * 34.0 + 8.0;
    }
    y - frame.top()
}

// columns here---
#[allow(clippy::too_many_arguments)]
pub fn columns(
    ui: &mut egui::Ui,
    doc: &mut Document,
    st: &mut CardsState,
    lengths: &std::collections::HashMap<u64, (usize, usize)>,
    live: Option<u64>,
    frame_no: u64,
    guides: bool,
) -> Out {
    let p = pal();
    let scenes = doc.scenes();
    let mut out = Out { changed: false, act: None, rename: None };
    let (spans, total) = spans(&scenes, lengths);
    let lanes = lanes_of(doc, &scenes, &st.extra_lanes);
    st.extra_lanes.retain(|t| !scenes.iter().any(|s| s.tint == Some(*t)) && doc.meta.storyline(*t).is_none());
    let avail = ui.available_width();
    let left = ui.min_rect().left();
    let n = lanes.len().max(1) as f32;
    let lane_w = ((avail - 4.0 - (n - 1.0) * LANE_GAP) / n).clamp(LANE_MIN, LANE_MAX.max(LANE_MIN));
    let full_w = n * lane_w + (n - 1.0) * LANE_GAP;
    let top0 = ui.cursor().top() + 2.0;
    let beat_h = beat_row(ui, doc, st, &scenes, &spans, total, guides, Rect::from_min_size(Pos2::new(left, top0), Vec2::new(avail - 4.0, 0.0)), &mut out);
    if let Some((t, name)) = out.rename.take() {
        doc.meta.set_storyline(t, &name);
        out.changed = true;
    }
    let top = top0 + beat_h + 4.0;
    let card_w = lane_w - 2.0 * LANE_PAD;

    let mut rects: Vec<Rect> = vec![Rect::NOTHING; scenes.len()];
    let mut lane_cards: Vec<Vec<usize>> = vec![Vec::new(); lanes.len()];
    let mut tallest = 0.0f32;
    for (li, t) in lanes.iter().enumerate() {
        let x = left + li as f32 * (lane_w + LANE_GAP) + LANE_PAD;
        let mut y = top + LANE_HEAD;
        for (k, sc) in scenes.iter().enumerate().filter(|(_, sc)| sc.tint == *t) {
            let h = card_height(ui, doc, sc, card_w);
            rects[k] = Rect::from_min_size(Pos2::new(x, y), Vec2::new(card_w, h));
            lane_cards[li].push(k);
            y += h + CARD_GAP;
        }
        tallest = tallest.max(y - top);
    }
    let floor = (ui.clip_rect().bottom() - top - 1.0).max(200.0);
    let lane_h = (tallest + LANE_PAD).max(floor);
    let (_whole, _) = ui.allocate_exact_size(Vec2::new(full_w + 4.0, (top - top0) + lane_h + 16.0), Sense::hover());

    st.lanes.clear();
    let dragging_tint = st.dragging.and_then(|k| scenes.get(k)).map(|s| s.tint);
    let pointer = ui.ctx().pointer_latest_pos();
    let mut target_lane: Option<usize> = None;
    for (li, t) in lanes.iter().enumerate() {
        let lane = Rect::from_min_size(Pos2::new(left + li as f32 * (lane_w + LANE_GAP), top), Vec2::new(lane_w, lane_h));
        st.lanes.push((*t, lane));
        let over = st.dragging.is_some() && pointer.map(|q| lane.expand2(Vec2::new(LANE_GAP * 0.5, 4000.0)).contains(q)).unwrap_or(false);
        if over {
            target_lane = Some(li);
        }
        ncf::theme::surface(ui.painter(), lane, ncf::theme::R_LG, ncf::theme::Lift::Flat);
        if over && dragging_tint != Some(*t) {
            ui.painter().rect_stroke(lane.shrink(1.0), egui::Rounding::same(ncf::theme::R_LG), egui::Stroke::new(2.0_f32, p.primary));
        }
        let hy = lane.top() + LANE_HEAD * 0.5 + 2.0;
        let dot = t.map(|t| p.group(t)).unwrap_or(p.text_faint);
        ui.painter().circle_filled(Pos2::new(lane.left() + 20.0, hy), 5.0, dot);
        let count = lane_cards[li].len();
        let count_s = if count == 1 { "1 scene".to_string() } else { format!("{count} scenes") };
        let cf = theme::font_med(theme::T_CAP);
        let cw = ui.painter().layout_no_wrap(count_s.clone(), cf.clone(), p.text).rect.width();
        ui.painter().text(Pos2::new(lane.right() - 16.0, hy), egui::Align2::RIGHT_CENTER, count_s, cf, p.text_faint);
        let nf = theme::font_bold(theme::T_SM + 1.0);
        let name = ui::elide_to_width(ui.ctx(), &lane_label(doc, *t), nf.clone(), 0.0, lane.width() - 52.0 - cw);
        ui.painter().text(Pos2::new(lane.left() + 32.0, hy), egui::Align2::LEFT_CENTER, name, nf, p.text);
        if count == 0 {
            let hint = if t.is_some() { "Drag a card here" } else { "Every scene has a storyline" };
            ui.painter().text(Pos2::new(lane.center().x, lane.top() + LANE_HEAD + 40.0), egui::Align2::CENTER_CENTER, hint, theme::font(theme::T_CAP), p.text_faint);
        }
    }

    if let Some(k) = st.scroll_to.take() {
        if let Some(r) = rects.get(k) {
            ui.scroll_to_rect(r.expand(LANE_GAP), Some(egui::Align::Center));
        }
    }

    st.last_rects = rects.clone();
    st.new_card_rect = None;
    st.chips.clear();
    st.segments.clear();
    let mut drag_started: Option<usize> = None;
    for (k, sc) in scenes.iter().enumerate() {
        let rect = rects[k];
        let grip = Rect::from_min_size(rect.min, Vec2::new(rect.width(), GRIP_H - 6.0));
        let body = ui.interact(rect, egui::Id::new(("ns-card", sc.id)), Sense::click());
        let head = ui.interact(grip, egui::Id::new(("ns-card-head", sc.id)), Sense::click_and_drag());
        let hovered = body.hovered() || head.hovered();
        let selected = live == Some(sc.id);
        let lift = anim::ease(ui.ctx(), ("ns-card-lift", sc.id), hovered || selected, 0.20);
        let fade: f32 = if st.dragging == Some(k) { 0.4 } else { 1.0 };
        let w = |c: egui::Color32| theme::wash(c, (c.a() as f32 * fade) as u8);
        let fill = if p.dark { theme::mix(p.raised, p.solid_hi, 0.5 * lift) } else { theme::mix(p.raised, p.sunken, 0.5 * lift) };
        ui.painter().rect_filled(rect, egui::Rounding::same(ncf::theme::R_MD), w(fill));
        if selected {
            ui.painter().rect_stroke(rect.shrink(0.5), egui::Rounding::same(ncf::theme::R_MD), egui::Stroke::new(1.5_f32, w(p.primary)));
        }
        let clip = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        clip.text(Pos2::new(rect.left() + CARD_PAD, rect.top() + CARD_PAD + 5.0), egui::Align2::LEFT_CENTER, format!("{:02}", sc.number), theme::font_semi(theme::T_MICRO + 1.0), w(p.text_faint));
        let heading = if sc.heading.trim().is_empty() { "UNTITLED SCENE".to_string() } else { sc.heading.trim().to_uppercase() };
        let hf = egui::FontId::new(13.0, theme::page_bold());
        clip.text(
            Pos2::new(rect.left() + CARD_PAD, rect.top() + CARD_PAD + 24.0),
            egui::Align2::LEFT_CENTER,
            ui::elide_to_width(ui.ctx(), &heading, hf.clone(), 0.0, rect.width() - 2.0 * CARD_PAD),
            hf,
            w(p.text),
        );
        let syn = Rect::from_min_max(Pos2::new(rect.left() + CARD_PAD, rect.top() + GRIP_H - 6.0), Pos2::new(rect.right() - CARD_PAD, rect.bottom() - CARD_PAD - if time_of(sc).is_some() { 30.0 } else { 0.0 }));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(syn).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(syn.expand(2.0).intersect(ui.clip_rect()));
        if let Some(i) = doc.index_of(sc.id) {
            let resp = child.add(
                egui::TextEdit::multiline(&mut doc.blocks[i].note)
                    .id(egui::Id::new(("ns-card-synopsis", sc.id)))
                    .frame(false)
                    .font(theme::font(theme::T_SM))
                    .text_color(w(p.text_dim))
                    .desired_width(syn.width())
                    .desired_rows(1)
                    .margin(egui::Margin::ZERO)
                    .hint_text(egui::RichText::new("What happens in this scene?").color(theme::wash(p.text_faint, 150))),
            );
            if resp.changed() {
                let b = &mut doc.blocks[i];
                if b.note.contains('\n') {
                    b.note = b.note.replace('\n', " ");
                }
                out.changed = true;
            }
        }
        if let Some((label, tone)) = time_of(sc) {
            let size = ui::tag_size(ui.ctx(), &label, false);
            let at = Rect::from_min_size(Pos2::new(rect.left() + CARD_PAD, rect.bottom() - CARD_PAD - size.y), size);
            ui::tag_at(&clip, at, &label, tone, false);
        }
        if head.hovered() {
            ui.ctx().set_cursor_icon(if st.dragging.is_some() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
        }
        if head.drag_started() {
            drag_started = Some(k);
        }
        if head.clicked() {
            out.act = Some(Act::Open(sc.id));
        }
        let _ = head.on_hover_text("Click to open · drag to another storyline or a new place");
        if body.secondary_clicked() {
            if let Some(q) = pointer {
                st.menu = Some((k, q, frame_no));
            }
        }
    }

    // dragging between lanes here---
    if let Some(k) = drag_started {
        st.dragging = Some(k);
    }
    let still_down = ui.input(|i| i.pointer.primary_down());
    if let Some(from) = st.dragging {
        let mut drop: Option<Act> = None;
        if let (Some(q), Some(li)) = (pointer, target_lane) {
            let t = lanes[li];
            let lane = st.lanes[li].1;
            if scenes.get(from).map(|s| s.tint) == Some(t) {
                let list = &lane_cards[li];
                let slot = list.iter().position(|k| q.y < rects[*k].center().y).unwrap_or(list.len());
                let to = list.get(slot).copied().unwrap_or_else(|| list.last().map(|k| k + 1).unwrap_or(from));
                let y = match (slot.checked_sub(1).and_then(|s| list.get(s)), list.get(slot)) {
                    (Some(a), Some(b)) => (rects[*a].bottom() + rects[*b].top()) * 0.5,
                    (None, Some(b)) => rects[*b].top() - CARD_GAP * 0.5,
                    (Some(a), None) => rects[*a].bottom() + CARD_GAP * 0.5,
                    (None, None) => lane.top() + LANE_HEAD,
                };
                if to != from && to != from + 1 {
                    ui.painter().rect_filled(Rect::from_center_size(Pos2::new(lane.center().x, y), Vec2::new(card_w - 24.0, 3.0)), egui::Rounding::same(2.0), p.primary);
                    drop = Some(Act::Move(from, to));
                }
            } else if let Some(sc) = scenes.get(from) {
                drop = Some(Act::Tint(sc.id, t));
            }
        }
        if let (Some(q), Some(sc)) = (pointer, scenes.get(from)) {
            let ghost = Rect::from_min_size(q - Vec2::new(40.0, 18.0), Vec2::new(card_w.min(260.0), 44.0));
            egui::Area::new(egui::Id::new("ns-card-ghost"))
                .order(egui::Order::Tooltip)
                .fixed_pos(ghost.min)
                .interactable(false)
                .show(ui.ctx(), |ui| {
                    let (r, _) = ui.allocate_exact_size(ghost.size(), Sense::hover());
                    ncf::theme::surface(ui.painter(), r, ncf::theme::R_MD, ncf::theme::Lift::Lift);
                    let dot = sc.tint.map(|t| p.group(t)).unwrap_or(p.text_faint);
                    ui.painter().circle_filled(Pos2::new(r.left() + 18.0, r.center().y), 4.5, dot);
                    let hf = egui::FontId::new(13.0, theme::page_bold());
                    ui.painter().text(Pos2::new(r.left() + 32.0, r.center().y), egui::Align2::LEFT_CENTER, ui::elide_to_width(ui.ctx(), &sc.heading.to_uppercase(), hf.clone(), 0.0, r.width() - 44.0), hf, p.text);
                });
            anim::keep_going(ui.ctx());
        }
        if !still_down {
            if drop.is_some() {
                out.act = drop;
            }
            st.dragging = None;
            st.drop_at = None;
        }
    }

    card_menu(ui, doc, st, &scenes, frame_no, &mut out);
    out
}
