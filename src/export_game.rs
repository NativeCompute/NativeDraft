use crate::model::{Block, Element};
use crate::timelines::{self, Bundle, EdgeKind, Graph, Step, Tl};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Game {
    Ink,
    Yarn,
    Json,
}

impl Game {
    pub fn ext(self) -> &'static str {
        match self {
            Game::Ink => "ink",
            Game::Yarn => "yarn",
            Game::Json => "json",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Game::Ink => "Ink",
            Game::Yarn => "Yarn Spinner",
            Game::Json => "JSON graph",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Next {
    Node(String),
    End,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub tl: Tl,
    pub number: usize,
    pub same: bool,
    pub blocks: Vec<Block>,
    pub choices: Vec<(String, Next)>,
    pub then: Next,
}

pub struct Plan {
    pub title: String,
    pub prelude: Vec<Block>,
    pub start: Vec<(String, Next)>,
    pub first: Next,
    pub nodes: Vec<Node>,
}

fn ident(s: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            if gap && !out.is_empty() {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            gap = false;
        } else {
            gap = true;
        }
    }
    if out.is_empty() || out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert_str(0, "t_");
    }
    out
}

fn row_steps(b: &Bundle, t: Tl) -> Vec<Step> {
    let st = timelines::story(b, t);
    st.steps[st.own.clone()].iter().filter(|s| s.scene).cloned().collect()
}

pub fn plan(b: &Bundle) -> Plan {
    let g: Graph = timelines::graph(b);
    let mut names: Vec<String> = Vec::new();
    let mut base: Vec<String> = Vec::new();
    for row in &g.rows {
        let mut n = ident(&b.name(row.tl));
        if base.contains(&n) {
            n = format!("{n}_{}", ident(&b.id_of(row.tl)));
        }
        base.push(n.clone());
        names.push(n);
    }
    let key = |r: usize, d: usize| format!("{}_s{}", names[r], g.rows[r].dots[d].number);
    let after = |r: usize, d: usize| -> Next {
        if d + 1 < g.rows[r].dots.len() {
            return Next::Node(key(r, d + 1));
        }
        match g.edges.iter().find(|e| e.kind == EdgeKind::Merge && e.from == Some((r, d))) {
            Some(e) => Next::Node(key(e.to.0, e.to.1)),
            None => Next::End,
        }
    };
    let label_of = |row: usize| -> String {
        let t = g.rows[row].tl;
        let m = b.meta(t);
        match m.map(|m| m.choice.trim().to_string()).filter(|c| !c.is_empty()) {
            Some(c) => c,
            None => b.name(t),
        }
    };
    let stay_of = |kids: &[usize]| -> String {
        kids.iter()
            .filter_map(|&r| b.meta(g.rows[r].tl).map(|m| m.stay.trim().to_string()))
            .find(|s| !s.is_empty())
            .unwrap_or_else(|| "Continue".to_string())
    };

    let mut nodes = Vec::new();
    for (r, row) in g.rows.iter().enumerate() {
        let steps = row_steps(b, row.tl);
        for (d, dot) in row.dots.iter().enumerate() {
            let blocks = steps.get(d).map(|s| s.blocks.clone()).unwrap_or_default();
            let kids: Vec<usize> = g
                .edges
                .iter()
                .filter(|e| e.kind == EdgeKind::Fork && e.from == Some((r, d)))
                .map(|e| e.to.0)
                .collect();
            let then = after(r, d);
            let mut choices = Vec::new();
            if !kids.is_empty() {
                choices.push((stay_of(&kids), then.clone()));
                for k in &kids {
                    choices.push((label_of(*k), Next::Node(key(*k, 0))));
                }
            }
            nodes.push(Node { name: key(r, d), tl: row.tl, number: dot.number, same: dot.same.is_some(), blocks, choices, then });
        }
    }

    let main_row = g.row_of(None);
    let first = match main_row {
        Some(r) if !g.rows[r].dots.is_empty() => Next::Node(key(r, 0)),
        _ => Next::End,
    };
    let start_kids: Vec<usize> = g.edges.iter().filter(|e| e.kind == EdgeKind::Fork && e.from.is_none()).map(|e| e.to.0).collect();
    let mut start = Vec::new();
    if !start_kids.is_empty() {
        start.push((stay_of(&start_kids), first.clone()));
        for k in &start_kids {
            start.push((label_of(*k), Next::Node(key(*k, 0))));
        }
    }
    let main_steps = timelines::story(b, None).steps;
    let prelude = main_steps.iter().take_while(|s| !s.scene).flat_map(|s| s.blocks.clone()).collect();
    Plan { title: b.main.meta.title.clone(), prelude, start, first, nodes }
}

// lines here---

enum Line {
    Heading(String),
    Act(String),
    Say(String, String),
    Text(String),
}

fn lines(blocks: &[Block]) -> Vec<Line> {
    let mut out = Vec::new();
    let mut who: Option<String> = None;
    let mut aside = String::new();
    for blk in blocks {
        let t = blk.text.trim();
        if t.is_empty() {
            continue;
        }
        match blk.element {
            Element::SceneHeading => out.push(Line::Heading(t.to_string())),
            Element::Act => out.push(Line::Act(t.to_string())),
            Element::Character => {
                who = Some(crate::model::base_character(t));
                aside.clear();
            }
            Element::Parenthetical => aside = t.to_string(),
            Element::Dialogue => {
                let name = who.clone().unwrap_or_default();
                let line = if aside.is_empty() { t.to_string() } else { format!("{aside} {t}") };
                aside.clear();
                if name.is_empty() {
                    out.push(Line::Text(line));
                } else {
                    out.push(Line::Say(name, line));
                }
            }
            _ => {
                who = None;
                out.push(Line::Text(t.to_string()));
            }
        }
    }
    out
}

fn ink_escape(s: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let next = chars.get(i + 1).copied();
        let lead = out.trim().is_empty();
        let special = matches!(c, '{' | '}' | '|' | '#' | '\\' | '[' | ']')
            || (c == '/' && matches!(next, Some('/') | Some('*')))
            || (c == '-' && next == Some('>'))
            || (c == '<' && matches!(next, Some('-') | Some('>')))
            || (lead && matches!(c, '*' | '+' | '-' | '=' | '~'));
        if special {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn yarn_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if matches!(c, '{' | '}' | '<' | '>' | '[' | ']' | '#' | '\\' | '/') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn ink_next(n: &Next) -> String {
    match n {
        Next::Node(k) => format!("-> {k}"),
        Next::End => "-> END".to_string(),
    }
}

pub fn ink(b: &Bundle) -> String {
    let p = plan(b);
    let mut s = String::new();
    s.push_str(&format!("# title: {}\n\n", ink_escape(p.title.trim())));
    for l in lines(&p.prelude) {
        push_ink_line(&mut s, &l);
    }
    if p.start.is_empty() {
        s.push_str(&format!("{}\n\n", ink_next(&p.first)));
    } else {
        for (label, to) in &p.start {
            s.push_str(&format!("* [{}] {}\n", ink_escape(label), ink_next(to)));
        }
        s.push('\n');
    }
    for n in &p.nodes {
        s.push_str(&format!("=== {} ===\n", n.name));
        if n.same {
            s.push_str("# same scene\n");
        }
        for l in lines(&n.blocks) {
            push_ink_line(&mut s, &l);
        }
        if n.choices.is_empty() {
            s.push_str(&format!("{}\n", ink_next(&n.then)));
        } else {
            for (label, to) in &n.choices {
                s.push_str(&format!("* [{}] {}\n", ink_escape(label), ink_next(to)));
            }
        }
        s.push('\n');
    }
    s
}

fn push_ink_line(s: &mut String, l: &Line) {
    match l {
        Line::Heading(h) => s.push_str(&format!("# scene: {}\n", ink_escape(h))),
        Line::Act(a) => s.push_str(&format!("# act: {}\n", ink_escape(a))),
        Line::Say(who, t) => s.push_str(&format!("{}: {}\n", ink_escape(who), ink_escape(t))),
        Line::Text(t) => s.push_str(&format!("{}\n", ink_escape(t))),
    }
}

fn yarn_jump(n: &Next) -> String {
    match n {
        Next::Node(k) => format!("<<jump {k}>>"),
        Next::End => "<<stop>>".to_string(),
    }
}

pub fn yarn(b: &Bundle) -> String {
    let p = plan(b);
    let mut s = String::new();
    s.push_str("title: Start\n---\n");
    for l in lines(&p.prelude) {
        push_yarn_line(&mut s, &l);
    }
    if p.start.is_empty() {
        s.push_str(&format!("{}\n", yarn_jump(&p.first)));
    } else {
        for (label, to) in &p.start {
            s.push_str(&format!("-> {}\n    {}\n", yarn_escape(label), yarn_jump(to)));
        }
    }
    s.push_str("===\n");
    for n in &p.nodes {
        s.push_str(&format!("title: {}\n", n.name));
        if n.same {
            s.push_str("tags: same\n");
        }
        s.push_str("---\n");
        for l in lines(&n.blocks) {
            push_yarn_line(&mut s, &l);
        }
        if n.choices.is_empty() {
            s.push_str(&format!("{}\n", yarn_jump(&n.then)));
        } else {
            for (label, to) in &n.choices {
                s.push_str(&format!("-> {}\n    {}\n", yarn_escape(label), yarn_jump(to)));
            }
        }
        s.push_str("===\n");
    }
    s
}

fn push_yarn_line(s: &mut String, l: &Line) {
    match l {
        Line::Heading(h) => s.push_str(&format!("<<scene \"{}\">>\n", h.replace('"', "'"))),
        Line::Act(a) => s.push_str(&format!("<<act \"{}\">>\n", a.replace('"', "'"))),
        Line::Say(who, t) => s.push_str(&format!("{}: {}\n", yarn_escape(&who.replace(':', " ")), yarn_escape(t))),
        Line::Text(t) => s.push_str(&format!("{}\n", yarn_escape(t))),
    }
}

fn block_json(b: &Block) -> serde_json::Value {
    serde_json::json!({ "element": b.element.label(), "text": b.text })
}

pub fn json(b: &Bundle) -> String {
    let g = timelines::graph(b);
    let p = plan(b);
    let mut timelines_out = Vec::new();
    for t in b.order() {
        let m = b.meta(t);
        let scenes: Vec<serde_json::Value> = row_steps(b, t)
            .iter()
            .map(|s| {
                serde_json::json!({
                    "sid": s.sid,
                    "same": s.same.is_some(),
                    "heading": s.heading(),
                    "blocks": s.blocks.iter().map(block_json).collect::<Vec<_>>(),
                })
            })
            .collect();
        timelines_out.push(serde_json::json!({
            "id": b.id_of(t),
            "name": b.name(t),
            "parent": m.map(|m| m.parent.clone().unwrap_or_else(|| "main".into())),
            "from": m.map(|m| m.from.clone().unwrap_or_else(|| "start".into())),
            "choice": m.map(|m| m.choice.clone()),
            "stay": m.map(|m| m.stay.clone()),
            "merge": m.and_then(|m| m.merges_into.clone()),
            "color": b.color(t),
            "scenes": scenes,
        }));
    }
    let nodes: Vec<serde_json::Value> = p
        .nodes
        .iter()
        .map(|n| {
            let next = |x: &Next| match x {
                Next::Node(k) => serde_json::Value::String(k.clone()),
                Next::End => serde_json::Value::Null,
            };
            serde_json::json!({
                "id": n.name,
                "timeline": b.id_of(n.tl),
                "scene": n.number,
                "same": n.same,
                "next": next(&n.then),
                "choices": n.choices.iter().map(|(l, to)| serde_json::json!({ "text": l, "to": next(to) })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let kind = |k: EdgeKind| match k {
        EdgeKind::Fork => "fork",
        EdgeKind::Merge => "merge",
        EdgeKind::Same => "same",
    };
    let edges: Vec<serde_json::Value> = g
        .edges
        .iter()
        .map(|e| {
            serde_json::json!({
                "kind": kind(e.kind),
                "from": e.from.map(|(r, d)| format!("{}_s{}", ident_of(&p, &g, r), g.rows[r].dots[d].number)),
                "to": format!("{}_s{}", ident_of(&p, &g, e.to.0), g.rows[e.to.0].dots[e.to.1].number),
                "label": e.label,
            })
        })
        .collect();
    let out = serde_json::json!({
        "title": b.main.meta.title,
        "author": b.main.meta.author,
        "timelines": timelines_out,
        "graph": { "nodes": nodes, "edges": edges },
        "endings": timelines::endings(&g).len(),
        "paths": timelines::paths(&g),
    });
    serde_json::to_string_pretty(&out).unwrap_or_default()
}

fn ident_of(p: &Plan, g: &Graph, r: usize) -> String {
    let t = g.rows[r].tl;
    p.nodes
        .iter()
        .find(|n| n.tl == t)
        .and_then(|n| n.name.rsplit_once("_s").map(|(a, _)| a.to_string()))
        .unwrap_or_default()
}

pub fn render(b: &Bundle, game: Game) -> (String, Vec<u8>) {
    let text = match game {
        Game::Ink => ink(b),
        Game::Yarn => yarn(b),
        Game::Json => json(b),
    };
    (format!("{}.{}", crate::storage::slugify(&b.main.meta.title), game.ext()), text.into_bytes())
}
