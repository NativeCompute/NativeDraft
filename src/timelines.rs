use std::path::{Path, PathBuf};

use crate::model::{Block, Document, Element, TimelineMeta};
use crate::storage::sid_ok;

pub type Tl = Option<usize>;

pub const COLORS: usize = 6;

#[derive(Clone, Debug)]
pub struct Timeline {
    pub path: PathBuf,
    pub doc: Document,
}

impl Timeline {
    pub fn meta(&self) -> &TimelineMeta {
        static NONE: std::sync::OnceLock<TimelineMeta> = std::sync::OnceLock::new();
        self.doc.meta.timeline.as_ref().unwrap_or_else(|| NONE.get_or_init(TimelineMeta::default))
    }

    pub fn meta_mut(&mut self) -> &mut TimelineMeta {
        self.doc.meta.timeline.get_or_insert_with(Default::default)
    }
}

#[derive(Clone, Debug)]
pub struct Bundle {
    pub main_path: PathBuf,
    pub main: Document,
    pub items: Vec<Timeline>,
}

#[derive(Clone, Debug)]
pub struct Step {
    pub tl: Tl,
    pub sid: Option<String>,
    pub scene: bool,
    pub same: Option<Tl>,
    pub gap: bool,
    pub head: Option<usize>,
    pub blocks: Vec<Block>,
    pub origin: Vec<(Tl, usize)>,
}

impl Step {
    pub fn heading(&self) -> String {
        self.blocks
            .iter()
            .find(|b| b.element == Element::SceneHeading)
            .map(|b| b.text.clone())
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Story {
    pub steps: Vec<Step>,
    pub own: std::ops::Range<usize>,
    pub merged_at: Option<usize>,
}

impl Story {
    pub fn scenes_before(&self, step: usize) -> usize {
        self.steps[..step.min(self.steps.len())].iter().filter(|s| s.scene).count()
    }

    pub fn find(&self, sid: &str) -> Option<usize> {
        self.steps.iter().rposition(|s| s.scene && s.sid.as_deref() == Some(sid))
    }
}

fn slug_id(path: &Path) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("timeline");
    let id: String = stem
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .take(16)
        .collect();
    if id.is_empty() {
        "timeline".to_string()
    } else {
        id
    }
}

impl Bundle {
    pub fn new(main_path: PathBuf, main: Document, items: Vec<(PathBuf, Document)>) -> Bundle {
        let mut out = Bundle { main_path, main, items: Vec::new() };
        let mut items = items;
        items.sort_by(|a, b| a.0.cmp(&b.0));
        for (path, mut doc) in items {
            let mut meta = doc.meta.timeline.take().unwrap_or_default();
            if !sid_ok(&meta.id) || meta.id == "main" {
                meta.id = slug_id(&path);
            }
            let base = meta.id.clone();
            let mut n = 2;
            while out.items.iter().any(|t| t.meta().id == meta.id) {
                meta.id = format!("{base}{n}");
                n += 1;
            }
            if meta.name.trim().is_empty() {
                meta.name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Timeline")
                    .replace('-', " ");
            }
            doc.meta.timeline = Some(meta);
            out.items.push(Timeline { path, doc });
        }
        out
    }

    pub fn alone(main_path: PathBuf, main: Document) -> Bundle {
        Bundle { main_path, main, items: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn doc(&self, t: Tl) -> &Document {
        match t {
            None => &self.main,
            Some(i) => &self.items[i].doc,
        }
    }

    pub fn doc_mut(&mut self, t: Tl) -> &mut Document {
        match t {
            None => &mut self.main,
            Some(i) => &mut self.items[i].doc,
        }
    }

    pub fn path(&self, t: Tl) -> &Path {
        match t {
            None => &self.main_path,
            Some(i) => &self.items[i].path,
        }
    }

    pub fn meta(&self, t: Tl) -> Option<&TimelineMeta> {
        t.map(|i| self.items[i].meta())
    }

    pub fn name(&self, t: Tl) -> String {
        match t {
            None => "Main".to_string(),
            Some(i) => self.items[i].meta().name.clone(),
        }
    }

    pub fn color(&self, t: Tl) -> Option<usize> {
        t.map(|i| self.items[i].meta().color.unwrap_or(i) % COLORS)
    }

    pub fn find_path(&self, path: &Path) -> Option<Tl> {
        if path == self.main_path {
            return Some(None);
        }
        self.items.iter().position(|t| t.path == path).map(Some)
    }

    pub fn by_id(&self, id: &str) -> Option<usize> {
        self.items.iter().position(|t| t.meta().id == id)
    }

    pub fn id_of(&self, t: Tl) -> String {
        match t {
            None => "main".to_string(),
            Some(i) => self.items[i].meta().id.clone(),
        }
    }

    pub fn parent(&self, t: Tl) -> Option<Tl> {
        let i = t?;
        match &self.items[i].meta().parent {
            None => Some(None),
            Some(p) => self.by_id(p).map(Some),
        }
    }

    pub fn ancestors(&self, t: Tl) -> Vec<Tl> {
        let mut out = Vec::new();
        let mut at = t;
        while let Some(p) = self.parent(at) {
            if p == t || out.contains(&p) {
                break;
            }
            out.push(p);
            at = p;
        }
        out
    }

    pub fn in_loop(&self, t: Tl) -> bool {
        let mut at = t;
        let mut seen = vec![t];
        while let Some(p) = self.parent(at) {
            if seen.contains(&p) {
                return p == t || self.in_loop(p);
            }
            seen.push(p);
            at = p;
        }
        false
    }

    pub fn rooted(&self, t: Tl) -> bool {
        t.is_none() || self.ancestors(t).last() == Some(&None)
    }

    pub fn children(&self, t: Tl) -> Vec<usize> {
        let id = self.id_of(t);
        (0..self.items.len())
            .filter(|&i| {
                let p = self.items[i].meta().parent.as_deref().unwrap_or("main");
                p == id && Some(i) != t
            })
            .collect()
    }

    pub fn descendants(&self, t: Tl) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        let mut todo = self.children(t);
        while let Some(i) = todo.pop() {
            if out.contains(&i) || Some(i) == t {
                continue;
            }
            out.push(i);
            todo.extend(self.children(Some(i)));
        }
        out.sort();
        out
    }

    pub fn depth(&self, t: Tl) -> usize {
        self.ancestors(t).len()
    }

    pub fn order(&self) -> Vec<Tl> {
        let mut out: Vec<Tl> = vec![None];
        self.walk(None, &mut out);
        for i in 0..self.items.len() {
            if !out.contains(&Some(i)) {
                out.push(Some(i));
                self.walk(Some(i), &mut out);
            }
        }
        out
    }

    fn walk(&self, t: Tl, out: &mut Vec<Tl>) {
        let mut kids = self.children(t);
        let story = story(self, t);
        kids.sort_by_key(|&k| {
            let at = match &self.items[k].meta().from {
                None => 0,
                Some(f) => story.find(f).map(|s| s + 1).unwrap_or(usize::MAX),
            };
            (at, self.items[k].meta().name.to_lowercase())
        });
        for k in kids {
            if out.contains(&Some(k)) {
                continue;
            }
            out.push(Some(k));
            self.walk(Some(k), out);
        }
    }

    pub fn owner_of(&self, sid: &str) -> Option<(Tl, usize)> {
        let all = std::iter::once(None).chain((0..self.items.len()).map(Some));
        for t in all {
            let doc = self.doc(t);
            if let Some(k) = doc
                .blocks
                .iter()
                .position(|b| b.element == Element::SceneHeading && b.link.is_none() && b.sid.as_deref() == Some(sid))
            {
                return Some((t, k));
            }
        }
        None
    }

    fn uses(&self, word: &str) -> bool {
        let all = std::iter::once(None).chain((0..self.items.len()).map(Some));
        for t in all {
            let doc = self.doc(t);
            if doc.blocks.iter().any(|b| b.sid.as_deref() == Some(word) || b.link.as_deref() == Some(word)) {
                return true;
            }
            if let Some(m) = self.meta(t) {
                if m.id == word || m.from.as_deref() == Some(word) || m.merges_into.as_deref() == Some(word) {
                    return true;
                }
            }
        }
        word == "main" || word == "start"
    }

    pub fn new_sid(&self) -> String {
        loop {
            let w = random_word();
            if !self.uses(&w) {
                return w;
            }
        }
    }

    pub fn new_timeline_id(&self) -> String {
        self.new_sid()
    }

    pub fn ensure_sid(&mut self, t: Tl, heading: usize) -> String {
        if let Some(id) = self.doc(t).blocks.get(heading).and_then(|b| b.sid.clone()) {
            return id;
        }
        let id = self.new_sid();
        if let Some(b) = self.doc_mut(t).blocks.get_mut(heading) {
            b.sid = Some(id.clone());
        }
        id
    }

    pub fn next_color(&self) -> usize {
        (0..COLORS)
            .find(|c| !self.items.iter().any(|t| t.meta().color == Some(*c)))
            .unwrap_or(self.items.len() % COLORS)
    }

    pub fn add_branch(&mut self, path: PathBuf, parent: Tl, from: Option<String>, name: &str, choice: &str, stay: &str) -> usize {
        let meta = TimelineMeta {
            id: self.new_timeline_id(),
            name: name.trim().to_string(),
            parent: parent.map(|p| self.items[p].meta().id.clone()),
            from,
            choice: choice.trim().to_string(),
            stay: stay.trim().to_string(),
            merges_into: None,
            color: Some(self.next_color()),
        };
        let mut doc = Document::default();
        doc.meta.title = name.trim().to_string();
        doc.meta.timeline = Some(meta);
        self.items.push(Timeline { path, doc });
        self.items.len() - 1
    }

    pub fn total_scenes(&self) -> usize {
        let mut n = story(self, None).steps.iter().filter(|s| s.scene).count();
        for i in 0..self.items.len() {
            let st = story(self, Some(i));
            n += st.steps[st.own.clone()].iter().filter(|s| s.scene).count();
        }
        n
    }
}

fn random_word() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEED: AtomicU64 = AtomicU64::new(0x2545_F491_4F6C_DD1D);
    let now = web_time::SystemTime::now()
        .duration_since(web_time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut x = now ^ SEED.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    const LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    const ALNUM: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut s = String::new();
    s.push(LETTERS[(x % 26) as usize] as char);
    x /= 26;
    for _ in 0..5 {
        s.push(ALNUM[(x % 36) as usize] as char);
        x /= 36;
    }
    s
}

// stories here---

struct Seg {
    start: usize,
    end: usize,
}

fn segments(doc: &Document) -> (usize, Vec<Seg>) {
    let heads: Vec<usize> = (0..doc.blocks.len())
        .filter(|&i| doc.blocks[i].element == Element::SceneHeading)
        .collect();
    let mut starts: Vec<usize> = Vec::new();
    for &h in &heads {
        let mut s = h;
        while s > 0 && doc.blocks[s - 1].element == Element::Act && !starts.contains(&(s - 1)) {
            s -= 1;
        }
        starts.push(s);
    }
    let prelude = starts.first().copied().unwrap_or(doc.blocks.len());
    let mut segs = Vec::new();
    for (k, &h) in heads.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(doc.blocks.len());
        segs.push(Seg { start: starts[k], end: end.max(h + 1) });
    }
    (prelude, segs)
}

fn head_of(doc: &Document, seg: &Seg) -> usize {
    (seg.start..seg.end)
        .find(|&i| doc.blocks[i].element == Element::SceneHeading)
        .unwrap_or(seg.start)
}

fn blank(doc: &Document, seg: &Seg) -> bool {
    let h = head_of(doc, seg);
    let b = &doc.blocks[h];
    b.link.is_none()
        && b.sid.is_none()
        && b.text.trim().is_empty()
        && b.note.trim().is_empty()
        && doc.blocks[seg.start..seg.end].iter().all(|x| x.text.trim().is_empty())
}

pub fn scene_blocks(doc: &Document, heading: usize) -> Vec<Block> {
    let (_, segs) = segments(doc);
    for seg in &segs {
        if head_of(doc, seg) == heading {
            return doc.blocks[heading..seg.end].to_vec();
        }
    }
    doc.blocks.get(heading).cloned().into_iter().collect()
}

fn gap_block(text: &str) -> Block {
    Block::new(0, Element::SceneHeading, text)
}

pub const GAP_SCENE: &str = "(A SCENE THAT IS NO LONGER THERE)";

pub fn own_steps(b: &Bundle, t: Tl) -> Vec<Step> {
    let doc = b.doc(t);
    let (prelude, segs) = segments(doc);
    let mut out = Vec::new();
    if doc.blocks[..prelude].iter().any(|x| !x.text.trim().is_empty()) {
        out.push(Step {
            tl: t,
            sid: None,
            scene: false,
            same: None,
            gap: false,
            head: None,
            blocks: doc.blocks[..prelude].to_vec(),
            origin: (0..prelude).map(|k| (t, k)).collect(),
        });
    }
    let scope = b.ancestors(t);
    for seg in &segs {
        if blank(doc, seg) {
            continue;
        }
        let h = head_of(doc, seg);
        let head = &doc.blocks[h];
        let lead = doc.blocks[seg.start..h].to_vec();
        let tail = doc.blocks[h + 1..seg.end].to_vec();
        if let Some(link) = &head.link {
            let source = b
                .owner_of(link)
                .filter(|(o, _)| scope.contains(o));
            let mut blocks = lead;
            let mut origin: Vec<(Tl, usize)> = (seg.start..h).map(|k| (t, k)).collect();
            let tail_origin = (h + 1..seg.end).map(|k| (t, k));
            match source {
                Some((o, k)) => {
                    let copy = scene_blocks(b.doc(o), k);
                    origin.extend((0..copy.len()).map(|j| (o, k + j)));
                    blocks.extend(copy);
                    blocks.extend(tail);
                    origin.extend(tail_origin);
                    out.push(Step { tl: t, sid: Some(link.clone()), scene: true, same: Some(o), gap: false, head: Some(h), blocks, origin });
                }
                None => {
                    blocks.push(gap_block(GAP_SCENE));
                    origin.push((t, h));
                    blocks.extend(tail);
                    origin.extend(tail_origin);
                    out.push(Step { tl: t, sid: Some(link.clone()), scene: true, same: None, gap: true, head: Some(h), blocks, origin });
                }
            }
            continue;
        }
        out.push(Step {
            tl: t,
            sid: head.sid.clone(),
            scene: true,
            same: None,
            gap: false,
            head: Some(h),
            blocks: doc.blocks[seg.start..seg.end].to_vec(),
            origin: (seg.start..seg.end).map(|k| (t, k)).collect(),
        });
    }
    out
}

pub fn story(b: &Bundle, t: Tl) -> Story {
    story_in(b, t, 0)
}

fn story_in(b: &Bundle, t: Tl, depth: usize) -> Story {
    let own = own_steps(b, t);
    let Some(i) = t else {
        let n = own.len();
        return Story { steps: own, own: 0..n, merged_at: None };
    };
    if depth > b.items.len() + 1 || b.in_loop(t) {
        let n = own.len();
        return Story { steps: own, own: 0..n, merged_at: None };
    }
    let meta = b.items[i].meta();
    let mut steps: Vec<Step> = Vec::new();
    if let Some(p) = b.parent(t) {
        let ps = story_in(b, p, depth + 1);
        match &meta.from {
            None => steps.extend(ps.steps.iter().take_while(|s| !s.scene).cloned()),
            Some(f) => match ps.find(f) {
                Some(k) => steps.extend(ps.steps[..=k].iter().cloned()),
                None => {
                    steps.extend(ps.steps.iter().take_while(|s| !s.scene).cloned());
                    steps.push(Step {
                        tl: p,
                        sid: Some(f.clone()),
                        scene: true,
                        same: None,
                        gap: true,
                        head: None,
                        blocks: vec![gap_block(GAP_SCENE)],
                        origin: vec![(p, 0)],
                    });
                }
            },
        }
    }
    let start = steps.len();
    steps.extend(own);
    let end = steps.len();
    let mut merged_at = None;
    if let Some(m) = &meta.merges_into {
        if let Some((o, _)) = b.owner_of(m).filter(|(o, _)| b.ancestors(t).contains(o)) {
            let os = story_in(b, o, depth + 1);
            if let Some(k) = os.find(m) {
                merged_at = Some(steps.len());
                steps.extend(os.steps[k..].iter().cloned());
            }
        }
    }
    Story { steps, own: start..end, merged_at }
}

pub fn first_number(b: &Bundle, t: Tl) -> usize {
    let st = story(b, t);
    st.scenes_before(st.own.start) + 1
}

pub fn assemble(b: &Bundle, t: Tl) -> Document {
    assemble_mapped(b, t).0
}

pub fn assemble_mapped(b: &Bundle, t: Tl) -> (Document, Vec<(Tl, usize)>) {
    let st = story(b, t);
    let mut meta = b.main.meta.clone();
    meta.timeline = None;
    if t.is_some() {
        meta.title = format!("{} ({})", b.main.meta.title.trim(), b.name(t));
    }
    let mut blocks = Vec::new();
    let mut origin = Vec::new();
    let mut id = 0u64;
    for step in st.steps {
        for (mut blk, from) in step.blocks.into_iter().zip(step.origin) {
            id += 1;
            blk.id = id;
            blk.sid = None;
            blk.link = None;
            blocks.push(blk);
            origin.push(from);
        }
    }
    (Document::from_parts(meta, blocks), origin)
}

// graph here---

#[derive(Clone, Debug, PartialEq)]
pub struct Dot {
    pub sid: Option<String>,
    pub heading: String,
    pub number: usize,
    pub col: usize,
    pub same: Option<Tl>,
    pub gap: bool,
    pub head: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Row {
    pub tl: Tl,
    pub depth: usize,
    pub dots: Vec<Dot>,
    pub start_col: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    Fork,
    Merge,
    Same,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub kind: EdgeKind,
    pub from: Option<(usize, usize)>,
    pub to: (usize, usize),
    pub label: String,
}

#[derive(Clone, Debug, Default)]
pub struct Graph {
    pub rows: Vec<Row>,
    pub edges: Vec<Edge>,
}

impl Graph {
    pub fn row_of(&self, t: Tl) -> Option<usize> {
        self.rows.iter().position(|r| r.tl == t)
    }

    pub fn cols(&self) -> usize {
        self.rows
            .iter()
            .flat_map(|r| r.dots.iter().map(|d| d.col + 1))
            .max()
            .unwrap_or(0)
    }

    fn dot_with(&self, chain: &[Tl], sid: &str, own_only: bool) -> Option<(usize, usize)> {
        for t in chain {
            let Some(r) = self.row_of(*t) else { continue };
            if let Some(d) = self.rows[r]
                .dots
                .iter()
                .position(|d| d.sid.as_deref() == Some(sid) && !(own_only && d.same.is_some()))
            {
                return Some((r, d));
            }
        }
        None
    }
}

pub fn graph(b: &Bundle) -> Graph {
    let mut g = Graph::default();
    for t in b.order() {
        let st = story(b, t);
        let before = st.scenes_before(st.own.start);
        let mut dots = Vec::new();
        for step in &st.steps[st.own.clone()] {
            if !step.scene {
                continue;
            }
            let number = before + dots.len() + 1;
            dots.push(Dot {
                sid: step.sid.clone(),
                heading: step.heading(),
                number,
                col: number - 1,
                same: step.same,
                gap: step.gap,
                head: step.head,
            });
        }
        g.rows.push(Row { tl: t, depth: b.depth(t), dots, start_col: before });
    }
    for r in 0..g.rows.len() {
        let t = g.rows[r].tl;
        let Some(i) = t else { continue };
        let meta = b.items[i].meta();
        let mut chain = vec![];
        if let Some(p) = b.parent(t) {
            chain.push(p);
            chain.extend(b.ancestors(p));
        }
        if !g.rows[r].dots.is_empty() {
            let from = match &meta.from {
                None => None,
                Some(f) => g.dot_with(&chain, f, false),
            };
            if meta.from.is_none() || from.is_some() {
                g.edges.push(Edge { kind: EdgeKind::Fork, from, to: (r, 0), label: meta.choice.clone() });
            }
        }
        if let Some(m) = &meta.merges_into {
            let last = g.rows[r].dots.len().checked_sub(1);
            if let (Some(last), Some(to)) = (last, g.dot_with(&b.ancestors(t), m, true)) {
                g.edges.push(Edge { kind: EdgeKind::Merge, from: Some((r, last)), to, label: String::new() });
            }
        }
    }
    for r in 0..g.rows.len() {
        for d in 0..g.rows[r].dots.len() {
            let dot = &g.rows[r].dots[d];
            let (Some(src), Some(sid)) = (dot.same, dot.sid.clone()) else { continue };
            if let Some(to) = g.dot_with(&[src], &sid, true) {
                g.edges.push(Edge { kind: EdgeKind::Same, from: Some(to), to: (r, d), label: String::new() });
            }
        }
    }
    g
}

fn outs(g: &Graph, node: (usize, usize)) -> Vec<(usize, usize)> {
    let (r, d) = node;
    let mut out = Vec::new();
    if d + 1 < g.rows[r].dots.len() {
        out.push((r, d + 1));
    }
    for e in &g.edges {
        if e.from == Some(node) && matches!(e.kind, EdgeKind::Fork | EdgeKind::Merge) {
            out.push(e.to);
        }
    }
    out
}

fn starts(g: &Graph) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if let Some(r) = g.row_of(None) {
        if !g.rows[r].dots.is_empty() {
            out.push((r, 0));
        }
    }
    for e in &g.edges {
        if e.kind == EdgeKind::Fork && e.from.is_none() && g.rows[e.to.0].tl.is_some() {
            out.push(e.to);
        }
    }
    out
}

pub fn endings(g: &Graph) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (r, row) in g.rows.iter().enumerate() {
        if row.dots.is_empty() {
            continue;
        }
        let last = (r, row.dots.len() - 1);
        let merges = g.edges.iter().any(|e| e.kind == EdgeKind::Merge && e.from == Some(last));
        if !merges {
            out.push(last);
        }
    }
    out
}

pub fn paths(g: &Graph) -> u64 {
    let mut memo: std::collections::HashMap<(usize, usize), u64> = std::collections::HashMap::new();
    let mut on: Vec<(usize, usize)> = Vec::new();
    starts(g)
        .into_iter()
        .fold(0u64, |acc, s| acc.saturating_add(count(g, s, &mut memo, &mut on)))
}

fn count(
    g: &Graph,
    node: (usize, usize),
    memo: &mut std::collections::HashMap<(usize, usize), u64>,
    on: &mut Vec<(usize, usize)>,
) -> u64 {
    if let Some(n) = memo.get(&node) {
        return *n;
    }
    if on.contains(&node) {
        return 0;
    }
    on.push(node);
    let (r, d) = node;
    let last = d + 1 == g.rows[r].dots.len();
    let merges = g.edges.iter().any(|e| e.kind == EdgeKind::Merge && e.from == Some(node));
    let end = u64::from(last && !merges);
    let n = outs(g, node)
        .into_iter()
        .fold(end, |acc, x| acc.saturating_add(count(g, x, memo, on)));
    on.pop();
    memo.insert(node, n);
    n
}

// issues here---

#[derive(Clone, Debug, PartialEq)]
pub enum Issue {
    MissingParent { tl: usize },
    ParentLoop { tl: usize },
    MissingFork { tl: usize, sid: String },
    MissingSame { tl: Tl, sid: String },
    MissingMerge { tl: usize, sid: String },
    OutsideAncestors { tl: Tl, sid: String },
    MergeBeforeFork { tl: usize },
    EmptyBranch { tl: usize },
}

impl Issue {
    pub fn tl(&self) -> Tl {
        match self {
            Issue::MissingParent { tl }
            | Issue::ParentLoop { tl }
            | Issue::MissingFork { tl, .. }
            | Issue::MissingMerge { tl, .. }
            | Issue::MergeBeforeFork { tl }
            | Issue::EmptyBranch { tl } => Some(*tl),
            Issue::MissingSame { tl, .. } | Issue::OutsideAncestors { tl, .. } => *tl,
        }
    }

    pub fn text(&self, b: &Bundle) -> String {
        let name = b.name(self.tl());
        match self {
            Issue::MissingParent { .. } => format!("{name} branches from a timeline that is no longer there"),
            Issue::ParentLoop { .. } => format!("{name} branches from itself through another timeline"),
            Issue::MissingFork { .. } => format!("{name} branches from a scene that is no longer there"),
            Issue::MissingSame { .. } => format!("{name} shows a Same scene that is no longer there"),
            Issue::MissingMerge { .. } => format!("{name} merges into a scene that is no longer there"),
            Issue::OutsideAncestors { .. } => format!("{name} uses a scene from a timeline it did not come from"),
            Issue::MergeBeforeFork { .. } => format!("{name} merges back before the scene it branched from"),
            Issue::EmptyBranch { .. } => format!("{name} has no scenes of its own yet"),
        }
    }
}

pub fn issues(b: &Bundle) -> Vec<Issue> {
    let mut out = Vec::new();
    for t in std::iter::once(None).chain((0..b.items.len()).map(Some)) {
        let scope = b.ancestors(t);
        for blk in &b.doc(t).blocks {
            let Some(link) = &blk.link else { continue };
            if blk.element != Element::SceneHeading {
                continue;
            }
            match b.owner_of(link) {
                None => out.push(Issue::MissingSame { tl: t, sid: link.clone() }),
                Some((o, _)) if !scope.contains(&o) => out.push(Issue::OutsideAncestors { tl: t, sid: link.clone() }),
                _ => {}
            }
        }
        let Some(i) = t else { continue };
        let meta = b.items[i].meta();
        match b.parent(t) {
            None => {
                out.push(Issue::MissingParent { tl: i });
                continue;
            }
            Some(_) if b.in_loop(t) => {
                out.push(Issue::ParentLoop { tl: i });
                continue;
            }
            _ => {}
        }
        if let Some(f) = &meta.from {
            let parent = b.parent(t).unwrap_or(None);
            if story(b, parent).find(f).is_none() {
                out.push(Issue::MissingFork { tl: i, sid: f.clone() });
            }
        }
        let st = story(b, t);
        if !st.steps[st.own.clone()].iter().any(|s| s.scene) {
            out.push(Issue::EmptyBranch { tl: i });
        }
        if let Some(m) = &meta.merges_into {
            match b.owner_of(m) {
                None => out.push(Issue::MissingMerge { tl: i, sid: m.clone() }),
                Some((o, _)) if !scope.contains(&o) => out.push(Issue::OutsideAncestors { tl: t, sid: m.clone() }),
                Some(_) => {
                    let told = st.steps[..st.own.start].iter().any(|s| s.scene && s.sid.as_deref() == Some(m.as_str()));
                    if told {
                        out.push(Issue::MergeBeforeFork { tl: i });
                    }
                }
            }
        }
    }
    out
}

// references here---

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Refs {
    pub forks: Vec<usize>,
    pub sames: Vec<Tl>,
    pub merges: Vec<usize>,
}

impl Refs {
    pub fn is_empty(&self) -> bool {
        self.forks.is_empty() && self.sames.is_empty() && self.merges.is_empty()
    }

    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        let plural = |n: usize, one: &str, many: &str| if n == 1 { one.to_string() } else { format!("{n} {many}") };
        if !self.forks.is_empty() {
            parts.push(format!(
                "{} from this scene",
                plural(self.forks.len(), "1 timeline branches", "timelines branch")
            ));
        }
        if !self.sames.is_empty() {
            parts.push(format!(
                "{} it as a Same scene",
                plural(self.sames.len(), "1 timeline shows", "timelines show")
            ));
        }
        if !self.merges.is_empty() {
            parts.push(format!(
                "{} into it",
                plural(self.merges.len(), "1 timeline merges", "timelines merge")
            ));
        }
        parts.join(", ")
    }
}

pub fn refs_to(b: &Bundle, sid: &str) -> Refs {
    let mut r = Refs::default();
    for (i, t) in b.items.iter().enumerate() {
        if t.meta().from.as_deref() == Some(sid) {
            r.forks.push(i);
        }
        if t.meta().merges_into.as_deref() == Some(sid) {
            r.merges.push(i);
        }
    }
    for t in std::iter::once(None).chain((0..b.items.len()).map(Some)) {
        if b.doc(t).blocks.iter().any(|x| x.element == Element::SceneHeading && x.link.as_deref() == Some(sid)) {
            r.sames.push(t);
        }
    }
    r
}

pub fn make_own(b: &mut Bundle, t: Tl, heading: usize) -> bool {
    let Some(link) = b.doc(t).blocks.get(heading).and_then(|x| x.link.clone()) else {
        return false;
    };
    let Some((o, k)) = b.owner_of(&link) else {
        return false;
    };
    let copy = scene_blocks(b.doc(o), k);
    let doc = b.doc_mut(t);
    take_in(doc, heading, &copy);
    true
}

pub fn take_in(doc: &mut Document, heading: usize, copy: &[Block]) {
    let mut fresh = Vec::new();
    for (n, src) in copy.iter().enumerate() {
        let mut nb = doc.new_block(src.element, &src.text);
        nb.note = src.note.clone();
        nb.tint = src.tint;
        nb.remark = src.remark.clone();
        nb.dual = src.dual;
        if n > 0 && src.element == Element::SceneHeading {
            nb.sid = None;
        }
        fresh.push(nb);
    }
    if fresh.is_empty() {
        return;
    }
    doc.blocks.splice(heading..=heading, fresh);
}

fn previous_scene(st: &Story, at: usize) -> Option<usize> {
    (0..at).rev().find(|&k| st.steps[k].scene)
}

fn next_scene(st: &Story, at: usize) -> Option<usize> {
    (at + 1..st.steps.len()).find(|&k| st.steps[k].scene)
}

fn anchor_at(b: &mut Bundle, st: &Story, k: usize, changed: &mut Vec<Tl>) -> Option<String> {
    let step = &st.steps[k];
    if let Some(s) = &step.sid {
        if !step.gap {
            return Some(s.clone());
        }
        return None;
    }
    let h = step.head?;
    let t = step.tl;
    let id = b.ensure_sid(t, h);
    if !changed.contains(&t) {
        changed.push(t);
    }
    Some(id)
}

pub fn reanchor_for_delete(b: &mut Bundle, owner: Tl, sid: &str) -> Vec<Tl> {
    let mut changed: Vec<Tl> = Vec::new();
    let refs = refs_to(b, sid);
    for &i in &refs.forks {
        let parent = b.parent(Some(i)).unwrap_or(None);
        let st = story(b, parent);
        let new = match st.find(sid) {
            Some(k) => match previous_scene(&st, k) {
                Some(p) => anchor_at(b, &st, p, &mut changed),
                None => None,
            },
            None => None,
        };
        b.items[i].meta_mut().from = new;
        if !changed.contains(&Some(i)) {
            changed.push(Some(i));
        }
    }
    for &i in &refs.merges {
        let st = story(b, owner);
        let new = match st.find(sid) {
            Some(k) => match next_scene(&st, k) {
                Some(n) => anchor_at(b, &st, n, &mut changed),
                None => None,
            },
            None => None,
        };
        b.items[i].meta_mut().merges_into = new;
        if !changed.contains(&Some(i)) {
            changed.push(Some(i));
        }
    }
    for &t in &refs.sames {
        loop {
            let at = b
                .doc(t)
                .blocks
                .iter()
                .position(|x| x.element == Element::SceneHeading && x.link.as_deref() == Some(sid));
            let Some(h) = at else { break };
            if !make_own(b, t, h) {
                break;
            }
        }
        if !changed.contains(&t) {
            changed.push(t);
        }
    }
    changed
}

pub fn scene_refs_in(b: &Bundle, t: Tl, from: usize, to: usize) -> Refs {
    let mut out = Refs::default();
    for blk in b.doc(t).blocks.get(from..to).unwrap_or(&[]) {
        if blk.element != Element::SceneHeading || blk.link.is_some() {
            continue;
        }
        let Some(sid) = &blk.sid else { continue };
        let r = refs_to(b, sid);
        out.forks.extend(r.forks);
        out.sames.extend(r.sames);
        out.merges.extend(r.merges);
    }
    out
}

pub fn missing_anchors(b: &Bundle, t: Tl) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..b.items.len() {
        let meta = b.items[i].meta();
        let parent = b.parent(Some(i)).unwrap_or(None);
        for sid in [meta.from.as_ref(), meta.merges_into.as_ref()].into_iter().flatten() {
            let lost = b.owner_of(sid).is_none();
            let touches = parent == t || b.ancestors(Some(i)).contains(&t) || Some(i) == t;
            if lost && touches && !out.contains(sid) {
                out.push(sid.clone());
            }
        }
    }
    out
}

// files here---

#[cfg(not(target_arch = "wasm32"))]
pub fn load_bundle(main_path: &Path) -> std::io::Result<Bundle> {
    let main = crate::storage::load(main_path)?;
    Ok(load_bundle_with(main_path, main))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_bundle_with(main_path: &Path, main: Document) -> Bundle {
    let mut items = Vec::new();
    for p in crate::storage::list_timelines(main_path) {
        if let Ok(doc) = crate::storage::load(&p) {
            items.push((p, doc));
        }
    }
    Bundle::new(main_path.to_path_buf(), main, items)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn bundle_of(path: &Path) -> std::io::Result<(Bundle, Tl)> {
    let main = crate::storage::main_of(path).unwrap_or_else(|| path.to_path_buf());
    let b = load_bundle(&main)?;
    let t = b.find_path(path).unwrap_or(None);
    Ok((b, t))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save(b: &Bundle, t: Tl) -> std::io::Result<()> {
    crate::storage::save(b.path(t), b.doc(t))
}
