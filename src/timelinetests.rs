use std::path::PathBuf;

use crate::backend::Store;
use crate::model::{Document, Element, TimelineMeta};
use crate::storage;
use crate::tests::sandbox;
use crate::timelines::{self, Bundle, EdgeKind, Issue};

const MAIN: &str = "---\ntitle: Harbour\nauthor: \ncontact: \ndraft: \n---\n\n## INT. ONE - DAY\n<!-- scene: | id=sone11 -->\n\nOne.\n\n## INT. TWO - DAY\n<!-- scene: | id=stwo22 -->\n\nTwo.\n\n## INT. THREE - DAY\n\nThree.\n\n## INT. FOUR - DAY\n<!-- scene: | id=sfour4 -->\n\nFour.\n\n## INT. FIVE - DAY\n<!-- scene: | id=sfive5 -->\n\nFive.\n\n";

fn branch(id: &str, parent: &str, from: &str, merge: Option<&str>, body: &str) -> String {
    let mut s = format!("---\ntitle: {id}\nauthor: \ncontact: \ndraft: \ntimeline-id: {id}\ntimeline: Name {id}\nparent: {parent}\nfrom: {from}\nchoice: Pick {id}\n");
    if let Some(m) = merge {
        s.push_str(&format!("merges-into: {m}\n"));
    }
    s.push_str("---\n\n");
    s.push_str(body);
    s
}

fn bundle(items: &[(&str, String)]) -> Bundle {
    let main = storage::from_markdown(MAIN);
    let items = items
        .iter()
        .map(|(name, text)| (PathBuf::from(format!("harbour.timelines/{name}.md")), storage::from_markdown(text)))
        .collect();
    Bundle::new(PathBuf::from("harbour.md"), main, items)
}

fn harbour() -> Bundle {
    bundle(&[
        ("a", branch("ta", "main", "stwo22", Some("sfive5"), "## EXT. A1 - NIGHT\n<!-- scene: | id=aone11 -->\n\nA1.\n\n## @same sfour4\n\n## EXT. A2 - NIGHT\n\nA2.\n\n")),
        ("b", branch("tb", "ta", "aone11", None, "## INT. B1 - DAY\n\nB1.\n\n")),
        ("c", branch("tc", "main", "start", None, "## INT. C1 - DAY\n\nC1.\n\n")),
        ("d", branch("td", "main", "sfive5", None, "## INT. D1 - DAY\n\nD1.\n\n")),
    ])
}

fn headings(d: &Document) -> Vec<String> {
    d.blocks
        .iter()
        .filter(|b| b.element == Element::SceneHeading)
        .map(|b| b.text.clone())
        .collect()
}

fn old_parse_card(line: &str) -> Option<(Option<usize>, String)> {
    let inner = line.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
    let rest = inner.strip_prefix("scene:")?.trim();
    if let Some(after) = rest.strip_prefix("tint=") {
        let (num, note) = match after.split_once('|') {
            Some((n, note)) => (n.trim(), note.trim()),
            None => (after.trim(), ""),
        };
        Some((num.parse::<usize>().ok(), note.to_string()))
    } else {
        Some((None, rest.to_string()))
    }
}

fn old_card_comment(tint: Option<usize>, note: &str) -> String {
    let note = note.replace('\n', " ").replace("-->", "- ->");
    let note = note.trim();
    match tint {
        Some(t) => format!("<!-- scene: tint={t} | {note} -->"),
        None => format!("<!-- scene: {note} -->"),
    }
}

fn card_line(md: &str) -> String {
    md.lines().find(|l| l.starts_with("<!-- scene:")).unwrap_or("").to_string()
}

#[test]
fn a_scene_id_rides_on_the_card_and_older_builds_write_it_back_unchanged() {
    let cases = [
        (Some(2), "Maria speaks", "<!-- scene: tint=2 | Maria speaks | id=k3f9q2 -->"),
        (Some(2), "", "<!-- scene: tint=2 | | id=k3f9q2 -->"),
        (None, "Maria speaks", "<!-- scene: Maria speaks | id=k3f9q2 -->"),
        (None, "", "<!-- scene: | id=k3f9q2 -->"),
        (None, "a | b", "<!-- scene: a | b | id=k3f9q2 -->"),
    ];
    for (tint, note, want) in cases {
        let mut d = Document::default();
        d.blocks[0].text = "INT. ROOM - DAY".into();
        d.blocks[0].tint = tint;
        d.blocks[0].note = note.into();
        d.blocks[0].sid = Some("k3f9q2".into());
        let md = storage::to_markdown(&d);
        assert_eq!(card_line(&md), want);
        let back = storage::from_markdown(&md);
        assert_eq!(back.blocks[0].sid.as_deref(), Some("k3f9q2"), "{want}");
        assert_eq!(back.blocks[0].note, note, "{want}");
        assert_eq!(back.blocks[0].tint, tint, "{want}");
        assert_eq!(storage::to_markdown(&back), md, "{want} must round-trip byte for byte");
        let (old_tint, old_note) = old_parse_card(want).unwrap();
        assert_eq!(old_card_comment(old_tint, &old_note), want, "an older build would rewrite {want}");
        assert_eq!(old_tint, tint);
    }
}

#[test]
fn cards_without_an_id_are_written_exactly_as_before() {
    for (tint, note) in [(Some(3), "Maria speaks first."), (None, "Just a note"), (Some(0), "")] {
        let mut d = Document::default();
        d.blocks[0].text = "INT. ROOM - DAY".into();
        d.blocks[0].tint = tint;
        d.blocks[0].note = note.into();
        assert_eq!(card_line(&storage::to_markdown(&d)), old_card_comment(tint, note));
    }
    let mut d = Document::default();
    d.blocks[0].text = "INT. ROOM - DAY".into();
    assert!(!storage::to_markdown(&d).contains("<!--"), "a plain scene gets no card");
    let odd = storage::from_markdown("## INT. X - DAY\n<!-- scene: talks about the | id=card -->\n");
    assert_eq!(odd.blocks[0].sid.as_deref(), Some("card"));
    let odd = storage::from_markdown("## INT. X - DAY\n<!-- scene: a note| id=Not Ok -->\n");
    assert_eq!(odd.blocks[0].sid, None);
    assert_eq!(odd.blocks[0].note, "a note| id=Not Ok");
}

#[test]
fn giving_a_scene_an_id_changes_only_its_own_card() {
    let before = storage::from_markdown(MAIN);
    let text = storage::to_markdown(&before);
    assert_eq!(text, MAIN, "the sample round-trips");
    let mut d = before.clone();
    let k = d.blocks.iter().position(|b| b.text == "INT. THREE - DAY").unwrap();
    d.blocks[k].sid = Some("sthree".into());
    let after = storage::to_markdown(&d);
    let a: Vec<&str> = text.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    assert_eq!(b.len(), a.len() + 1);
    let added: Vec<&&str> = b.iter().filter(|l| !a.contains(l)).collect();
    assert_eq!(added, vec![&"<!-- scene: | id=sthree -->"]);
}

#[test]
fn a_repeated_scene_id_is_kept_only_on_the_first_scene() {
    let d = storage::from_markdown("## INT. A - DAY\n<!-- scene: | id=same11 -->\n\n## INT. B - DAY\n<!-- scene: Note | id=same11 -->\n");
    assert_eq!(d.blocks[0].sid.as_deref(), Some("same11"));
    assert_eq!(d.blocks[1].sid, None);
    assert_eq!(d.blocks[1].note, "Note");
}

#[test]
fn a_branch_file_round_trips() {
    let text = "---\ntitle: Maria says no\nauthor: \ncontact: \ndraft: \ntimeline-id: t7k2qa\ntimeline: Maria says no\nparent: main\nfrom: stwo22\nchoice: Maria says no\nstay: Maria says yes\nmerges-into: sfive5\ncolor: 2\n---\n\n## EXT. PIER - NIGHT\n<!-- scene: | id=aone11 -->\n\nWaves.\n\n## @same sfour4\n\n## INT. VAN - NIGHT\n\nEngines.\n\n";
    let d = storage::from_markdown(text);
    let t = d.meta.timeline.clone().unwrap();
    assert_eq!(
        t,
        TimelineMeta {
            id: "t7k2qa".into(),
            name: "Maria says no".into(),
            parent: None,
            from: Some("stwo22".into()),
            choice: "Maria says no".into(),
            stay: "Maria says yes".into(),
            merges_into: Some("sfive5".into()),
            color: Some(2),
        }
    );
    assert_eq!(d.blocks[2].element, Element::SceneHeading);
    assert_eq!(d.blocks[2].link.as_deref(), Some("sfour4"));
    assert_eq!(d.blocks[2].text, "");
    assert_eq!(storage::to_markdown(&d), text);
    let nested = storage::from_markdown("---\ntimeline-id: x\nparent: t7k2qa\nfrom: start\n---\n\n## INT. A - DAY\n");
    let t = nested.meta.timeline.unwrap();
    assert_eq!(t.parent.as_deref(), Some("t7k2qa"));
    assert_eq!(t.from, None);
    assert!(storage::from_markdown(MAIN).meta.timeline.is_none(), "a plain script is no timeline");
    assert!(!MAIN.contains("timeline"), "and writes no timeline keys");
}

#[test]
fn a_typed_heading_that_looks_like_a_link_stays_a_heading() {
    let d = storage::from_markdown("## @SAME ABC123\n\n## @same Not-An-Id\n");
    assert_eq!(d.blocks[0].link, None);
    assert_eq!(d.blocks[0].text, "@SAME ABC123");
    assert_eq!(d.blocks[1].link, None);
}

#[test]
fn a_timeline_tells_its_whole_story() {
    let b = harbour();
    assert_eq!(b.items.len(), 4);
    assert_eq!(b.name(Some(0)), "Name ta");
    let a = timelines::assemble(&b, Some(0));
    assert_eq!(
        headings(&a),
        vec!["INT. ONE - DAY", "INT. TWO - DAY", "EXT. A1 - NIGHT", "INT. FOUR - DAY", "EXT. A2 - NIGHT", "INT. FIVE - DAY"]
    );
    assert_eq!(a.meta.title, "Harbour (Name ta)");
    assert!(a.meta.timeline.is_none());
    let md = storage::to_markdown(&a);
    assert!(!md.contains("id=") && !md.contains("@same"), "an assembled story carries no ids: {md}");
    assert!(md.contains("Four."), "the Same scene brings its content");
    assert_eq!(timelines::first_number(&b, Some(0)), 3);

    let nested = timelines::assemble(&b, Some(1));
    assert_eq!(headings(&nested), vec!["INT. ONE - DAY", "INT. TWO - DAY", "EXT. A1 - NIGHT", "INT. B1 - DAY"]);
    assert_eq!(timelines::first_number(&b, Some(1)), 4);
    let start = timelines::assemble(&b, Some(2));
    assert_eq!(headings(&start), vec!["INT. C1 - DAY"]);
    assert_eq!(timelines::first_number(&b, Some(2)), 1);
    let main = timelines::assemble(&b, None);
    assert_eq!(headings(&main).len(), 5);
    assert_eq!(main.meta.title, "Harbour");
    let st = timelines::story(&b, Some(0));
    assert_eq!(st.own, 2..5);
    assert_eq!(st.merged_at, Some(5));
    assert_eq!(st.steps[3].same, Some(None));
}

#[test]
fn a_merge_into_a_grandparent_and_three_levels_deep() {
    let b = bundle(&[
        ("a", branch("ta", "main", "stwo22", None, "## INT. A1 - DAY\n<!-- scene: | id=aone11 -->\n\n")),
        ("b", branch("tb", "ta", "aone11", None, "## INT. B1 - DAY\n<!-- scene: | id=bone11 -->\n\n")),
        ("c", branch("tc", "tb", "bone11", Some("sfour4"), "## INT. C1 - DAY\n\n## @same aone11\n\n")),
    ]);
    let c = timelines::assemble(&b, Some(2));
    assert_eq!(
        headings(&c),
        vec!["INT. ONE - DAY", "INT. TWO - DAY", "INT. A1 - DAY", "INT. B1 - DAY", "INT. C1 - DAY", "INT. A1 - DAY", "INT. FOUR - DAY", "INT. FIVE - DAY"]
    );
    assert_eq!(b.depth(Some(2)), 3);
    assert_eq!(b.ancestors(Some(2)), vec![Some(1), Some(0), None]);
    assert_eq!(b.order(), vec![None, Some(0), Some(1), Some(2)]);
    assert!(timelines::issues(&b).is_empty(), "{:?}", timelines::issues(&b));
}

#[test]
fn the_graph_counts_endings_and_paths() {
    let b = harbour();
    let g = timelines::graph(&b);
    let order: Vec<_> = g.rows.iter().map(|r| r.tl).collect();
    assert_eq!(order, vec![None, Some(2), Some(0), Some(1), Some(3)], "Main, then branches in story order");
    let row = |t| g.row_of(t).unwrap();
    let cols: Vec<usize> = g.rows[row(Some(0))].dots.iter().map(|d| d.col).collect();
    assert_eq!(cols, vec![2, 3, 4]);
    assert_eq!(g.rows[row(Some(0))].dots[1].same, Some(None));
    assert_eq!(g.rows[row(Some(1))].dots[0].col, 3);
    assert_eq!(g.rows[row(Some(1))].depth, 2);
    assert_eq!(g.cols(), 6);
    let has = |kind, from: Option<(usize, usize)>, to: (usize, usize)| g.edges.iter().any(|e| e.kind == kind && e.from == from && e.to == to);
    assert!(has(EdgeKind::Fork, Some((row(None), 1)), (row(Some(0)), 0)));
    assert!(g.edges.iter().any(|e| e.kind == EdgeKind::Fork && e.label == "Pick ta"));
    assert!(has(EdgeKind::Merge, Some((row(Some(0)), 2)), (row(None), 4)));
    assert!(has(EdgeKind::Same, Some((row(None), 3)), (row(Some(0)), 1)));
    assert!(has(EdgeKind::Fork, Some((row(Some(0)), 0)), (row(Some(1)), 0)));
    assert!(has(EdgeKind::Fork, None, (row(Some(2)), 0)));
    assert!(has(EdgeKind::Fork, Some((row(None), 4)), (row(Some(3)), 0)));
    assert_eq!(timelines::endings(&g).len(), 4, "Main, B, C and D end; A merges");
    assert_eq!(timelines::paths(&g), 6, "Main, Main then D, A, A then D, B and C");
    assert!(timelines::issues(&b).is_empty(), "{:?}", timelines::issues(&b));
    assert_eq!(b.total_scenes(), 5 + 3 + 1 + 1 + 1);
    let alone = Bundle::alone(PathBuf::from("x.md"), storage::from_markdown(MAIN));
    let g = timelines::graph(&alone);
    assert_eq!((timelines::endings(&g).len(), timelines::paths(&g)), (1, 1));
}

#[test]
fn every_kind_of_problem_is_found_and_nothing_hangs() {
    let b = bundle(&[
        ("a", branch("ta", "main", "nope11", Some("gone11"), "## INT. A - DAY\n\n## @same zzzz11\n\n")),
        ("b", branch("tb", "tz", "stwo22", None, "## INT. B - DAY\n\n")),
        ("c", branch("tc", "td", "stwo22", None, "## INT. C - DAY\n\n")),
        ("d", branch("td", "tc", "stwo22", None, "## INT. D - DAY\n\n")),
        ("e", branch("te", "main", "sfour4", Some("sone11"), "## INT. E - DAY\n<!-- scene: | id=eone11 -->\n\n")),
        ("f", branch("tf", "main", "stwo22", None, "## @same eone11\n\n")),
        ("g", branch("tg", "main", "stwo22", None, "")),
    ]);
    let found = timelines::issues(&b);
    let want = [
        Issue::MissingFork { tl: 0, sid: "nope11".into() },
        Issue::MissingMerge { tl: 0, sid: "gone11".into() },
        Issue::MissingSame { tl: Some(0), sid: "zzzz11".into() },
        Issue::MissingParent { tl: 1 },
        Issue::ParentLoop { tl: 2 },
        Issue::ParentLoop { tl: 3 },
        Issue::MergeBeforeFork { tl: 4 },
        Issue::OutsideAncestors { tl: Some(5), sid: "eone11".into() },
        Issue::EmptyBranch { tl: 6 },
    ];
    for w in &want {
        assert!(found.contains(w), "missing {w:?} in {found:?}");
        assert!(!w.text(&b).is_empty());
    }
    let a = timelines::assemble(&b, Some(0));
    assert!(headings(&a).contains(&timelines::GAP_SCENE.to_string()), "a lost anchor shows as a gap, not a crash");
    let _ = timelines::assemble(&b, Some(2));
    let g = timelines::graph(&b);
    let _ = timelines::paths(&g);
    assert_eq!(b.order().len(), 8);
}

#[test]
fn deleting_a_scene_re_anchors_what_depended_on_it() {
    let mut b = harbour();
    let refs = timelines::refs_to(&b, "sfour4");
    assert_eq!(refs.sames, vec![Some(0)]);
    assert_eq!(refs.describe(), "1 timeline shows it as a Same scene");
    let changed = timelines::reanchor_for_delete(&mut b, None, "sfour4");
    assert_eq!(changed, vec![Some(0)]);
    let a = &b.items[0].doc;
    assert!(a.blocks.iter().all(|x| x.link.is_none()));
    assert!(a.blocks.iter().any(|x| x.text == "INT. FOUR - DAY" && x.sid.is_none()));
    assert!(a.blocks.iter().any(|x| x.text == "Four."));

    let mut b = harbour();
    let changed = timelines::reanchor_for_delete(&mut b, None, "stwo22");
    assert_eq!(changed, vec![Some(0)]);
    assert_eq!(b.items[0].meta().from.as_deref(), Some("sone11"), "the fork steps back a scene");

    let mut b = harbour();
    let refs = timelines::refs_to(&b, "sfive5");
    assert_eq!(refs.describe(), "1 timeline branches from this scene, 1 timeline merges into it");
    timelines::reanchor_for_delete(&mut b, None, "sfive5");
    assert_eq!(b.items[0].meta().merges_into, None, "nothing after it, so the merge goes");
    assert_eq!(b.items[3].meta().from.as_deref(), Some("sfour4"));

    let mut b = bundle(&[("x", branch("tx", "main", "sfour4", Some("stwo22"), "## INT. X - DAY\n\n"))]);
    let changed = timelines::reanchor_for_delete(&mut b, None, "sfour4");
    let three = b.main.blocks.iter().find(|x| x.text == "INT. THREE - DAY").unwrap();
    let sid = three.sid.clone().expect("the scene before gets an id");
    assert_eq!(b.items[0].meta().from.as_deref(), Some(sid.as_str()));
    assert!(changed.contains(&None) && changed.contains(&Some(0)));
    let changed = timelines::reanchor_for_delete(&mut b, None, "stwo22");
    assert_eq!(b.items[0].meta().merges_into.as_deref(), Some(sid.as_str()), "the merge steps on a scene");
    assert!(changed.contains(&Some(0)));

    let mut b = bundle(&[("x", branch("tx", "main", "sone11", None, "## INT. X - DAY\n\n"))]);
    timelines::reanchor_for_delete(&mut b, None, "sone11");
    assert_eq!(b.items[0].meta().from, None, "nothing before it, so it starts at the start");
    assert!(timelines::refs_to(&b, "sone11").is_empty());
}

#[test]
fn make_it_its_own_copies_the_scene_in() {
    let mut b = harbour();
    let k = b.items[0].doc.blocks.iter().position(|x| x.link.is_some()).unwrap();
    assert!(timelines::make_own(&mut b, Some(0), k));
    let a = timelines::assemble(&b, Some(0));
    assert_eq!(headings(&a)[3], "INT. FOUR - DAY");
    assert!(timelines::refs_to(&b, "sfour4").is_empty());
    let ids: Vec<u64> = b.items[0].doc.blocks.iter().map(|x| x.id).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len(), "fresh block ids");
}

#[test]
fn new_ids_are_unique_in_the_set() {
    let mut b = harbour();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..200 {
        let id = b.new_sid();
        assert!(storage::sid_ok(&id) && id.len() == 6, "{id}");
        assert!(seen.insert(id));
    }
    let k = b.main.blocks.iter().position(|x| x.text == "INT. THREE - DAY").unwrap();
    let id = b.ensure_sid(None, k);
    assert_eq!(b.ensure_sid(None, k), id, "an id is given once");
    let n = b.add_branch(PathBuf::from("harbour.timelines/new.md"), Some(0), Some("aone11".into()), "Cole runs", "Run", "");
    let m = b.items[n].meta().clone();
    assert_eq!(m.parent.as_deref(), Some("ta"));
    assert!(b.items.iter().filter(|t| t.meta().id == m.id).count() == 1);
    assert_eq!(m.color, Some(0));
}

#[test]
fn branch_files_live_beside_their_script_and_follow_it() {
    let _g = sandbox("timelines");
    storage::ensure_dirs().unwrap();
    let dir = storage::scripts_dir();
    let main = dir.join("harbour.md");
    std::fs::write(&main, MAIN).unwrap();
    let a = storage::unique_timeline_path(&main, "Maria says no", &[]);
    assert_eq!(a, dir.join("harbour.timelines/maria-says-no.md"));
    assert!(storage::is_timeline_path(&a));
    assert_eq!(storage::main_of(&a), Some(main.clone()));
    assert_eq!(storage::main_of(&main), None);
    let text = branch("ta", "main", "stwo22", None, "## EXT. PIER - NIGHT\n\nA lighthouse blinks.\n\n");
    storage::save(&a, &storage::from_markdown(&text)).unwrap();
    assert_eq!(std::fs::read_to_string(&a).unwrap(), text);
    assert_eq!(storage::unique_timeline_path(&main, "Maria says no", &[]), dir.join("harbour.timelines/maria-says-no-2.md"));

    let entries = storage::list_scripts();
    assert_eq!(entries.len(), 1, "a timelines folder is not a script");
    assert_eq!(entries[0].timelines, 1);

    let (b, t) = timelines::bundle_of(&a).unwrap();
    assert_eq!(t, Some(0));
    assert_eq!(b.main_path, main);
    let (_, t) = timelines::bundle_of(&main).unwrap();
    assert_eq!(t, None);

    let found = storage::search_scripts("lighthouse", 3);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, a);
    assert_eq!(found[0].title, "Harbour · Name ta");

    storage::take_snapshot(&a, &storage::from_markdown(&text)).unwrap();
    let snaps = storage::snapshots_dir().join("harbour.timelines").join("maria-says-no");
    assert!(snaps.is_dir());
    assert_eq!(storage::list_snapshots(&a).len(), 1);
    assert!(storage::list_snapshots(&main).is_empty(), "a branch keeps its own snapshots");

    let mut store = crate::backend::LocalBackend::new();
    let moved = store.follow_title(&main, "Harbour Lights");
    assert_eq!(moved, dir.join("harbour-lights.md"));
    assert!(!dir.join("harbour.timelines").exists());
    let a2 = dir.join("harbour-lights.timelines/maria-says-no.md");
    assert_eq!(std::fs::read_to_string(&a2).unwrap(), text, "the branch moves untouched");
    assert!(storage::snapshots_dir().join("harbour-lights.timelines/maria-says-no").is_dir());

    store.duplicate(&moved).unwrap();
    let copy = dir.join("harbour-copy.md");
    assert!(copy.exists());
    assert_eq!(std::fs::read_to_string(dir.join("harbour-copy.timelines/maria-says-no.md")).unwrap(), text);
    assert_eq!(std::fs::read_to_string(&a2).unwrap(), text);

    store.delete(&a2).unwrap();
    assert!(moved.exists(), "deleting a branch leaves its script");
    store.delete(&copy).unwrap();
    assert!(!dir.join("harbour-copy.timelines").exists(), "deleting a script takes its timelines");
    let mut names: Vec<String> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    assert_eq!(names, vec!["harbour-lights.md", "harbour-lights.timelines"]);
}

// game exports here---

fn targets(text: &str, open: &str, close: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find(open) {
        let after = &rest[i + open.len()..];
        let end = after.find(close).unwrap_or(after.len());
        out.push(after[..end].trim().to_string());
        rest = &after[end..];
    }
    out
}

#[test]
fn ink_has_a_knot_for_every_divert_and_a_choice_at_every_fork() {
    let b = harbour();
    let ink = crate::export_game::ink(&b);
    let knots: Vec<String> = ink.lines().filter_map(|l| l.strip_prefix("=== ").and_then(|l| l.strip_suffix(" ==="))).map(String::from).collect();
    assert_eq!(knots.len(), 5 + 3 + 1 + 1 + 1, "{ink}");
    for t in targets(&ink, "-> ", "\n") {
        assert!(t == "END" || knots.contains(&t), "-> {t} goes nowhere");
    }
    assert_eq!(ink.lines().filter(|l| l.starts_with("* [")).count(), 8, "two options at each of the four forks");
    assert!(ink.contains("* [Pick ta] -> name_ta_s3"), "{ink}");
    assert!(ink.contains("* [Continue] -> main_s3"));
    assert!(ink.contains("# scene: INT. ONE - DAY"));
    assert!(ink.contains("# same scene"));
    assert!(ink.lines().filter(|l| *l == "-> END").count() >= 3, "B, C and D end");
}

#[test]
fn yarn_jumps_land_on_nodes_and_json_describes_the_graph() {
    let b = harbour();
    let yarn = crate::export_game::yarn(&b);
    let titles: Vec<String> = yarn.lines().filter_map(|l| l.strip_prefix("title: ")).map(String::from).collect();
    for t in targets(&yarn, "<<jump ", ">>") {
        assert!(titles.contains(&t), "<<jump {t}>> goes nowhere");
    }
    assert_eq!(yarn.lines().filter(|l| l.starts_with("-> ")).count(), 8);
    assert_eq!(yarn.matches("===\n").count(), titles.len());

    let v: serde_json::Value = serde_json::from_str(&crate::export_game::json(&b)).expect("valid JSON");
    assert_eq!(v["timelines"].as_array().unwrap().len(), 5);
    let nodes = v["graph"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 11);
    let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    for n in nodes {
        for c in n["choices"].as_array().unwrap() {
            assert!(c["to"].is_null() || ids.contains(&c["to"].as_str().unwrap()));
        }
    }
    for e in v["graph"]["edges"].as_array().unwrap() {
        assert!(ids.contains(&e["to"].as_str().unwrap()), "{e}");
    }
    assert_eq!(v["paths"], 6);
    assert_eq!(v["endings"], 4);
}

#[test]
fn game_text_is_escaped() {
    let main = "---\ntitle: X\n---\n\n## INT. A - DAY\n\n-> not a divert {not logic} # not a tag\n\n";
    let b = Bundle::alone(PathBuf::from("x.md"), storage::from_markdown(main));
    let ink = crate::export_game::ink(&b);
    assert!(ink.contains("\\-> not a divert \\{not logic\\} \\# not a tag"), "{ink}");
    let yarn = crate::export_game::yarn(&b);
    assert!(yarn.contains("-\\> not a divert \\{not logic\\} \\# not a tag"), "{yarn}");
}
