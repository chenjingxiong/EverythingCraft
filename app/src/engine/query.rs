use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use regex::Regex;

use super::db::{CompactEntry, PathPool};
use super::types::{FilterCategory, SearchOptions, SearchResultEntry};

#[inline]
pub fn compare_ignore_case(a: &str, b: &str) -> std::cmp::Ordering {
    let mut it_a = a.chars();
    let mut it_b = b.chars();
    loop {
        match (it_a.next(), it_b.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca == cb {
                    continue;
                }
                let la = ca.to_lowercase();
                let lb = cb.to_lowercase();
                let cmp = la.cmp(lb);
                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
            }
        }
    }
}

#[inline]
pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.is_ascii() && needle.is_ascii() {
        let h_bytes = haystack.as_bytes();
        let n_bytes = needle.as_bytes();
        if n_bytes.len() > h_bytes.len() {
            return false;
        }
        return h_bytes.windows(n_bytes.len()).any(|w| w.eq_ignore_ascii_case(n_bytes));
    }
    if haystack.contains(needle) {
        return true;
    }
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

#[derive(Debug, Clone)]
pub enum Condition {
    Substring {
        pattern: String,
        is_wildcard: bool,
    },
    Regex(Regex),
    Extension(Vec<String>),
    Size(SizeFilter),
    DateModified(DateFilter),
    DateCreated(DateFilter),
    PathContains(String),
    Category(FilterCategory),
    IsFolder(bool),
    IsHidden,
    IsSystem,
    Not(Box<Condition>),
    Or(Vec<Condition>),
    And(Vec<Condition>),
}

#[derive(Debug, Clone)]
pub enum SizeFilter {
    GreaterThan(u64),
    LessThan(u64),
    Between(u64, u64),
    Exact(u64),
}

#[derive(Debug, Clone)]
pub enum DateFilter {
    After(i64),
    Before(i64),
    Between(i64, i64),
}

pub struct QueryEvaluator {
    conditions: Vec<Condition>,
    options: SearchOptions,
}

impl QueryEvaluator {
    pub fn parse(raw_query: &str, options: &SearchOptions) -> Self {
        let trimmed = raw_query.trim();
        let mut conditions = Vec::new();

        if !trimmed.is_empty() {
            // Check for OR splits at top-level
            if trimmed.contains('|') {
                let parts: Vec<&str> = trimmed.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                if parts.len() > 1 {
                    let mut or_branches = Vec::new();
                    for part in parts {
                        let parsed_branch = Self::parse_terms(part, options);
                        if !parsed_branch.is_empty() {
                            or_branches.push(Condition::And(parsed_branch));
                        }
                    }
                    if !or_branches.is_empty() {
                        conditions.push(Condition::Or(or_branches));
                    }
                } else {
                    conditions.extend(Self::parse_terms(trimmed, options));
                }
            } else {
                conditions.extend(Self::parse_terms(trimmed, options));
            }
        }

        // Add filter category if set to something other than Everything
        if options.filter_category != FilterCategory::Everything {
            conditions.push(Condition::Category(options.filter_category));
        }

        Self {
            conditions,
            options: options.clone(),
        }
    }

    fn parse_terms(input: &str, options: &SearchOptions) -> Vec<Condition> {
        let mut conditions = Vec::new();
        let tokens = tokenize(input);

        for token in tokens {
            if let Some(cond) = parse_token(&token, options) {
                conditions.push(cond);
            }
        }

        conditions
    }

    pub fn matches(&self, entry: &SearchResultEntry) -> bool {
        for cond in &self.conditions {
            if !eval_condition(cond, entry, &self.options) {
                return false;
            }
        }
        true
    }

    pub fn matches_compact(&self, entry: &CompactEntry, pool: &PathPool) -> bool {
        for cond in &self.conditions {
            if !eval_condition_compact(cond, entry, pool, &self.options) {
                return false;
            }
        }
        true
    }
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in input.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            current.push(ch);
        } else if ch.is_whitespace() && !in_quotes {
            if !current.is_empty() {
                tokens.push(current);
                current = String::new();
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn parse_token(token: &str, options: &SearchOptions) -> Option<Condition> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }

    // Negation
    if token.starts_with('!') && token.len() > 1 {
        let sub = parse_token(&token[1..], options)?;
        return Some(Condition::Not(Box::new(sub)));
    }

    // Stripping outer quotes
    if token.starts_with('"') && token.ends_with('"') && token.len() >= 2 {
        let inner = &token[1..token.len() - 1];
        return Some(Condition::Substring {
            pattern: if options.match_case { inner.to_string() } else { inner.to_lowercase() },
            is_wildcard: false,
        });
    }

    let lower = token.to_lowercase();

    // Built-in keyword filters
    if lower == "folder:" || lower == "folders:" {
        return Some(Condition::IsFolder(true));
    }
    if lower == "file:" || lower == "files:" {
        return Some(Condition::IsFolder(false));
    }
    if lower == "audio:" {
        return Some(Condition::Category(FilterCategory::Audio));
    }
    if lower == "video:" {
        return Some(Condition::Category(FilterCategory::Video));
    }
    if lower == "pic:" || lower == "picture:" || lower == "image:" {
        return Some(Condition::Category(FilterCategory::Picture));
    }
    if lower == "doc:" || lower == "document:" {
        return Some(Condition::Category(FilterCategory::Document));
    }
    if lower == "exe:" || lower == "executable:" {
        return Some(Condition::Category(FilterCategory::Executable));
    }
    if lower == "zip:" || lower == "compressed:" {
        return Some(Condition::Category(FilterCategory::Compressed));
    }
    if lower == "empty:" {
        return Some(Condition::Size(SizeFilter::Exact(0)));
    }
    if lower == "hidden:" {
        return Some(Condition::IsHidden);
    }
    if lower == "system:" {
        return Some(Condition::IsSystem);
    }

    // Size macros
    if lower == "tiny:" || lower == "size:tiny" {
        return Some(Condition::Size(SizeFilter::Between(0, 16 * 1024)));
    }
    if lower == "small:" || lower == "size:small" {
        return Some(Condition::Size(SizeFilter::Between(16 * 1024, 1024 * 1024)));
    }
    if lower == "medium:" || lower == "size:medium" {
        return Some(Condition::Size(SizeFilter::Between(1024 * 1024, 128 * 1024 * 1024)));
    }
    if lower == "large:" || lower == "size:large" {
        return Some(Condition::Size(SizeFilter::Between(128 * 1024 * 1024, 1024 * 1024 * 1024)));
    }
    if lower == "huge:" || lower == "size:huge" {
        return Some(Condition::Size(SizeFilter::Between(1024 * 1024 * 1024, 16 * 1024 * 1024 * 1024)));
    }
    if lower == "gigantic:" || lower == "size:gigantic" {
        return Some(Condition::Size(SizeFilter::GreaterThan(16 * 1024 * 1024 * 1024)));
    }

    // Prefix-based filters: ext:
    if let Some(rest) = lower.strip_prefix("ext:") {
        let exts: Vec<String> = rest
            .split(';')
            .map(|s| s.trim().trim_start_matches('.').to_string())
            .filter(|s| !s.is_empty())
            .collect();
        return Some(Condition::Extension(exts));
    }

    // Prefix-based filters: size:
    if let Some(rest) = lower.strip_prefix("size:") {
        if let Some(sf) = parse_size_filter(rest) {
            return Some(Condition::Size(sf));
        }
    }

    // Prefix-based filters: dm: (Date Modified)
    if let Some(rest) = lower.strip_prefix("dm:") {
        if let Some(df) = parse_date_filter(rest) {
            return Some(Condition::DateModified(df));
        }
    }

    // Prefix-based filters: dc: (Date Created)
    if let Some(rest) = lower.strip_prefix("dc:") {
        if let Some(df) = parse_date_filter(rest) {
            return Some(Condition::DateCreated(df));
        }
    }

    // Prefix-based filters: path:
    if let Some(rest) = token.strip_prefix("path:") {
        return Some(Condition::PathContains(rest.to_string()));
    }

    // Prefix-based filters: regex:
    if let Some(rest) = token.strip_prefix("regex:") {
        if let Ok(re) = Regex::new(rest) {
            return Some(Condition::Regex(re));
        }
    }

    // If match_regex option is enabled
    if options.match_regex {
        if let Ok(re) = Regex::new(token) {
            return Some(Condition::Regex(re));
        }
    }

    // Wildcards (*, ?)
    if token.contains('*') || token.contains('?') {
        return Some(Condition::Substring {
            pattern: if options.match_case { token.to_string() } else { token.to_lowercase() },
            is_wildcard: true,
        });
    }

    // General Substring
    Some(Condition::Substring {
        pattern: if options.match_case { token.to_string() } else { token.to_lowercase() },
        is_wildcard: false,
    })
}

fn parse_size_filter(input: &str) -> Option<SizeFilter> {
    let input = input.trim();
    if let Some((low_s, high_s)) = input.split_once("..") {
        let low = parse_bytes(low_s)?;
        let high = parse_bytes(high_s)?;
        return Some(SizeFilter::Between(low, high));
    }
    if let Some(rest) = input.strip_prefix('>') {
        let val = parse_bytes(rest.trim_start_matches('='))?;
        return Some(SizeFilter::GreaterThan(val));
    }
    if let Some(rest) = input.strip_prefix('<') {
        let val = parse_bytes(rest.trim_start_matches('='))?;
        return Some(SizeFilter::LessThan(val));
    }
    if let Some(rest) = input.strip_prefix('=') {
        let val = parse_bytes(rest)?;
        return Some(SizeFilter::Exact(val));
    }
    parse_bytes(input).map(SizeFilter::Exact)
}

fn parse_bytes(s: &str) -> Option<u64> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }
    let num_end = s.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(s.len());
    let (num_part, unit_part) = s.split_at(num_end);
    let num: f64 = num_part.parse().ok()?;
    let unit = unit_part.trim();

    let multiplier = match unit {
        "" | "b" | "byte" | "bytes" => 1.0,
        "k" | "kb" => 1024.0,
        "m" | "mb" => 1024.0 * 1024.0,
        "g" | "gb" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };

    Some((num * multiplier) as u64)
}

fn parse_date_filter(input: &str) -> Option<DateFilter> {
    let input = input.trim().to_lowercase();
    let now = Local::now();
    let today = now.date_naive();

    match input.as_str() {
        "today" => {
            let start = Local.from_local_datetime(&today.and_hms_opt(0, 0, 0)?).single()?.timestamp();
            let end = Local.from_local_datetime(&today.and_hms_opt(23, 59, 59)?).single()?.timestamp();
            Some(DateFilter::Between(start, end))
        }
        "yesterday" => {
            let yday = today - Duration::days(1);
            let start = Local.from_local_datetime(&yday.and_hms_opt(0, 0, 0)?).single()?.timestamp();
            let end = Local.from_local_datetime(&yday.and_hms_opt(23, 59, 59)?).single()?.timestamp();
            Some(DateFilter::Between(start, end))
        }
        "thisweek" => {
            let weekday = today.weekday().num_days_from_monday();
            let start_of_week = today - Duration::days(weekday as i64);
            let start = Local.from_local_datetime(&start_of_week.and_hms_opt(0, 0, 0)?).single()?.timestamp();
            Some(DateFilter::After(start))
        }
        "lastweek" => {
            let weekday = today.weekday().num_days_from_monday();
            let start_of_this_week = today - Duration::days(weekday as i64);
            let start_of_last_week = start_of_this_week - Duration::days(7);
            let end_of_last_week = start_of_this_week - Duration::seconds(1);
            let start = Local.from_local_datetime(&start_of_last_week.and_hms_opt(0, 0, 0)?).single()?.timestamp();
            let end = Local.from_local_datetime(&end_of_last_week.and_hms_opt(23, 59, 59)?).single()?.timestamp();
            Some(DateFilter::Between(start, end))
        }
        "thismonth" => {
            let start_of_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)?;
            let start = Local.from_local_datetime(&start_of_month.and_hms_opt(0, 0, 0)?).single()?.timestamp();
            Some(DateFilter::After(start))
        }
        "thisyear" => {
            let start_of_year = NaiveDate::from_ymd_opt(today.year(), 1, 1)?;
            let start = Local.from_local_datetime(&start_of_year.and_hms_opt(0, 0, 0)?).single()?.timestamp();
            Some(DateFilter::After(start))
        }
        _ => {
            // Range: YYYY-MM-DD..YYYY-MM-DD
            if let Some((start_s, end_s)) = input.split_once("..") {
                let start_date = NaiveDate::parse_from_str(start_s.trim(), "%Y-%m-%d").ok()?;
                let end_date = NaiveDate::parse_from_str(end_s.trim(), "%Y-%m-%d").ok()?;
                let start = Local.from_local_datetime(&start_date.and_hms_opt(0, 0, 0)?).single()?.timestamp();
                let end = Local.from_local_datetime(&end_date.and_hms_opt(23, 59, 59)?).single()?.timestamp();
                return Some(DateFilter::Between(start, end));
            }
            if let Some(rest) = input.strip_prefix('>') {
                let d = NaiveDate::parse_from_str(rest.trim(), "%Y-%m-%d").ok()?;
                let ts = Local.from_local_datetime(&d.and_hms_opt(0, 0, 0)?).single()?.timestamp();
                return Some(DateFilter::After(ts));
            }
            if let Some(rest) = input.strip_prefix('<') {
                let d = NaiveDate::parse_from_str(rest.trim(), "%Y-%m-%d").ok()?;
                let ts = Local.from_local_datetime(&d.and_hms_opt(23, 59, 59)?).single()?.timestamp();
                return Some(DateFilter::Before(ts));
            }
            if let Ok(d) = NaiveDate::parse_from_str(&input, "%Y-%m-%d") {
                let start = Local.from_local_datetime(&d.and_hms_opt(0, 0, 0)?).single()?.timestamp();
                let end = Local.from_local_datetime(&d.and_hms_opt(23, 59, 59)?).single()?.timestamp();
                return Some(DateFilter::Between(start, end));
            }
            None
        }
    }
}

fn eval_condition(cond: &Condition, entry: &SearchResultEntry, options: &SearchOptions) -> bool {
    match cond {
        Condition::Substring { pattern, is_wildcard } => {
            let target_str = if options.match_path {
                &entry.full_path
            } else {
                &entry.name
            };

            if *is_wildcard {
                wildcard_match(pattern, target_str, options.match_case)
            } else if options.match_whole_word {
                whole_word_match(pattern, target_str, options.match_case)
            } else if options.match_case {
                target_str.contains(pattern)
            } else {
                contains_ignore_case(target_str, pattern)
            }
        }
        Condition::Regex(re) => {
            let target_str = if options.match_path {
                &entry.full_path
            } else {
                &entry.name
            };
            re.is_match(target_str)
        }
        Condition::Extension(exts) => {
            exts.iter().any(|ext| entry.extension.eq_ignore_ascii_case(ext))
        }
        Condition::Size(sf) => {
            if entry.is_dir {
                true // folders match size queries or are treated gracefully
            } else {
                match sf {
                    SizeFilter::GreaterThan(val) => entry.size > *val,
                    SizeFilter::LessThan(val) => entry.size < *val,
                    SizeFilter::Exact(val) => entry.size == *val,
                    SizeFilter::Between(low, high) => entry.size >= *low && entry.size <= *high,
                }
            }
        }
        Condition::DateModified(df) => match df {
            DateFilter::After(ts) => entry.modified >= *ts,
            DateFilter::Before(ts) => entry.modified <= *ts,
            DateFilter::Between(start, end) => entry.modified >= *start && entry.modified <= *end,
        },
        Condition::DateCreated(df) => match df {
            DateFilter::After(ts) => entry.created >= *ts,
            DateFilter::Before(ts) => entry.created <= *ts,
            DateFilter::Between(start, end) => entry.created >= *start && entry.created <= *end,
        },
        Condition::PathContains(p) => {
            if options.match_case {
                entry.full_path.contains(p)
            } else {
                contains_ignore_case(&entry.full_path, p)
            }
        }
        Condition::Category(cat) => cat.matches_extension(&entry.extension, entry.is_dir),
        Condition::IsFolder(should_be_dir) => entry.is_dir == *should_be_dir,
        Condition::IsHidden => (entry.attributes & 0x02) != 0,
        Condition::IsSystem => (entry.attributes & 0x04) != 0,
        Condition::Not(sub) => !eval_condition(sub, entry, options),
        Condition::Or(subs) => subs.iter().any(|s| eval_condition(s, entry, options)),
        Condition::And(subs) => subs.iter().all(|s| eval_condition(s, entry, options)),
    }
}

fn eval_condition_compact(
    cond: &Condition,
    entry: &CompactEntry,
    pool: &PathPool,
    options: &SearchOptions,
) -> bool {
    match cond {
        Condition::Substring { pattern, is_wildcard } => {
            let full_path;
            let target_str: &str = if options.match_path {
                let dir = pool.get(entry.dir_id);
                full_path = if dir.is_empty() {
                    entry.name.clone()
                } else if dir.ends_with('\\') || dir.ends_with('/') {
                    format!("{}{}", dir, entry.name)
                } else {
                    format!("{}\\{}", dir, entry.name)
                };
                &full_path
            } else {
                &entry.name
            };

            if *is_wildcard {
                wildcard_match(pattern, target_str, options.match_case)
            } else if options.match_whole_word {
                whole_word_match(pattern, target_str, options.match_case)
            } else if options.match_case {
                target_str.contains(pattern)
            } else {
                contains_ignore_case(target_str, pattern)
            }
        }
        Condition::Regex(re) => {
            let full_path;
            let target_str: &str = if options.match_path {
                let dir = pool.get(entry.dir_id);
                full_path = if dir.is_empty() {
                    entry.name.clone()
                } else if dir.ends_with('\\') || dir.ends_with('/') {
                    format!("{}{}", dir, entry.name)
                } else {
                    format!("{}\\{}", dir, entry.name)
                };
                &full_path
            } else {
                &entry.name
            };
            re.is_match(target_str)
        }
        Condition::Extension(exts) => {
            exts.iter().any(|ext| entry.extension.eq_ignore_ascii_case(ext))
        }
        Condition::Size(sf) => {
            if entry.is_dir {
                true
            } else {
                match sf {
                    SizeFilter::GreaterThan(val) => entry.size > *val,
                    SizeFilter::LessThan(val) => entry.size < *val,
                    SizeFilter::Exact(val) => entry.size == *val,
                    SizeFilter::Between(low, high) => entry.size >= *low && entry.size <= *high,
                }
            }
        }
        Condition::DateModified(df) => match df {
            DateFilter::After(ts) => entry.modified >= *ts,
            DateFilter::Before(ts) => entry.modified <= *ts,
            DateFilter::Between(start, end) => entry.modified >= *start && entry.modified <= *end,
        },
        Condition::DateCreated(df) => match df {
            DateFilter::After(ts) => entry.created >= *ts,
            DateFilter::Before(ts) => entry.created <= *ts,
            DateFilter::Between(start, end) => entry.created >= *start && entry.created <= *end,
        },
        Condition::PathContains(p) => {
            let dir = pool.get(entry.dir_id);
            if options.match_case {
                dir.contains(p) || entry.name.contains(p)
            } else {
                contains_ignore_case(dir, p) || contains_ignore_case(&entry.name, p)
            }
        }
        Condition::Category(cat) => cat.matches_extension(&entry.extension, entry.is_dir),
        Condition::IsFolder(should_be_dir) => entry.is_dir == *should_be_dir,
        Condition::IsHidden => (entry.attributes & 0x02) != 0,
        Condition::IsSystem => (entry.attributes & 0x04) != 0,
        Condition::Not(sub) => !eval_condition_compact(sub, entry, pool, options),
        Condition::Or(subs) => subs.iter().any(|s| eval_condition_compact(s, entry, pool, options)),
        Condition::And(subs) => subs.iter().all(|s| eval_condition_compact(s, entry, pool, options)),
    }
}

pub fn wildcard_match(pattern: &str, text: &str, case_sensitive: bool) -> bool {
    let p_vec: Vec<char> = if case_sensitive {
        pattern.chars().collect()
    } else {
        pattern.to_lowercase().chars().collect()
    };
    let t_vec: Vec<char> = if case_sensitive {
        text.chars().collect()
    } else {
        text.to_lowercase().chars().collect()
    };

    fn rec(p: &[char], t: &[char]) -> bool {
        match (p.is_empty(), t.is_empty()) {
            (true, true) => true,
            (true, false) => false,
            (false, true) => p.iter().all(|&c| c == '*'),
            (false, false) => {
                if p[0] == '*' {
                    rec(&p[1..], t) || rec(p, &t[1..])
                } else if p[0] == '?' || p[0] == t[0] {
                    rec(&p[1..], &t[1..])
                } else {
                    false
                }
            }
        }
    }

    rec(&p_vec, &t_vec)
}

fn whole_word_match(pattern: &str, text: &str, case_sensitive: bool) -> bool {
    let (p, t) = if case_sensitive {
        (pattern.to_string(), text.to_string())
    } else {
        (pattern.to_lowercase(), text.to_lowercase())
    };

    for (start_idx, _) in t.match_indices(&p) {
        let end_idx = start_idx + p.len();
        let prev_ok = if start_idx == 0 {
            true
        } else {
            let prev_char = t[..start_idx].chars().last().unwrap();
            !prev_char.is_alphanumeric() && prev_char != '_'
        };
        let next_ok = if end_idx >= t.len() {
            true
        } else {
            let next_char = t[end_idx..].chars().next().unwrap();
            !next_char.is_alphanumeric() && next_char != '_'
        };
        if prev_ok && next_ok {
            return true;
        }
    }
    false
}
