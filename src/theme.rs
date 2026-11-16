#![allow(dead_code)]

use crate::model::Element;

use eframe::egui::{
    self, Color32, FontData, FontFamily, Mesh, Pos2, Rect, Rounding,
    Shape, Stroke, Vec2,
};

// palette here---

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,

    pub backdrop: Color32,

    pub glass: Color32,
    pub glass_hi: Color32,
    pub glass_line: Color32,
    pub glass_edge: Color32,

    pub solid: Color32,
    pub solid_hi: Color32,
    pub sunken: Color32,
    pub raised: Color32,

    pub line: Color32,
    pub line_strong: Color32,

    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    pub ink: Color32,

    pub primary: Color32,
    pub primary_light: Color32,
    pub primary_deep: Color32,
    pub primary_quiet: Color32,
    pub primary_quiet_hi: Color32,
    pub prim_grad: (Color32, Color32),

    pub sec: Color32,
    pub sec_light: Color32,
    pub sec_deep: Color32,
    pub sec_grad: (Color32, Color32),

    pub danger: Color32,
    pub danger_light: Color32,
    pub warn: Color32,
    pub ok: Color32,

    pub groups: [Color32; 6],
    pub brights: [Color32; 6],
    pub on_tile: Color32,
    pub focus: Color32,
    pub frost: Color32,
}

const fn c(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

impl Palette {
    pub fn group(&self, hue: usize) -> Color32 {
        self.groups[hue % self.groups.len()]
    }

    pub fn bright(&self, i: usize) -> Color32 {
        self.brights[i % self.brights.len()]
    }

    pub fn tile(&self, i: usize) -> Color32 {
        self.brights[i % 4]
    }
}


pub use ncf::theme::{
    apply, bold, darken, display, fill_grad_poly, font, font_bold, font_display, font_hud, font_med, font_mono, font_semi,
    grad_poly_shape, grad_rect, hover_surface, hsl, hud, lighten, medium, mix, on, rounded_poly, semibold, tint, top_edge,
    tracked_text, tracked_width, wash, T_BODY, T_CAP, T_H, T_LABEL, T_MICRO, T_SM, T_TITLE,
};

pub fn palette(dark: bool) -> Palette {
    let t = if dark { ncf::tokens::DARK } else { ncf::tokens::LIGHT };
    Palette {
        dark,
        backdrop: t.void,
        glass: t.deep,
        glass_hi: t.panel_2,
        glass_line: t.line,
        glass_edge: t.line_strong,
        solid: t.deep,
        solid_hi: t.panel_2,
        sunken: t.sunken,
        raised: t.panel_2,
        line: t.line,
        line_strong: t.line_strong,
        text: t.text,
        text_dim: t.text_2,
        text_faint: t.text_3,
        ink: t.ink,
        primary: t.signal,
        primary_light: t.signal_hi,
        primary_deep: t.signal_deep,
        primary_quiet: t.signal_soft,
        primary_quiet_hi: t.signal_soft_hi,
        prim_grad: (t.signal, t.signal),
        sec: t.chrome,
        sec_light: t.chrome_hi,
        sec_deep: t.chrome_lo,
        sec_grad: if dark { (t.chrome_lo, t.chrome_hi) } else { (t.chrome_shade, t.chrome) },
        danger: t.danger,
        danger_light: t.danger_hi,
        warn: t.warn,
        ok: t.ok,
        groups: t.groups,
        brights: t.brights,
        on_tile: t.on_tile,
        focus: t.focus,
        frost: t.glass,
    }
}

pub fn pal() -> Palette {
    palette(ncf::theme::pal().dark)
}

pub fn set_palette(dark: bool) {
    ncf::theme::set_palette(dark);
}

pub fn glass_surface(painter: &egui::Painter, rect: Rect, radius: f32, opacity: f32) {
    let _ = opacity;
    ncf::theme::surface(painter, rect, radius, ncf::theme::Lift::Flat);
}

pub fn scrim(t: f32) -> Color32 {
    let s = ncf::theme::pal().scrim();
    wash(s, (s.a() as f32 * t.clamp(0.0, 1.0)) as u8)
}

pub fn window_edge() -> Color32 {
    let p = pal();
    if p.dark {
        p.line
    } else {
        Color32::TRANSPARENT
    }
}

pub fn pill_shape(rect: Rect) -> Shape {
    let p = ncf::theme::pal();
    let r = Rounding::same(rect.height() * 0.5);
    Shape::Vec(vec![
        Shape::Rect(ncf::theme::shadow_of(ncf::theme::Lift::Float).as_shape(rect, r)),
        Shape::rect_filled(rect, r, p.glass),
        Shape::rect_stroke(rect.shrink(0.5), r, Stroke::new(1.0_f32, p.glass_line)),
    ])
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = ncf::theme::font_definitions();
    fonts.font_data.insert("CourierPrime".to_string(), FontData::from_static(PAGE_REGULAR));
    fonts.font_data.insert("CourierPrimeBold".to_string(), FontData::from_static(PAGE_BOLD));
    let mono = fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();
    let mut page_stack = vec!["CourierPrime".to_string()];
    page_stack.extend(mono);
    fonts.families.insert(page(), page_stack.clone());
    let mut bold_stack = vec!["CourierPrimeBold".to_string()];
    bold_stack.extend(page_stack);
    fonts.families.insert(page_bold(), bold_stack);
    ctx.set_fonts(fonts);
}

// metrics here---

pub const R_WINDOW: f32 = ncf::theme::R_SM;
pub const R_SM: f32 = ncf::theme::R_SM;
pub const R_MD: f32 = ncf::theme::R_MD;
pub const R_CTRL: f32 = ncf::theme::R_SM;
pub const R_CARD: f32 = ncf::theme::R_MD;
pub const R_ISLAND: f32 = ncf::theme::R_MD;
pub const R_PANEL: f32 = ncf::theme::R_MD;
pub const R_PILL: f32 = ncf::theme::R_PILL;
pub const R_SHEET: f32 = ncf::theme::R_LG;
pub const R_ROW: f32 = 8.0;

pub const RAIL_W: f32 = 268.0;
pub const OUTLINE_W: f32 = 246.0;
pub const INSPECT_W: f32 = 272.0;
pub const TOPBAR_H: f32 = 92.0;
pub const DOC_MAX: f32 = 820.0;
pub const GAP: f32 = 14.0;


// colour math here---

// shapes here---

pub fn grad_diamond(painter: &egui::Painter, centre: Pos2, r: f32, a: Color32, b: Color32) {
    let pts = vec![
        Pos2::new(centre.x, centre.y - r),
        Pos2::new(centre.x + r, centre.y),
        Pos2::new(centre.x, centre.y + r),
        Pos2::new(centre.x - r, centre.y),
    ];
    fill_grad_poly(painter, &pts, a, b, Vec2::new(0.6, 1.0));
}

pub fn fading_arc(
    painter: &egui::Painter,
    centre: Pos2,
    radius: f32,
    from: f32,
    to: f32,
    width: f32,
    color: Color32,
) {
    let steps = 22;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let a0 = from + (to - from) * t0;
        let a1 = from + (to - from) * t1;
        let mid = (t0 + t1) * 0.5;
        let fade = (mid * std::f32::consts::PI).sin().powf(0.7);
        let col = wash(color, (color.a() as f32 * fade) as u8);
        painter.line_segment(
            [
                Pos2::new(centre.x + a0.cos() * radius, centre.y + a0.sin() * radius),
                Pos2::new(centre.x + a1.cos() * radius, centre.y + a1.sin() * radius),
            ],
            Stroke::new(width, col),
        );
    }
}

// bloom here---

fn bloom_rings(spread: f32) -> usize {
    ((spread * 0.40) as usize).clamp(10, 30)
}

fn falloff(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let near = (-3.2 * t).exp();
    let far = (1.0 - t).powf(2.6);
    (0.22 * near + 0.78 * far) * 0.82
}

const GLOWS: bool = false;

fn bloom_mesh<F>(
    painter: &egui::Painter,
    centre: Pos2,
    spread: f32,
    color: Color32,
    strength: f32,
    ring: F,
) where
    F: Fn(f32) -> Vec<Pos2>,
{
    if !GLOWS || strength <= 0.002 || spread <= 0.1 {
        return;
    }
    let peak = (strength.clamp(0.0, 2.0) * 62.0).min(255.0) as u8;
    let rings = bloom_rings(spread);

    let mut mesh = Mesh::default();
    mesh.colored_vertex(centre, wash(color, peak));

    let mut prev: Option<(u32, usize)> = None;
    for k in 0..=rings {
        let t = k as f32 / rings as f32;
        let pts = ring(spread * t);
        if pts.len() < 3 {
            return;
        }
        let col = wash(color, (peak as f32 * falloff(t)) as u8);
        let base = mesh.vertices.len() as u32;
        for q in &pts {
            mesh.colored_vertex(*q, col);
        }
        let n = pts.len() as u32;
        match prev {
            None => {
                for i in 0..n {
                    mesh.add_triangle(0, base + i, base + (i + 1) % n);
                }
            }
            Some((pbase, pcount)) => {
                if pcount != pts.len() {
                    return;
                }
                for i in 0..n {
                    let j = (i + 1) % n;
                    mesh.add_triangle(pbase + i, base + i, base + j);
                    mesh.add_triangle(pbase + i, base + j, pbase + j);
                }
            }
        }
        prev = Some((base, pts.len()));
    }
    painter.add(Shape::mesh(mesh));
}

const BLOOM_SEGMENTS: usize = 44;

pub fn glow_circle(
    painter: &egui::Painter,
    centre: Pos2,
    r: f32,
    color: Color32,
    spread: f32,
    strength: f32,
) {
    bloom_mesh(painter, centre, spread, color, strength, |grow| {
        let rr = r + grow;
        (0..BLOOM_SEGMENTS)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / BLOOM_SEGMENTS as f32;
                Pos2::new(centre.x + a.cos() * rr, centre.y + a.sin() * rr)
            })
            .collect()
    });
}

pub fn glow_rect(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    color: Color32,
    spread: f32,
    strength: f32,
) {
    let steps = 9usize;
    bloom_mesh(painter, rect.center(), spread, color, strength, move |grow| {
        let r = rect.expand(grow);
        let rad = (radius + grow).min(r.width() * 0.5).min(r.height() * 0.5);
        let quarter = std::f32::consts::FRAC_PI_2;
        let corners = [
            (Pos2::new(r.right() - rad, r.top() + rad), -quarter),
            (Pos2::new(r.right() - rad, r.bottom() - rad), 0.0),
            (Pos2::new(r.left() + rad, r.bottom() - rad), quarter),
            (Pos2::new(r.left() + rad, r.top() + rad), std::f32::consts::PI),
        ];
        let mut pts = Vec::with_capacity(steps * 4 + 4);
        for (c, a0) in corners {
            for i in 0..=steps {
                let a = a0 + quarter * (i as f32 / steps as f32);
                pts.push(Pos2::new(c.x + a.cos() * rad, c.y + a.sin() * rad));
            }
        }
        pts
    });
}

pub fn glow_diamond(
    painter: &egui::Painter,
    centre: Pos2,
    r: f32,
    color: Color32,
    spread: f32,
    strength: f32,
) {
    bloom_mesh(painter, centre, spread, color, strength, move |grow| {
        let rr = r + grow;
        let n = 1.0 + 1.15 * (grow / spread.max(0.001)).clamp(0.0, 1.0);
        (0..BLOOM_SEGMENTS)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / BLOOM_SEGMENTS as f32;
                let (c, s) = (a.cos(), a.sin());
                let d = (c.abs().powf(n) + s.abs().powf(n)).powf(1.0 / n);
                let k = rr / d.max(0.001);
                Pos2::new(centre.x + c * k, centre.y + s * k)
            })
            .collect()
    });
}
pub fn star_points(centre: Pos2, r: f32, waist: f32) -> [Pos2; 8] {
    let w = r * waist * std::f32::consts::FRAC_1_SQRT_2;
    [
        Pos2::new(centre.x, centre.y - r),
        Pos2::new(centre.x + w, centre.y - w),
        Pos2::new(centre.x + r, centre.y),
        Pos2::new(centre.x + w, centre.y + w),
        Pos2::new(centre.x, centre.y + r),
        Pos2::new(centre.x - w, centre.y + w),
        Pos2::new(centre.x - r, centre.y),
        Pos2::new(centre.x - w, centre.y - w),
    ]
}

pub fn grad_star(painter: &egui::Painter, centre: Pos2, r: f32, _a: Color32, b: Color32) {
    painter.add(marker(centre, r, b));
}

pub fn marker(centre: Pos2, r: f32, color: Color32) -> Shape {
    Shape::circle_filled(centre, (r * 0.6).max(2.0), color)
}

pub fn star_shapes(centre: Pos2, r: f32, _waist: f32, _a: Color32, b: Color32) -> Vec<Shape> {
    vec![marker(centre, r, b)]
}

pub fn star_kites(centre: Pos2, r: f32, waist: f32, a: Color32, b: Color32) -> Vec<Shape> {
    let pts = star_points(centre, r, waist);
    let dir = Vec2::new(0.6, 1.0).normalized();
    let (lo, hi) = (
        (centre.to_vec2() - Vec2::splat(r)).dot(dir),
        (centre.to_vec2() + Vec2::splat(r)).dot(dir),
    );
    let at = |p: Pos2| mix(a, b, (p.to_vec2().dot(dir) - lo) / (hi - lo).max(0.0001));
    (0..4)
        .map(|k| {
            let tip = pts[k * 2];
            let right = pts[k * 2 + 1];
            let left = pts[(k * 2 + 7) % 8];
            let kite = [tip, right, centre, left];
            let (ca, cb) = (at(tip), at(centre));
            let kd = centre - tip;
            let kd = if kd.length_sq() < 1e-6 { dir } else { kd };
            grad_poly_shape(&kite, ca, cb, kd)
        })
        .collect()
}

pub fn glow_star(
    painter: &egui::Painter,
    centre: Pos2,
    r: f32,
    color: Color32,
    spread: f32,
    strength: f32,
) {
    bloom_mesh(painter, centre, spread, color, strength, move |grow| {
        let rr = r + grow;
        let n = 0.72 + 1.43 * (grow / spread.max(0.001)).clamp(0.0, 1.0);
        (0..BLOOM_SEGMENTS)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / BLOOM_SEGMENTS as f32;
                let (c, s) = (a.cos(), a.sin());
                let d = (c.abs().powf(n) + s.abs().powf(n)).powf(1.0 / n);
                let k = rr / d.max(0.001);
                Pos2::new(centre.x + c * k, centre.y + s * k)
            })
            .collect()
    });
}

pub fn glass_body(opacity: f32) -> Color32 {
    let _ = opacity;
    pal().glass
}

pub fn glass_plate(painter: &egui::Painter, rect: Rect, radius: f32, opacity: f32) {
    let _ = opacity;
    ncf::theme::glass(painter, rect, radius);
}

pub fn island_frame(radius: f32) -> egui::Frame {
    let lift = if pal().dark { ncf::theme::Lift::Flat } else { ncf::theme::Lift::Rest };
    ncf::theme::surface_frame(radius, lift)
}

pub fn sheet_frame() -> egui::Frame {
    let p = pal();
    let lift = if p.dark { ncf::theme::Lift::Rest } else { ncf::theme::Lift::Float };
    ncf::theme::surface_frame(R_SHEET, lift).fill(if p.dark { p.solid } else { p.raised })
}

pub fn lift_shadow(painter: &egui::Painter, rect: Rect, radius: f32, lift: f32) {
    if !GLOWS {
        let level = if lift > 0.5 { ncf::theme::Lift::Float } else if lift > 0.01 { ncf::theme::Lift::Rest } else { ncf::theme::Lift::Flat };
        ncf::theme::shadow(painter, rect, radius, level);
        return;
    }
    let p = pal();
    let lift = lift.clamp(0.0, 1.0);
    if lift <= 0.01 {
        return;
    }
    let rounding = Rounding::same(radius.min(rect.width() * 0.5).min(rect.height() * 0.5));
    let base = if p.dark { 1.0 } else { 0.32 };
    for (dy, blur, alpha) in [(1.0_f32, 2.0_f32, 60.0_f32), (4.0, 10.0, 38.0), (12.0, 28.0, 26.0)] {
        painter.add(
            egui::epaint::Shadow {
                offset: Vec2::new(0.0, dy * (0.5 + 0.5 * lift)),
                blur: blur * (0.6 + 0.4 * lift),
                spread: 0.0,
                color: Color32::from_black_alpha((alpha * base * lift) as u8),
            }
            .as_shape(rect, rounding),
        );
    }
}

// visuals here---

// script colours here---

pub fn element_color(e: Element) -> Color32 {
    let p = pal();
    match e {
        Element::SceneHeading => p.sec_light,
        Element::Action => p.text_faint,
        Element::Character => p.primary_light,
        Element::Parenthetical => p.group(3),
        Element::Dialogue => p.group(2),
        Element::Transition => p.group(4),
        Element::Shot => p.group(5),
        Element::Act => p.primary_light,
    }
}

pub fn element_band(e: Element) -> Color32 {
    let p = pal();
    match e {
        Element::Action | Element::Act => Color32::TRANSPARENT,
        _ => tint(element_color(e), if p.dark { 0.035 } else { 0.05 }),
    }
}

pub fn character_colors(names: &[String], seed: u32) -> std::collections::HashMap<String, Color32> {
    voice_colors(names, seed, &[])
}

fn voice_sl() -> (f32, f32) {
    if pal().dark {
        (0.86, 0.72)
    } else {
        (0.70, 0.36)
    }
}

pub const BENTO_HUES: [f32; 6] = [14.0, 208.0, 253.0, 41.0, 150.0, 340.0];

pub fn hues(names: &[String], seed: u32) -> Vec<f32> {
    let turn = seed as usize % BENTO_HUES.len();
    let start = (seed as f32 * 0.137_5).fract();
    (0..names.len())
        .map(|k| {
            if k < BENTO_HUES.len() {
                BENTO_HUES[(k + turn) % BENTO_HUES.len()] / 360.0
            } else {
                (start + k as f32 * 0.618_034).fract()
            }
        })
        .collect()
}

pub fn voice_colors(names: &[String], seed: u32, chosen: &[(String, u16)]) -> std::collections::HashMap<String, Color32> {
    let (s, l) = voice_sl();
    with_chosen(voices_at(names, seed, s, l), chosen, s, l)
}

pub fn voice_inks(names: &[String], seed: u32, chosen: &[(String, u16)]) -> std::collections::HashMap<String, [u8; 3]> {
    with_chosen(voices_at(names, seed, 0.70, 0.34), chosen, 0.70, 0.34)
        .into_iter()
        .map(|(n, c)| (n, [c.r(), c.g(), c.b()]))
        .collect()
}

pub fn voice_of_hue(hue: u16) -> Color32 {
    let (s, l) = voice_sl();
    hsl(hue as f32 / 360.0, s, l)
}

fn with_chosen(
    mut dealt: std::collections::HashMap<String, Color32>,
    chosen: &[(String, u16)],
    s: f32,
    l: f32,
) -> std::collections::HashMap<String, Color32> {
    for (name, hue) in chosen {
        if let Some(c) = dealt.get_mut(name) {
            *c = hsl(*hue as f32 / 360.0, s, l);
        }
    }
    dealt
}

pub fn character_inks(names: &[String], seed: u32) -> std::collections::HashMap<String, [u8; 3]> {
    voice_inks(names, seed, &[])
}

fn voices_at(names: &[String], seed: u32, s: f32, l: f32) -> std::collections::HashMap<String, Color32> {
    names.iter().zip(hues(names, seed)).map(|(n, h)| (n.clone(), hsl(h, s, l))).collect()
}

fn c3(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

pub fn element_ink(e: Element) -> Color32 {
    let p = pal();
    match e {
        Element::SceneHeading | Element::Shot | Element::Character | Element::Act => p.text,
        Element::Dialogue => mix(p.text, p.text_dim, 0.15),
        Element::Action => mix(p.text, p.text_dim, 0.35),
        Element::Parenthetical | Element::Transition => p.text_dim,
    }
}

// fonts here---


const PAGE_REGULAR: &[u8] = include_bytes!("../assets/fonts/CourierPrime-Regular.ttf");
const PAGE_BOLD: &[u8] = include_bytes!("../assets/fonts/CourierPrime-Bold.ttf");

pub const PAGE_FONT_NAME: &str = "Courier Prime";

pub fn page() -> FontFamily {
    FontFamily::Name("page".into())
}
pub fn page_bold() -> FontFamily {
    FontFamily::Name("page-bold".into())
}

pub fn font_page(size: f32) -> egui::FontId {
    egui::FontId::new(size, page())
}
pub fn font_page_bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, page_bold())
}

pub fn paper_sheet(rect: Rect) -> Shape {
    let p = pal();
    let r = Rounding::same(ncf::theme::R_MD);
    let mut v = Vec::new();
    if !p.dark {
        v.push(Shape::Rect(ncf::theme::shadow_of(ncf::theme::Lift::Rest).as_shape(rect, r)));
    }
    v.push(Shape::rect_filled(rect, r, if p.dark { p.raised } else { Color32::WHITE }));
    v.push(Shape::rect_stroke(rect.shrink(0.5), r, Stroke::new(1.0_f32, p.line)));
    Shape::Vec(v)
}

