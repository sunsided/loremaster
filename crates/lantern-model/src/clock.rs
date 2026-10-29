use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// A time of day in minutes after midnight, written `HH:MM`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeOfDay(pub u16);

impl TimeOfDay {
    pub const MINUTES_PER_DAY: u32 = 24 * 60;

    pub fn hm(hours: u16, minutes: u16) -> Self {
        Self(hours * 60 + minutes)
    }
}

impl fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.0 / 60, self.0 % 60)
    }
}

impl FromStr for TimeOfDay {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (h, m) = s
            .split_once(':')
            .ok_or_else(|| format!("expected HH:MM, got `{s}`"))?;
        let h: u16 = h.trim().parse().map_err(|_| format!("bad hour in `{s}`"))?;
        let m: u16 = m
            .trim()
            .parse()
            .map_err(|_| format!("bad minute in `{s}`"))?;
        if h > 23 || m > 59 {
            return Err(format!("time out of range: `{s}`"));
        }
        Ok(Self::hm(h, m))
    }
}

impl Serialize for TimeOfDay {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for TimeOfDay {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// A named range of the day, starting at `from` and lasting until the next phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Phase {
    pub name: String,
    pub from: TimeOfDay,
}

/// How turns map to time, and which phases the day has.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockConfig {
    pub start: TimeOfDay,
    pub minutes_per_turn: u16,
    pub phases: Vec<Phase>,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            start: TimeOfDay::hm(9, 0),
            minutes_per_turn: 1,
            phases: vec![
                Phase {
                    name: "night".into(),
                    from: TimeOfDay::hm(0, 0),
                },
                Phase {
                    name: "dawn".into(),
                    from: TimeOfDay::hm(5, 0),
                },
                Phase {
                    name: "day".into(),
                    from: TimeOfDay::hm(7, 0),
                },
                Phase {
                    name: "dusk".into(),
                    from: TimeOfDay::hm(19, 0),
                },
                Phase {
                    name: "night".into(),
                    from: TimeOfDay::hm(21, 0),
                },
            ],
        }
    }
}

impl ClockConfig {
    /// The phase active at the given time of day. Phases wrap around midnight.
    pub fn phase_at(&self, time: TimeOfDay) -> Option<&str> {
        let mut sorted: Vec<&Phase> = self.phases.iter().collect();
        sorted.sort_by_key(|p| p.from);
        sorted
            .iter()
            .rev()
            .find(|p| p.from <= time)
            .or_else(|| sorted.last())
            .map(|p| p.name.as_str())
    }

    /// Distinct phase names in declaration order.
    pub fn phase_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = Vec::new();
        for p in &self.phases {
            if !names.contains(&p.name.as_str()) {
                names.push(&p.name);
            }
        }
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::check;

    #[test]
    fn phases_wrap_around_midnight() {
        let clock = ClockConfig {
            phases: vec![
                Phase {
                    name: "day".into(),
                    from: TimeOfDay::hm(7, 0),
                },
                Phase {
                    name: "night".into(),
                    from: TimeOfDay::hm(21, 0),
                },
            ],
            ..ClockConfig::default()
        };
        check!(clock.phase_at(TimeOfDay::hm(3, 0)) == Some("night"));
        check!(clock.phase_at(TimeOfDay::hm(7, 0)) == Some("day"));
        check!(clock.phase_at(TimeOfDay::hm(22, 30)) == Some("night"));
    }

    #[test]
    fn time_round_trips() {
        let t: TimeOfDay = "08:05".parse().unwrap();
        check!(t == TimeOfDay::hm(8, 5));
        check!(t.to_string() == "08:05");
        check!("24:00".parse::<TimeOfDay>().is_err());
    }
}
