use chrono::{DateTime, FixedOffset, TimeZone};

pub fn parse_datetime(str: &str) -> Option<DateTime<FixedOffset>> {
    let values = DateTimeValues::from(str);
    FixedOffset::east_opt(values.timezone.secs()).and_then(|tz| {
        tz.with_ymd_and_hms(
            values.year,
            values.month,
            values.date,
            values.hours,
            values.minutes,
            values.seconds,
        )
        .single()
    })
}

struct DateTimeValues {
    year: i32,
    month: u32,
    date: u32,
    hours: u32,
    minutes: u32,
    seconds: u32,
    timezone: TimezoneValues,
}

impl DateTimeValues {
    fn default() -> Self {
        Self {
            year: 1970,
            month: 1,
            date: 1,
            hours: 0,
            minutes: 0,
            seconds: 0,
            timezone: TimezoneValues::default(),
        }
    }

    fn from(str: &str) -> Self {
        let mut values = Self::default();
        values.parse(str, &ParserState::Year);
        return values;
    }

    fn parse(&mut self, str: &str, state: &ParserState) {
        match state {
            ParserState::Timezone => self.timezone.parse(str, TimezoneParserState::Sign),
            _ => self.parse_state(str, state),
        }
    }

    fn parse_state(&mut self, str: &str, state: &ParserState) {
        if let Some(from) = state.parse_from(str) {
            let size = state.size();
            let field = str
                .get(from..from + size)
                .and_then(|value| state.parse(value));

            if let Some(field) = field {
                if let Some(next) = state.next() {
                    self.parse(&str[from + size..], &next);
                }
                self.set(field);
            }
        }
    }

    fn set(&mut self, field: DateTimeField) {
        match field {
            DateTimeField::Year(value) => self.year = value,
            DateTimeField::Month(value) => self.month = value,
            DateTimeField::Date(value) => self.date = value,
            DateTimeField::Hours(value) => self.hours = value,
            DateTimeField::Minutes(value) => self.minutes = value,
            DateTimeField::Seconds(value) => self.seconds = value,
            DateTimeField::Timezone(value) => self.timezone = value,
        }
    }
}

enum DateTimeField {
    Year(i32),
    Month(u32),
    Date(u32),
    Hours(u32),
    Minutes(u32),
    Seconds(u32),
    Timezone(TimezoneValues),
}

#[derive(Debug, PartialEq)]
enum ParserState {
    Year,
    Month,
    Date,
    Hours,
    Minutes,
    Seconds,
    Timezone,
}

impl ParserState {
    fn next(&self) -> Option<Self> {
        match self {
            ParserState::Year => Some(ParserState::Month),
            ParserState::Month => Some(ParserState::Date),
            ParserState::Date => Some(ParserState::Hours),
            ParserState::Hours => Some(ParserState::Minutes),
            ParserState::Minutes => Some(ParserState::Seconds),
            ParserState::Seconds => Some(ParserState::Timezone),
            _ => None,
        }
    }

    fn size(&self) -> usize {
        match self {
            ParserState::Year => 4,
            ParserState::Month
            | ParserState::Date
            | ParserState::Hours
            | ParserState::Minutes
            | ParserState::Seconds => 2,
            _ => 1,
        }
    }

    fn prefix(&self) -> Option<ParserPrefix> {
        match self {
            ParserState::Month | ParserState::Date => Some(ParserPrefix::Dash),
            ParserState::Hours => Some(ParserPrefix::T),
            ParserState::Minutes | ParserState::Seconds => Some(ParserPrefix::Colon),
            _ => None,
        }
    }

    fn parse(&self, str: &str) -> Option<DateTimeField> {
        match self {
            ParserState::Year => str.parse().ok().map(|v| DateTimeField::Year(v)),
            ParserState::Month => str.parse().ok().map(|v| DateTimeField::Month(v)),
            ParserState::Date => str.parse().ok().map(|v| DateTimeField::Date(v)),
            ParserState::Hours => str.parse().ok().map(|v| DateTimeField::Hours(v)),
            ParserState::Minutes => str.parse().ok().map(|v| DateTimeField::Minutes(v)),
            ParserState::Seconds => str.parse().ok().map(|v| DateTimeField::Seconds(v)),
            _ => None,
        }
    }

    fn parse_from(&self, str: &str) -> Option<usize> {
        match self.prefix() {
            Some(ParserPrefix::Dash) => str.get(..1).map(|s| if s == "-" { 1 } else { 0 }),
            Some(ParserPrefix::Colon) => str.get(..1).map(|s| if s == ":" { 1 } else { 0 }),
            Some(ParserPrefix::T) => {
                str.get(..1)
                    .and_then(|s| if s == "T" || s == " " { Some(1) } else { None })
            }
            None => Some(0),
        }
    }
}

enum ParserPrefix {
    Dash,
    Colon,
    T,
}

struct TimezoneValues {
    sign: bool,
    hours: i32,
    minutes: i32,
}

impl TimezoneValues {
    fn default() -> Self {
        Self {
            sign: true,
            hours: 0,
            minutes: 0,
        }
    }

    fn secs(&self) -> i32 {
        (self.hours * 60 + self.minutes) * 60 * if self.sign { 1 } else { -1 }
    }

    fn parse(&mut self, str: &str, state: TimezoneParserState) {
        match state {
            TimezoneParserState::Sign => self.parse_sign(str),
            TimezoneParserState::Hours => self.parse_hours(str),
            TimezoneParserState::Minutes => self.parse_minutes(str),
        }
    }

    fn parse_sign(&mut self, str: &str) {
        if let Some(first) = str.get(..1) {
            if first == "+" || first == "-" {
                self.sign = first == "+";

                if let Some(str) = str.get(1..) {
                    self.parse(str, TimezoneParserState::Hours)
                }
            }
        }
    }

    fn parse_hours(&mut self, str: &str) {
        if let Some(hours) = str.get(..2).and_then(|v| v.parse::<i32>().ok()) {
            self.hours = hours;

            if let Some(str) = str.get(2..) {
                self.parse(str, TimezoneParserState::Minutes)
            }
        }
    }

    fn parse_minutes(&mut self, str: &str) {
        match str.get(..1) {
            Some(":") => {
                if let Some(str) = str.get(1..) {
                    self.parse(str, TimezoneParserState::Minutes);
                }
            }

            Some(_) => {
                if let Some(minutes) = str.get(..2).and_then(|v| v.parse::<i32>().ok()) {
                    self.minutes = minutes;
                }
            }

            None => {}
        }
    }
}

enum TimezoneParserState {
    Sign,
    Hours,
    Minutes,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_datetime_from_file() {
        let values = vec![
            // Year
            ("2024", "2024-01-01T00:00:00Z"), // #0
            // Month
            ("2024-02", "2024-02-01T00:00:00Z"), // #1
            ("202402", "2024-02-01T00:00:00Z"),  // #2
            // Date
            ("2024-02-11", "2024-02-11T00:00:00Z"), // #3
            ("20240211", "2024-02-11T00:00:00Z"),   // #4
            // Hours
            ("2024-02-11T14", "2024-02-11T14:00:00Z"), // #5
            ("2024-02-11 14", "2024-02-11T14:00:00Z"), // #6
            // Minutes
            ("2024-02-11T14:15", "2024-02-11T14:15:00Z"), // #7
            ("2024-02-11 14:15", "2024-02-11T14:15:00Z"), // #8
            // Seconds
            ("2024-02-11T14:15:45", "2024-02-11T14:15:45Z"), // #9
            ("2024-02-11 14:15:45", "2024-02-11T14:15:45Z"), // #10
            // Timezone
            ("2024-02-11T14:15:45+05:00", "2024-02-11T14:15:45+05:00"), // #11
            ("2024-02-11T14:15:45-05:00", "2024-02-11T14:15:45-05:00"), // #12
            ("2024-02-11T14:15:45+04:30", "2024-02-11T14:15:45+04:30"), // #13
            ("2024-02-11T14:15:45+04:30", "2024-02-11T14:15:45+04:30"), // #14
            ("2024-02-11T14:15:45-04:30", "2024-02-11T14:15:45-04:30"), // #15
            ("2024-02-11T14:15:45+0500", "2024-02-11T14:15:45+05:00"),  // #16
            ("2024-02-11T14:15:45-0445", "2024-02-11T14:15:45-04:45"),  // #17
            ("2024-02-11T14:15:45+05", "2024-02-11T14:15:45+05:00"),    // #18
            ("2024-02-11T14:15:45-04", "2024-02-11T14:15:45-04:00"),    // #19
            ("2024-02-11T14:15:45+00:30", "2024-02-11T14:15:45+00:30"), // #20
            ("2024-02-11T14:15:45-00:30", "2024-02-11T14:15:45-00:30"), // #21
        ];

        for (index, (input, expected)) in values.iter().enumerate() {
            let result = parse_datetime(input).unwrap();
            assert_eq!(
                result,
                chrono_datetime(expected),
                "Failed to parse date #{index} \"{}\": expected \"{}\" but got \"{}\"",
                input,
                expected,
                result
            );
        }
    }

    fn chrono_datetime(str: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(str).unwrap()
    }
}
