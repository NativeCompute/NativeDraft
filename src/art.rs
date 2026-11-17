use eframe::egui::{self, pos2, Color32, ColorImage, Rect, TextureHandle, TextureOptions, Vec2};

const DARK: &[u8] = include_bytes!("../assets/art/hero-dark.png");
const LIGHT: &[u8] = include_bytes!("../assets/art/hero-light.png");
pub const ASPECT: f32 = 2.5;

fn to8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub fn texture(ctx: &egui::Context, dark: bool) -> Option<TextureHandle> {
    let id = egui::Id::new(("ns-hero-art", dark));
    if let Some(t) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return Some(t);
    }
    let img = ncf::icon::decode(if dark { DARK } else { LIGHT })?;
    let pixels = img.px.iter().map(|p| Color32::from_rgba_premultiplied(to8(p[0]), to8(p[1]), to8(p[2]), to8(p[3]))).collect();
    let tex = ctx.load_texture(if dark { "ns-hero-dark" } else { "ns-hero-light" }, ColorImage { size: [img.width, img.height], pixels }, TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    Some(tex)
}

pub fn fit(rect: Rect) -> Rect {
    let w = rect.width().min(rect.height() * ASPECT);
    Rect::from_center_size(rect.center(), Vec2::new(w, w / ASPECT))
}

pub fn paint_hero(ui: &egui::Ui, rect: Rect) {
    let Some(tex) = texture(ui.ctx(), crate::theme::pal().dark) else {
        return;
    };
    ui.painter().image(tex.id(), fit(rect), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
}

pub fn update_banner() -> &'static [u8] {
    DARK
}
