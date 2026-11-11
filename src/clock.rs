//! The report's "Generated on" date. The plugin host gives a plugin no clock, so each run reads the
//! app's: it adds a temporary comment, reads the `created` time (Unix seconds) the app stamped on it
//! and deletes it again. The time is then shifted by the page's fixed time zone and shown as a date.
//!
//! The host is reached through a closure, so the whole sequence can be tested without an app.

use crate::i18n::{Key, Lang, t};
use figcraft_plugin_sdk::{Result, Value, json};

/// The text of the temporary comment.
pub const TMP_TEXT: &str = "tmp (handoff-components date)";
/// The default time zone: São Paulo, hours from UTC. Fixed: there is no daylight saving time.
pub const DEFAULT_TZ: f64 = -3.0;
/// The accepted range, in hours from UTC.
const TZ_LIMIT: f64 = 14.0;

/// A time zone typed as hours from UTC (`-3`, `0`, `+9`, `5.5`), or none when it isn't a number
/// from -14 to 14.
pub fn parse_tz(text: &str) -> Option<f64> {
    let hours: f64 = text.trim().parse().ok()?;
    (-TZ_LIMIT..=TZ_LIMIT).contains(&hours).then_some(if hours == 0.0 { 0.0 } else { hours })
}

/// A time zone as stored: `-3`, `5.5`.
pub fn tz_text(hours: f64) -> String {
    hours.to_string()
}

/// A time zone in hours as seconds from UTC.
pub fn offset_secs(hours: f64) -> i64 {
    (hours * 3600.0).round() as i64
}

/// Days since 1970-01-01 as (year, month, day), proleptic Gregorian (Hinnant's algorithm).
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// Unix seconds plus a time zone, as the (year, month, day) there.
pub fn civil(created: i64, tz_hours: f64) -> (i64, u32, u32) {
    civil_from_days((created + offset_secs(tz_hours)).div_euclid(86_400))
}

/// The date as the report shows it: `AAAA-MM-DD` in English, `DD/MM/AAAA` in Portuguese.
pub fn date_text(created: i64, tz_hours: f64, lang: Lang) -> String {
    let (y, m, d) = civil(created, tz_hours);
    match lang {
        Lang::En => format!("{y:04}-{m:02}-{d:02}"),
        Lang::Pt => format!("{d:02}/{m:02}/{y:04}"),
    }
}

/// The report's line: "Generated on: 2026-11-11" (en), "Gerado em: 11/11/2026" (pt), or the word for
/// a date that couldn't be read ("unavailable", "indisponível").
pub fn generated_line(created: Option<i64>, tz_hours: f64, lang: Lang) -> String {
    let when = created.map_or_else(|| t(lang, Key::DateUnavailable).to_string(), |c| date_text(c, tz_hours, lang));
    format!("{}: {when}", t(lang, Key::GeneratedOn))
}

/// What reading the clock gave.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stamp {
    /// The `created` time of the temporary comment, in Unix seconds; none when it couldn't be read.
    pub created: Option<i64>,
    /// Why `created` is none.
    pub failure: Option<String>,
    /// The id of the temporary comment when it could not be deleted (even after a second try).
    pub leftover: Option<u64>,
}

/// Reads the app's clock through `call` (a host command): `comment.add`, `comment.list` to find the
/// new comment's `created`, `comment.delete` (twice, if the first try fails). Nothing here returns an
/// error: a failure is told in the [`Stamp`], so the report is still made. The comments are not
/// counted before and after: someone else may comment while this runs.
pub fn read(page: u64, call: &mut dyn FnMut(&str, &Value) -> Result<Value>) -> Stamp {
    let added = match call("comment.add", &json!({"x": 0, "y": 0, "text": TMP_TEXT, "page": page})) {
        Ok(v) => v,
        Err(e) => return Stamp { failure: Some(format!("comment.add: {e}")), ..Stamp::default() },
    };
    let Some(id) = added.get("id").and_then(Value::as_u64) else {
        return Stamp { failure: Some("comment.add: no id in the reply".into()), ..Stamp::default() };
    };
    let (created, failure) = match call("comment.list", &json!({"page": "all"})) {
        Err(e) => (None, Some(format!("comment.list: {e}"))),
        Ok(list) => {
            let created = list
                .get("comments")
                .and_then(Value::as_array)
                .and_then(|a| a.iter().find(|c| c.get("id").and_then(Value::as_u64) == Some(id)))
                .and_then(|c| c.get("created"))
                .and_then(Value::as_i64)
                .filter(|t| *t > 0);
            (created, created.is_none().then(|| "comment.list: the new comment (or its `created`) isn't in the list".to_string()))
        }
    };
    // The comment goes even when its time couldn't be read.
    let params = json!({"id": id});
    let deleted = call("comment.delete", &params).or_else(|_| call("comment.delete", &params));
    Stamp { created, failure, leftover: deleted.is_err().then_some(id) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// 2026-11-11 23:30:00 UTC.
    const LATE: i64 = 1_794_439_800;

    fn date(created: i64, tz: f64) -> String {
        date_text(created, tz, Lang::En)
    }

    #[test]
    fn created_to_date_across_zones() {
        assert_eq!(date(LATE, 0.0), "2026-11-11");
        // -3: 20:30 on the 11th; +9: 08:30 on the 12th.
        assert_eq!(date(LATE, -3.0), "2026-11-11");
        assert_eq!(date(LATE, 9.0), "2026-11-12");
    }

    #[test]
    fn the_night_turn_depends_on_the_zone() {
        // 00:30 UTC on the 12th is still 21:30 on the 11th in São Paulo.
        let after = LATE + 3600;
        assert_eq!(date(after, 0.0), "2026-11-12");
        assert_eq!(date(after, -3.0), "2026-11-11");
        // 02:59:59 UTC on the 12th is 23:59:59 on the 11th at -3; one second later it is the 12th.
        let edge = after + 2 * 3600 + 29 * 60 + 59;
        assert_eq!(date(edge, -3.0), "2026-11-11");
        assert_eq!(date(edge + 1, -3.0), "2026-11-12");
    }

    #[test]
    fn epoch_leap_day_and_year_end() {
        assert_eq!(date(0, 0.0), "1970-01-01");
        assert_eq!(date(0, -3.0), "1969-12-31");
        assert_eq!(date(1_709_164_800, 0.0), "2024-02-29");
        assert_eq!(date(1_709_164_800 - 1, 0.0), "2024-02-28");
        assert_eq!(date(1_709_164_800 + 86_400, 0.0), "2024-03-01");
        // 2100 is not a leap year: Feb 28 is followed by Mar 1.
        assert_eq!(date(4_107_542_400, 0.0), "2100-03-01");
        assert_eq!(date(1_767_225_599, 0.0), "2025-12-31");
        assert_eq!(date(1_767_225_600, 0.0), "2026-01-01");
        assert_eq!(date(1_767_225_600, -3.0), "2025-12-31");
    }

    #[test]
    fn a_fractional_zone_moves_by_minutes() {
        assert_eq!(offset_secs(5.5), 19_800);
        assert_eq!(offset_secs(-3.0), -10_800);
        assert_eq!(offset_secs(-3.5), -12_600);
        // 18:29:59 UTC + 5:30 is 23:59:59 the same day; one second later it is the next.
        let t = 1_767_225_600 - 5 * 3600 - 30 * 60 - 1;
        assert_eq!(date(t, 5.5), "2025-12-31");
        assert_eq!(date(t + 1, 5.5), "2026-01-01");
        assert_eq!(date(0, 5.5), "1970-01-01");
        assert_eq!(date(0, -3.5), "1969-12-31");
    }

    #[test]
    fn the_format_follows_the_language() {
        assert_eq!(date_text(LATE, -3.0, Lang::En), "2026-11-11");
        assert_eq!(date_text(LATE, -3.0, Lang::Pt), "11/11/2026");
        assert_eq!(date_text(0, 0.0, Lang::Pt), "01/01/1970");
        assert_eq!(date_text(1_709_164_800, 0.0, Lang::Pt), "29/02/2024");
    }

    #[test]
    fn the_report_line_in_both_languages_and_without_a_date() {
        assert_eq!(generated_line(Some(LATE), -3.0, Lang::En), "Generated on: 2026-11-11");
        assert_eq!(generated_line(Some(LATE), -3.0, Lang::Pt), "Gerado em: 11/11/2026");
        assert_eq!(generated_line(Some(LATE), 9.0, Lang::En), "Generated on: 2026-11-12");
        assert_eq!(generated_line(None, -3.0, Lang::En), "Generated on: unavailable");
        assert_eq!(generated_line(None, -3.0, Lang::Pt), "Gerado em: indisponível");
    }

    #[test]
    fn the_zone_is_a_number_of_hours_from_minus_14_to_14() {
        for (text, hours) in [("-3", -3.0), ("0", 0.0), ("-0", 0.0), ("+9", 9.0), (" 5.5 ", 5.5), ("-3.75", -3.75), ("14", 14.0), ("-14", -14.0), ("-12.5", -12.5)] {
            assert_eq!(parse_tz(text), Some(hours), "{text}");
        }
        for bad in ["", "abc", "UTC-3", "14.5", "-15", "15", "99", "1e3", "inf", "NaN", "5,5", "3h", "- 3"] {
            assert_eq!(parse_tz(bad), None, "{bad}");
        }
        assert_eq!(tz_text(-3.0), "-3");
        assert_eq!(tz_text(5.5), "5.5");
        assert_eq!(tz_text(parse_tz("-0").unwrap_or(1.0)), "0");
        assert_eq!(parse_tz(&tz_text(-3.75)), Some(-3.75));
    }

    /// A fake host: `fail` names commands that fail (a command listed twice fails twice).
    struct Host {
        fail: Vec<&'static str>,
        calls: RefCell<Vec<String>>,
        /// What `comment.list` returns for the comment.
        listed: Value,
    }

    impl Host {
        fn new(fail: &[&'static str]) -> Host {
            Host { fail: fail.to_vec(), calls: RefCell::new(Vec::new()), listed: json!({"comments": [{"id": 6, "created": 1}, {"id": 7, "created": LATE}]}) }
        }

        fn run(&mut self, page: u64) -> (Stamp, Vec<String>) {
            let stamp = read(page, &mut |cmd: &str, params: &Value| {
                self.calls.borrow_mut().push(format!("{cmd} {params}"));
                // The n-th call of a command fails when the command is listed n times or more.
                let tries = self.calls.borrow().iter().filter(|c| c.starts_with(cmd)).count();
                if self.fail.iter().filter(|f| **f == cmd).count() >= tries {
                    return Err("boom".to_string());
                }
                match cmd {
                    "comment.add" => Ok(json!({"id": 7})),
                    "comment.list" => Ok(self.listed.clone()),
                    _ => Ok(json!({})),
                }
            });
            let calls = self.calls.borrow().clone();
            (stamp, calls)
        }
    }

    #[test]
    fn the_clock_is_read_by_add_list_delete_in_that_order() {
        let (stamp, calls) = Host::new(&[]).run(3);
        assert_eq!(stamp, Stamp { created: Some(LATE), failure: None, leftover: None });
        assert_eq!(
            calls,
            vec![
                format!(r#"comment.add {{"page":3,"text":"{TMP_TEXT}","x":0,"y":0}}"#),
                r#"comment.list {"page":"all"}"#.to_string(),
                r#"comment.delete {"id":7}"#.to_string(),
            ]
        );
    }

    #[test]
    fn the_new_comment_is_found_by_its_id_not_by_position_or_count() {
        let mut host = Host::new(&[]);
        // Someone else's comments around it, newer and older.
        host.listed = json!({"comments": [{"id": 9, "created": 5}, {"id": 7, "created": LATE}, {"id": 8, "created": 99}]});
        assert_eq!(host.run(1).0.created, Some(LATE));
    }

    #[test]
    fn a_failed_add_means_no_date_and_nothing_to_clean() {
        let (stamp, calls) = Host::new(&["comment.add"]).run(1);
        assert_eq!(stamp.created, None);
        assert_eq!(stamp.failure.as_deref(), Some("comment.add: boom"));
        assert_eq!(stamp.leftover, None);
        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn a_failed_list_means_no_date_but_the_comment_is_still_deleted() {
        let (stamp, calls) = Host::new(&["comment.list"]).run(1);
        assert_eq!(stamp.created, None);
        assert_eq!(stamp.failure.as_deref(), Some("comment.list: boom"));
        assert_eq!(stamp.leftover, None);
        assert_eq!(calls.last().map(String::as_str), Some(r#"comment.delete {"id":7}"#));
    }

    #[test]
    fn a_comment_missing_from_the_list_or_without_a_time_means_no_date() {
        for listed in [json!({"comments": [{"id": 6, "created": 1}]}), json!({"comments": [{"id": 7}]}), json!({"comments": [{"id": 7, "created": 0}]}), json!({}), json!({"comments": [{"id": 7, "created": "x"}]})] {
            let mut host = Host::new(&[]);
            host.listed = listed;
            let (stamp, _) = host.run(1);
            assert_eq!(stamp.created, None);
            assert!(stamp.failure.is_some());
            assert_eq!(stamp.leftover, None);
        }
    }

    #[test]
    fn a_failed_delete_is_tried_once_more() {
        let (stamp, calls) = Host::new(&["comment.delete"]).run(1);
        assert_eq!(stamp, Stamp { created: Some(LATE), failure: None, leftover: None });
        assert_eq!(calls.iter().filter(|c| c.starts_with("comment.delete")).count(), 2);
    }

    #[test]
    fn two_failed_deletes_keep_the_date_and_name_the_comment() {
        let (stamp, calls) = Host::new(&["comment.delete", "comment.delete"]).run(1);
        assert_eq!(stamp, Stamp { created: Some(LATE), failure: None, leftover: Some(7) });
        assert_eq!(calls.iter().filter(|c| c.starts_with("comment.delete")).count(), 2);
    }

    #[test]
    fn nothing_read_and_nothing_deleted_tells_both() {
        let (stamp, _) = Host::new(&["comment.list", "comment.delete", "comment.delete"]).run(1);
        assert_eq!((stamp.created, stamp.leftover), (None, Some(7)));
        assert!(stamp.failure.is_some());
    }
}
