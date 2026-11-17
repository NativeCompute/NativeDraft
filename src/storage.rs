#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::io;
use std::path::{Path, PathBuf};
use web_time::SystemTime;

#[cfg(not(target_arch = "wasm32"))]
use crate::export;
use crate::backend::Person;
use crate::model::{Block, Document, Element, Meta};
#[cfg(not(target_arch = "wasm32"))]
use crate::settings::Settings;

#[cfg(not(target_arch = "wasm32"))]
fn data_home() -> PathBuf {
    if let Some(x) = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from) {
        if x.is_absolute() {
            return x;
        }
    }
    dirs::data_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local/share")
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn library_root() -> PathBuf {
    library_in(&data_home())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn library_in(home: &Path) -> PathBuf {
    let found = crate::INFO.folder_in(home, ncf::OS);
    let legacy = home.join("northstar");
    if !found.exists() && legacy.is_dir() {
        legacy
    } else {
        found
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn scripts_dir() -> PathBuf {
    library_root().join("scripts")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn exports_dir() -> PathBuf {
    library_root().join("exports")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn snapshots_dir() -> PathBuf {
    library_root().join("snapshots")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn settings_path() -> PathBuf {
    library_root().join("settings.conf")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ensure_dirs() -> io::Result<()> {
    fs::create_dir_all(scripts_dir())?;
    fs::create_dir_all(exports_dir())?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub title: String,
    pub modified: SystemTime,
    pub preview: String,
    pub starred: bool,
    pub pages: usize,
    pub scenes: usize,
    pub edited_by: Option<Person>,
    pub editing: Option<Person>,
    pub author: String,
    pub cloud: Option<String>,
    pub waiting: bool,
    pub cover: Option<usize>,
    pub deadline: Option<(chrono::NaiveDate, String)>,
    pub draft: String,
    pub revision: Option<usize>,
    pub notes: Vec<(String, String)>,
    pub timelines: usize,
}

impl Default for Entry {
    fn default() -> Entry {
        Entry {
            path: PathBuf::new(),
            title: String::new(),
            modified: SystemTime::UNIX_EPOCH,
            preview: String::new(),
            starred: false,
            pages: 0,
            scenes: 0,
            edited_by: None,
            editing: None,
            author: String::new(),
            cloud: None,
            waiting: false,
            cover: None,
            deadline: None,
            draft: String::new(),
            revision: None,
            notes: Vec::new(),
            timelines: 0,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn list_scripts() -> Vec<Entry> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(scripts_dir()) else {
        return out;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let text = fs::read_to_string(&path).unwrap_or_default();
        let timelines = list_timelines(&path).len();
        out.push(Entry { timelines, ..entry_from_text(path, &text, modified) });
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.modified));
    out
}

#[cfg(not(target_arch = "wasm32"))]
pub fn entry_from_text(path: PathBuf, text: &str, modified: SystemTime) -> Entry {
    let (meta, body) = split_front_matter(text);
    let title = if meta.title.trim().is_empty() {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string()
    } else {
        meta.title.clone()
    };
    let preview = body
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with("<!--"))
        .unwrap_or("")
        .trim_start_matches(['#', '*', '>', '`', ' '])
        .chars()
        .take(70)
        .collect::<String>();
    let doc = from_markdown(text);
    Entry {
        path,
        title,
        modified,
        preview,
        starred: meta.starred,
        cover: meta.cover,
        deadline: meta.deadline.clone(),
        draft: meta.draft.trim().to_string(),
        revision: meta.revision,
        notes: notes_of(&doc),
        pages: export::page_count(&doc),
        scenes: doc.scene_count(),
        author: meta.author.clone(),
        ..Default::default()
    }
}

// covers here---

pub const COVERS: [&str; 4] = ["coral", "lilac", "sun", "sky"];

pub fn cover_named(v: &str) -> Option<usize> {
    COVERS.iter().position(|c| c.eq_ignore_ascii_case(v.trim()))
}

pub fn stem_cover(path: &Path) -> usize {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    let mut h: u32 = 0x811c_9dc5;
    for b in stem.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    (h % COVERS.len() as u32) as usize
}

pub const NOTES_KEPT: usize = 6;

pub fn notes_of(doc: &Document) -> Vec<(String, String)> {
    let scenes = doc.scenes();
    doc.blocks
        .iter()
        .filter(|b| !b.remark.trim().is_empty())
        .take(NOTES_KEPT)
        .map(|b| {
            let at = doc
                .scene_of(b.id)
                .map(|k| match scenes.get(k) {
                    Some(sc) if !sc.heading.trim().is_empty() => format!("scene {}", k + 1),
                    _ => String::new(),
                })
                .unwrap_or_default();
            (b.remark.trim().to_string(), at)
        })
        .collect()
}

pub fn cover_of(e: &Entry) -> usize {
    e.cover.unwrap_or_else(|| stem_cover(&e.path)) % COVERS.len()
}

pub fn slugify(title: &str) -> String {
    let mut s = String::new();
    let mut prev_dash = false;
    for ch in title.trim().chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                s.push(lower);
            }
            prev_dash = false;
        } else if !prev_dash && !s.is_empty() {
            s.push('-');
            prev_dash = true;
        }
    }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "untitled".to_string()
    } else {
        s
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn unique_path(title: &str, ignore: Option<&Path>) -> PathBuf {
    let base = slugify(title);
    let dir = scripts_dir();
    let mut n = 1;
    loop {
        let name = if n == 1 {
            format!("{base}.md")
        } else {
            format!("{base}-{n}.md")
        };
        let candidate = dir.join(name);
        let taken = candidate.exists() && Some(candidate.as_path()) != ignore;
        if !taken {
            return candidate;
        }
        n += 1;
    }
}

// serialisation here---

pub fn to_markdown(doc: &Document) -> String {
    let mut s = String::new();
    s.push_str("---\n");
    s.push_str(&format!("title: {}\n", esc(&doc.meta.title)));
    s.push_str(&format!("author: {}\n", esc(&doc.meta.author)));
    s.push_str(&format!("contact: {}\n", esc(&doc.meta.contact)));
    s.push_str(&format!("draft: {}\n", esc(&doc.meta.draft)));
    if doc.meta.starred {
        s.push_str("starred: yes\n");
    }
    if let Some(c) = doc.meta.cover {
        s.push_str(&format!("cover: {}\n", COVERS[c % COVERS.len()]));
    }
    if let Some((d, label)) = &doc.meta.deadline {
        s.push_str(&format!("deadline: {}\n", crate::stats::write_deadline(*d, label)));
    }
    let voices: Vec<String> = doc
        .meta
        .voices
        .iter()
        .filter(|(n, _)| voice_name_ok(n))
        .map(|(n, h)| format!("{}={}", n.trim(), h % 360))
        .collect();
    if !voices.is_empty() {
        s.push_str(&format!("voices: {}\n", voices.join("; ")));
    }
    let lines: Vec<String> = doc
        .meta
        .storylines
        .iter()
        .filter(|(_, n)| voice_name_ok(n))
        .map(|(t, n)| format!("{t}={}", n.trim()))
        .collect();
    if !lines.is_empty() {
        s.push_str(&format!("storylines: {}\n", lines.join("; ")));
    }
    if let Some(r) = doc.meta.revision {
        s.push_str(&format!("revision: {}\n", crate::revision::slug(r)));
        if !doc.meta.revision_date.trim().is_empty() {
            s.push_str(&format!("revision_date: {}\n", esc(&doc.meta.revision_date)));
        }
        if !doc.meta.revision_base.trim().is_empty() {
            s.push_str(&format!("revision_base: {}\n", esc(&doc.meta.revision_base)));
        }
        if doc.meta.locked {
            s.push_str("locked: yes\n");
        }
    }
    if let Some(t) = &doc.meta.timeline {
        s.push_str(&timeline_keys(t));
    }
    s.push_str("---\n\n");

    for b in &doc.blocks {
        let text = b.text.trim();
        if b.element == Element::SceneHeading {
            if let Some(link) = b.link.as_deref().filter(|l| sid_ok(l)) {
                s.push_str(&format!("## @same {link}\n\n"));
                continue;
            }
        }
        if text.is_empty() && !(b.element == Element::SceneHeading && has_card(b)) && b.element != Element::Act {
            continue;
        }
        let line = match b.element {
            Element::SceneHeading => format!("## {}", with_remark(text, &b.remark)),
            Element::Shot => format!("### {}", with_remark(text, &b.remark)),
            Element::Action => with_remark(text, &b.remark),
            Element::Character => {
                let cue = if b.dual && !text.is_empty() { format!("{text} ^") } else { text.to_string() };
                format!("**{}**", with_remark(&cue, &b.remark))
            }
            Element::Parenthetical => {
                let inner = text.trim_matches(|c| c == '(' || c == ')').trim();
                format!("*({})*", with_remark(inner, &b.remark))
            }
            Element::Dialogue => format!("> {}", with_remark(text, &b.remark)),
            Element::Transition => format!("`{}`", with_remark(text, &b.remark)),
            Element::Act => format!("# {}", with_remark(text, &b.remark)),
        };
        s.push_str(line.trim_end());
        if b.element == Element::SceneHeading && has_card(b) {
            s.push('\n');
            s.push_str(&card_comment(b));
        }
        s.push_str("\n\n");
    }
    s
}

fn with_remark(text: &str, remark: &str) -> String {
    let r = remark.replace('\n', " ").replace("]]", "] ]");
    let r = r.trim();
    if r.is_empty() || text.trim().is_empty() {
        text.to_string()
    } else {
        format!("{text} [[{r}]]")
    }
}

fn split_remark(s: &str) -> (&str, &str) {
    if let Some(body) = s.strip_suffix("]]") {
        if let Some(i) = body.rfind(" [[") {
            let text = &body[..i];
            let remark = &body[i + 3..];
            let tidy = !text.trim().is_empty()
                && !text.ends_with(char::is_whitespace)
                && !remark.is_empty()
                && remark.trim() == remark
                && !remark.contains("]]");
            if tidy {
                return (text, remark);
            }
        }
    }
    (s, "")
}

fn split_dual(cue: &str) -> (&str, bool) {
    match cue.strip_suffix(" ^") {
        Some(name) if !name.trim().is_empty() && !name.ends_with(char::is_whitespace) => (name, true),
        _ => (cue, false),
    }
}

fn has_card(b: &Block) -> bool {
    !b.note.trim().is_empty() || b.tint.is_some() || b.sid.as_deref().is_some_and(sid_ok)
}

pub fn sid_ok(id: &str) -> bool {
    (1..=16).contains(&id.len()) && id.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

fn card_comment(b: &Block) -> String {
    let note = b.note.replace('\n', " ").replace("-->", "- ->");
    let note = note.trim();
    let body = match b.sid.as_deref().filter(|id| sid_ok(id)) {
        Some(id) if note.is_empty() => format!("| id={id}"),
        Some(id) => format!("{note} | id={id}"),
        None => note.to_string(),
    };
    match b.tint {
        Some(t) => format!("<!-- scene: tint={t} | {body} -->"),
        None => format!("<!-- scene: {body} -->"),
    }
}

struct Card {
    tint: Option<usize>,
    note: String,
    sid: Option<String>,
}

fn parse_card(line: &str) -> Option<Card> {
    let inner = line.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
    let rest = inner.strip_prefix("scene:")?.trim();
    let (tint, note) = if let Some(after) = rest.strip_prefix("tint=") {
        let (num, note) = match after.split_once('|') {
            Some((n, note)) => (n.trim(), note.trim()),
            None => (after.trim(), ""),
        };
        (num.parse::<usize>().ok(), note.to_string())
    } else {
        (None, rest.to_string())
    };
    let (note, sid) = split_sid(&note);
    Some(Card { tint, note, sid })
}

fn split_sid(note: &str) -> (String, Option<String>) {
    let Some(at) = note.rfind("| id=") else {
        return (note.to_string(), None);
    };
    let id = note[at + 5..].trim_end();
    let head = &note[..at];
    let clean = head.is_empty() || head.ends_with(' ');
    if !clean || !sid_ok(id) {
        return (note.to_string(), None);
    }
    (head.trim_end().to_string(), Some(id.to_string()))
}

fn timeline_keys(t: &crate::model::TimelineMeta) -> String {
    let mut s = String::new();
    s.push_str(&format!("timeline-id: {}\n", esc(&t.id)));
    s.push_str(&format!("timeline: {}\n", esc(&t.name)));
    s.push_str(&format!("parent: {}\n", t.parent.as_deref().map(esc).unwrap_or_else(|| "main".into())));
    s.push_str(&format!("from: {}\n", t.from.as_deref().map(esc).unwrap_or_else(|| "start".into())));
    if !t.choice.trim().is_empty() {
        s.push_str(&format!("choice: {}\n", esc(&t.choice)));
    }
    if !t.stay.trim().is_empty() {
        s.push_str(&format!("stay: {}\n", esc(&t.stay)));
    }
    if let Some(m) = &t.merges_into {
        s.push_str(&format!("merges-into: {}\n", esc(m)));
    }
    if let Some(c) = t.color {
        s.push_str(&format!("color: {c}\n"));
    }
    s
}

fn voice_name_ok(name: &str) -> bool {
    let n = name.trim();
    !n.is_empty() && !n.contains([';', '=', '\n'])
}

fn parse_voices(v: &str) -> Vec<(String, u16)> {
    let mut out: Vec<(String, u16)> = Vec::new();
    for part in v.split(';') {
        let Some((name, hue)) = part.split_once('=') else {
            continue;
        };
        let name = name.trim().to_uppercase();
        let Ok(hue) = hue.trim().parse::<u16>() else {
            continue;
        };
        if voice_name_ok(&name) && !out.iter().any(|(n, _)| *n == name) {
            out.push((name, hue % 360));
        }
    }
    out
}

fn parse_storylines(v: &str) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    for part in v.split(';') {
        let Some((tint, name)) = part.split_once('=') else {
            continue;
        };
        let Ok(tint) = tint.trim().parse::<usize>() else {
            continue;
        };
        let name = name.trim();
        if voice_name_ok(name) && !out.iter().any(|(t, _)| *t == tint) {
            out.push((tint, name.to_string()));
        }
    }
    out.sort_by_key(|(t, _)| *t);
    out
}

fn esc(s: &str) -> String {
    s.replace('\n', " ").trim().to_string()
}

fn split_front_matter(text: &str) -> (Meta, &str) {
    let mut meta = Meta::default();
    let rest = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n"));
    let Some(rest) = rest else {
        return (meta, text);
    };
    let mut idx = 0usize;
    let mut body_start = None;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end();
        idx += line.len();
        if trimmed == "---" {
            body_start = Some(idx);
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            let v = v.trim().to_string();
            match k.trim() {
                "title" => meta.title = v,
                "author" => meta.author = v,
                "contact" => meta.contact = v,
                "draft" => meta.draft = v,
                "starred" => meta.starred = matches!(v.as_str(), "yes" | "true" | "1"),
                "cover" => meta.cover = cover_named(&v),
                "deadline" => meta.deadline = crate::stats::read_deadline(&v),
                "voices" => meta.voices = parse_voices(&v),
                "storylines" => meta.storylines = parse_storylines(&v),
                "revision" => meta.revision = crate::revision::from_slug(&v),
                "revision_date" => meta.revision_date = v,
                "revision_base" => meta.revision_base = v,
                "locked" => meta.locked = matches!(v.as_str(), "yes" | "true" | "1"),
                "timeline-id" => meta.timeline.get_or_insert_with(Default::default).id = v,
                "timeline" => meta.timeline.get_or_insert_with(Default::default).name = v,
                "parent" => {
                    meta.timeline.get_or_insert_with(Default::default).parent = (v != "main" && !v.is_empty()).then_some(v)
                }
                "from" => meta.timeline.get_or_insert_with(Default::default).from = (v != "start" && !v.is_empty()).then_some(v),
                "choice" => meta.timeline.get_or_insert_with(Default::default).choice = v,
                "stay" => meta.timeline.get_or_insert_with(Default::default).stay = v,
                "merges-into" => meta.timeline.get_or_insert_with(Default::default).merges_into = (!v.is_empty()).then_some(v),
                "color" => meta.timeline.get_or_insert_with(Default::default).color = v.parse().ok(),
                _ => {}
            }
        }
    }
    match body_start {
        Some(i) => (meta, &rest[i..]),
        None => (meta, text),
    }
}

pub fn from_markdown(text: &str) -> Document {
    let (meta, body) = split_front_matter(text);
    let mut doc = Document::from_parts(meta, Vec::new());
    doc.blocks.clear();

    let mut id = 0u64;

    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(card) = parse_card(line) {
            if let Some(last) = doc.blocks.last_mut() {
                if last.element == Element::SceneHeading && last.link.is_none() {
                    last.tint = card.tint;
                    last.note = card.note;
                    last.sid = card.sid;
                }
            }
            continue;
        }
        if let Some(link) = line.strip_prefix("## @same ").map(str::trim).filter(|l| sid_ok(l)) {
            add(&mut doc, &mut id, Element::SceneHeading, "", false, false);
            if let Some(last) = doc.blocks.last_mut() {
                last.link = Some(link.to_string());
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("### ") {
            add(&mut doc, &mut id, Element::Shot, rest, true, false);
        } else if let Some(rest) = line.strip_prefix("## ") {
            add(&mut doc, &mut id, Element::SceneHeading, rest, true, false);
        } else if line == "##" {
            add(&mut doc, &mut id, Element::SceneHeading, "", false, false);
        } else if let Some(rest) = line.strip_prefix("# ") {
            add(&mut doc, &mut id, Element::Act, rest, true, false);
        } else if line == "#" {
            add(&mut doc, &mut id, Element::Act, "", false, false);
        } else if let Some(rest) = line.strip_prefix("> ") {
            add(&mut doc, &mut id, Element::Dialogue, rest, false, false);
        } else if line == ">" {
            add(&mut doc, &mut id, Element::Dialogue, "", false, false);
        } else if line.len() >= 4 && line.starts_with("**") && line.ends_with("**") {
            add(&mut doc, &mut id, Element::Character, &line[2..line.len() - 2], true, true);
        } else if line.len() >= 2
            && line.starts_with('`')
            && line.ends_with('`')
            && !line[1..line.len() - 1].contains('`')
        {
            add(&mut doc, &mut id, Element::Transition, &line[1..line.len() - 1], true, false);
        } else if line.starts_with("*(") && line.ends_with(")*") {
            push_paren(&mut doc, &mut id, &line[2..line.len() - 2]);
        } else if line.starts_with('(') && line.ends_with(')') {
            push_paren(&mut doc, &mut id, &line[1..line.len() - 1]);
        } else {
            add(&mut doc, &mut id, Element::Action, line, false, false);
        }
    }

    doc.ensure_not_empty();
    doc.dedupe_sids();
    doc.reseed_ids();
    doc
}

fn add(doc: &mut Document, id: &mut u64, element: Element, raw: &str, upper: bool, dual: bool) {
    let (text, remark) = split_remark(raw.trim());
    let (text, dual) = if dual { split_dual(text) } else { (text, false) };
    let text = if upper { text.trim().to_uppercase() } else { text.trim().to_string() };
    *id += 1;
    let mut b = Block::new(*id, element, &text);
    b.remark = remark.to_string();
    b.dual = dual;
    doc.blocks.push(b);
}

fn push_paren(doc: &mut Document, id: &mut u64, inside: &str) {
    let (text, remark) = split_remark(inside.trim());
    add(doc, id, Element::Parenthetical, &format!("({})", text.trim()), false, false);
    if let Some(last) = doc.blocks.last_mut() {
        last.remark = remark.to_string();
    }
}

#[cfg(not(target_arch = "wasm32"))]
static FAIL_NEXT_SAVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(not(target_arch = "wasm32"))]
pub fn fail_next_save() {
    FAIL_NEXT_SAVE.store(true, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save(path: &Path, doc: &Document) -> io::Result<()> {
    if FAIL_NEXT_SAVE.swap(false, std::sync::atomic::Ordering::SeqCst) {
        return Err(io::Error::other("the disk refused the write (a test failure from debug mode)"));
    }
    ensure_dirs()?;
    if is_timeline_path(path) {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
    }
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, to_markdown(doc))?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load(path: &Path) -> io::Result<Document> {
    let text = fs::read_to_string(path)?;
    Ok(from_markdown(&text))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn delete(path: &Path) -> io::Result<()> {
    fs::remove_file(path)
}

// search inside here---

#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub block: usize,
    pub line: String,
    pub start: usize,
    pub len: usize,
    pub note: bool,
}

#[derive(Clone, Debug)]
pub struct Found {
    pub path: PathBuf,
    pub title: String,
    pub hits: Vec<Hit>,
    pub total: usize,
}

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn find_chars(hay: &[char], needle: &[char]) -> Option<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()].iter().zip(needle).all(|(a, b)| fold(*a) == *b))
}

fn excerpt(chars: &[char], at: usize, len: usize) -> (String, usize) {
    const LEAD: usize = 28;
    const ROOM: usize = 110;
    let from = at.saturating_sub(LEAD);
    let to = (from + ROOM).max(at + len).min(chars.len());
    let mut out: String = chars[from..to].iter().collect();
    let mut start = at - from;
    if from > 0 {
        out.insert(0, '…');
        start += 1;
    }
    if to < chars.len() {
        out.push('…');
    }
    (out, start)
}

fn flat(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn find_in(text: &str, needle: &str, keep: usize) -> (Vec<Hit>, usize) {
    let needle: Vec<char> = flat(needle).chars().map(fold).collect();
    let doc = from_markdown(text);
    let mut hits = Vec::new();
    let mut total = 0;
    for (k, b) in doc.blocks.iter().enumerate() {
        for (body, note) in [(&b.text, false), (&b.remark, true)] {
            let hay: Vec<char> = flat(body).chars().collect();
            if let Some(at) = find_chars(&hay, &needle) {
                total += 1;
                if hits.len() < keep {
                    let (line, start) = excerpt(&hay, at, needle.len());
                    hits.push(Hit { block: k, line, start, len: needle.len(), note });
                }
            }
        }
    }
    (hits, total)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn search_scripts(needle: &str, keep: usize) -> Vec<Found> {
    let mut entries = Vec::new();
    for e in list_scripts() {
        for p in list_timelines(&e.path) {
            let name = fs::read_to_string(&p)
                .map(|t| split_front_matter(&t).0)
                .ok()
                .and_then(|m| m.timeline)
                .map(|t| t.name)
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string());
            entries.push(Entry { path: p, title: format!("{} · {}", e.title, name), ..Default::default() });
        }
        entries.push(e);
    }
    search_entries(entries, needle, keep)
}

// timelines here---

pub const TIMELINES_EXT: &str = "timelines";

pub fn timelines_dir(main: &Path) -> PathBuf {
    let stem = main.file_stem().and_then(|s| s.to_str()).unwrap_or("untitled");
    main.with_file_name(format!("{stem}.{TIMELINES_EXT}"))
}

pub fn is_timeline_path(p: &Path) -> bool {
    p.parent()
        .and_then(|d| d.extension())
        .and_then(|e| e.to_str())
        == Some(TIMELINES_EXT)
}

pub fn main_of(branch: &Path) -> Option<PathBuf> {
    if !is_timeline_path(branch) {
        return None;
    }
    let dir = branch.parent()?;
    let stem = dir.file_stem()?.to_str()?;
    Some(dir.with_file_name(format!("{stem}.md")))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn list_timelines(main: &Path) -> Vec<PathBuf> {
    if is_timeline_path(main) {
        return Vec::new();
    }
    let Ok(rd) = fs::read_dir(timelines_dir(main)) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect();
    out.sort();
    out
}

#[cfg(not(target_arch = "wasm32"))]
pub fn unique_timeline_path(main: &Path, name: &str, taken: &[PathBuf]) -> PathBuf {
    let dir = timelines_dir(main);
    let base = slugify(name);
    let mut n = 1;
    loop {
        let file = if n == 1 { format!("{base}.md") } else { format!("{base}-{n}.md") };
        let p = dir.join(file);
        if !p.exists() && !taken.contains(&p) {
            return p;
        }
        n += 1;
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn delete_timelines(main: &Path) -> io::Result<()> {
    let dir = timelines_dir(main);
    if dir.is_dir() && !is_timeline_path(main) {
        for p in list_timelines(main) {
            fs::remove_file(p)?;
        }
        let _ = fs::remove_dir(&dir);
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn copy_timelines(from: &Path, to: &Path) -> io::Result<usize> {
    let files = list_timelines(from);
    if files.is_empty() {
        return Ok(0);
    }
    let dir = timelines_dir(to);
    fs::create_dir_all(&dir)?;
    for p in &files {
        if let Some(name) = p.file_name() {
            fs::copy(p, dir.join(name))?;
        }
    }
    Ok(files.len())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn search_entries(entries: Vec<Entry>, needle: &str, keep: usize) -> Vec<Found> {
    let mut out = Vec::new();
    if needle.trim().chars().count() < 2 {
        return out;
    }
    for e in entries {
        let Ok(text) = fs::read_to_string(&e.path) else {
            continue;
        };
        let (hits, total) = find_in(&text, needle, keep);
        if total > 0 {
            out.push(Found { path: e.path, title: e.title, hits, total });
        }
    }
    out.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase())));
    out
}

// snapshots here---

#[cfg(not(target_arch = "wasm32"))]
pub fn snapshot_folder(script: &Path) -> PathBuf {
    let stem = script
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled");
    if let Some(dir) = script.parent().filter(|_| is_timeline_path(script)).and_then(|d| d.file_name()) {
        return snapshots_dir().join(dir).join(stem);
    }
    snapshots_dir().join(stem)
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub path: PathBuf,
    pub when: String,
    pub pages: usize,
    pub by: Option<Person>,
    pub label: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn take_snapshot(script: &Path, doc: &Document) -> io::Result<PathBuf> {
    let dir = snapshot_folder(script);
    fs::create_dir_all(&dir)?;
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let mut path = dir.join(format!("{stamp}.md"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stamp}-{n}.md"));
        n += 1;
    }
    fs::write(&path, to_markdown(doc))?;
    Ok(path)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn list_snapshots(script: &Path) -> Vec<Snapshot> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(snapshot_folder(script)) else {
        return out;
    };
    for e in rd.flatten() {
        let path = e.path();
        if path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let when = match stem.split_once('_') {
            Some((d, t)) => format!("{d} {}", t.replace('-', ":")),
            None => stem.clone(),
        };
        let pages = load(&path).map(|d| export::page_count(&d)).unwrap_or(0);
        out.push(Snapshot {
            path,
            when,
            pages,
            by: None,
            label: None,
        });
    }
    out.sort_by(|a, b| b.path.cmp(&a.path));
    out
}

#[cfg(not(target_arch = "wasm32"))]
pub fn follow_rename(from: &Path, to: &Path) {
    let old = snapshot_folder(from);
    if old.is_dir() {
        let new = snapshot_folder(to);
        if !new.exists() {
            let _ = fs::rename(old, new);
        }
    }
    if is_timeline_path(from) {
        return;
    }
    let old = timelines_dir(from);
    if old.is_dir() {
        let new = timelines_dir(to);
        if !new.exists() {
            let _ = fs::rename(old, new);
        }
    }
    let name = |p: &Path| timelines_dir(p).file_name().map(|n| snapshots_dir().join(n));
    if let (Some(old), Some(new)) = (name(from), name(to)) {
        if old.is_dir() && !new.exists() {
            let _ = fs::rename(old, new);
        }
    }
}

// settings here---

#[cfg(not(target_arch = "wasm32"))]
pub fn read_settings() -> Settings {
    fs::read_to_string(settings_path())
        .map(|t| Settings::parse(&t))
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_settings(s: &Settings) -> io::Result<()> {
    fs::create_dir_all(library_root())?;
    fs::write(settings_path(), s.serialize())
}

// desktop here---

#[cfg(not(target_arch = "wasm32"))]
fn quiet(program: &str) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_error_log(detail: &str) -> Option<PathBuf> {
    let root = library_root();
    let log = root.join("error.log");
    let line = format!(
        "{} nativedraft {}\n{}\n\n",
        chrono::Local::now().to_rfc3339(),
        env!("CARGO_PKG_VERSION"),
        detail
    );
    let saved = fs::create_dir_all(&root).is_ok()
        && fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .and_then(|mut f| io::Write::write_all(&mut f, line.as_bytes()))
            .is_ok();
    saved.then_some(log)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_with_desktop(path: &Path) {
    open_target(path.as_os_str());
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_target(target: &std::ffi::OsStr) {
    #[cfg(windows)]
    let _ = quiet("explorer").arg(target).spawn();
    #[cfg(target_os = "macos")]
    let _ = quiet("open").arg(target).spawn();
    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = quiet("xdg-open").arg(target).spawn();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn reveal_in_file_manager(path: &Path) {
    #[cfg(windows)]
    {
        let mut select = std::ffi::OsString::from("/select,");
        select.push(path.as_os_str());
        let _ = quiet("explorer").arg(select).spawn();
    }
    #[cfg(target_os = "macos")]
    ncf::platform::reveal(path);
    #[cfg(not(any(windows, target_os = "macos")))]
    reveal_freedesktop(path);
}

#[cfg(all(not(target_arch = "wasm32"), not(windows), not(target_os = "macos")))]
fn reveal_freedesktop(path: &Path) {
    let uri = format!("file://{}", path.display());
    let spoke = quiet("dbus-send")
        .args([
            "--session",
            "--print-reply",
            "--dest=org.freedesktop.FileManager1",
            "--type=method_call",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1.ShowItems",
        ])
        .arg(format!("array:string:{uri}"))
        .arg("string:")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !spoke {
        let dir = path.parent().unwrap_or(path);
        open_with_desktop(dir);
    }
}

#[cfg(windows)]
pub fn pick_file_to_import() -> Result<Option<PathBuf>, String> {
    let script = "Add-Type -AssemblyName System.Windows.Forms; \
        $d = New-Object System.Windows.Forms.OpenFileDialog; \
        $d.Title = 'Import a screenplay'; \
        $d.Filter = 'Screenplays|*.fountain;*.spmd;*.fdx;*.md;*.txt|All files|*.*'; \
        if ($d.ShowDialog() -eq 'OK') { [Console]::Out.Write($d.FileName) }";
    let out = quiet("powershell")
        .args(["-NoProfile", "-NonInteractive", "-STA", "-Command", script])
        .output()
        .map_err(|e| format!("Could not open the file dialog ({e}) — drop the file onto the window instead."))?;
    let picked = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(if out.status.success() && !picked.is_empty() {
        Some(PathBuf::from(picked))
    } else {
        None
    })
}

pub const IMPORTABLE: [&str; 5] = ["fountain", "spmd", "fdx", "md", "txt"];

#[cfg(target_os = "macos")]
pub fn pick_file_to_import() -> Result<Option<PathBuf>, String> {
    ncf::platform::pick_file(&ncf::platform::FileFilter::new("Import a screenplay", "Screenplays", &IMPORTABLE))
}

#[cfg(all(not(target_arch = "wasm32"), not(windows), not(target_os = "macos")))]
pub fn pick_file_to_import() -> Result<Option<PathBuf>, String> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let tries: [(&str, Vec<String>); 2] = [
        (
            "kdialog",
            vec![
                "--title".into(),
                "Import a screenplay".into(),
                "--getopenfilename".into(),
                home.display().to_string(),
                "Screenplays (*.fountain *.spmd *.fdx *.md *.txt)".into(),
            ],
        ),
        (
            "zenity",
            vec![
                "--file-selection".into(),
                "--title=Import a screenplay".into(),
                "--file-filter=Screenplays | *.fountain *.spmd *.fdx *.md *.txt".into(),
            ],
        ),
    ];
    for (tool, args) in tries {
        match std::process::Command::new(tool).args(&args).output() {
            Ok(out) => {
                let picked = String::from_utf8_lossy(&out.stdout).trim().to_string();
                return Ok(if out.status.success() && !picked.is_empty() {
                    Some(PathBuf::from(picked))
                } else {
                    None
                });
            }
            Err(_) => continue,
        }
    }
    Err("Neither kdialog nor zenity is installed — drop the file onto the window instead.".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn import_file(path: &Path) -> Result<Document, String> {
    import_file_checked(path).map(|(d, _)| d)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn import_file_checked(path: &Path) -> Result<(Document, Vec<crate::fountain::Issue>), String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    Ok(import_text_checked(name, &text))
}

pub fn import_text(name: &str, text: &str) -> Document {
    import_text_checked(name, text).0
}

pub fn import_text_checked(name: &str, text: &str) -> (Document, Vec<crate::fountain::Issue>) {
    let path = Path::new(name);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let (mut doc, issues) = match ext.as_str() {
        "md" | "markdown" => (from_markdown(text), Vec::new()),
        "fdx" => crate::fountain::parse_fdx_checked(text),
        _ => crate::fountain::parse_checked(text),
    };
    if doc.meta.title.trim().is_empty() {
        doc.meta.title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Imported Script")
            .replace(['-', '_'], " ");
    }
    doc.normalize();
    (doc, issues)
}
