use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct LyricLine {
    pub at: Duration,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Lyrics {
    #[default]
    None,
    Loading,
    NotFound,
    Ready(Arc<Vec<LyricLine>>),
}

/// Parses LRC text such as `[01:23.45] words`; a line may carry several timestamps.
pub fn parse_lrc(src: &str) -> Vec<LyricLine> {
    let mut out = Vec::new();
    for raw in src.lines() {
        let mut rest = raw.trim();
        let mut stamps = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(end) = stripped.find(']') else { break };
            match parse_stamp(&stripped[..end]) {
                Some(t) => stamps.push(t),
                // Metadata tags like [ar:Artist] carry no timing.
                None if stamps.is_empty() => break,
                None => {}
            }
            rest = stripped[end + 1..].trim_start();
        }
        for at in stamps {
            out.push(LyricLine { at, text: rest.to_string() });
        }
    }
    out.sort_by_key(|l| l.at);
    out
}

fn parse_stamp(s: &str) -> Option<Duration> {
    let (m, sec) = s.split_once(':')?;
    let m: u64 = m.trim().parse().ok()?;
    let sec: f64 = sec.trim().parse().ok()?;
    if !(0.0..100.0).contains(&sec) {
        return None;
    }
    Some(Duration::from_secs(m * 60) + Duration::from_secs_f64(sec))
}

/// Index of the line being sung at `pos`; None before the first line.
pub fn current_index(lines: &[LyricLine], pos: Duration) -> Option<usize> {
    lines.partition_point(|l| l.at <= pos).checked_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stamps_tags_and_repeats() {
        let l = parse_lrc("[ar:Someone]\n[00:12.50] first\n[00:05.00][00:30.00]chorus\n\n[01:00.1]");
        let texts: Vec<_> = l.iter().map(|x| x.text.as_str()).collect();
        assert_eq!(texts, ["chorus", "first", "chorus", ""]);
        assert_eq!(l[1].at, Duration::from_millis(12_500));
        assert_eq!(l[2].at, Duration::from_secs(30));
    }

    #[test]
    fn finds_current_line() {
        let l = parse_lrc("[00:05.00]a\n[00:10.00]b");
        assert_eq!(current_index(&l, Duration::from_secs(1)), None);
        assert_eq!(current_index(&l, Duration::from_secs(5)), Some(0));
        assert_eq!(current_index(&l, Duration::from_secs(99)), Some(1));
    }
}
