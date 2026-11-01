//! Versioned, repository-name-free commit observations.
use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
/// Commits grouped by primary language.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Language {
    /// GitHub language name.
    pub name: String,
    /// Optional RGB hex color.
    pub color: Option<String>,
    /// First calendar date on which cumulative public language commits reached 25.
    pub arm_earned_on: Option<NaiveDate>,
    /// Commit weight.
    pub commits: u64,
}
/// Portable account snapshot.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// Schema identifier.
    pub schema_version: u32,
    /// Stable numeric account identifier.
    pub account_id: u64,
    /// Account login.
    pub login: String,
    /// Account creation date.
    pub created_on: NaiveDate,
    /// Last complete UTC day.
    pub observed_on: NaiveDate,
    /// Exact lifetime commit total.
    pub total_commits: u64,
    /// Sparse daily commit detail; absent dates mean zero.
    pub daily_commits: BTreeMap<NaiveDate, u64>,
    /// Primary language aggregates.
    pub languages: Vec<Language>,
    /// At least one single-day window reached the 100-repository cap.
    pub detail_truncated: bool,
}
/// Monday containing a date.
pub fn monday(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday().into())
}
impl Snapshot {
    /// Decode and validate, with a repair instruction.
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_slice(bytes)
            .map_err(|e| format!("invalid snapshot: {e}; collect again"))?;
        if value["schema_version"] != 1 {
            return Err(format!(
                "unsupported schema version; upgrade {} or collect again",
                crate::DISPLAY_NAME
            ));
        }
        let snapshot: Self = serde_json::from_value(value)
            .map_err(|e| format!("invalid snapshot: {e}; collect again"))?;
        snapshot.validate()?;
        Ok(snapshot)
    }
    /// Reject inconsistent or malformed observations.
    pub fn validate(&self) -> Result<(), String> {
        let invalid = || {
            Err(format!(
                "invalid snapshot; run {} collect again",
                crate::DISPLAY_NAME
            ))
        };
        if self.schema_version != 1 {
            return Err(format!(
                "unsupported schema version; upgrade {} or collect again",
                crate::DISPLAY_NAME
            ));
        }
        if self.login.is_empty()
            || self.login.contains('/')
            || self.created_on > self.observed_on.succ_opt().unwrap_or(self.observed_on)
            || self
                .daily_commits
                .keys()
                .any(|d| *d < self.created_on || *d > self.observed_on)
            || self.languages.iter().any(|l| {
                l.name.is_empty()
                    || l.color.as_deref().is_some_and(|c| parse_color(c).is_none())
                    || l.arm_earned_on.is_some_and(|day| {
                        l.commits < 25 || day < self.created_on || day > self.observed_on
                    })
            })
        {
            return invalid();
        }
        let daily: u128 = self.daily_commits.values().map(|v| u128::from(*v)).sum();
        let languages: u128 = self.languages.iter().map(|l| u128::from(l.commits)).sum();
        if (!self.detail_truncated && daily > u128::from(self.total_commits))
            || languages > u128::from(self.total_commits)
        {
            return invalid();
        }
        Ok(())
    }
    /// Commits within the latest number of calendar days.
    pub fn recent(&self, days: i64) -> u64 {
        self.daily_commits
            .range((self.observed_on - Duration::days(days - 1))..=self.observed_on)
            .fold(0u64, |sum, (_, count)| sum.saturating_add(*count))
    }
}
/// Parse GitHub RGB notation.
pub fn parse_color(color: &str) -> Option<[u8; 3]> {
    let digits = color.strip_prefix('#')?;
    if digits.len() != 6 || !digits.is_ascii() {
        return None;
    }
    Some([
        u8::from_str_radix(&digits[0..2], 16).ok()?,
        u8::from_str_radix(&digits[2..4], 16).ok()?,
        u8::from_str_radix(&digits[4..6], 16).ok()?,
    ])
}
