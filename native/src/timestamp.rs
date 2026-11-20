//! YAML timestamps (`2024-03-05`, `2024-03-05T06:07:08.5+05:30`, `2024-03-05 06:07:08`), which the JVM module reads as
//! dates. A date or time with no offset is in the system zone.

use crate::model::DateTimeValue;
use crate::zone::{local_to_instant, offset_name};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

/// The date and time `text` stands for, or `None` when it is not a timestamp.
pub fn parse(text: &str) -> Option<DateTimeValue> {
    let (year, rest) = digits(text, 4, 4)?;
    let rest = rest.strip_prefix('-')?;
    let (month, rest) = digits(rest, 1, 2)?;
    let rest = rest.strip_prefix('-')?;
    let (day, rest) = digits(rest, 1, 2)?;
    let date = NaiveDate::from_ymd_opt(year as i32, month, day)?;
    if rest.is_empty() {
        return local(date.and_time(NaiveTime::MIN), 0);
    }
    let rest = match rest.strip_prefix(['T', 't']) {
        Some(after) => after,
        None => {
            let trimmed = rest.trim_start_matches([' ', '\t']);
            if trimmed.len() == rest.len() { return None; }
            trimmed
        }
    };
    let (hour, rest) = digits(rest, 1, 2)?;
    let rest = rest.strip_prefix(':')?;
    let (minute, rest) = digits(rest, 2, 2)?;
    let rest = rest.strip_prefix(':')?;
    let (second, rest) = digits(rest, 2, 2)?;
    let (nanos, rest) = match rest.strip_prefix('.') {
        Some(after) => {
            let end = after.find(|c: char| !c.is_ascii_digit()).unwrap_or(after.len());
            let fraction = &after[..end];
            let padded = format!("{:0<9}", &fraction[..fraction.len().min(9)]);
            (padded.parse::<u32>().ok()?, &after[end..])
        }
        None => (0, rest),
    };
    let wall = date.and_time(NaiveTime::from_hms_nano_opt(hour, minute, second, nanos)?);
    let rest = rest.trim_start_matches([' ', '\t']);
    if rest.is_empty() {
        return local(wall, nanos);
    }
    if rest == "Z" {
        return Some(DateTimeValue { seconds: wall.and_utc().timestamp(), nanos, zone: "UTC".into() });
    }
    let sign = match rest.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let (offset_hours, rest) = digits(&rest[1..], 1, 2)?;
    let offset_minutes = match rest.strip_prefix(':') {
        Some(after) if after.is_empty() => 0,
        Some(after) => {
            let (minutes, tail) = digits(after, 2, 2)?;
            if !tail.is_empty() { return None; }
            minutes
        }
        None if rest.is_empty() => 0,
        None => return None,
    };
    let total = sign * (offset_hours as i32 * 60 + offset_minutes as i32);
    Some(DateTimeValue { seconds: wall.and_utc().timestamp() - i64::from(total) * 60, nanos, zone: offset_name(total) })
}

fn local(wall: NaiveDateTime, nanos: u32) -> Option<DateTimeValue> {
    Some(DateTimeValue { seconds: local_to_instant(wall).ok()?, nanos, zone: String::new() })
}

/// Between `min` and `max` ASCII digits at the start of `text`, and what follows them.
fn digits(text: &str, min: usize, max: usize) -> Option<(u32, &str)> {
    let count = text.bytes().take(max).take_while(u8::is_ascii_digit).count();
    if count < min {
        return None;
    }
    Some((text[..count].parse().ok()?, &text[count..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_and_zulu_keep_their_instant() {
        assert_eq!(parse("2024-03-05T06:07:08Z"), Some(DateTimeValue { seconds: 1_709_618_828, nanos: 0, zone: "UTC".into() }));
        assert_eq!(parse("2024-03-05T06:07:08.5+05:30"), Some(DateTimeValue { seconds: 1_709_599_028, nanos: 500_000_000, zone: "+05:30".into() }));
        assert_eq!(parse("2024-03-05 06:07:08 -08").unwrap().zone, "-08:00");
        assert_eq!(parse("2024-03-05t06:07:08z").is_none(), true);
    }

    #[test]
    fn dates_and_local_times_use_the_system_zone() {
        for text in ["2024-03-05", "2024-3-5", "2024-03-05T06:07:08", "2024-03-05 06:07:08.25"] {
            assert_eq!(parse(text).unwrap().zone, "", "{text}");
        }
    }

    #[test]
    fn other_text_is_not_a_timestamp() {
        for text in ["", "2024", "2024-03", "2024-13-05", "2024-03-05x", "2024-03-05T", "2024-03-05T25:00:00", "12345-01-01", "a2024-03-05"] {
            assert!(parse(text).is_none(), "{text}");
        }
    }
}
