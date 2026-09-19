#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i64)]
pub enum Priority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
}

impl Priority {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "low" => Some(Self::Low),
            "" | "normal" => Some(Self::Normal),
            "high" => Some(Self::High),
            _ => None,
        }
    }

    pub fn from_number(value: i64) -> Self {
        match value {
            0 => Self::Low,
            2 => Self::High,
            _ => Self::Normal,
        }
    }

    pub fn value(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "低",
            Self::Normal => "通常",
            Self::High => "高",
        }
    }
}

pub fn validated_title(input: &str) -> Option<&str> {
    let title = input.trim();
    (!title.is_empty() && title.chars().count() <= 200).then_some(title)
}

// Dates are calendar dates, without a time or timezone conversion.
pub fn valid_date(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(i, b)| i != 4 && i != 7 && !b.is_ascii_digit())
    {
        return false;
    }
    let year: u32 = value[..4].parse().unwrap_or(0);
    let month: u32 = value[5..7].parse().unwrap_or(0);
    let day: u32 = value[8..].parse().unwrap_or(0);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let maximum = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    year > 0 && day > 0 && day <= maximum
}

pub fn title_matches(title: &str, query: &str) -> bool {
    title.to_lowercase().contains(&query.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_accepts_optional_valid_calendar_dates_and_leap_years() {
        for value in [
            "",
            "2026-09-19",
            "2024-02-29",
            "2000-02-29",
            "0001-01-01",
            "9999-12-31",
        ] {
            assert!(valid_date(value), "{value}");
        }
    }

    #[test]
    fn date_rejects_impossible_dates_and_noncanonical_formats() {
        for value in [
            "2026-02-29",
            "2100-02-29",
            "2026-04-31",
            "0000-01-01",
            "2026-00-10",
            "2026-01-00",
            "2026-13-01",
            "26-01-01",
            "2026-1-01",
            "２０２６-01-01",
            "2026-01-01T00:00:00",
            " 2026-01-01",
        ] {
            assert!(!valid_date(value), "{value}");
        }
    }

    #[test]
    fn priority_accepts_only_supported_values() {
        assert_eq!(Priority::parse(""), Some(Priority::Normal));
        assert_eq!(Priority::parse("high"), Some(Priority::High));
        assert_eq!(Priority::parse("low"), Some(Priority::Low));
        for value in ["urgent", "HIGH", "2", "-1"] {
            assert_eq!(Priority::parse(value), None);
        }
    }

    #[test]
    fn search_is_literal_unicode_and_ascii_case_insensitive() {
        assert!(title_matches("Rustで買い物を整理", "rust"));
        assert!(title_matches("Rustで買い物を整理", "買い物"));
        assert!(!title_matches("買い物", "%"));
        assert!(title_matches("100%_done", "%_"));
        assert!(title_matches("なんでも", ""));
    }

    #[test]
    fn dates_check_every_month_end_in_common_and_leap_years() {
        for (year, last_days) in [
            (2026, [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]),
            (2028, [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]),
        ] {
            for (index, last_day) in last_days.into_iter().enumerate() {
                let month = index + 1;
                assert!(valid_date(&format!("{year}-{month:02}-01")));
                assert!(valid_date(&format!("{year}-{month:02}-{last_day:02}")));
                assert!(!valid_date(&format!("{year}-{month:02}-00")));
                assert!(!valid_date(&format!(
                    "{year}-{month:02}-{:02}",
                    last_day + 1
                )));
            }
        }
    }
}
