use chrono::{Datelike, Duration, NaiveDate};

pub const KEEP_DAYS: i64 = 60;
pub const KEEP_PLACES: usize = 24;

pub fn day(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub fn parse_day(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
}

pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

// words log here---
pub fn add_words(log: &mut Vec<(String, i64)>, on: NaiveDate, delta: i64) {
    if delta == 0 {
        return;
    }
    let key = day(on);
    match log.iter_mut().find(|(d, _)| *d == key) {
        Some(slot) => slot.1 += delta,
        None => log.push((key, delta)),
    }
    let oldest = on - Duration::days(KEEP_DAYS);
    log.retain(|(d, _)| parse_day(d).map(|x| x > oldest).unwrap_or(false));
    log.sort_by(|a, b| b.0.cmp(&a.0));
}

pub fn on(log: &[(String, i64)], d: NaiveDate) -> i64 {
    let key = day(d);
    log.iter().filter(|(k, _)| *k == key).map(|(_, n)| (*n).max(0)).sum()
}

pub fn week(log: &[(String, i64)], today: NaiveDate) -> i64 {
    let back = today.weekday().num_days_from_monday() as i64;
    (0..=back).map(|k| on(log, today - Duration::days(k))).sum()
}

pub fn streak(log: &[(String, i64)], today: NaiveDate) -> u32 {
    let mut d = if on(log, today) > 0 { today } else { today - Duration::days(1) };
    let mut n = 0;
    while on(log, d) > 0 {
        n += 1;
        d -= Duration::days(1);
    }
    n
}

pub fn write_words(log: &[(String, i64)]) -> String {
    log.iter().map(|(d, n)| format!("{d}={n}")).collect::<Vec<_>>().join("; ")
}

pub fn read_words(v: &str) -> Vec<(String, i64)> {
    v.split(';')
        .filter_map(|bit| {
            let (d, n) = bit.split_once('=')?;
            let d = parse_day(d)?;
            Some((day(d), n.trim().parse::<i64>().ok()?))
        })
        .collect()
}

// places here---
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub path: String,
    pub page: usize,
    pub scene: usize,
    pub block: usize,
    pub heading: String,
}

pub fn remember(places: &mut Vec<Place>, place: Place) {
    places.retain(|p| p.path != place.path);
    places.insert(0, place);
    places.truncate(KEEP_PLACES);
}

pub fn write_place(p: &Place) -> String {
    format!("{}|{}|{}|{}|{}", p.page, p.scene, p.block, p.heading.replace('|', "/"), p.path)
}

pub fn read_place(v: &str) -> Option<Place> {
    let mut bits = v.trim().splitn(5, '|');
    let page = bits.next()?.trim().parse().ok()?;
    let scene = bits.next()?.trim().parse().ok()?;
    let block = bits.next()?.trim().parse().ok()?;
    let heading = bits.next()?.trim().to_string();
    let path = bits.next()?.trim().to_string();
    if path.is_empty() {
        return None;
    }
    Some(Place { path, page, scene, block, heading })
}

// deadlines here---
pub fn read_deadline(v: &str) -> Option<(NaiveDate, String)> {
    let v = v.trim();
    let (d, label) = v.split_once(' ').unwrap_or((v, ""));
    Some((parse_day(d)?, label.trim().to_string()))
}

pub fn write_deadline(d: NaiveDate, label: &str) -> String {
    let label = label.split_whitespace().collect::<Vec<_>>().join(" ");
    if label.is_empty() {
        day(d)
    } else {
        format!("{} {label}", day(d))
    }
}

pub fn days_left(d: NaiveDate, today: NaiveDate) -> i64 {
    (d - today).num_days()
}

pub fn when(days: i64) -> String {
    match days {
        0 => "Today".to_string(),
        1 => "Tomorrow".to_string(),
        -1 => "A day late".to_string(),
        n if n < 0 => format!("{} days late", -n),
        n => format!("{n} days"),
    }
}
