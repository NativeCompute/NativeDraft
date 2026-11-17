use eframe::egui::{self, Align2, Color32, Key, Modifiers, Pos2, Rect, Sense, Stroke, Vec2};
use web_time::Instant;

use super::App;
use crate::anim;
use crate::chrome;
use crate::keys::Command;
use crate::theme::{self, pal};
use crate::ui;

pub const SEEN: &str = "new-look";
pub const BLUE_PAGES: &str = "blue-pages";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Full,
    New,
}

pub struct Tour {
    pub kind: Kind,
    pub step: usize,
    born: Instant,
    dir: f32,
}

#[derive(Clone, Copy)]
enum Sketch {
    Welcome,
    Writing,
    Views,
    Cards,
    Notes,
    Revisions,
    Files,
    Dual,
    Panel,
    Reports,
    Look,
    Palette,
    Week,
    Appearance,
    Covers,
    Timelines,
}

struct Step {
    title: &'static str,
    body: String,
    sketch: Sketch,
}

pub fn wanted(onboarded: &str, fresh: bool) -> Option<Kind> {
    if onboarded == SEEN {
        None
    } else if fresh {
        Some(Kind::Full)
    } else {
        Some(Kind::New)
    }
}

impl App {
    pub(super) fn start_tour(&mut self, kind: Kind) {
        self.tour = Some(Tour { kind, step: 0, born: Instant::now(), dir: 1.0 });
        self.popover = None;
    }

    fn steps(&self, kind: Kind) -> Vec<Step> {
        let k = |c: Command| self.keys_for(c);
        match kind {
            Kind::Full => vec![
                Step {
                    title: "Welcome to NativeDraft",
                    body: "A screenplay editor that keeps to industry format, so you can keep your mind on the story. This takes a minute, and you can skip it any time.".into(),
                    sketch: Sketch::Welcome,
                },
                Step {
                    title: "Write, and the format follows",
                    body: "Press Enter for the element that usually comes next, and Tab to change it: scene heading, action, character, dialogue. Margins and page breaks are set the way the page will print.".into(),
                    sketch: Sketch::Writing,
                },
                Step {
                    title: "Four ways to see a script",
                    body: format!("Write is where you type. Cards shows each scene as a card you can drag. Timelines draws every branch of the story. Read shows the printed pages. Switch in the toolbar, or with {}.", k(Command::NextView)),
                    sketch: Sketch::Views,
                },
                Step {
                    title: "Search or jump to anything",
                    body: format!("{} opens one search for your scripts, the scenes in this one, lines in every script and every command, with its shortcut.", k(Command::Palette)),
                    sketch: Sketch::Palette,
                },
                Step {
                    title: "Cards and storylines",
                    body: "Each storyline gets a column. Drag a card into another column to change its storyline, or up and down to move the scene. The grid shows the whole script to scale.".into(),
                    sketch: Sketch::Cards,
                },
                Step {
                    title: "Notes in the margin",
                    body: format!("Put the cursor on a line and press {} to leave yourself a note. Every note is listed under Notes on the right, and none of them print unless you ask.", k(Command::AddNote)),
                    sketch: Sketch::Notes,
                },
                Step {
                    title: "Revisions and export",
                    body: "When the script goes out, start a revision from the Changes tab. Every change is marked with an asterisk, and each round takes the next colour. Export a PDF, Fountain or Final Draft file from the toolbar.".into(),
                    sketch: Sketch::Revisions,
                },
                Step {
                    title: "Timelines",
                    body: format!("Branch the story at any scene: {} starts a timeline that shares everything up to there and then goes its own way. Show a scene from Main as a Same scene, merge back when the paths meet, and see every timeline as a graph in the Timelines view.", k(Command::NewTimeline)),
                    sketch: Sketch::Timelines,
                },
                Step {
                    title: "Your scripts stay yours",
                    body: "Each script is a plain text file in your library folder, with snapshots kept beside it. The sidebar on the left takes you back to all of them.".into(),
                    sketch: Sketch::Files,
                },
            ],
            Kind::New => {
                let mut steps = vec![
                    Step {
                        title: "A new look",
                        body: "Your library sits in the sidebar, the tools float above the page, and Scenes, Notes, Changes and Cast share the tabs on the right. Click the title on the page for the title page.".into(),
                        sketch: Sketch::Look,
                    },
                    Step {
                        title: "Search or jump to anything",
                        body: format!("{} finds scripts, scenes, lines and commands in one place. Adding a note stays on {}.", k(Command::Palette), k(Command::AddNote)),
                        sketch: Sketch::Palette,
                    },
                    Step {
                        title: "Storylines in columns",
                        body: "Cards now sit in a column per storyline. Drag a card across to change its storyline, or up and down to move the scene. The grid and its timeline are a click away.".into(),
                        sketch: Sketch::Cards,
                    },
                    Step {
                        title: "Your week, and what's due",
                        body: "Home counts the words you write each day and remembers where you were in every script. Set a deadline on a script's title page and the nearest one shows on Home.".into(),
                        sketch: Sketch::Week,
                    },
                    Step {
                        title: "Light, dark, or your system",
                        body: "NativeDraft now follows your computer's light or dark setting and changes when it does. To keep one look, pick Light or Dark under Settings, Appearance.".into(),
                        sketch: Sketch::Appearance,
                    },
                    Step {
                        title: "Timelines",
                        body: format!("Branch the story at a choice with {}. Each timeline shares the script up to that scene, can show a scene from Main as a Same scene and merge back, and exports as a script or as Ink, Yarn or JSON for games.", k(Command::NewTimeline)),
                        sketch: Sketch::Timelines,
                    },
                    Step {
                        title: "A cover for every script",
                        body: "Each script gets a cover colour so you can find it at a glance. Change it from the script's menu on Home or on its title page. A cloud script changes its cover while it's open.".into(),
                        sketch: Sketch::Covers,
                    },
                ];
                if self.settings.onboarded != BLUE_PAGES {
                    steps.extend(self.blue_pages_steps());
                }
                steps
            }
        }
    }

    fn blue_pages_steps(&self) -> Vec<Step> {
        let k = |c: Command| self.keys_for(c);
        vec![
            Step {
                title: "Dual dialogue",
                body: format!("Two characters talking at once: put the cursor in the second speech and press {}. The pair sits side by side, on screen and in print.", k(Command::DualDialogue)),
                sketch: Sketch::Dual,
            },
            Step {
                title: "Notes in the margin",
                body: format!("Leave a note on any line with {}. Your notes and changes are listed in the tabs on the right.", k(Command::AddNote)),
                sketch: Sketch::Panel,
            },
            Step {
                title: "Revisions in colour",
                body: "Start a revision from the Changes tab. Changed lines get an asterisk, locked pages keep their numbers with new material on A-pages, and you can print just the changed pages.".into(),
                sketch: Sketch::Revisions,
            },
            Step {
                title: "Reports, and find everywhere",
                body: format!("{} opens reports on characters, scenes, locations and time of day. {} searches inside every script.", k(Command::Reports), k(Command::Palette)),
                sketch: Sketch::Reports,
            },
        ]
    }

    fn end_tour(&mut self) {
        self.tour = None;
        if self.settings.onboarded != SEEN {
            self.settings.onboarded = SEEN.to_string();
            self.save_settings();
        }
    }

    fn turn(&mut self, by: isize, count: usize) {
        let Some(t) = self.tour.as_mut() else {
            return;
        };
        let next = t.step as isize + by;
        if next < 0 {
            return;
        }
        if next as usize >= count {
            self.end_tour();
            return;
        }
        t.step = next as usize;
        t.dir = by.signum() as f32;
        t.born = Instant::now();
    }

    pub(super) fn tour_keys(&mut self, ctx: &egui::Context) {
        let Some(kind) = self.tour.as_ref().map(|t| t.kind) else {
            return;
        };
        let count = self.steps(kind).len();
        ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
        let (skip, back, next) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::Escape),
                i.consume_key(Modifiers::NONE, Key::ArrowLeft),
                i.consume_key(Modifiers::NONE, Key::ArrowRight) || i.consume_key(Modifiers::NONE, Key::Enter),
            )
        });
        if skip {
            self.end_tour();
        } else if back {
            self.turn(-1, count);
        } else if next {
            self.turn(1, count);
        }
    }

    pub(super) fn tour_overlay(&mut self, ctx: &egui::Context) {
        let Some((kind, step, born, dir)) = self.tour.as_ref().map(|t| (t.kind, t.step, t.born, t.dir)) else {
            return;
        };
        let steps = self.steps(kind);
        let Some(here) = steps.get(step) else {
            self.end_tour();
            return;
        };
        let p = pal();
        let screen = ctx.screen_rect();
        let t = if anim::enabled() { (born.elapsed().as_secs_f32() / 0.30).clamp(0.0, 1.0) } else { 1.0 };
        if t < 1.0 {
            ctx.request_repaint();
        }
        let e = anim::smootherstep(t);
        let mut action: Option<isize> = None;
        let mut skip = false;

        egui::Area::new(egui::Id::new("ns-tour"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .interactable(true)
            .show(ctx, |ui| {
                let (whole, _) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(
                    whole,
                    egui::Rounding::same(chrome::window_radius(ctx, theme::R_WINDOW)),
                    theme::wash(p.backdrop, 238),
                );
                chrome::column_guides(ui.painter(), whole);

                let size = Vec2::new((screen.width() - 64.0).min(820.0), (screen.height() - 64.0).min(440.0));
                let card = Rect::from_center_size(screen.center(), size);
                ncf::theme::surface(ui.painter(), card, ncf::theme::R_XL, ncf::theme::Lift::Float);

                let art_w = (card.width() * 0.42).min(330.0);
                let art = Rect::from_min_size(card.min, Vec2::new(art_w, card.height())).shrink(28.0);
                let shift = (1.0 - e) * 24.0 * dir;
                let fade = |c: Color32| theme::wash(c, (c.a() as f32 * (0.15 + 0.85 * e)) as u8);
                sketch(ui.painter(), art.translate(Vec2::new(shift, 0.0)), here.sketch, &fade);

                let text = Rect::from_min_max(Pos2::new(card.left() + art_w + 8.0, card.top() + 32.0), Pos2::new(card.right() - 36.0, card.bottom() - 28.0));
                let mut col = ui.new_child(egui::UiBuilder::new().max_rect(text.translate(Vec2::new(shift * 0.6, 0.0))).layout(egui::Layout::top_down(egui::Align::Min)));
                col.set_opacity(0.15 + 0.85 * e);
                let label = match kind {
                    Kind::Full => format!("Tour · {} of {}", step + 1, steps.len()),
                    Kind::New => format!("What's new · {} of {}", step + 1, steps.len()),
                };
                ui::hud(&mut col, &label, Some(p.primary_light));
                col.add_space(14.0);
                let size_t = 28.0;
                let mut job = egui::text::LayoutJob::single_section(
                    here.title.to_string(),
                    egui::TextFormat { font_id: theme::font_display(size_t), color: p.text, extra_letter_spacing: ncf::theme::display_tracking(size_t), ..Default::default() },
                );
                job.wrap.max_width = text.width();
                let galley = col.ctx().fonts(|f| f.layout_job(job));
                let (r, _) = col.allocate_exact_size(Vec2::new(text.width(), galley.rect.height()), Sense::hover());
                col.painter().galley(r.min, galley, p.text);
                col.add_space(14.0);
                let body = col.painter().layout(here.body.clone(), theme::font(theme::T_BODY), p.text_dim, text.width());
                let (r, _) = col.allocate_exact_size(Vec2::new(text.width(), body.rect.height()), Sense::hover());
                col.painter().galley(r.min, body, p.text_dim);

                let foot = Rect::from_min_max(Pos2::new(text.left(), card.bottom() - 62.0), Pos2::new(card.right() - 28.0, card.bottom() - 22.0));
                for k in 0..steps.len() {
                    let c = Pos2::new(foot.left() + 4.0 + k as f32 * 14.0, foot.center().y);
                    let on = k == step;
                    ui.painter().rect_filled(
                        Rect::from_center_size(c, Vec2::new(if on { 10.0 } else { 6.0 }, 6.0)),
                        egui::Rounding::same(3.0),
                        if on { p.primary } else { p.line_strong },
                    );
                }
                let mut buttons = ui.new_child(egui::UiBuilder::new().max_rect(foot).layout(egui::Layout::right_to_left(egui::Align::Center)));
                let last = step + 1 == steps.len();
                if ui::button(&mut buttons, if last { "Start writing" } else { "Next" }, None, true) {
                    action = Some(1);
                }
                if step > 0 && ui::button(&mut buttons, "Back", None, false) {
                    action = Some(-1);
                }
                if !last && ui::quiet_button(&mut buttons, "Skip", None) {
                    skip = true;
                }
            });

        if skip {
            self.end_tour();
        } else if let Some(by) = action {
            self.turn(by, steps.len());
        }
    }

    pub fn debug_tour_at(&mut self, kind: Kind, step: usize) {
        self.start_tour(kind);
        if let Some(t) = self.tour.as_mut() {
            t.step = step;
        }
    }
    pub fn debug_tour(&self) -> Option<(Kind, usize)> {
        self.tour.as_ref().map(|t| (t.kind, t.step))
    }
    pub fn debug_skip_tour(&mut self) {
        self.end_tour();
    }
    pub fn debug_start_tour(&mut self) {
        self.start_tour(Kind::Full);
    }
}

fn page(painter: &egui::Painter, r: Rect, fade: &dyn Fn(Color32) -> Color32) -> Rect {
    let p = pal();
    painter.rect_filled(r, egui::Rounding::same(theme::R_SM), fade(if p.dark { p.solid_hi } else { Color32::WHITE }));
    painter.rect_stroke(r.shrink(0.5), egui::Rounding::same(theme::R_SM), Stroke::new(1.0_f32, fade(p.line_strong)));
    r.shrink2(Vec2::new(r.width() * 0.14, r.height() * 0.08))
}

fn bar(painter: &egui::Painter, x: f32, y: f32, w: f32, c: Color32) {
    painter.rect_filled(Rect::from_min_size(Pos2::new(x, y), Vec2::new(w.max(2.0), 3.0)), egui::Rounding::same(theme::R_ROW), c);
}

fn tag(painter: &egui::Painter, at: Pos2, text: &str, c: Color32) {
    painter.text(at, Align2::LEFT_CENTER, text, theme::font_semi(theme::T_MICRO + 1.0), c);
}

fn tag_w(painter: &egui::Painter, text: &str) -> f32 {
    painter.layout_no_wrap(text.to_string(), theme::font_semi(theme::T_MICRO + 1.0), Color32::WHITE).rect.width()
}

fn sketch(painter: &egui::Painter, area: Rect, kind: Sketch, fade: &dyn Fn(Color32) -> Color32) {
    let p = pal();
    let ink = fade(theme::mix(p.text_dim, p.text_faint, 0.3));
    let strong = fade(p.text);
    let blue = fade(p.primary_light);
    let sheet = Rect::from_center_size(area.center(), Vec2::new(area.width().min(area.height() * 0.78), area.height()).min(area.size()));
    match kind {
        Sketch::Welcome => {
            let inner = page(painter, sheet, fade);
            let c = inner.center();
            painter.text(c - Vec2::new(0.0, 24.0), Align2::CENTER_CENTER, "FADE IN:", theme::font_page_bold(15.0), strong);
            bar(painter, inner.left(), c.y + 6.0, inner.width() * 0.9, ink);
            bar(painter, inner.left(), c.y + 14.0, inner.width() * 0.6, ink);
            painter.rect_filled(Rect::from_min_size(Pos2::new(inner.left() + inner.width() * 0.6 + 4.0, c.y + 12.0), Vec2::new(2.0, 9.0)), egui::Rounding::same(theme::R_ROW), blue);
        }
        Sketch::Writing => {
            let room = 84.0;
            let sheet = Rect::from_min_max(Pos2::new(area.left() + room, sheet.top()), Pos2::new(area.right(), sheet.bottom()));
            let inner = page(painter, sheet, fade);
            let rows: [(&str, f32, f32, bool); 5] = [
                ("Scene", 0.0, 0.7, true),
                ("Action", 0.0, 0.95, false),
                ("Character", 0.38, 0.22, true),
                ("Dialogue", 0.18, 0.6, false),
                ("Action", 0.0, 0.8, false),
            ];
            for (k, (name, x, w, bold)) in rows.iter().enumerate() {
                let y = inner.top() + 14.0 + k as f32 * (inner.height() - 20.0) / 5.0;
                bar(painter, inner.left() + inner.width() * x, y, inner.width() * w, if *bold { strong } else { ink });
                tag(painter, Pos2::new(sheet.left() - 10.0 - tag_w(painter, name), y + 1.0), name, if k == 3 { blue } else { fade(p.text_faint) });
            }
            let key = Rect::from_center_size(Pos2::new(sheet.right() - 26.0, sheet.bottom() - 22.0), Vec2::new(38.0, 22.0));
            painter.rect_stroke(key, egui::Rounding::same(4.0), Stroke::new(1.0_f32, blue));
            painter.text(key.center(), Align2::CENTER_CENTER, "Tab", theme::font_semi(theme::T_MICRO + 1.0), blue);
        }
        Sketch::Views => {
            let w = (area.width() - 16.0) / 3.0;
            for k in 0..3 {
                let r = Rect::from_min_size(Pos2::new(area.left() + k as f32 * (w + 8.0), area.center().y - w * 0.65), Vec2::new(w, w * 1.3));
                let inner = page(painter, r, fade);
                match k {
                    0 => (0..6).for_each(|i| bar(painter, inner.left(), inner.top() + i as f32 * 9.0, inner.width() * if i % 2 == 0 { 0.9 } else { 0.5 }, ink)),
                    1 => (0..4).for_each(|i| {
                        let c = Rect::from_min_size(Pos2::new(inner.left() + (i % 2) as f32 * inner.width() * 0.52, inner.top() + (i / 2) as f32 * inner.height() * 0.5), Vec2::new(inner.width() * 0.46, inner.height() * 0.44));
                        painter.rect_stroke(c, egui::Rounding::same(1.0), Stroke::new(1.0_f32, ink));
                        painter.rect_filled(Rect::from_min_size(c.min, Vec2::new(2.0, c.height())), egui::Rounding::same(theme::R_ROW), fade(p.group(i)));
                    }),
                    _ => (0..7).for_each(|i| bar(painter, inner.left() + if i % 3 == 1 { inner.width() * 0.3 } else { 0.0 }, inner.top() + i as f32 * 8.0, inner.width() * 0.6, ink)),
                }
                let name = ["Write", "Cards", "Read"][k];
                tag(painter, Pos2::new(r.left(), r.bottom() + 14.0), name, if k == 1 { blue } else { fade(p.text_faint) });
            }
        }
        Sketch::Cards => {
            let mut x = area.left();
            for (k, name) in ["Inciting incident", "Midpoint", "Climax"].iter().enumerate() {
                let w = tag_w(painter, name) + 16.0;
                let chip = Rect::from_min_size(Pos2::new(x, area.top() + 10.0), Vec2::new(w, 20.0));
                painter.rect_filled(chip, egui::Rounding::same(10.0), fade(theme::wash(p.primary, 46)));
                painter.text(chip.center(), Align2::CENTER_CENTER, *name, theme::font_semi(theme::T_MICRO + 1.0), blue);
                x = chip.right() + 6.0 + k as f32;
            }
            let gap = 10.0;
            let w = (area.width() - 2.0 * gap) / 3.0;
            let top = area.top() + 44.0;
            for (k, (name, cards)) in [("Maria", 3usize), ("The heist", 2), ("Cole", 1)].iter().enumerate() {
                let lane = Rect::from_min_size(Pos2::new(area.left() + k as f32 * (w + gap), top), Vec2::new(w, area.bottom() - top));
                painter.rect_filled(lane, egui::Rounding::same(ncf::theme::R_MD), fade(if p.dark { p.solid } else { Color32::WHITE }));
                painter.circle_filled(Pos2::new(lane.left() + 14.0, lane.top() + 16.0), 4.0, fade(p.group(k)));
                tag(painter, Pos2::new(lane.left() + 24.0, lane.top() + 16.0), name, strong);
                for c in 0..*cards {
                    let r = Rect::from_min_size(Pos2::new(lane.left() + 7.0, lane.top() + 32.0 + c as f32 * 62.0), Vec2::new(w - 14.0, 54.0));
                    painter.rect_filled(r, egui::Rounding::same(10.0), fade(p.raised));
                    bar(painter, r.left() + 9.0, r.top() + 12.0, r.width() * 0.66, strong);
                    bar(painter, r.left() + 9.0, r.top() + 24.0, r.width() * 0.8, ink);
                    painter.rect_filled(Rect::from_min_size(Pos2::new(r.left() + 9.0, r.top() + 34.0), Vec2::new(24.0, 10.0)), egui::Rounding::same(5.0), fade(theme::wash(p.group(1), 70)));
                }
            }
        }
        Sketch::Notes | Sketch::Panel => {
            let sheet = Rect::from_min_size(Pos2::new(area.left(), sheet.top()), Vec2::new(area.width() * 0.66, sheet.height()));
            let inner = page(painter, sheet, fade);
            for i in 0..8 {
                let y = inner.top() + 6.0 + i as f32 * (inner.height() - 10.0) / 8.0;
                bar(painter, inner.left(), y, inner.width() * if i % 3 == 2 { 0.45 } else { 0.85 }, if i == 3 { strong } else { ink });
                if i == 3 {
                    painter.rect_filled(Rect::from_center_size(Pos2::new(sheet.right() - 10.0, y + 1.5), Vec2::splat(6.0)), egui::Rounding::same(1.0), blue);
                }
            }
            let note = Rect::from_min_size(Pos2::new(sheet.right() - 24.0, sheet.top() + sheet.height() * 0.44), Vec2::new(area.right() - sheet.right() + 24.0, 52.0));
            painter.rect_filled(note, egui::Rounding::same(theme::R_SM), fade(p.solid));
            painter.rect_stroke(note.shrink(0.5), egui::Rounding::same(theme::R_SM), Stroke::new(1.0_f32, blue));
            bar(painter, note.left() + 8.0, note.top() + 12.0, note.width() - 20.0, strong);
            bar(painter, note.left() + 8.0, note.top() + 24.0, note.width() * 0.5, ink);
            if matches!(kind, Sketch::Panel) {
                tag(painter, Pos2::new(inner.left(), sheet.top() + 12.0), "Notes · Changes", blue);
            }
        }
        Sketch::Revisions => {
            let tint = crate::revision::paper(0);
            painter.rect_filled(sheet, egui::Rounding::same(theme::R_SM), fade(Color32::from_rgb(tint[0], tint[1], tint[2])));
            let inner = sheet.shrink2(Vec2::new(sheet.width() * 0.14, sheet.height() * 0.08));
            let paper_ink = fade(Color32::from_gray(60));
            painter.text(Pos2::new(inner.right(), inner.top() - 2.0), Align2::RIGHT_TOP, "Blue Rev.  12A.", theme::font_page(10.0), paper_ink);
            for i in 0..9 {
                let y = inner.top() + 22.0 + i as f32 * (inner.height() - 30.0) / 9.0;
                bar(painter, inner.left(), y, inner.width() * if i % 2 == 0 { 0.85 } else { 0.55 }, paper_ink);
                if i == 2 || i == 3 || i == 6 {
                    painter.text(Pos2::new(sheet.right() - 12.0, y + 1.5), Align2::CENTER_CENTER, "*", theme::font_page_bold(13.0), paper_ink);
                }
            }
        }
        Sketch::Files => {
            let names = ["the-long-night.md", "crossfire.md", "snapshots/"];
            for (k, n) in names.iter().enumerate() {
                let r = Rect::from_min_size(Pos2::new(area.left(), area.center().y - 72.0 + k as f32 * 52.0), Vec2::new(area.width(), 40.0));
                painter.rect_stroke(r.shrink(0.5), egui::Rounding::same(theme::R_SM), Stroke::new(1.0_f32, fade(p.line_strong)));
                crate::icons::draw(painter, Rect::from_center_size(Pos2::new(r.left() + 20.0, r.center().y), Vec2::splat(14.0)), if k == 2 { crate::icons::Icon::Folder } else { crate::icons::Icon::File }, if k == 0 { blue } else { ink });
                painter.text(Pos2::new(r.left() + 40.0, r.center().y), Align2::LEFT_CENTER, *n, theme::font_mono(theme::T_SM), if k == 0 { strong } else { ink });
            }
        }
        Sketch::Dual => {
            let inner = page(painter, sheet, fade);
            let half = inner.width() * 0.46;
            for (k, x) in [inner.left(), inner.left() + inner.width() * 0.54].into_iter().enumerate() {
                tag(painter, Pos2::new(x + half * 0.3, inner.top() + 30.0), ["Maria", "Jonas"][k], strong);
                for i in 0..4 {
                    bar(painter, x, inner.top() + 46.0 + i as f32 * 9.0, half * if i == 3 { 0.5 } else { 0.95 }, ink);
                }
            }
            painter.line_segment([Pos2::new(sheet.left() - 8.0, inner.top() + 24.0), Pos2::new(sheet.left() - 8.0, inner.top() + 82.0)], Stroke::new(1.5_f32, blue));
            bar(painter, inner.left(), inner.top() + 4.0, inner.width() * 0.8, ink);
            bar(painter, inner.left(), inner.top() + 104.0, inner.width() * 0.9, ink);
        }
        Sketch::Timelines => {
            let rows = [area.center().y - 46.0, area.center().y + 6.0, area.center().y + 52.0];
            let cols: Vec<f32> = (0..5).map(|i| area.left() + 16.0 + i as f32 * (area.width() - 32.0) / 4.0).collect();
            let coral = fade(p.primary);
            let lilac = fade(p.groups[1]);
            let green = fade(p.groups[4 % p.groups.len()]);
            let line = |a: Pos2, b: Pos2, c: Color32| {
                let pts: Vec<Pos2> = (0..=20)
                    .map(|i| {
                        let t = i as f32 / 20.0;
                        let e = t * t * (3.0 - 2.0 * t);
                        Pos2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * e)
                    })
                    .collect();
                painter.add(egui::Shape::line(pts, Stroke::new(2.5_f32, c)));
            };
            line(Pos2::new(cols[0], rows[0]), Pos2::new(cols[4], rows[0]), coral);
            line(Pos2::new(cols[1], rows[0]), Pos2::new(cols[2], rows[1]), lilac);
            line(Pos2::new(cols[2], rows[1]), Pos2::new(cols[3], rows[1]), lilac);
            line(Pos2::new(cols[3], rows[1]), Pos2::new(cols[4], rows[0]), lilac);
            line(Pos2::new(cols[2], rows[1]), Pos2::new(cols[3], rows[2]), green);
            for (k, x) in cols.iter().enumerate() {
                painter.circle_filled(Pos2::new(*x, rows[0]), 7.0, coral);
                painter.text(Pos2::new(*x, rows[0]), Align2::CENTER_CENTER, (k + 1).to_string(), theme::font_bold(theme::T_MICRO), fade(theme::on(p.primary)));
            }
            painter.circle_filled(Pos2::new(cols[2], rows[1]), 7.0, lilac);
            painter.circle_filled(Pos2::new(cols[3], rows[1]), 7.0, fade(p.solid));
            painter.circle_stroke(Pos2::new(cols[3], rows[1]), 7.0, Stroke::new(2.0_f32, lilac));
            painter.circle_filled(Pos2::new(cols[3], rows[2]), 7.0, green);
            tag(painter, Pos2::new(cols[1] + 4.0, rows[1] - 14.0), "She says no", strong);
            tag(painter, Pos2::new(cols[0] - 8.0, rows[0] - 22.0), "Main", ink);
        }
        Sketch::Reports => {
            let field = Rect::from_min_size(Pos2::new(area.left(), area.center().y - 84.0), Vec2::new(area.width(), 30.0));
            painter.rect_stroke(field.shrink(0.5), egui::Rounding::same(theme::R_SM), Stroke::new(1.0_f32, blue));
            crate::icons::draw(painter, Rect::from_center_size(Pos2::new(field.left() + 16.0, field.center().y), Vec2::splat(12.0)), crate::icons::Icon::Search, blue);
            painter.text(Pos2::new(field.left() + 30.0, field.center().y), Align2::LEFT_CENTER, "kettle", theme::font(theme::T_SM), strong);
            let rows = [("Maria", 0.39), ("Cole", 0.36), ("Jonas", 0.25)];
            for (k, (name, share)) in rows.iter().enumerate() {
                let y = field.bottom() + 36.0 + k as f32 * 34.0;
                tag(painter, Pos2::new(area.left(), y), name, strong);
                let track = Rect::from_min_size(Pos2::new(area.left() + area.width() * 0.4, y - 3.0), Vec2::new(area.width() * 0.6, 6.0));
                painter.rect_filled(track, egui::Rounding::same(3.0), fade(p.line));
                painter.rect_filled(Rect::from_min_size(track.min, Vec2::new(track.width() * share * 2.0, 6.0)), egui::Rounding::same(3.0), blue);
            }
        }
        Sketch::Look => {
            let back = Rect::from_center_size(area.center(), Vec2::new(area.width(), area.height().min(area.width() * 1.05)));
            painter.rect_filled(back, egui::Rounding::same(ncf::theme::R_MD), fade(p.sunken));
            let side = Rect::from_min_size(back.min + Vec2::new(10.0, 10.0), Vec2::new(back.width() * 0.24, back.height() - 20.0));
            painter.rect_filled(side, egui::Rounding::same(12.0), fade(p.raised));
            painter.rect_filled(Rect::from_min_size(side.min + Vec2::new(6.0, 8.0), Vec2::new(side.width() - 12.0, 16.0)), egui::Rounding::same(8.0), fade(p.primary));
            for i in 0..4 {
                let y = side.top() + 38.0 + i as f32 * 16.0;
                if i == 1 {
                    painter.rect_filled(Rect::from_min_size(Pos2::new(side.left() + 5.0, y - 5.0), Vec2::new(side.width() - 10.0, 13.0)), egui::Rounding::same(6.5), strong);
                } else {
                    bar(painter, side.left() + 10.0, y, side.width() * 0.5, ink);
                }
            }
            let main = Rect::from_min_max(Pos2::new(side.right() + 8.0, side.top()), Pos2::new(back.right() - 10.0, side.bottom()));
            let tools = Rect::from_min_size(main.min, Vec2::new(main.width(), 22.0));
            painter.rect_filled(tools, egui::Rounding::same(11.0), fade(p.raised));
            painter.rect_filled(Rect::from_min_size(tools.min + Vec2::new(tools.width() * 0.16, 4.0), Vec2::new(tools.width() * 0.16, 14.0)), egui::Rounding::same(7.0), strong);
            painter.rect_filled(Rect::from_min_size(Pos2::new(tools.right() - 34.0, tools.top() + 4.0), Vec2::new(28.0, 14.0)), egui::Rounding::same(7.0), fade(p.primary));
            let tabs_w = main.width() * 0.3;
            let page = Rect::from_min_max(Pos2::new(main.left(), tools.bottom() + 8.0), Pos2::new(main.right() - tabs_w - 8.0, main.bottom() - 24.0));
            painter.rect_filled(page, egui::Rounding::same(ncf::theme::R_SM), fade(if p.dark { p.solid_hi } else { Color32::WHITE }));
            let inner = page.shrink2(Vec2::new(page.width() * 0.14, 14.0));
            for i in 0..8 {
                let y = inner.top() + i as f32 * 13.0;
                if y > inner.bottom() {
                    break;
                }
                bar(painter, inner.left() + if i % 3 == 1 { inner.width() * 0.3 } else { 0.0 }, y, inner.width() * if i == 0 { 0.6 } else if i % 3 == 1 { 0.4 } else { 0.9 }, if i == 0 { strong } else { ink });
            }
            let tabs = Rect::from_min_max(Pos2::new(page.right() + 8.0, page.top()), Pos2::new(main.right(), page.bottom()));
            painter.rect_filled(tabs, egui::Rounding::same(10.0), fade(p.raised));
            for i in 0..4 {
                let y = tabs.top() + 14.0 + i as f32 * 16.0;
                painter.rect_filled(Rect::from_min_size(Pos2::new(tabs.left() + 6.0, y - 4.0), Vec2::new(9.0, 9.0)), egui::Rounding::same(3.0), if i == 0 { fade(p.primary) } else { ink });
                bar(painter, tabs.left() + 20.0, y - 1.0, tabs.width() * 0.55, ink);
            }
            let status = Rect::from_min_size(Pos2::new(main.left(), main.bottom() - 16.0), Vec2::new(main.width(), 16.0));
            painter.rect_filled(status, egui::Rounding::same(8.0), fade(p.raised));
        }
        Sketch::Palette => {
            let win = Rect::from_center_size(area.center(), Vec2::new(area.width(), (area.height() * 0.8).min(250.0)));
            painter.rect_filled(win, egui::Rounding::same(ncf::theme::R_MD), fade(if p.dark { p.solid_hi } else { Color32::WHITE }));
            painter.rect_stroke(win.shrink(0.5), egui::Rounding::same(ncf::theme::R_MD), Stroke::new(1.0_f32, fade(p.line_strong)));
            let field = Rect::from_min_size(win.min + Vec2::new(10.0, 10.0), Vec2::new(win.width() - 20.0, 28.0));
            painter.rect_filled(field, egui::Rounding::same(14.0), fade(p.raised));
            crate::icons::draw(painter, Rect::from_center_size(Pos2::new(field.left() + 16.0, field.center().y), Vec2::splat(12.0)), crate::icons::Icon::Search, ink);
            painter.text(Pos2::new(field.left() + 30.0, field.center().y), Align2::LEFT_CENTER, "garage", theme::font(theme::T_SM), strong);
            let rows = [("Scenes", "INT. GARAGE - NIGHT", true), ("Scenes", "EXT. GARAGE - DAWN", false), ("Commands", "Quick Export (PDF)", false)];
            let mut y = field.bottom() + 10.0;
            let mut last = "";
            for (head, line, on) in rows {
                let first = head != last;
                last = head;
                if first {
                    tag(painter, Pos2::new(win.left() + 14.0, y + 6.0), head, fade(p.text_faint));
                    y += 16.0;
                }
                let r = Rect::from_min_size(Pos2::new(win.left() + 6.0, y), Vec2::new(win.width() - 12.0, 30.0));
                if on {
                    painter.rect_filled(r, egui::Rounding::same(8.0), fade(p.raised));
                }
                let chip = Rect::from_center_size(Pos2::new(r.left() + 16.0, r.center().y), Vec2::splat(20.0));
                painter.rect_filled(chip, egui::Rounding::same(6.0), if on { fade(p.primary) } else { fade(p.raised) });
                let icon = if head == "Commands" { crate::icons::Icon::Sparkle } else { crate::icons::Icon::List };
                crate::icons::draw(painter, chip.shrink(5.0), icon, if on { fade(p.ink) } else { ink });
                painter.text(Pos2::new(chip.right() + 10.0, r.center().y), Align2::LEFT_CENTER, line, theme::font_semi(theme::T_CAP), strong);
                y = r.bottom() + 2.0;
            }
        }
        Sketch::Week => {
            let gap = 10.0;
            let w = (area.width() - gap) / 2.0;
            let h = (area.height() * 0.62).min(w * 1.1);
            let top = area.center().y - h * 0.5;
            let tiles = [(p.tile(1), "This week", "1,240", "words written"), (p.tile(2), "Next deadline", "4 days", "Pilot draft due")];
            for (k, (fill, eyebrow, big, small)) in tiles.into_iter().enumerate() {
                let r = Rect::from_min_size(Pos2::new(area.left() + k as f32 * (w + gap), top), Vec2::new(w, h));
                painter.rect_filled(r, egui::Rounding::same(ncf::theme::R_MD + 4.0), fade(fill));
                tag(painter, Pos2::new(r.left() + 14.0, r.top() + 20.0), eyebrow, fade(p.on_tile));
                painter.text(Pos2::new(r.left() + 14.0, r.bottom() - 40.0), Align2::LEFT_BOTTOM, big, theme::font_display(28.0), fade(p.on_tile));
                tag(painter, Pos2::new(r.left() + 14.0, r.bottom() - 24.0), small, fade(p.on_tile));
            }
        }
        Sketch::Appearance => {
            let w = (area.width() - 12.0) / 2.0;
            let h = (area.height() - 60.0).min(w * 1.3);
            let top = area.center().y - (h + 48.0) * 0.5;
            for (k, (fill, line, icon)) in [
                (Color32::from_rgb(0xF3, 0xF4, 0xF7), Color32::from_rgb(0x45, 0x49, 0x56), crate::icons::Icon::Sun),
                (Color32::from_rgb(0x1A, 0x1B, 0x21), Color32::from_rgb(0xC8, 0xCA, 0xD2), crate::icons::Icon::Moon),
            ]
            .into_iter()
            .enumerate()
            {
                let r = Rect::from_min_size(Pos2::new(area.left() + k as f32 * (w + 12.0), top), Vec2::new(w, h));
                painter.rect_filled(r, egui::Rounding::same(ncf::theme::R_MD), fade(fill));
                painter.rect_stroke(r.shrink(0.5), egui::Rounding::same(ncf::theme::R_MD), Stroke::new(1.0_f32, fade(p.line_strong)));
                crate::icons::draw(painter, Rect::from_center_size(Pos2::new(r.left() + 22.0, r.top() + 22.0), Vec2::splat(14.0)), icon, fade(line));
                for i in 0..5 {
                    bar(painter, r.left() + 16.0, r.top() + 48.0 + i as f32 * 14.0, (w - 32.0) * if i % 2 == 0 { 0.9 } else { 0.55 }, fade(line));
                }
            }
            let pill = Rect::from_center_size(Pos2::new(area.center().x, top + h + 34.0), Vec2::new(area.width().min(240.0), 30.0));
            painter.rect_filled(pill, egui::Rounding::same(15.0), fade(p.raised));
            let seg = (pill.width() - 8.0) / 3.0;
            for (k, name) in ["System", "Light", "Dark"].iter().enumerate() {
                let r = Rect::from_min_size(Pos2::new(pill.left() + 4.0 + k as f32 * seg, pill.top() + 4.0), Vec2::new(seg, 22.0));
                if k == 0 {
                    painter.rect_filled(r, egui::Rounding::same(11.0), strong);
                }
                let wn = tag_w(painter, name);
                tag(painter, Pos2::new(r.center().x - wn * 0.5, r.center().y), name, if k == 0 { fade(p.solid) } else { ink });
            }
        }
        Sketch::Covers => {
            let gap = 12.0;
            let w = (area.width() - gap) / 2.0;
            let h = ((area.height() - gap) / 2.0).min(w * 0.9);
            let top = area.center().y - h - gap * 0.5;
            for (k, initial) in ["L", "C", "T", "N"].iter().enumerate() {
                let r = Rect::from_min_size(Pos2::new(area.left() + (k % 2) as f32 * (w + gap), top + (k / 2) as f32 * (h + gap)), Vec2::new(w, h));
                painter.rect_filled(r, egui::Rounding::same(ncf::theme::R_MD), fade(p.raised));
                let chip = Rect::from_min_size(r.min + Vec2::new(14.0, 14.0), Vec2::splat(34.0));
                painter.rect_filled(chip, egui::Rounding::same(10.0), fade(p.tile(k)));
                painter.text(chip.center(), Align2::CENTER_CENTER, *initial, theme::font_display(17.0), fade(p.on_tile));
                bar(painter, r.left() + 14.0, chip.bottom() + 16.0, w * 0.6, strong);
                bar(painter, r.left() + 14.0, chip.bottom() + 28.0, w * 0.4, ink);
            }
        }
    }
}
