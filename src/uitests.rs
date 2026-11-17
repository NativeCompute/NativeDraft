use std::time::Duration;

use crate::app::{App, Mode};
use crate::editor;
use crate::model::{Document, Element};
use crate::storage;
use crate::tests::VAULT;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect};

static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

struct Harness {
    ctx: egui::Context,
    app: App,
    size: egui::Vec2,
    _guard: std::sync::MutexGuard<'static, ()>,
}

fn sandbox(settings: &str) -> std::sync::MutexGuard<'static, ()> {
    let guard = VAULT.lock().unwrap_or_else(|e| e.into_inner());
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("nativedraft-ui-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(dir.join("nativedraft"));
    std::env::set_var("XDG_DATA_HOME", &dir);
    let _ = std::fs::write(
        dir.join("nativedraft/settings.conf"),
        format!("splash = no\nanimations = no\nautosave_ms = 150\nonboarded = new-look\n{settings}"),
    );
    guard
}

impl Harness {
    fn new() -> Harness {
        Harness::with(1320.0, 900.0, "")
    }

    fn with(w: f32, h: f32, settings: &str) -> Harness {
        let guard = sandbox(settings);
        let ctx = egui::Context::default();
        let warm = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(w, h))),
            ..Default::default()
        };
        let _ = ctx.run(warm, |_| {});
        let app = App::with_context(&ctx);
        let mut h = Harness {
            ctx,
            app,
            size: vec2(w, h),
            _guard: guard,
        };
        h.frames(3);
        h
    }

    fn frame_with(&mut self, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), self.size)),
            events,
            ..Default::default()
        };
        let ctx = self.ctx.clone();
        let app = &mut self.app;
        let _ = ctx.run(input, |c| app.frame(c));
    }

    fn frame(&mut self) {
        self.frame_with(Vec::new());
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame();
        }
    }

    fn type_text(&mut self, s: &str) {
        self.frame_with(vec![Event::Text(s.to_string())]);
        self.frame();
    }

    fn press(&mut self, key: Key, modifiers: Modifiers) {
        self.frame_with(vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }]);
        self.frames(2);
    }

    fn click(&mut self, at: Pos2) {
        self.frame_with(vec![Event::PointerMoved(at)]);
        self.frame_with(vec![
            Event::PointerMoved(at),
            Event::PointerButton {
                pos: at,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        self.frame_with(vec![Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        self.frame();
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.frame_with(vec![Event::PointerMoved(from)]);
        self.frame_with(vec![
            Event::PointerMoved(from),
            Event::PointerButton {
                pos: from,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        for i in 1..=6 {
            let t = i as f32 / 6.0;
            let p = pos2(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
            self.frame_with(vec![Event::PointerMoved(p)]);
        }
        self.frame_with(vec![Event::PointerButton {
            pos: to,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        self.frames(2);
    }

    fn focus(&mut self, id: u64) {
        self.app.focus_on(id);
        self.frames(3);
        assert_eq!(self.app.focus_block(), Some(id), "the block should have the caret");
    }

    fn block_rect(&self, id: u64) -> Option<Rect> {
        self.ctx.read_response(editor::block_id(id)).map(|r| r.rect)
    }

    fn confirm(&mut self) {
        assert!(self.app.debug_pending_ask(), "expected the app to ask before doing that");
        self.app.debug_answer_yes(None);
        self.frames(2);
    }

    fn saved_text(&self) -> String {
        std::fs::read_to_string(self.app.path().expect("a path")).unwrap_or_default()
    }

    fn idle(&mut self) {
        std::thread::sleep(Duration::from_millis(200));
        self.frames(2);
    }

    fn load(&mut self, doc: Document) {
        let path = storage::unique_path(&doc.meta.title, None);
        storage::save(&path, &doc).unwrap();
        std::thread::sleep(Duration::from_millis(1600));
        self.frames(2);
        assert!(
            self.app.entries().iter().any(|e| e.path == path),
            "the library should pick up a script added from outside"
        );
        self.app.debug_open(path);
        self.frames(3);
    }
}

fn three_scenes() -> Document {
    let mut d = Document::default();
    d.blocks.clear();
    d.meta.title = "Three Scenes".into();
    d.push(Element::SceneHeading, "INT. ONE - DAY");
    d.push(Element::Character, "DETECTIVE COLE");
    d.push(Element::Dialogue, "One.");
    d.push(Element::SceneHeading, "EXT. TWO - NIGHT");
    d.push(Element::Action, "Two.");
    d.push(Element::SceneHeading, "INT. THREE - DAWN");
    d.push(Element::Action, "Three.");
    d.reseed_ids();
    d
}

fn ids(app: &App) -> Vec<u64> {
    app.doc().blocks.iter().map(|b| b.id).collect()
}

// library here---

#[test]
fn a_first_run_opens_on_a_starter_script_that_is_on_disk() {
    let h = Harness::new();
    assert_eq!(h.app.entry_count(), 1);
    assert!(h.app.path().unwrap().exists());
    assert!(h.saved_text().contains("## INT. EDIT BAY - NIGHT"));
    assert_eq!(h.app.doc().scenes().len(), 1);
}

#[test]
fn ctrl_n_makes_a_new_script_with_its_title_ready_to_type_over() {
    let mut h = Harness::new();
    h.press(Key::N, Modifiers::COMMAND);
    assert_eq!(h.app.entry_count(), 2);
    assert_eq!(h.app.doc().meta.title, "Untitled Script");
    h.type_text("Heist");
    assert_eq!(h.app.doc().meta.title, "Heist");
    h.press(Key::S, Modifiers::COMMAND);
    assert!(h.app.path().unwrap().ends_with("heist.md"), "the file follows the title");
}

#[test]
fn renaming_a_script_keeps_the_cover_it_had() {
    let mut h = Harness::new();
    h.press(Key::N, Modifiers::COMMAND);
    let before = crate::storage::stem_cover(&h.app.path().unwrap());
    h.type_text("A title that hashes elsewhere");
    h.press(Key::S, Modifiers::COMMAND);
    assert!(h.app.path().unwrap().ends_with("a-title-that-hashes-elsewhere.md"));
    assert_eq!(h.app.doc().meta.cover, Some(before));
    assert!(h.saved_text().contains(&format!("cover: {}", crate::storage::COVERS[before])));
}

#[test]
fn a_cover_picked_on_an_open_script_is_saved_with_it() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let path = h.app.path().unwrap();
    h.app.debug_set_cover(&path, 3);
    h.frames(2);
    assert_eq!(h.app.doc().meta.cover, Some(3));
    assert!(h.saved_text().contains("cover: sky"));
}

#[test]
fn deleting_a_script_from_the_menu_asks_first() {
    let mut h = Harness::new();
    h.load(three_scenes());
    assert_eq!(h.app.entry_count(), 2);
    let doomed = h.app.path().unwrap();
    let menu = h.app.debug_menu_button();
    h.click(menu.center());
    assert!(h.app.debug_popover_open(), "the menu should open");
    let mut last = None;
    for n in 0..30 {
        if let Some(r) = h.app.debug_menu_item(n) {
            last = Some(r);
        }
    }
    h.click(last.expect("menu rows").center());
    assert!(doomed.exists(), "nothing is destroyed on a single click");
    h.confirm();
    assert!(!doomed.exists());
    assert_eq!(h.app.entry_count(), 1);
    assert_ne!(h.app.path(), Some(doomed), "another script is opened in its place");
}

#[test]
fn the_library_button_goes_home_and_a_card_opens_the_script() {
    let mut h = Harness::new();
    let starter = h.app.path().unwrap();
    h.load(three_scenes());
    let other = h.app.path().unwrap();
    let button = h.app.debug_library_button();
    assert!(button.width() > 60.0, "a labelled button, not just an icon: {button:?}");
    h.click(button.center());
    h.frames(2);
    assert!(h.app.is_home());
    h.app.debug_open_from_home(starter.clone());
    h.frames(2);
    assert!(!h.app.is_home());
    assert_eq!(h.app.path(), Some(starter));
    assert_ne!(h.app.path(), Some(other));
}

#[test]
fn ctrl_shift_f_searches_every_script_from_home() {
    let mut h = Harness::new();
    h.load(three_scenes());
    h.press(Key::F, Modifiers::COMMAND | Modifiers::SHIFT);
    h.frames(3);
    assert!(h.app.is_home());
    assert!(h.ctx.memory(|m| m.has_focus(egui::Id::new("ns-home-search"))));
}

// palette here---

#[test]
fn ctrl_k_opens_the_palette_and_typing_never_reaches_the_page() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let id = h.app.doc().blocks[2].id;
    h.focus(id);
    let before = h.app.doc().clone();
    h.press(Key::K, Modifiers::COMMAND);
    h.frames(2);
    assert!(h.app.debug_palette_open());
    assert!(h.app.debug_palette_rect().width() > 300.0);
    assert_eq!(h.app.debug_note_open(), None, "Ctrl+K no longer adds a note");
    h.type_text("zz");
    h.press(Key::Enter, Modifiers::NONE);
    h.frames(2);
    assert_eq!(h.app.doc().blocks, before.blocks, "nothing typed in the palette reaches the page");
    h.press(Key::Escape, Modifiers::NONE);
    h.frames(2);
    assert!(!h.app.debug_palette_open(), "Esc closes it");
    h.type_text(" Again.");
    assert!(h.app.doc().blocks[2].text.ends_with("Again."), "and the caret is back where it was");
}

#[test]
fn the_palette_finds_a_scene_and_enter_takes_you_there() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let rows = h.app.debug_palette_rows("three");
    assert!(rows.iter().any(|(s, t)| s == "Scenes" && t == "INT. THREE - DAWN"), "{rows:?}");
    h.press(Key::K, Modifiers::COMMAND);
    h.frames(2);
    h.type_text("dawn");
    h.frames(1);
    h.press(Key::Enter, Modifiers::NONE);
    h.frames(3);
    assert!(!h.app.debug_palette_open());
    assert_eq!(h.app.mode(), Mode::Write);
    let want = h.app.doc().blocks[5].id;
    assert_eq!(h.app.debug_focus_block(), Some(want));
}

#[test]
fn the_palette_runs_a_command_and_arrows_move_the_pick() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let rows = h.app.debug_palette_rows("title page");
    assert_eq!(rows.first().map(|r| r.0.as_str()), Some("Commands"), "{rows:?}");
    h.press(Key::K, Modifiers::COMMAND);
    h.frames(2);
    h.type_text("title page");
    h.frames(1);
    h.press(Key::Enter, Modifiers::NONE);
    h.frames(2);
    assert!(!h.app.debug_palette_open());
    assert!(h.app.debug_title_sheet_open(), "Enter ran the command");
    h.press(Key::Escape, Modifiers::NONE);
    h.frames(2);
    h.press(Key::K, Modifiers::COMMAND);
    h.frames(2);
    let rows = h.app.debug_palette_row_rects();
    assert!(rows.len() >= 2, "{rows:?}");
    h.press(Key::ArrowDown, Modifiers::NONE);
    h.press(Key::ArrowUp, Modifiers::NONE);
    h.press(Key::ArrowUp, Modifiers::NONE);
    h.frames(1);
    assert!(h.app.debug_palette_open(), "arrows move the pick, they never close it");
    h.press(Key::K, Modifiers::COMMAND);
    h.frames(2);
    assert!(!h.app.debug_palette_open(), "Ctrl+K again puts it away");
}

#[test]
fn the_palette_opens_from_home_and_a_click_outside_closes_it() {
    let mut h = Harness::new();
    h.load(three_scenes());
    h.app.debug_go_home();
    h.frames(2);
    h.press(Key::K, Modifiers::COMMAND);
    h.frames(3);
    assert!(h.app.debug_palette_open());
    let rows = h.app.debug_palette_rows("");
    assert!(rows.iter().any(|(s, t)| s == "Scripts" && t == "Three Scenes"), "{rows:?}");
    assert!(rows.iter().all(|(s, _)| s != "Scenes"), "no scenes from Home");
    let rect = h.app.debug_palette_rect();
    h.click(Pos2::new(rect.center().x, rect.bottom() + 40.0));
    h.frames(2);
    assert!(!h.app.debug_palette_open());
}

// writing here---

#[test]
fn typing_in_a_block_reaches_the_file() {
    let mut h = Harness::new();
    let id = h.app.doc().blocks[1].id;
    h.focus(id);
    h.type_text(" Rain.");
    assert!(h.app.doc().blocks[1].text.ends_with("Rain."));
    assert!(h.app.is_dirty());
    h.idle();
    assert!(!h.app.is_dirty(), "autosave ran");
    assert!(h.saved_text().contains("empty document. Rain."));
}

#[test]
fn enter_makes_the_element_that_comes_next() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let cue = h.app.doc().blocks[1].id;
    h.focus(cue);
    h.press(Key::Enter, Modifiers::NONE);
    assert_eq!(h.app.doc().blocks[2].element, Element::Dialogue, "Character → Dialogue");
    h.type_text("Hello.");
    h.press(Key::Enter, Modifiers::NONE);
    assert_eq!(h.app.doc().blocks[3].element, Element::Action, "Dialogue → Action");
    h.press(Key::Tab, Modifiers::NONE);
    assert_eq!(h.app.doc().blocks[3].element, Element::Character);
    h.press(Key::Enter, Modifiers::SHIFT);
    assert_eq!(
        h.app.doc().blocks[4].element,
        Element::Character,
        "Shift+Enter keeps the type rather than moving on to dialogue"
    );
}

#[test]
fn tab_cycles_and_ctrl_digits_set_the_element() {
    let mut h = Harness::new();
    let id = h.app.doc().blocks[1].id;
    h.focus(id);
    h.press(Key::Tab, Modifiers::NONE);
    assert_eq!(h.app.doc().blocks[1].element, Element::Character);
    assert_eq!(h.app.doc().blocks[1].text, h.app.doc().blocks[1].text.to_uppercase(), "a cue is capitals");
    h.press(Key::Tab, Modifiers::SHIFT);
    assert_eq!(h.app.doc().blocks[1].element, Element::Action);
    h.press(Key::Num6, Modifiers::COMMAND);
    assert_eq!(h.app.doc().blocks[1].element, Element::Transition);
    h.press(Key::Z, Modifiers::COMMAND);
    h.press(Key::Z, Modifiers::COMMAND);
    assert_ne!(h.app.doc().blocks[1].element, Element::Transition, "undo walks it back");
}

#[test]
fn smart_type_offers_a_name_and_tab_takes_it() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let dialogue = h.app.doc().blocks[2].id;
    h.focus(dialogue);
    h.press(Key::Enter, Modifiers::NONE);
    h.press(Key::Tab, Modifiers::NONE);
    let cue = h.app.focus_block().unwrap();
    assert_eq!(h.app.doc().block(cue).unwrap().element, Element::Character);
    h.type_text("DET");
    h.press(Key::Tab, Modifiers::NONE);
    assert_eq!(h.app.doc().block(cue).unwrap().text, "DETECTIVE COLE");
    assert_eq!(h.app.doc().block(cue).unwrap().element, Element::Character, "Tab took the name, not the next type");
}

#[test]
fn a_multi_line_paste_fans_out_into_typed_blocks() {
    let mut h = Harness::new();
    let id = h.app.doc().blocks[1].id;
    h.focus(id);
    let before = h.app.doc().blocks.len();
    h.frame_with(vec![Event::Paste(
        "\nINT. CAR - DAY\nMARIA\n(beat)\nDrive.\nCUT TO:".to_string(),
    )]);
    h.frames(3);
    let kinds: Vec<Element> = h.app.doc().blocks.iter().map(|b| b.element).collect();
    assert_eq!(h.app.doc().blocks.len(), before + 5);
    assert_eq!(
        kinds[2..7],
        [Element::SceneHeading, Element::Character, Element::Parenthetical, Element::Dialogue, Element::Transition]
    );
}

#[test]
fn backspace_at_the_head_of_an_empty_block_removes_it() {
    let mut h = Harness::new();
    let id = h.app.doc().blocks[1].id;
    h.focus(id);
    h.press(Key::Enter, Modifiers::NONE);
    let n = h.app.doc().blocks.len();
    h.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(h.app.doc().blocks.len(), n - 1);
    assert_eq!(h.app.focus_block(), Some(id));
}

#[test]
fn alt_arrows_move_the_block() {
    let mut h = Harness::new();
    let before = ids(&h.app);
    h.focus(before[2]);
    h.press(Key::ArrowUp, Modifiers::ALT);
    let after = ids(&h.app);
    assert_eq!(after[1], before[2]);
    assert_eq!(after[2], before[1]);
}

#[test]
fn ctrl_enter_opens_a_new_scene_after_this_one() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let in_first = h.app.doc().blocks[2].id;
    h.focus(in_first);
    h.press(Key::Enter, Modifiers::COMMAND);
    let sc = h.app.doc().scenes();
    assert_eq!(sc.len(), 4);
    assert_eq!(sc[1].heading, "", "a fresh heading straight after the first scene");
    assert_eq!(sc[2].heading, "EXT. TWO - NIGHT");
    h.type_text("INT. NEW - DAY");
    assert_eq!(h.app.doc().scenes()[1].heading, "INT. NEW - DAY");
}

#[test]
fn page_text_size_follows_ctrl_plus_and_minus_and_is_remembered() {
    let mut h = Harness::new();
    let px = h.app.settings().page_px;
    h.press(Key::Plus, Modifiers::COMMAND);
    h.press(Key::Plus, Modifiers::COMMAND);
    assert_eq!(h.app.settings().page_px, px + 2.0);
    h.press(Key::Minus, Modifiers::COMMAND);
    assert_eq!(h.app.settings().page_px, px + 1.0);
    assert!(storage::read_settings().page_px == px + 1.0);
}

#[test]
fn page_breaks_are_marked_where_the_pdf_breaks() {
    let mut h = Harness::new();
    let mut d = three_scenes();
    for _ in 0..90 {
        d.push(Element::Action, "They run, and they keep running until the corridor ends.");
    }
    d.reseed_ids();
    h.load(d);
    let starts = h.app.debug_page_starts();
    assert_eq!(starts.len() + 1, crate::export::page_count(h.app.doc()));
    assert!(starts.iter().any(|(_, n)| *n == 2));
}

// other two views here---

#[test]
fn cards_move_a_whole_scene_by_dragging_its_head() {
    let mut h = Harness::new();
    h.load(three_scenes());
    h.app.set_mode(Mode::Cards);
    h.frames(3);
    let first = h.app.debug_card_rect(0).expect("a card");
    let third = h.app.debug_card_rect(2).expect("three cards");
    h.drag(first.center_top() + vec2(0.0, 14.0), third.right_center() + vec2(10.0, 0.0));
    let order: Vec<String> = h.app.doc().scenes().iter().map(|s| s.heading.clone()).collect();
    assert_eq!(order, vec!["EXT. TWO - NIGHT", "INT. THREE - DAWN", "INT. ONE - DAY"]);
    assert_eq!(h.app.doc().blocks.last().unwrap().text, "One.");
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.doc().scenes()[0].heading, "INT. ONE - DAY", "and Ctrl+Z puts it back");
}

#[test]
fn clicking_a_card_opens_its_scene_to_write() {
    let mut h = Harness::new();
    h.load(three_scenes());
    h.app.set_mode(Mode::Cards);
    h.frames(3);
    let second = h.app.debug_card_rect(1).unwrap();
    h.click(second.center_top() + vec2(0.0, 14.0));
    assert_eq!(h.app.mode(), Mode::Write);
    h.frames(3);
    let heading = h.app.doc().scenes()[1].id;
    assert_eq!(h.app.focus_block(), Some(heading));
}

#[test]
fn a_synopsis_typed_on_a_card_is_saved_with_the_scene() {
    let mut h = Harness::new();
    h.load(three_scenes());
    h.app.set_mode(Mode::Cards);
    h.frames(3);
    let card = h.app.debug_card_rect(0).unwrap();
    h.click(card.center());
    h.type_text("Cole gets the call.");
    h.idle();
    assert_eq!(h.app.doc().scenes()[0].synopsis, "Cole gets the call.");
    assert!(h.saved_text().contains("<!-- scene: Cole gets the call. -->"));
}

#[test]
fn pages_draw_the_title_page_and_every_printed_page() {
    let mut h = Harness::new();
    let mut d = three_scenes();
    for _ in 0..70 {
        d.push(Element::Action, "Line after line after line of action on the page.");
    }
    d.reseed_ids();
    h.load(d);
    h.app.set_mode(Mode::Read);
    h.frames(3);
    assert_eq!(h.app.debug_sheet_count(), 1 + crate::export::page_count(h.app.doc()));
}

#[test]
fn ctrl_g_steps_through_the_four_views() {
    let mut h = Harness::new();
    assert_eq!(h.app.mode(), Mode::Write);
    h.press(Key::G, Modifiers::COMMAND);
    assert_eq!(h.app.mode(), Mode::Cards);
    h.press(Key::G, Modifiers::COMMAND);
    assert_eq!(h.app.mode(), Mode::Timelines);
    h.press(Key::G, Modifiers::COMMAND);
    assert_eq!(h.app.mode(), Mode::Read);
    h.press(Key::G, Modifiers::COMMAND);
    assert_eq!(h.app.mode(), Mode::Write);
}

// find snapshots import here---

#[test]
fn ctrl_f_opens_find_and_escape_closes_it() {
    let mut h = Harness::new();
    h.press(Key::F, Modifiers::COMMAND);
    assert!(h.app.debug_find_open());
    h.press(Key::Escape, Modifiers::NONE);
    assert!(!h.app.debug_find_open());
}

#[test]
fn deleting_a_scene_from_its_card_asks_and_can_be_undone() {
    let mut h = Harness::new();
    h.load(three_scenes());
    h.app.set_mode(Mode::Cards);
    h.frames(2);
    h.app.debug_ask_delete_scene(1);
    h.frames(2);
    assert_eq!(h.app.doc().scenes().len(), 3, "nothing goes before the answer");
    h.confirm();
    let left: Vec<String> = h.app.doc().scenes().iter().map(|s| s.heading.clone()).collect();
    assert_eq!(left, vec!["INT. ONE - DAY", "INT. THREE - DAWN"]);
    assert!(!h.app.doc().blocks.iter().any(|b| b.text == "Two."), "its action went with it");
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.doc().scenes().len(), 3);
    assert_eq!(h.app.doc().scenes()[1].heading, "EXT. TWO - NIGHT");
}

#[test]
fn a_snapshot_can_be_restored_and_nothing_is_lost() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let path = h.app.path().unwrap();
    h.app.debug_snapshot();
    h.app.doc_mut().blocks[1].text = "SOMEONE ELSE".into();
    h.frames(2);
    let snaps = storage::list_snapshots(&path);
    assert_eq!(snaps.len(), 1);
    h.app.debug_ask_restore(&snaps[0]);
    h.frames(2);
    assert_eq!(h.app.doc().blocks[1].text, "SOMEONE ELSE", "restoring asks first");
    h.confirm();
    assert_eq!(h.app.doc().blocks[1].text, "DETECTIVE COLE");
    assert_eq!(h.app.doc().meta.title, "Three Scenes", "the title is kept");
    let after = storage::list_snapshots(&path);
    assert_eq!(after.len(), 2, "what was there before the restore was kept too");
    assert!(std::fs::read_to_string(&after[0].path).unwrap().contains("SOMEONE ELSE")
        || std::fs::read_to_string(&after[1].path).unwrap().contains("SOMEONE ELSE"));
    assert!(h.saved_text().contains("**DETECTIVE COLE**"));
}

#[test]
fn a_fountain_file_is_imported_as_a_new_script() {
    let mut h = Harness::new();
    let f = std::env::temp_dir().join(format!("ns-ui-import-{}.fountain", std::process::id()));
    std::fs::write(
        &f,
        "Title: Bank Job\n\nEXT. BANK - DAY\n\nNobody moves.\n\nROSA\nNow.\n\nINT. VAULT - LATER\n\nEmpty.\n",
    )
    .unwrap();
    h.app.debug_import(&f);
    h.frames(2);
    assert_eq!(h.app.entry_count(), 2);
    assert_eq!(h.app.doc().meta.title, "Bank Job");
    assert_eq!(h.app.doc().scenes().len(), 2);
    assert!(h.app.path().unwrap().ends_with("bank-job.md"));
}

// look here---

#[test]
fn the_settings_panel_opens_from_the_ribbon() {
    let mut h = Harness::new();
    let r = h.app.debug_settings_button();
    h.click(r.center());
    assert!(h.app.debug_settings_open());
    h.press(Key::Escape, Modifiers::NONE);
    assert!(!h.app.debug_settings_open());
}

#[test]
fn both_casts_paint_every_view_without_panicking() {
    let mut h = Harness::new();
    h.load(three_scenes());
    for light in [false, true] {
        h.app.debug_set_light(light);
        for m in [Mode::Write, Mode::Cards, Mode::Read] {
            h.app.set_mode(m);
            h.frames(2);
        }
        assert_eq!(theme::pal().dark, !light);
    }
}

#[test]
fn every_view_holds_at_the_minimum_window_size() {
    let mut h = Harness::with(880.0, 580.0, "show_cast = yes\n");
    h.load(three_scenes());
    for m in [Mode::Write, Mode::Cards, Mode::Read] {
        h.app.set_mode(m);
        h.frames(3);
        let page = h.app.debug_page_rect();
        assert!(page.width() > 200.0, "{m:?}: the page keeps some room ({page:?})");
    }
    h.app.debug_focus_mode(true);
    h.app.set_mode(Mode::Write);
    h.frames(3);
    assert!(h.app.debug_page_rect().width() > 800.0, "focus mode gives the page the window");
}

#[test]
fn the_session_counts_the_words_written() {
    let mut h = Harness::new();
    let id = h.app.doc().blocks[1].id;
    h.focus(id);
    h.type_text(" one two three");
    std::thread::sleep(Duration::from_millis(400));
    h.frames(2);
    assert_eq!(h.app.debug_session_words(), 3);
}

#[test]
fn nothing_is_drawn_outside_the_block_being_typed_into() {
    let mut h = Harness::new();
    let id = h.app.doc().blocks[1].id;
    h.focus(id);
    let r = h.block_rect(id).expect("drawn");
    assert!(h.app.debug_page_rect().contains_rect(r.shrink(1.0)), "{r:?}");
}

#[test]
fn the_sidebar_page_and_tabs_line_up_with_the_status_pill() {
    let mut h = Harness::with(1320.0, 900.0, "show_scenes = yes\n");
    let side = h.app.debug_side_rect();
    let status = h.app.debug_status_rect();
    let page = h.app.debug_page_rect();
    let foot = h.app.debug_foot_rect();
    assert!(side.contains_rect(foot), "your name sits inside the sidebar: {foot:?} in {side:?}");
    assert!((side.bottom() - status.bottom()).abs() < 1.5, "sidebar ends at {}, status pill at {}", side.bottom(), status.bottom());
    assert!((page.bottom() + crate::theme::GAP - status.top()).abs() < 1.5, "page ends at {}, status pill starts at {}", page.bottom(), status.top());
    assert!((status.left() - side.right() - crate::theme::GAP).abs() < 1.5, "the same gap beside the sidebar");
    assert!(status.left() > side.right(), "the status pill sits right of the sidebar");
    let aside = h.app.debug_aside_rect();
    assert!((aside.bottom() - page.bottom()).abs() < 1.5);
    assert!((aside.left() - page.right() - crate::theme::GAP).abs() < 1.5, "one gap between the page and the tabs: {} to {}", page.right(), aside.left());
    h.app.debug_set_light(true);
    h.frames(2);
    assert!(!theme::pal().dark);
}

#[test]
fn ctrl_i_opens_the_title_page_and_ctrl_shift_i_the_tabs() {
    let mut h = Harness::new();
    assert!(!h.app.debug_title_sheet_open());
    let scenes = h.app.settings().show_scenes;
    h.press(Key::I, Modifiers::COMMAND);
    assert!(h.app.debug_title_sheet_open());
    assert_eq!(h.app.settings().show_scenes, scenes, "only the title page opened");
    h.press(Key::Escape, Modifiers::NONE);
    h.frames(2);
    assert!(!h.app.debug_title_sheet_open(), "Esc puts it away");
    h.press(Key::I, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(h.app.settings().show_scenes, !scenes);
    assert!(!h.app.debug_title_sheet_open(), "and only the tabs moved");
    assert_eq!(storage::read_settings().show_scenes, !scenes, "remembered");
}

#[test]
fn every_colour_option_paints_every_view() {
    let mut h = Harness::with(880.0, 580.0, "show_details = yes\nshow_cast = yes\n");
    h.load(three_scenes());
    for (elements, voices, reading) in [(true, false, false), (false, true, false), (true, true, true)] {
        {
            let s = h.app.settings_mut();
            s.element_colors = elements;
            s.character_colors = voices;
            s.character_colors_reading = reading;
        }
        for m in [Mode::Write, Mode::Cards, Mode::Read] {
            h.app.set_mode(m);
            h.frames(2);
        }
    }
    let aside = h.app.debug_aside_rect();
    assert!(aside.bottom() <= 580.0 - 14.0 + 1.0, "{aside:?}");
}

#[test]
fn no_island_on_the_right_ever_runs_past_the_page() {
    let mut one = Document::default();
    one.blocks.clear();
    one.meta.title = "Narrated".into();
    for i in 0..30 {
        one.push(Element::SceneHeading, &format!("INT. ROOM {i} - DAY"));
        one.push(Element::Character, "NARRATOR");
        one.push(Element::Dialogue, "And so it went.");
    }
    one.reseed_ids();
    for (w, h) in [(880.0, 580.0), (1100.0, 700.0), (1320.0, 900.0)] {
        for (details, scenes, cast) in [
            (false, false, true),
            (false, true, false),
            (true, false, false),
            (false, true, true),
            (true, true, true),
            (true, false, true),
        ] {
            let yes = |b: bool| if b { "yes" } else { "no" };
            let mut hh = Harness::with(
                w,
                h,
                &format!(
                    "show_details = {}\nshow_scenes = {}\nshow_cast = {}\n",
                    yes(details),
                    yes(scenes),
                    yes(cast)
                ),
            );
            hh.load(one.clone());
            hh.frames(3);
            let page = hh.app.debug_page_rect();
            let islands = hh.app.debug_aside_islands();
            assert_eq!(islands.is_empty(), !scenes, "the tabs show exactly when they are switched on");
            for r in islands {
                assert!(
                    r.bottom() <= page.bottom() + 0.5,
                    "{w}x{h} details={details} scenes={scenes} cast={cast}: island ends at {} but the page at {}",
                    r.bottom(),
                    page.bottom()
                );
            }
        }
    }
}

#[test]
fn reading_mode_shows_one_page_at_a_time_and_turns() {
    let mut h = Harness::new();
    let mut d = three_scenes();
    for _ in 0..160 {
        d.push(Element::Action, "Line after line after line of action on the page.");
    }
    d.push(Element::Action, "THE LAST LINE.");
    d.reseed_ids();
    h.load(d);
    let body = crate::export::page_count(h.app.doc());
    assert!(body >= 3);
    let last = h.app.doc().blocks.last().unwrap().id;
    h.focus(last);
    h.app.set_mode(Mode::Read);
    h.frames(3);
    assert_eq!(h.app.debug_sheet_count(), 1 + body);
    assert_eq!(h.app.debug_sheet_on_show(), body, "opens where the writing was");
    let sheet = h.app.debug_sheet_rect();
    let page = h.app.debug_page_rect();
    assert!(page.contains_rect(sheet), "{sheet:?} inside {page:?}");
    assert!((sheet.height() / sheet.width() - 11.0 / 8.5).abs() < 0.01, "US letter");
    h.press(Key::Home, Modifiers::NONE);
    assert_eq!(h.app.debug_sheet_on_show(), 0, "Home is the title page");
    h.press(Key::ArrowRight, Modifiers::NONE);
    h.press(Key::PageDown, Modifiers::NONE);
    assert_eq!(h.app.debug_sheet_on_show(), 2);
    h.press(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(h.app.debug_sheet_on_show(), 1);
    h.press(Key::End, Modifiers::NONE);
    assert_eq!(h.app.debug_sheet_on_show(), body);
    h.press(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.app.debug_sheet_on_show(), body, "no page past the last");
    h.press(Key::Home, Modifiers::NONE);
    let at = page.center();
    for _ in 0..3 {
        h.frame_with(vec![
            Event::PointerMoved(at),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -40.0),
                modifiers: Modifiers::NONE,
            },
        ]);
    }
    h.frames(2);
    assert_eq!(h.app.debug_sheet_on_show(), 1);
}

#[test]
fn settings_is_a_window_that_closes_by_esc_button_or_outside() {
    let mut h = Harness::new();
    let button = h.app.debug_settings_button();
    h.click(button.center());
    assert!(h.app.debug_settings_open());
    h.press(Key::Escape, Modifiers::NONE);
    assert!(!h.app.debug_settings_open(), "Esc closes it");
    h.click(button.center());
    assert!(h.app.debug_settings_open());
    h.click(pos2(20.0, 880.0));
    assert!(!h.app.debug_settings_open(), "a click outside closes it");
    h.click(button.center());
    let win_left = (1320.0 - 780.0) / 2.0;
    h.click(pos2(win_left + 60.0, (900.0 - 620.0) / 2.0 + 56.0 + 17.0 + 36.0));
    assert!(h.app.debug_settings_open(), "a click inside keeps it open");
    assert_eq!(h.app.debug_settings_tab(), 1);
}

#[test]
fn the_export_window_exports_the_pages_asked_for() {
    use crate::app::PageChoice;
    let mut h = Harness::with(1320.0, 900.0, "after_export = nothing\n");
    let mut d = three_scenes();
    for _ in 0..140 {
        d.push(Element::Action, "Line after line after line of action on the page.");
    }
    d.reseed_ids();
    h.load(d);
    let id = h.app.doc().blocks[1].id;
    h.focus(id);
    h.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
    assert!(h.app.debug_export_open());
    assert_eq!(h.app.focus_block(), Some(id));
    let blocks = h.app.doc().blocks.len();
    h.app.debug_export_choose(PageChoice::Chosen, "2-3");
    h.frames(2);
    h.press(Key::Enter, Modifiers::NONE);
    assert_eq!(h.app.doc().blocks.len(), blocks, "no line was split behind the window");
    assert!(!h.app.debug_export_open(), "exporting closes it");
    let out = storage::exports_dir().join("three-scenes-pages-2-3.pdf");
    assert!(out.exists(), "{out:?}");

    h.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
    h.app.debug_export_choose(PageChoice::Chosen, "40-2");
    h.frames(2);
    h.press(Key::Enter, Modifiers::NONE);
    assert!(h.app.debug_export_open());
    h.press(Key::Escape, Modifiers::NONE);
    assert!(!h.app.debug_export_open());

    h.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
    h.app.debug_export_choose(PageChoice::This, "");
    h.frames(2);
    h.press(Key::Enter, Modifiers::NONE);
    assert!(storage::exports_dir().join("three-scenes-pages-1.pdf").exists());
    assert!(storage::read_settings().pdf_title_page);
}

#[test]
fn the_desktop_makes_acts_as_the_web_does() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let in_first = h.app.doc().blocks[2].id;
    h.focus(in_first);
    h.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
    let acts: Vec<String> = h.app.doc().acts().iter().map(|a| a.title.clone()).collect();
    assert_eq!(acts, ["ACT ONE"]);
    assert_eq!(h.app.doc().scenes().len(), 3, "no scene was made or lost");
    h.app.debug_save();
    let path = h.app.path().unwrap();
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains("\n# ACT ONE\n"), "{text}");
}

#[test]
fn a_script_with_acts_shows_them_everywhere_on_the_desktop_too() {
    let mut h = Harness::new();
    let mut d = three_scenes();
    let act = d.new_block(Element::Act, "ACT ONE");
    d.blocks.insert(0, act);
    let act = d.new_block(Element::Act, "");
    d.blocks.insert(4, act);
    h.load(d);
    for mode in [Mode::Write, Mode::Cards, Mode::Read] {
        h.app.set_mode(mode);
        h.frames(3);
    }
    h.app.settings_mut().show_scenes = true;
    h.app.set_mode(Mode::Write);
    h.frames(3);
    let first = h.app.doc().blocks[0].id;
    h.focus(first);
    h.press(Key::Tab, Modifiers::NONE);
    assert_eq!(h.app.doc().blocks[0].element, Element::Act);
    h.press(Key::Enter, Modifiers::NONE);
    assert_eq!(h.app.doc().blocks[0].text, "ACT ONE", "the title is never split");
    assert_eq!(h.app.doc().blocks[1].element, Element::SceneHeading);
    assert_eq!(h.app.doc().blocks[1].text, "");
}

#[test]
fn the_desktop_chooses_character_colours_as_the_web_does() {
    let mut h = Harness::with(1320.0, 900.0, "character_colors = yes\nshow_cast = yes\n");
    h.load(three_scenes());
    h.app.debug_open_voice_picker("DETECTIVE COLE");
    h.frames(2);
    assert!(h.app.debug_voice_picker_open());
    h.app.debug_set_voice("DETECTIVE COLE", Some(200));
    h.frames(2);
    assert!(h.app.settings().custom_colors, "choosing a colour turns Custom on");
    let voices = h.app.debug_voices().unwrap();
    assert_eq!(voices["DETECTIVE COLE"], crate::theme::voice_of_hue(200));
    h.app.debug_save();
    let text = std::fs::read_to_string(h.app.path().unwrap()).unwrap();
    assert!(text.contains("voices: DETECTIVE COLE=200"), "{text}");
    for tab in [2, 8] {
        h.app.debug_open_settings(tab);
        h.frames(3);
        assert_eq!(h.app.debug_settings_tab(), tab);
    }
}

// dual dialogue here---

fn crossfire() -> Document {
    let mut d = Document::default();
    d.blocks.clear();
    d.meta.title = "Crossfire".into();
    d.push(Element::SceneHeading, "INT. KITCHEN - NIGHT");
    d.push(Element::Character, "MARIA");
    d.push(Element::Dialogue, "I said stop.");
    d.push(Element::Character, "JONAS");
    d.push(Element::Dialogue, "I'm not stopping!");
    d.push(Element::Action, "Silence.");
    d.reseed_ids();
    d
}

#[test]
fn ctrl_d_puts_two_speeches_side_by_side_and_back() {
    let mut h = Harness::new();
    h.load(crossfire());
    let maria = h.app.doc().blocks[1].id;
    let jonas = h.app.doc().blocks[3].id;
    let before = (h.block_rect(maria).unwrap(), h.block_rect(jonas).unwrap());
    assert!(before.1.top() > before.0.bottom(), "one after the other to begin with");
    h.focus(jonas);
    h.press(Key::D, Modifiers::COMMAND);
    h.frames(3);
    assert!(h.app.doc().blocks[3].dual);
    let (m, j) = (h.block_rect(maria).unwrap(), h.block_rect(jonas).unwrap());
    assert!((m.top() - j.top()).abs() < 2.0, "the cues share a row: {m:?} {j:?}");
    assert!(j.left() > m.right(), "JONAS sits to the right of MARIA");
    let silence = h.block_rect(h.app.doc().blocks[5].id).unwrap();
    assert!(silence.top() > j.bottom(), "what follows starts below the pair");
    h.idle();
    assert!(h.saved_text().contains("**JONAS ^**"));
    h.focus(maria);
    h.press(Key::D, Modifiers::COMMAND);
    h.frames(3);
    assert!(!h.app.doc().blocks[3].dual);
}

#[test]
fn enter_at_the_end_of_the_left_speech_moves_to_the_right_one() {
    let mut h = Harness::new();
    let mut d = crossfire();
    d.blocks[3].dual = true;
    h.load(d);
    let left_line = h.app.doc().blocks[2].id;
    h.focus(left_line);
    h.press(Key::End, Modifiers::NONE);
    h.press(Key::Enter, Modifiers::NONE);
    h.frames(2);
    assert_eq!(h.app.doc().blocks.len(), 6, "no block was added");
    assert_eq!(h.app.focus_block(), Some(h.app.doc().blocks[3].id));
}

// script notes here---

#[test]
fn ctrl_m_adds_a_note_that_lands_in_the_file() {
    let mut h = Harness::new();
    h.load(crossfire());
    let line = h.app.doc().blocks[2].id;
    h.focus(line);
    h.press(Key::M, Modifiers::COMMAND);
    h.frames(3);
    assert_eq!(h.app.debug_note_open(), Some(line));
    h.type_text("say it softer");
    h.press(Key::Enter, Modifiers::COMMAND);
    h.frames(2);
    assert_eq!(h.app.debug_note_open(), None);
    assert_eq!(h.app.doc().blocks[2].remark, "say it softer");
    h.idle();
    assert!(h.saved_text().contains("> I said stop. [[say it softer]]"), "{}", h.saved_text());
    h.focus(line);
    h.press(Key::M, Modifiers::COMMAND);
    h.frames(3);
    h.press(Key::Escape, Modifiers::NONE);
    h.frames(2);
    assert_eq!(h.app.doc().blocks[2].remark, "say it softer", "Esc keeps the note as it was");
}

#[test]
fn a_note_in_the_left_panel_jumps_to_its_line() {
    let mut h = Harness::with(1320.0, 900.0, "show_library = yes\n");
    let mut doc = crossfire();
    doc.blocks[2].remark = "softer".into();
    doc.blocks[5].remark = "hold the silence".into();
    h.load(doc);
    h.app.debug_left_tab(0);
    h.frames(3);
    let noted: Vec<u64> = h.app.doc().blocks.iter().filter(|b| !b.remark.is_empty()).map(|b| b.id).collect();
    let rows = h.app.debug_left_rows();
    assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), noted, "every note, in script order");
    let (id, rect) = *rows.last().unwrap();
    h.click(rect.center());
    h.frames(3);
    assert_eq!(h.app.focus_block(), Some(id));
}

#[test]
fn a_revision_lists_each_changed_line_and_ends_clean() {
    let mut h = Harness::with(1320.0, 900.0, "show_library = yes\n");
    h.load(crossfire());
    h.app.debug_left_tab(1);
    h.app.debug_start_revision();
    h.frames(2);
    assert!(h.app.doc().meta.revising());
    assert_eq!(h.app.doc().meta.revision, Some(0));
    assert!(h.app.debug_changes().is_empty(), "nothing changed yet");
    let line = h.app.doc().blocks[2].id;
    h.focus(line);
    h.type_text(" Now.");
    h.frames(2);
    assert_eq!(h.app.debug_changes(), vec![line]);
    h.frames(2);
    assert_eq!(h.app.debug_left_rows().iter().map(|r| r.0).collect::<Vec<_>>(), vec![line]);
    h.idle();
    let saved = h.saved_text();
    assert!(saved.contains("revision: blue"), "{saved}");
    assert!(saved.contains("revision_base: "), "{saved}");
}

#[test]
fn home_remembers_where_you_left_off_and_takes_you_back_there() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let path = h.app.path().unwrap();
    let two = h.app.doc().blocks[4].id;
    h.app.focus_on(two);
    h.frames(2);
    h.app.debug_go_home();
    h.frames(3);
    assert_eq!(h.app.debug_left_off_at(&path), Some((1, "EXT. TWO - NIGHT".to_string())));
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(!saved.contains("left_off") && !saved.contains("EXT. TWO - NIGHT|"), "the place is never written into the script");
    let settings = storage::read_settings();
    assert_eq!(settings.places.first().map(|p| (p.page, p.scene, p.block)), Some((1, 2, 4)), "kept with your settings, not the script");
    h.app.debug_open_from_home_where_you_were(path.clone());
    h.frames(3);
    assert_eq!(h.app.focus_block(), Some(two), "the caret is back on the line");
}

#[test]
fn home_search_finds_a_line_inside_a_script_and_opens_it_there() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let three = h.app.path().unwrap();
    let cole = h.app.doc().blocks[1].id;
    h.app.debug_go_home();
    h.frames(2);
    h.app.debug_home_search("detective  cole");
    h.frames(3);
    let rows = h.app.debug_hit_rows();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!((rows[0].0.clone(), rows[0].1), (three.clone(), 1));
    assert!(h.app.debug_home_order().is_empty(), "no title matches, only the line");
    let row = rows[0].2;
    h.click(row.center());
    h.frames(3);
    assert!(!h.app.is_home());
    assert_eq!(h.app.path(), Some(three));
    assert_eq!(h.app.focus_block(), Some(cole));
}

#[test]
fn ctrl_shift_r_opens_the_reports_and_a_scene_row_jumps_there() {
    let mut h = Harness::with(1320.0, 900.0, "");
    h.load(three_scenes());
    h.press(Key::R, Modifiers::COMMAND | Modifiers::SHIFT);
    h.frames(3);
    assert!(h.app.debug_popover_open());
    assert_eq!(h.app.debug_report().scenes.len(), 3);
    assert_eq!(h.app.debug_report().speakers[0].name, "DETECTIVE COLE");
    h.press(Key::Escape, Modifiers::NONE);
    h.frames(2);
    assert!(!h.app.debug_popover_open());

    h.press(Key::R, Modifiers::COMMAND | Modifiers::SHIFT);
    h.app.debug_report_tab(1);
    h.frames(3);
    let rows = h.app.debug_report_rows();
    assert_eq!(rows.len(), 3);
    let (third, rect) = rows[2];
    h.click(rect.center());
    h.frames(3);
    assert!(!h.app.debug_popover_open(), "going to a scene closes the window");
    assert_eq!(h.app.focus_block(), Some(third));
}

fn storylines() -> Document {
    let mut d = Document::default();
    d.blocks.clear();
    d.meta.title = "Storylines".into();
    d.meta.set_storyline(1, "The heist");
    for n in 0..24 {
        d.push(Element::SceneHeading, &format!("INT. ROOM {n} - DAY"));
        d.push(Element::Action, "Somebody waits by the window and counts the cars going past, slowly.");
    }
    d.reseed_ids();
    let heads: Vec<usize> = (0..d.blocks.len()).filter(|i| d.blocks[*i].element == Element::SceneHeading).collect();
    for (k, i) in heads.into_iter().enumerate() {
        d.blocks[i].tint = Some(k % 3);
    }
    d
}

#[test]
fn a_storyline_filters_the_cards_and_the_timeline_scrolls_to_a_card() {
    let mut h = Harness::with(1320.0, 900.0, "cards_view = grid\n");
    h.load(storylines());
    h.app.set_mode(Mode::Cards);
    h.frames(3);
    let chips = h.app.debug_cards().chips.clone();
    assert_eq!(chips.iter().map(|c| c.0).collect::<Vec<_>>(), vec![0, 1, 2]);
    h.click(chips[1].1.center());
    h.frames(2);
    assert_eq!(h.app.debug_cards().only, vec![1]);
    h.click(chips[1].1.center());
    h.frames(2);
    assert!(h.app.debug_cards().only.is_empty());

    let segments = h.app.debug_cards().segments.clone();
    assert_eq!(segments.len(), 24);
    assert!(segments.windows(2).all(|w| w[0].1.right() <= w[1].1.left() + 0.5), "in script order, left to right");
    let last = h.app.debug_card_rect(23).unwrap();
    assert!(last.top() > 900.0, "the last card starts out of sight");
    h.click(segments[23].1.center());
    h.frames(40);
    let last = h.app.debug_card_rect(23).unwrap();
    assert!(last.top() > 0.0 && last.bottom() < 900.0, "scrolled into view: {last:?}");
}

#[test]
fn columns_hold_one_storyline_each_and_a_drag_across_changes_it() {
    let mut h = Harness::with(1320.0, 900.0, "");
    h.load(storylines());
    h.app.set_mode(Mode::Cards);
    h.frames(3);
    let lanes = h.app.debug_cards().lanes.clone();
    assert_eq!(lanes.iter().map(|l| l.0).collect::<Vec<_>>(), vec![Some(0), Some(1), Some(2)]);
    assert!(lanes.windows(2).all(|w| w[0].1.right() < w[1].1.left()), "side by side: {lanes:?}");
    let first = h.app.debug_card_rect(0).unwrap();
    assert!(lanes[0].1.contains(first.center()), "scene one sits in its storyline");
    assert!(h.app.debug_card_rect(1).map(|r| lanes[1].1.contains(r.center())).unwrap_or(false));
    let id = h.app.doc().scenes()[0].id;
    h.drag(first.center_top() + vec2(0.0, 14.0), lanes[1].1.center());
    assert_eq!(h.app.doc().scenes()[0].tint, Some(1), "it joined the heist");
    assert_eq!(h.app.doc().scenes()[0].id, id, "and kept its place in the script");
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.doc().scenes()[0].tint, Some(0), "Ctrl+Z puts it back");
}

#[test]
fn a_drag_within_a_column_moves_the_scene_in_the_script() {
    let mut h = Harness::with(1320.0, 900.0, "");
    h.load(storylines());
    h.app.set_mode(Mode::Cards);
    h.frames(3);
    let heading = h.app.doc().scenes()[3].heading.clone();
    let top = h.app.debug_card_rect(0).unwrap();
    let second = h.app.debug_card_rect(3).unwrap();
    h.drag(second.center_top() + vec2(0.0, 14.0), top.center_top() + vec2(0.0, 6.0));
    let scenes = h.app.doc().scenes();
    assert_eq!(scenes[0].heading, heading, "it went before the first card of its column");
    assert_eq!(scenes[0].tint, Some(0));
    assert_eq!(scenes.len(), 24);
}

#[test]
fn the_cards_view_switches_to_the_grid_and_remembers_it() {
    let mut h = Harness::with(1320.0, 900.0, "");
    h.load(storylines());
    h.app.set_mode(Mode::Cards);
    h.frames(3);
    assert!(h.app.settings().cards_columns, "columns by default");
    assert_eq!(h.app.debug_cards().lanes.len(), 3);
    h.app.settings_mut().cards_columns = false;
    h.frames(3);
    assert_eq!(h.app.debug_cards().segments.len(), 24, "the grid has its timeline");
    let a = h.app.debug_card_rect(0).unwrap();
    let b = h.app.debug_card_rect(1).unwrap();
    assert!((a.top() - b.top()).abs() < 1.0, "cards run along a row");
}

#[test]
fn an_import_with_problems_opens_a_report_that_points_at_them() {
    let mut h = Harness::with(1320.0, 900.0, "");
    let dir = std::env::temp_dir().join(format!("ns-import-report-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("odd.fountain");
    std::fs::write(&f, "INT. ROOM - DAY\n\nShe waits.\n\n~A song she knows\n\n===\n\nShe leaves.\n").unwrap();
    h.app.debug_import(&f);
    h.frames(3);
    assert!(h.app.debug_popover_open());
    let issues = h.app.debug_imported().unwrap();
    assert_eq!(issues.len(), 2, "{issues:?}");
    h.press(Key::Escape, Modifiers::NONE);
    h.frames(2);
    assert!(!h.app.debug_popover_open());

    let clean = dir.join("clean.fountain");
    std::fs::write(&clean, "INT. ROOM - DAY\n\nShe waits.\n").unwrap();
    h.app.debug_import(&clean);
    h.frames(3);
    assert!(!h.app.debug_popover_open(), "nothing to report, no window");
}

#[test]
fn a_new_version_asks_before_anything_else_until_answered() {
    use crate::update::{Release, Status};
    use ncf::splash::Choice;
    let mut h = Harness::with(1320.0, 900.0, "");
    h.load(three_scenes());
    let id = h.app.doc().blocks[2].id;
    h.focus(id);
    let newer = |v: &str| Status::Newer(Release { version: v.into(), url: String::new() });
    assert!(h.app.debug_update_offer().is_none(), "nothing to say yet");
    h.app.debug_check_answers(newer("9.9.0"));
    h.frames(3);
    let offer = h.app.debug_update_offer().expect("the update is offered");
    assert_eq!(offer.version, "9.9.0");
    assert!(!offer.can_update, "a test build isn't installed the standard way, so it offers the download");
    let before = h.app.doc().blocks.clone();
    h.type_text("typed behind the popup");
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.doc().blocks, before, "nothing reaches the script until the popup is answered");
    h.press(Key::Escape, Modifiers::NONE);
    assert!(h.app.debug_update_offer().is_none(), "Escape is Not now");
    assert_eq!(storage::read_settings().dismissed_update, "");

    h.app.debug_check_answers(newer("9.9.0"));
    h.frames(2);
    assert!(h.app.debug_update_offer().is_some(), "Not now asks again next time");
    h.app.debug_answer_offer(Choice::Skip("9.9.0".into()));
    h.frames(2);
    assert_eq!(storage::read_settings().dismissed_update, "9.9.0", "a skipped version is remembered");
    h.app.debug_check_answers(newer("9.9.0"));
    h.frames(2);
    assert!(h.app.debug_update_offer().is_none(), "and not offered again");
    h.app.debug_check_answers(newer("9.9.1"));
    h.frames(2);
    assert!(h.app.debug_update_offer().is_some(), "a later version asks again");
}

#[test]
fn update_now_asks_first_and_a_copy_that_cannot_update_itself_stays_open() {
    use crate::update::{Release, Status};
    let mut h = Harness::with(1320.0, 900.0, "");
    h.app.debug_go_home();
    h.app.debug_set_update(Status::Newer(Release { version: "9.9.0".into(), url: String::new() }));
    h.frames(3);
    assert!(!h.app.debug_update_offered(), "a test build isn't installed the standard way, so Download is offered instead");
    h.app.debug_ask_update("9.9.0");
    h.frames(2);
    assert!(h.app.debug_pending_ask(), "Update now always asks first");
    h.press(Key::Enter, Modifiers::NONE);
    h.frames(3);
    assert!(!h.app.debug_pending_ask());
    assert!(!h.app.debug_quitting_for_update(), "nothing to hand over to, so NativeDraft stays open");
}

#[test]
fn people_who_saw_blue_pages_only_get_the_new_look() {
    use crate::app::onboarding::Kind;
    let mut h = Harness::with(1320.0, 900.0, "onboarded = blue-pages\n");
    assert_eq!(h.app.debug_tour(), Some((Kind::New, 0)));
    for step in 1..6 {
        h.press(Key::Enter, Modifiers::NONE);
        assert_eq!(h.app.debug_tour(), Some((Kind::New, step)));
    }
    h.press(Key::Enter, Modifiers::NONE);
    assert_eq!(h.app.debug_tour(), None, "six slides, then back to work");
}

#[test]
fn whats_new_shows_once_takes_the_keys_and_can_be_replayed() {
    use crate::app::onboarding::Kind;
    let mut h = Harness::with(1320.0, 900.0, "onboarded = \n");
    assert_eq!(h.app.debug_tour(), Some((Kind::New, 0)));
    let before = h.app.doc().clone();
    h.type_text("typed while the tour was up");
    h.press(Key::Enter, Modifiers::NONE);
    assert_eq!(h.app.debug_tour(), Some((Kind::New, 1)));
    h.press(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(h.app.debug_tour(), Some((Kind::New, 0)));
    assert_eq!(h.app.doc().blocks, before.blocks, "nothing reaches the script underneath");
    h.press(Key::Escape, Modifiers::NONE);
    assert_eq!(h.app.debug_tour(), None);
    h.idle();
    assert_eq!(storage::read_settings().onboarded, "new-look", "seen, so it will not show again");

    h.app.debug_start_tour();
    h.frames(1);
    for _ in 0..8 {
        assert!(h.app.debug_tour().is_some());
        h.press(Key::Enter, Modifiers::NONE);
    }
    assert_eq!(h.app.debug_tour(), None, "eight steps, then back to writing");
}

#[test]
fn quitting_saves_what_has_not_been_saved_yet() {
    let mut h = Harness::with(1320.0, 900.0, "autosave_ms = 10000\n");
    h.load(three_scenes());
    let id = h.app.doc().blocks[2].id;
    h.focus(id);
    h.type_text(" Typed just before quitting.");
    assert!(h.app.is_dirty());
    assert!(!h.saved_text().contains("Typed just before quitting."));
    eframe::App::on_exit(&mut h.app, None);
    assert!(!h.app.is_dirty());
    assert!(h.saved_text().contains("Typed just before quitting."), "{}", h.saved_text());
}

// debug mode here---

impl Harness {
    fn click_held(&mut self, at: Pos2, modifiers: Modifiers) {
        let held = |events: Vec<Event>| RawInput { modifiers, events, ..Default::default() };
        for events in [
            vec![Event::PointerMoved(at)],
            vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers }],
            vec![Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers }],
        ] {
            let mut input = held(events);
            input.screen_rect = Some(Rect::from_min_size(pos2(0.0, 0.0), self.size));
            let ctx = self.ctx.clone();
            let app = &mut self.app;
            let _ = ctx.run(input, |c| app.frame(c));
        }
        self.frame();
    }

    fn knock(&mut self, times: usize, modifiers: Modifiers) {
        self.app.debug_open_settings(6);
        self.frames(3);
        let at = self.app.debug_version_rect().center();
        assert!(self.app.debug_version_rect().is_positive(), "the version line should be on screen");
        for _ in 0..times {
            self.click_held(at, modifiers);
        }
    }

    fn debug_on(&mut self) {
        self.knock(5, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(self.app.debug_mode().on);
        self.app.debug_close_popover();
        self.frames(2);
    }
}

#[test]
fn debug_mode_is_off_and_unwritten_by_default() {
    let h = Harness::new();
    assert!(!h.app.debug_mode().on);
    assert!(!storage::read_settings().serialize().contains("debug"), "the settings file is unchanged while it is off");
}

#[test]
fn the_hidden_gesture_turns_debug_mode_on_and_off() {
    let mut h = Harness::new();
    h.knock(5, Modifiers::NONE);
    assert!(!h.app.debug_mode().on, "plain clicks do nothing");
    h.knock(5, Modifiers::SHIFT);
    assert!(!h.app.debug_mode().on, "one key is not enough");
    h.knock(4, Modifiers::COMMAND | Modifiers::SHIFT);
    assert!(!h.app.debug_mode().on, "four is not enough");
    h.knock(1, Modifiers::COMMAND | Modifiers::SHIFT);
    assert!(h.app.debug_mode().on, "the fifth click switches it on");
    assert!(h.app.debug_mode().open);
    h.idle();
    assert!(storage::read_settings().debug, "it stays on next launch");

    h.knock(5, Modifiers::COMMAND | Modifiers::SHIFT);
    assert!(!h.app.debug_mode().on);
    h.idle();
    assert!(!storage::read_settings().serialize().contains("debug"));
}

#[test]
fn debug_mode_draws_its_panel_and_every_display_button_runs() {
    use ncf::debug::Request;
    let mut h = Harness::new();
    h.debug_on();
    h.frames(2);
    for request in [
        Request::FakeUpdate,
        Request::Notice(crate::alerts::Tone::Warn),
        Request::TextScale(0.1),
        Request::Light(true),
        Request::Animations(true),
        Request::Inspector(true),
        Request::Inspector(false),
        Request::WindowSize(ncf::debug::SMALL),
    ] {
        let ctx = h.ctx.clone();
        h.app.apply_debug(&ctx, &request);
        h.frames(2);
    }
    assert!(matches!(h.app.update_status(), crate::update::Status::Newer(_)), "the pretend release is offered");
    assert!((h.app.settings().font_scale - 1.1).abs() < 1e-3);
    assert_eq!(h.app.settings().appearance, ncf::settings::Appearance::Light);

    let ctx = h.ctx.clone();
    h.app.apply_debug(&ctx, &Request::Confirm);
    h.frames(2);
    h.confirm();

    h.app.apply_debug(&ctx, &Request::Splash(crate::update::Status::Failed));
    h.frames(2);
    assert!(h.app.debug_splash_replaying(), "the splash comes back with the chosen status");

    h.app.apply_debug(&ctx, &Request::Off);
    h.frames(2);
    assert!(!h.app.debug_mode().on);
    h.idle();
    assert!(!storage::read_settings().debug);
}

#[test]
fn debug_mode_can_fail_one_save_and_the_next_one_recovers() {
    use crate::app::debug::Tool;
    let mut h = Harness::with(1320.0, 900.0, "autosave_ms = 10000\n");
    h.load(three_scenes());
    h.debug_on();
    let id = h.app.doc().blocks[2].id;
    h.focus(id);
    h.type_text(" First try.");
    h.app.use_tool(Tool::FailSave);
    h.app.debug_save();
    h.frames(2);
    assert!(h.app.is_dirty(), "the failed save leaves the script marked unsaved");
    assert!(!h.saved_text().contains("First try."));
    h.app.debug_save();
    h.frames(2);
    assert!(!h.app.is_dirty());
    assert!(h.saved_text().contains("First try."), "the save after it works");
}

#[test]
fn debug_mode_adds_a_script_from_outside_and_imports_a_problem_file() {
    use crate::app::debug::Tool;
    let mut h = Harness::new();
    h.debug_on();
    h.app.use_tool(Tool::Outside);
    std::thread::sleep(Duration::from_millis(1600));
    h.frames(2);
    assert!(h.app.entries().iter().any(|e| e.title == "Written Outside"), "the library notices it");

    h.app.use_tool(Tool::Problems);
    h.frames(2);
    assert!(h.app.debug_imported().is_some(), "the import report opens");

    h.app.use_tool(Tool::Tour);
    h.frames(1);
    assert!(h.app.debug_tour().is_some());
}

#[test]
fn a_blackline_library_opens_where_it_is_and_nothing_in_it_changes() {
    let guard = VAULT.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("nativedraft-ui-blackline-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let old = dir.join(if cfg!(windows) || cfg!(target_os = "macos") { "Blackline" } else { "blackline" });
    std::fs::create_dir_all(old.join("scripts")).unwrap();
    std::fs::write(old.join("settings.conf"), "splash = no\nanimations = no\nonboarded = new-look\n").unwrap();
    let script = "---\ntitle: Old Friend\nauthor: A. Writer\n---\n\n## INT. KITCHEN - NIGHT\n\nThe kettle screams.\n";
    std::fs::write(old.join("scripts/old-friend.md"), script).unwrap();
    std::env::set_var("XDG_DATA_HOME", &dir);
    assert_eq!(storage::library_root(), old);
    let ctx = egui::Context::default();
    let _ = ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1320.0, 900.0))), ..Default::default() }, |_| {});
    let app = App::with_context(&ctx);
    let mut h = Harness { ctx, app, size: vec2(1320.0, 900.0), _guard: guard };
    h.frames(5);
    assert!(h.app.entries().iter().any(|e| e.title == "Old Friend"), "the old script is on Home");
    h.idle();
    assert_eq!(std::fs::read_to_string(old.join("scripts/old-friend.md")).unwrap(), script, "byte for byte");
    assert!(!dir.join("nativedraft").exists() && !dir.join("NativeDraft").exists(), "no second library appears");
    let _ = std::fs::remove_dir_all(&dir);
}

// timelines here---

const HARBOUR: &str = "---\ntitle: Harbour\nauthor: \ncontact: \ndraft: \n---\n\n## INT. DOCK - NIGHT\n\nRain.\n\n## INT. OFFICE - NIGHT\n\n**MARIA**\n\n> No.\n\n## EXT. PIER - DAWN\n\nGulls.\n\n";

impl Harness {
    fn harbour(&mut self) -> std::path::PathBuf {
        let path = storage::scripts_dir().join("harbour.md");
        std::fs::write(&path, HARBOUR).unwrap();
        self.app.debug_open(path.clone());
        self.frames(3);
        path
    }

    fn right_click(&mut self, at: Pos2) {
        self.frame_with(vec![Event::PointerMoved(at)]);
        self.frame_with(vec![
            Event::PointerMoved(at),
            Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed: true, modifiers: Modifiers::NONE },
        ]);
        self.frame_with(vec![Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed: false, modifiers: Modifiers::NONE }]);
        self.frame();
    }

    fn branch_at_scene(&mut self, n: usize, name: &str) {
        let id = self.app.doc().scenes()[n].id;
        self.app.focus_on(id);
        self.frames(2);
        self.press(Key::B, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(self.app.debug_branch_open(), "Ctrl+Shift+B asks how to branch");
        self.type_text(name);
        self.press(Key::Enter, Modifiers::NONE);
        self.frames(3);
    }
}

fn added_lines(before: &str, after: &str) -> Vec<String> {
    let a: Vec<&str> = before.lines().collect();
    after.lines().filter(|l| !a.contains(l)).map(|l| l.to_string()).collect()
}

#[test]
fn branching_from_a_scene_writes_one_new_file_and_one_id() {
    let mut h = Harness::new();
    let main = h.harbour();
    h.branch_at_scene(1, "Maria says no");
    assert!(!h.app.debug_branch_open());
    let branch = storage::scripts_dir().join("harbour.timelines/maria-says-no.md");
    assert!(branch.exists(), "the timeline is a file in the hidden folder");
    let after = std::fs::read_to_string(&main).unwrap();
    let added = added_lines(HARBOUR, &after);
    assert_eq!(added.len(), 1, "{added:?}\n{after}");
    assert!(added[0].starts_with("<!-- scene: | id="), "{added:?}");
    assert_eq!(after.lines().count(), HARBOUR.lines().count() + 1, "nothing else in the script moved");
    assert_eq!(h.app.path(), Some(branch.clone()), "the new timeline opens");
    assert_eq!(h.app.debug_tl(), Some(0));
    assert_eq!(h.app.mode(), Mode::Write);
    let text = std::fs::read_to_string(&branch).unwrap();
    assert!(text.contains("timeline: Maria says no") && text.contains("parent: main"), "{text}");

    h.type_text("EXT. ROOFTOP - NIGHT");
    h.app.debug_save();
    h.frames(2);
    assert_eq!(std::fs::read_to_string(&main).unwrap(), after, "writing in a timeline leaves the script alone");
    assert!(std::fs::read_to_string(&branch).unwrap().contains("## EXT. ROOFTOP - NIGHT"));
    let entries = storage::list_scripts();
    let e = entries.iter().find(|e| e.path == main).expect("the script is listed");
    assert_eq!(e.timelines, 1);
    assert!(entries.iter().all(|e| !storage::is_timeline_path(&e.path)), "a timeline is not a script of its own");
}

#[test]
fn the_timelines_view_draws_dots_and_answers_the_keyboard() {
    let mut h = Harness::new();
    let main = h.harbour();
    h.branch_at_scene(1, "Maria says no");
    h.type_text("EXT. ROOFTOP - NIGHT");
    h.app.debug_save();
    h.app.debug_open(main.clone());
    h.frames(2);
    h.press(Key::G, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(h.app.mode(), Mode::Timelines);
    h.frames(3);
    let dots = h.app.debug_timeline_dots();
    assert_eq!(dots.len(), 4, "three scenes on Main and one on the branch");
    let chips = h.app.debug_timeline_chips();
    assert_eq!(chips.len(), 2);
    let ((r, d), rect) = dots[0];
    h.click(rect.center());
    assert_eq!(h.app.debug_timeline_sel(), Some((r, d)));
    h.press(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.app.debug_timeline_sel(), Some((r, d + 1)));
    h.press(Key::ArrowDown, Modifiers::NONE);
    let (r2, _) = h.app.debug_timeline_sel().unwrap();
    assert_eq!(r2, r + 1, "down moves to the next timeline");
    h.press(Key::Enter, Modifiers::NONE);
    assert_eq!(h.app.mode(), Mode::Write);
    assert_eq!(h.app.debug_tl(), Some(0), "Enter opens the chosen scene's timeline");

    h.press(Key::G, Modifiers::COMMAND | Modifiers::SHIFT);
    h.frames(3);
    let dot = h.app.debug_timeline_dots()[1].1;
    h.right_click(dot.center());
    assert!(h.app.debug_timeline_menu_open(), "a right-click opens the scene menu");
}

#[test]
fn deleting_a_scene_a_timeline_uses_asks_re_anchors_and_undoes_everywhere() {
    let mut h = Harness::new();
    let main = h.harbour();
    h.branch_at_scene(1, "Maria says no");
    h.type_text("EXT. ROOFTOP - NIGHT");
    h.app.debug_save();
    let branch = h.app.path().unwrap();
    h.app.debug_open(main.clone());
    h.frames(2);
    let main_before = std::fs::read_to_string(&main).unwrap();
    let branch_before = std::fs::read_to_string(&branch).unwrap();
    let refs = h.app.debug_scene_refs(1).expect("the scene is used");
    assert_eq!(refs, "1 timeline branches from this scene");
    assert!(h.app.debug_scene_refs(0).is_none());
    h.app.debug_ask_delete_scene(1);
    h.frames(1);
    h.app.debug_answer_yes(None);
    h.frames(3);
    h.app.debug_save();
    let main_after = std::fs::read_to_string(&main).unwrap();
    assert!(!main_after.contains("INT. OFFICE"));
    let b = crate::timelines::load_bundle(&main).unwrap();
    let from = b.items[0].meta().from.clone().expect("re-anchored, not dropped");
    assert!(main_after.contains(&format!("id={from}")), "the scene before now carries the id");
    assert_eq!(crate::timelines::assemble(&b, Some(0)).blocks[0].text, "INT. DOCK - NIGHT");
    assert!(crate::timelines::issues(&b).is_empty());

    h.app.debug_undo();
    h.frames(2);
    h.app.debug_save();
    assert_eq!(std::fs::read_to_string(&main).unwrap(), main_before, "undo brings the scene back");
    assert_eq!(std::fs::read_to_string(&branch).unwrap(), branch_before, "and puts the timeline back where it was");
}

#[test]
fn typing_away_a_used_scene_shows_a_strip_that_re_anchors() {
    let mut h = Harness::new();
    let main = h.harbour();
    h.branch_at_scene(1, "Maria says no");
    h.app.debug_open(main.clone());
    h.frames(2);
    let k = h.app.doc().scenes()[1].start;
    h.app.doc_mut().blocks[k].element = Element::Action;
    h.app.debug_append("");
    h.frames(2);
    std::thread::sleep(Duration::from_millis(400));
    h.frames(3);
    assert_eq!(h.app.debug_lost().len(), 1, "the strip knows which scene went");
    h.app.debug_reanchor();
    h.frames(2);
    assert!(h.app.debug_lost().is_empty());
    let b = crate::timelines::load_bundle(&main).unwrap();
    let issues: Vec<_> = crate::timelines::issues(&b).into_iter().filter(|i| !matches!(i, crate::timelines::Issue::EmptyBranch { .. })).collect();
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(b.items[0].meta().from.as_deref(), b.main.blocks[0].sid.as_deref(), "it holds on to the scene before");
}

#[test]
fn deleting_a_script_names_its_timelines_and_takes_them() {
    let mut h = Harness::new();
    let main = h.harbour();
    h.branch_at_scene(1, "Maria says no");
    h.app.debug_open(main.clone());
    h.frames(2);
    let text = h.app.debug_delete_text(&main);
    assert!(text.contains("its timeline go for good: Maria says no"), "{text}");
    h.app.debug_go_home();
    h.frames(2);
    assert!(storage::scripts_dir().join("harbour.timelines").is_dir());
}

#[test]
fn the_palette_lists_timelines_and_their_scenes() {
    let mut h = Harness::new();
    let main = h.harbour();
    h.branch_at_scene(1, "Maria says no");
    h.type_text("EXT. ROOFTOP - NIGHT");
    h.app.debug_save();
    h.app.debug_open(main);
    h.frames(2);
    let rows = h.app.debug_palette_rows("maria");
    assert!(rows.iter().any(|(s, t)| s == "Timelines" && t == "Maria says no"), "{rows:?}");
    let rows = h.app.debug_palette_rows("rooftop");
    assert!(rows.iter().any(|(s, t)| s == "Scenes" && t == "EXT. ROOFTOP - NIGHT"), "{rows:?}");
}

#[test]
fn opening_a_library_with_timelines_writes_nothing() {
    let mut h = Harness::new();
    let main = h.harbour();
    h.branch_at_scene(1, "Maria says no");
    h.app.debug_save();
    h.app.debug_go_home();
    h.frames(2);
    let branch = storage::scripts_dir().join("harbour.timelines/maria-says-no.md");
    let a = std::fs::read_to_string(&main).unwrap();
    let b = std::fs::read_to_string(&branch).unwrap();
    for p in [&main, &branch] {
        h.app.debug_open(p.clone());
        h.frames(3);
        h.app.debug_save();
        h.press(Key::G, Modifiers::COMMAND | Modifiers::SHIFT);
        h.frames(2);
        h.app.debug_go_home();
        h.frames(2);
    }
    assert_eq!(std::fs::read_to_string(&main).unwrap(), a);
    assert_eq!(std::fs::read_to_string(&branch).unwrap(), b);
}

const HARBOUR_IDS: &str = "---\ntitle: Harbour\nauthor: \ncontact: \ndraft: \n---\n\n## INT. DOCK - NIGHT\n<!-- scene: | id=dock01 -->\n\nRain.\n\n## INT. OFFICE - NIGHT\n<!-- scene: | id=offi01 -->\n\n**MARIA**\n\n> No.\n\n## EXT. PIER - DAWN\n<!-- scene: | id=pier01 -->\n\nGulls.\n\n";

const ROOF: &str = "---\ntitle: Roof\nauthor: \ncontact: \ndraft: \ntimeline-id: roof01\ntimeline: Roof\nparent: main\nfrom: offi01\nchoice: She climbs\n---\n\n## INT. ROOF - NIGHT\n\nWind.\n\n## @same pier01\n\n## INT. CAR - NIGHT\n\nEngine.\n\n";

impl Harness {
    fn roof(&mut self) -> (std::path::PathBuf, std::path::PathBuf) {
        let main = storage::scripts_dir().join("harbour.md");
        std::fs::write(&main, HARBOUR_IDS).unwrap();
        let branch = storage::scripts_dir().join("harbour.timelines/roof.md");
        std::fs::create_dir_all(branch.parent().unwrap()).unwrap();
        std::fs::write(&branch, ROOF).unwrap();
        self.app.debug_open(branch.clone());
        self.frames(3);
        (main, branch)
    }
}

#[test]
fn a_branch_numbers_its_scenes_after_the_story_so_far_and_counts_the_whole_story() {
    let mut h = Harness::new();
    let (main, branch) = h.roof();
    assert_eq!(h.app.debug_tl(), Some(0));
    assert_eq!(h.app.doc().scene_base, 2);
    let numbers: Vec<usize> = h.app.doc().scenes().iter().map(|s| s.number).collect();
    assert_eq!(numbers, vec![3, 4, 5]);
    assert_eq!(h.app.debug_story_counts().map(|c| c.1), Some(5), "Dock, Office, Roof, the Pier and the Car");
    h.app.set_mode(Mode::Read);
    h.frames(3);
    let b = crate::timelines::load_bundle(&main).unwrap();
    let story = crate::timelines::assemble(&b, Some(0));
    assert_eq!(h.app.debug_sheet_count(), 1 + crate::export::page_count(&story), "Read shows the whole story");
    h.app.set_mode(Mode::Write);
    h.frames(2);
    assert_eq!(std::fs::read_to_string(&branch).unwrap(), ROOF, "looking is not writing");
    assert_eq!(std::fs::read_to_string(&main).unwrap(), HARBOUR_IDS);
}

#[test]
fn the_caret_steps_over_a_same_scene_and_backspace_cannot_eat_it() {
    let mut h = Harness::new();
    let (_, branch) = h.roof();
    let wind = h.app.doc().blocks.iter().find(|b| b.text == "Wind.").unwrap().id;
    let car = h.app.doc().blocks.iter().find(|b| b.text == "INT. CAR - NIGHT").unwrap().id;
    h.app.focus_on(wind);
    h.frames(2);
    h.press(Key::ArrowDown, Modifiers::NONE);
    h.frames(2);
    assert_eq!(h.app.focus_block(), Some(car), "down from the line above lands below the Same scene");
    h.app.focus_start(car);
    h.frames(2);
    h.press(Key::Backspace, Modifiers::NONE);
    h.frames(2);
    assert!(h.app.doc().blocks.iter().any(|b| b.link.as_deref() == Some("pier01")), "the Same scene is still there");
    assert!(h.app.doc().blocks.iter().any(|b| b.text == "INT. CAR - NIGHT"), "and so is the heading");
    h.app.debug_save();
    assert!(std::fs::read_to_string(&branch).unwrap().contains("## @same pier01"));
}

#[test]
fn make_it_its_own_copies_the_scene_in_and_undo_links_it_again() {
    let mut h = Harness::new();
    let (main, branch) = h.roof();
    let link = h.app.doc().blocks.iter().find(|b| b.link.is_some()).unwrap().id;
    h.app.debug_mirror_act(link, true);
    h.frames(2);
    h.app.debug_save();
    let text = std::fs::read_to_string(&branch).unwrap();
    assert!(!text.contains("@same") && text.contains("## EXT. PIER - DAWN") && text.contains("Gulls."), "{text}");
    assert_eq!(std::fs::read_to_string(&main).unwrap(), HARBOUR_IDS, "the original stays as it was");
    h.app.debug_undo();
    h.frames(2);
    h.app.debug_save();
    assert_eq!(std::fs::read_to_string(&branch).unwrap(), ROOF);
}
