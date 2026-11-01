//! Calendar feed: fetches an ICS link or file, expands simple daily/weekly repeats and reports upcoming events.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use visus_core::{CalEvent, Event, Notch};
use visus_platform::Emit;

const REFRESH: Duration = Duration::from_secs(15 * 60);
const WINDOW_DAYS: i64 = 14;

static STATE: Mutex<(Option<String>, Option<Instant>)> = Mutex::new((None, None));

/// Starts a background fetch when the feed changed or is stale; call every frame cheaply.
pub fn tick(notch: &Notch, emit: Option<&Emit>) {
    let (Some(emit), Some(url)) = (emit, notch.settings().calendar_url.clone()) else { return };
    let mut st = STATE.lock().unwrap();
    let changed = st.0.as_deref() != Some(url.as_str());
    if !changed && st.1.is_some_and(|t| t.elapsed() < REFRESH) {
        return;
    }
    *st = (Some(url.clone()), Some(Instant::now()));
    let emit = emit.clone();
    std::thread::spawn(move || {
        let text = if url.starts_with("http") || url.starts_with("webcal") {
            let url = url.replacen("webcal://", "https://", 1);
            crate::updater::http_get(&url).ok().and_then(|b| String::from_utf8(b).ok())
        } else {
            std::fs::read_to_string(&url).ok()
        };
        if let Some(text) = text {
            emit(Event::Calendar(Arc::new(parse(&text, Utc::now().timestamp()))));
        }
    });
}

fn unfold(src: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in src.lines() {
        if let (Some(c), Some(last)) = (line.chars().next(), out.last_mut()) {
            if c == ' ' || c == '\t' {
                last.push_str(&line[1..]);
                continue;
            }
        }
        out.push(line.to_string());
    }
    out
}

fn unescape(s: &str) -> String {
    s.replace("\\n", " ").replace("\\N", " ").replace("\\,", ",").replace("\\;", ";").replace("\\\\", "\\")
}

/// (local datetime, all-day)
fn parse_dt(params: &str, value: &str) -> Option<(DateTime<Local>, bool)> {
    let value = value.trim();
    if value.len() == 8 || params.contains("VALUE=DATE") && !value.contains('T') {
        let d = NaiveDate::parse_from_str(value, "%Y%m%d").ok()?;
        return Some((Local.from_local_datetime(&d.and_hms_opt(0, 0, 0)?).single()?, true));
    }
    if let Some(utc) = value.strip_suffix('Z') {
        let n = NaiveDateTime::parse_from_str(utc, "%Y%m%dT%H%M%S").ok()?;
        return Some((Utc.from_utc_datetime(&n).with_timezone(&Local), false));
    }
    // Floating or TZID times are read as local time.
    let n = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").ok()?;
    Some((Local.from_local_datetime(&n).single()?, false))
}

fn find_url(texts: &[&str]) -> Option<String> {
    let mut first = None;
    for t in texts {
        for word in t.split(|c: char| c.is_whitespace() || c == '<' || c == '>' || c == '"') {
            if word.starts_with("https://") {
                let w = word.trim_end_matches([',', ')', '.']).to_string();
                if ["zoom.us", "meet.google", "teams.microsoft", "webex", "whereby"].iter().any(|k| w.contains(k)) {
                    return Some(w);
                }
                first.get_or_insert(w);
            }
        }
    }
    first
}

pub fn parse(src: &str, now: i64) -> Vec<CalEvent> {
    let (lo, hi) = (now - 24 * 3600, now + WINDOW_DAYS * 24 * 3600);
    let mut out = Vec::new();
    let mut cur: Option<Vec<(String, String, String)>> = None;
    for line in unfold(src) {
        if line == "BEGIN:VEVENT" {
            cur = Some(Vec::new());
        } else if line == "END:VEVENT" {
            if let Some(props) = cur.take() {
                expand(&props, lo, hi, &mut out);
            }
        } else if let (Some(props), Some((head, value))) = (cur.as_mut(), line.split_once(':')) {
            let (name, params) = head.split_once(';').unwrap_or((head, ""));
            props.push((name.to_uppercase(), params.to_uppercase(), value.to_string()));
        }
    }
    out.sort_by_key(|e| e.start);
    out
}

fn expand(props: &[(String, String, String)], lo: i64, hi: i64, out: &mut Vec<CalEvent>) {
    let get = |n: &str| props.iter().find(|p| p.0 == n);
    let Some((start, all_day)) = get("DTSTART").and_then(|p| parse_dt(&p.1, &p.2)) else { return };
    let end = get("DTEND").and_then(|p| parse_dt(&p.1, &p.2)).map(|e| e.0).unwrap_or(start + chrono::Duration::hours(if all_day { 24 } else { 1 }));
    let len = (end - start).num_seconds().max(0);
    let title = get("SUMMARY").map_or("Event".to_string(), |p| unescape(&p.2));
    let loc = get("LOCATION").map_or(String::new(), |p| unescape(&p.2));
    let desc = get("DESCRIPTION").map_or(String::new(), |p| p.2.replace("\\n", " ").replace("\\N", " "));
    let url = find_url(&[get("URL").map_or("", |p| p.2.as_str()), &loc, &desc]);
    let mk = |s: i64| CalEvent { start: s, end: s + len, title: title.clone(), all_day, url: url.clone() };

    let Some(rule) = get("RRULE").map(|p| p.2.to_uppercase()) else {
        if start.timestamp() + len >= lo && start.timestamp() <= hi {
            out.push(mk(start.timestamp()));
        }
        return;
    };
    let kv = |k: &str| rule.split(';').find_map(|p| p.strip_prefix(&format!("{k}=")));
    let interval = kv("INTERVAL").and_then(|v| v.parse::<i64>().ok()).unwrap_or(1).max(1);
    let until = kv("UNTIL").and_then(|v| parse_dt("", v)).map(|d| d.0.timestamp());
    let days: Vec<u32> = kv("BYDAY").map(|v| v.split(',').filter_map(|d| ["MO", "TU", "WE", "TH", "FR", "SA", "SU"].iter().position(|x| d.ends_with(x)).map(|i| i as u32)).collect()).unwrap_or_default();
    let (freq, base) = (kv("FREQ").unwrap_or(""), start.naive_local());
    if freq != "DAILY" && freq != "WEEKLY" {
        if start.timestamp() >= lo && start.timestamp() <= hi {
            out.push(mk(start.timestamp()));
        }
        return;
    }
    let first_day = DateTime::from_timestamp(lo, 0).map_or(base.date(), |d| d.with_timezone(&Local).date_naive());
    for k in 0..=(WINDOW_DAYS + 2) {
        let day = first_day + chrono::Duration::days(k);
        let n = (day - base.date()).num_days();
        if n < 0 {
            continue;
        }
        let hit = if freq == "DAILY" {
            n % interval == 0
        } else {
            let week = n / 7 - if (day.weekday().num_days_from_monday() as i64) < (base.date().weekday().num_days_from_monday() as i64) && days.is_empty() { 0 } else { 0 };
            let ok_day = if days.is_empty() { day.weekday() == base.date().weekday() } else { days.contains(&day.weekday().num_days_from_monday()) };
            ok_day && (n / 7) % interval == 0 && week >= 0
        };
        if !hit {
            continue;
        }
        let Some(s) = Local.from_local_datetime(&day.and_time(base.time())).single().map(|d| d.timestamp()) else { continue };
        if until.is_some_and(|u| s > u) || s + len < lo || s > hi {
            continue;
        }
        out.push(mk(s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_and_weekly_events() {
        let ics = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nDTSTART:20300101T100000Z\nDTEND:20300101T110000Z\nSUMMARY:Planning\\, big\nDESCRIPTION:Join https://meet.google.com/abc-defg-hij now\nEND:VEVENT\nEND:VCALENDAR\n";
        let t = Utc.with_ymd_and_hms(2029, 12, 31, 0, 0, 0).unwrap().timestamp();
        let ev = parse(ics, t);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].title, "Planning, big");
        assert_eq!(ev[0].url.as_deref(), Some("https://meet.google.com/abc-defg-hij"));

        let weekly = "BEGIN:VEVENT\nDTSTART:20300101T100000Z\nDTEND:20300101T110000Z\nRRULE:FREQ=DAILY\nSUMMARY:Standup\nEND:VEVENT\n";
        assert!(parse(weekly, t).len() >= 10);
    }
}
