use eframe::egui::{self, Align2, Color32, Pos2, Rect, Sense, Shape, Stroke, Vec2};

use crate::icons::{self, Icon};
use crate::theme::{self, pal};
use crate::timelines::{EdgeKind, Graph};
use crate::ui;

pub const CHIP_W: f32 = 196.0;
pub const COL_W: f32 = 132.0;
pub const ROW_H: f32 = 78.0;
pub const DOT_R: f32 = 12.0;
const TOP: f32 = 18.0;

#[derive(Default)]
pub struct ViewState {
    pub sel: Option<(usize, usize)>,
    pub chip_sel: Option<usize>,
    pub reveal: bool,
    pub dots: Vec<((usize, usize), Rect)>,
    pub chips: Vec<(usize, Rect)>,
    pub edges: Vec<(usize, Pos2)>,
}

pub struct RowInfo {
    pub name: String,
    pub sub: String,
    pub color: Color32,
    pub open: bool,
    pub issues: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Select(usize, usize),
    Open(usize, usize),
    DotMenu(usize, usize, Pos2),
    OpenRow(usize),
    ChipMenu(usize, Pos2),
}

pub fn dot_center(origin: Pos2, row: usize, col: usize) -> Pos2 {
    Pos2::new(origin.x + CHIP_W + COL_W * (col as f32 + 0.5), origin.y + TOP + ROW_H * (row as f32 + 0.5))
}

fn start_point(origin: Pos2) -> Pos2 {
    Pos2::new(origin.x + CHIP_W + 14.0, origin.y + TOP + ROW_H * 0.5)
}

fn bezier(a: Pos2, b: Pos2) -> [Pos2; 4] {
    let dx = ((b.x - a.x) * 0.5).max(COL_W * 0.35);
    [a, Pos2::new(a.x + dx, a.y), Pos2::new(b.x - dx, b.y), b]
}

fn point(c: &[Pos2; 4], t: f32) -> Pos2 {
    let u = 1.0 - t;
    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    Pos2::new(
        c.iter().zip(w).map(|(p, k)| p.x * k).sum(),
        c.iter().zip(w).map(|(p, k)| p.y * k).sum(),
    )
}

fn curve(painter: &egui::Painter, c: [Pos2; 4], stroke: Stroke, dashed: bool) {
    let n = 40;
    let pts: Vec<Pos2> = (0..=n).map(|i| point(&c, i as f32 / n as f32)).collect();
    if dashed {
        for (k, w) in pts.windows(2).enumerate() {
            if k % 2 == 0 {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
    } else {
        painter.add(Shape::line(pts, stroke));
    }
}

fn arrow(painter: &egui::Painter, c: &[Pos2; 4], color: Color32) {
    let tip = point(c, 0.97);
    let back = point(c, 0.90);
    let dir = (tip - back).normalized();
    let side = Vec2::new(-dir.y, dir.x);
    let at = tip - dir * (DOT_R - 2.0);
    let base = at - dir * 9.0;
    painter.add(Shape::convex_polygon(vec![at, base + side * 5.0, base - side * 5.0], color, Stroke::NONE));
}

pub fn size(g: &Graph) -> Vec2 {
    Vec2::new(CHIP_W + COL_W * (g.cols().max(1) as f32) + 80.0, TOP * 2.0 + ROW_H * g.rows.len().max(1) as f32)
}

pub fn show(ui: &mut egui::Ui, g: &Graph, rows: &[RowInfo], st: &mut ViewState) -> Vec<Act> {
    let p = pal();
    let mut acts = Vec::new();
    let want = size(g);
    let avail = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(want.x.max(avail.x), want.y.max(avail.y)), Sense::hover());
    let origin = rect.min;
    let clip = ui.clip_rect();
    let painter = ui.painter_at(rect);
    st.dots.clear();
    st.chips.clear();
    st.edges.clear();

    let ground = if p.dark { p.solid } else { p.raised };
    let stripe = if p.dark { theme::mix(p.solid, p.raised, 0.16) } else { theme::mix(p.raised, p.sunken, 0.45) };
    let band_of = |r: usize| if r % 2 == 1 { stripe } else { ground };
    for (r, _) in g.rows.iter().enumerate() {
        if r % 2 == 0 {
            continue;
        }
        let y = origin.y + TOP + ROW_H * r as f32;
        let band = Rect::from_min_max(Pos2::new(rect.left(), y), Pos2::new(rect.right(), y + ROW_H));
        painter.rect_filled(band, egui::Rounding::same(14.0), stripe);
    }

    for (r, row) in g.rows.iter().enumerate() {
        if row.dots.len() < 2 {
            continue;
        }
        let a = dot_center(origin, r, row.dots[0].col);
        let b = dot_center(origin, r, row.dots[row.dots.len() - 1].col);
        let color = rows.get(r).map(|x| x.color).unwrap_or(p.text_dim);
        painter.line_segment([a, b], Stroke::new(3.0_f32, theme::wash(color, 170)));
    }

    for (k, e) in g.edges.iter().enumerate() {
        let to_row = e.to.0;
        let to = dot_center(origin, to_row, g.rows[to_row].dots[e.to.1].col);
        let from = match e.from {
            Some((r, d)) => dot_center(origin, r, g.rows[r].dots[d].col),
            None => start_point(origin),
        };
        match e.kind {
            EdgeKind::Fork => {
                let color = rows.get(to_row).map(|x| x.color).unwrap_or(p.text_dim);
                let c = bezier(from, to);
                curve(&painter, c, Stroke::new(2.5_f32, theme::wash(color, 200)), false);
                let mid = point(&c, 0.5);
                st.edges.push((k, mid));
                if !e.label.trim().is_empty() {
                    let font = theme::font_semi(theme::T_CAP);
                    let text = ui::elide_to_width(ui.ctx(), e.label.trim(), font.clone(), 0.0, COL_W * 1.4);
                    let w = theme::tracked_width(ui.ctx(), &text, font.clone(), 0.0) + 16.0;
                    let pill = Rect::from_center_size(mid, Vec2::new(w, 20.0));
                    painter.rect_filled(pill, egui::Rounding::same(10.0), p.solid);
                    painter.rect_stroke(pill, egui::Rounding::same(10.0), Stroke::new(1.2_f32, color));
                    painter.text(pill.center(), Align2::CENTER_CENTER, text, font, p.text);
                }
            }
            EdgeKind::Merge => {
                let from_row = e.from.map(|f| f.0).unwrap_or(0);
                let color = rows.get(from_row).map(|x| x.color).unwrap_or(p.text_dim);
                let c = bezier(from, to);
                curve(&painter, c, Stroke::new(2.5_f32, theme::wash(color, 200)), false);
                arrow(&painter, &c, color);
                st.edges.push((k, point(&c, 0.5)));
            }
            EdgeKind::Same => {
                let c = bezier(from, to);
                curve(&painter, c, Stroke::new(1.6_f32, theme::wash(p.text_faint, 180)), true);
            }
        }
    }

    if g.edges.iter().any(|e| e.kind == EdgeKind::Fork && e.from.is_none()) {
        let s = start_point(origin);
        painter.circle_filled(s, 5.0, p.text_dim);
    }

    let hover_font = theme::font_bold(theme::T_CAP);
    for (r, row) in g.rows.iter().enumerate() {
        let color = rows.get(r).map(|x| x.color).unwrap_or(p.text_dim);
        for (d, dot) in row.dots.iter().enumerate() {
            let c = dot_center(origin, r, dot.col);
            let hit = Rect::from_center_size(c, Vec2::splat(DOT_R * 2.0 + 10.0));
            st.dots.push(((r, d), hit));
            let resp = ui.interact(hit, ui.id().with(("tl-dot", r, d)), Sense::click());
            let hot = resp.hovered();
            let on = st.sel == Some((r, d));
            let rad = DOT_R + if hot { 1.5 } else { 0.0 };
            if on {
                painter.circle_stroke(c, rad + 5.0, Stroke::new(2.5_f32, p.text));
            }
            if dot.gap {
                painter.circle_filled(c, rad, p.solid);
                painter.circle_stroke(c, rad, Stroke::new(2.0_f32, p.warn));
                painter.text(c, Align2::CENTER_CENTER, "!", hover_font.clone(), p.warn);
            } else if dot.same.is_some() {
                painter.circle_filled(c, rad, p.solid);
                painter.circle_stroke(c, rad, Stroke::new(2.5_f32, color));
                icons::draw(&painter, Rect::from_center_size(c, Vec2::splat(12.0)), Icon::Link, color);
            } else {
                painter.circle_filled(c, rad, color);
                painter.text(c, Align2::CENTER_CENTER, dot.number.to_string(), hover_font.clone(), theme::on(color));
            }
            let heading = if dot.heading.trim().is_empty() { format!("Scene {}", dot.number) } else { dot.heading.trim().to_string() };
            let font = theme::font(theme::T_CAP - 0.5);
            let label = ui::elide_to_width(ui.ctx(), &heading, font.clone(), 0.0, COL_W - 26.0);
            let lw = theme::tracked_width(ui.ctx(), &label, font.clone(), 0.0) + 10.0;
            let at = Rect::from_center_size(Pos2::new(c.x, c.y + DOT_R + 13.0), Vec2::new(lw, 16.0));
            painter.rect_filled(at, egui::Rounding::same(8.0), band_of(r));
            painter.text(at.center(), Align2::CENTER_CENTER, label, font, if on { p.text } else { p.text_faint });
            let tip = match (dot.same.is_some(), dot.gap) {
                (_, true) => format!("{heading}\nThis scene is no longer there"),
                (true, _) => format!("{heading}\nSame scene · scene {}", dot.number),
                _ => format!("{heading}\nScene {}", dot.number),
            };
            let resp = resp.on_hover_text(tip);
            if resp.double_clicked() {
                acts.push(Act::Open(r, d));
            } else if resp.clicked() {
                acts.push(Act::Select(r, d));
            }
            if resp.secondary_clicked() {
                acts.push(Act::Select(r, d));
                acts.push(Act::DotMenu(r, d, resp.interact_pointer_pos().unwrap_or(c)));
            }
            if on && st.reveal {
                ui.scroll_to_rect(hit.expand(40.0), None);
                st.reveal = false;
            }
        }
    }

    let chip_x = clip.left().max(rect.left());
    let column = Rect::from_min_max(Pos2::new(chip_x, rect.top()), Pos2::new(chip_x + CHIP_W - 8.0, rect.bottom()));
    let cp = ui.painter_at(column);
    cp.rect_filled(column, egui::Rounding::ZERO, ground);
    for (r, _) in g.rows.iter().enumerate() {
        let Some(info) = rows.get(r) else { continue };
        let y = origin.y + TOP + ROW_H * (r as f32 + 0.5);
        let indent = (g.rows[r].depth as f32 * 10.0).min(40.0);
        let chip = Rect::from_min_max(Pos2::new(chip_x + 8.0 + indent, y - 24.0), Pos2::new(chip_x + CHIP_W - 14.0, y + 24.0));
        st.chips.push((r, chip));
        let resp = ui.interact(chip, ui.id().with(("tl-chip", r)), Sense::click());
        let hot = crate::anim::ease(ui.ctx(), resp.id, resp.hovered(), crate::anim::HOVER);
        let fill = if info.open || st.chip_sel == Some(r) {
            theme::wash(info.color, if p.dark { 60 } else { 40 })
        } else if p.dark {
            theme::wash(p.raised, (255.0 * (0.55 + 0.45 * hot)) as u8)
        } else {
            theme::mix(p.solid, p.sunken, 0.3 + 0.4 * hot)
        };
        cp.rect_filled(chip, egui::Rounding::same(14.0), fill);
        if info.open {
            cp.rect_stroke(chip, egui::Rounding::same(14.0), Stroke::new(1.5_f32, info.color));
        }
        cp.circle_filled(Pos2::new(chip.left() + 16.0, chip.center().y), 6.0, info.color);
        let room = chip.width() - 40.0 - if info.issues.is_empty() { 0.0 } else { 20.0 };
        let nf = theme::font_semi(theme::T_SM);
        cp.text(
            Pos2::new(chip.left() + 30.0, chip.center().y - 8.0),
            Align2::LEFT_CENTER,
            ui::elide_to_width(ui.ctx(), &info.name, nf.clone(), 0.0, room),
            nf,
            p.text,
        );
        let sf = theme::font(theme::T_CAP);
        cp.text(
            Pos2::new(chip.left() + 30.0, chip.center().y + 9.0),
            Align2::LEFT_CENTER,
            ui::elide_to_width(ui.ctx(), &info.sub, sf.clone(), 0.0, room),
            sf,
            p.text_faint,
        );
        let mut tip = info.name.clone();
        if !info.issues.is_empty() {
            let w = Rect::from_center_size(Pos2::new(chip.right() - 16.0, chip.center().y), Vec2::splat(15.0));
            icons::draw(&cp, w, Icon::Warning, p.warn);
            tip = format!("{tip}\n{}", info.issues.join("\n"));
        }
        let resp = resp.on_hover_text(tip);
        if resp.clicked() {
            acts.push(Act::OpenRow(r));
        }
        if resp.secondary_clicked() {
            acts.push(Act::ChipMenu(r, resp.interact_pointer_pos().unwrap_or(chip.center())));
        }
    }
    acts
}

pub fn step(g: &Graph, sel: Option<(usize, usize)>, key: egui::Key) -> Option<(usize, usize)> {
    let first = g.rows.iter().position(|r| !r.dots.is_empty()).map(|r| (r, 0));
    let Some((r, d)) = sel else { return first };
    let row = g.rows.get(r)?;
    let col = row.dots.get(d)?.col;
    match key {
        egui::Key::ArrowLeft => Some((r, d.saturating_sub(1))),
        egui::Key::ArrowRight => Some((r, (d + 1).min(row.dots.len().saturating_sub(1)))),
        egui::Key::ArrowUp | egui::Key::ArrowDown => {
            let rows: Vec<usize> = if key == egui::Key::ArrowUp { (0..r).rev().collect() } else { (r + 1..g.rows.len()).collect() };
            for k in rows {
                let dots = &g.rows[k].dots;
                if dots.is_empty() {
                    continue;
                }
                let best = (0..dots.len()).min_by_key(|&i| (dots[i].col as i64 - col as i64).abs()).unwrap_or(0);
                return Some((k, best));
            }
            Some((r, d))
        }
        _ => Some((r, d)),
    }
}
