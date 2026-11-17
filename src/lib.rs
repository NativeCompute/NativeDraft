#[cfg(test)]
mod tests;
#[cfg(test)]
mod teamtests;
#[cfg(test)]
mod uitests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod timelinetests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod cloudtests;

pub mod app;
pub mod art;
pub mod backend;
pub mod cards;
pub mod chrome;
#[cfg(not(target_arch = "wasm32"))]
pub mod cloud;
pub mod editor;
pub mod export;
pub mod export_game;
pub mod fountain;
pub mod keys;
pub mod logo;
pub mod model;
pub mod pages;
pub mod people;
pub mod report;
pub mod revision;
pub mod settings;
#[cfg(not(target_arch = "wasm32"))]
pub mod smoke;
pub mod splash;
pub mod stats;
pub mod storage;
pub mod theme;
pub mod timelines;
pub mod timelines_view;
pub mod ui;

pub use eframe;
pub use ncf::{alerts, anim, caret, icons, update};

pub const MARK: ncf::mark::AppMark = ncf::mark::AppMark::raster("NativeDraft", include_bytes!("../assets/icon/mark.png"));

pub const INFO: ncf::AppInfo = ncf::AppInfo {
    name: "NativeDraft",
    id: "blackline",
    version: env!("CARGO_PKG_VERSION"),
    tagline: "Screenwriting in industry format",
    repo: "NativeCompute/NativeDraft",
    mark: MARK,
    formerly: &["Blackline"],
};

pub const NATIVECOMPUTE_URL: &str = ncf::NATIVECOMPUTE_URL;
pub const YOUTRACK_INFO_URL: &str = "https://www.jetbrains.com/youtrack/";
pub const MAC_INSTALL: &str = "curl -fsSL https://github.com/NativeCompute/NativeDraft/releases/latest/download/install-macos.sh | bash";

pub const WEB: bool = cfg!(target_arch = "wasm32");
