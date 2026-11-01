//! First-match daily mood rules.
use crate::{
    development::weeks,
    snapshot::{Snapshot, monday},
};
use chrono::{Datelike, Weekday};
/// Whole-loop atmosphere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    /// At least sixty quiet days.
    Abandoned,
    /// At least fourteen quiet days.
    Night,
    /// First commit day in this week.
    MarketDay,
    /// Recent milestone or busy Sunday.
    Festival,
    /// Ordinary activity.
    Workday,
}
/// Select the first matching rule using calendar dates, never sparse entry counts.
pub fn select(snapshot: &Snapshot) -> Mood {
    let quiet = snapshot
        .daily_commits
        .iter()
        .rev()
        .find(|(_, count)| **count > 0)
        .map_or(
            (snapshot.observed_on - snapshot.created_on).num_days() + 1,
            |(day, _)| (snapshot.observed_on - *day).num_days(),
        );
    if quiet >= 60 {
        return Mood::Abandoned;
    }
    if quiet >= 14 {
        return Mood::Night;
    }
    let week = monday(snapshot.observed_on);
    if snapshot
        .daily_commits
        .get(&snapshot.observed_on)
        .copied()
        .unwrap_or(0)
        > 0
        && snapshot
            .daily_commits
            .range(week..snapshot.observed_on)
            .all(|(_, count)| *count == 0)
    {
        return Mood::MarketDay;
    }
    let previous = snapshot.total_commits.saturating_sub(snapshot.recent(7));
    let milestone = [100, 500, 1000, 5000, 10000, 25000, 50000]
        .iter()
        .any(|&m| previous < m && m <= snapshot.total_commits);
    let weeks = weeks(snapshot);
    let compared: Vec<_> = (1..=52)
        .map(|offset| week - chrono::Duration::days(offset * 7))
        .take_while(|monday| *monday >= snapshot.created_on)
        .map(|monday| weeks.get(&monday).copied().unwrap_or(0))
        .collect();
    let current = weeks.get(&week).copied().unwrap_or(0);
    let busy = snapshot.observed_on.weekday() == Weekday::Sun
        && compared.len() >= 10
        && current > 0
        && compared.iter().filter(|&&count| count >= current).count() * 10 <= compared.len();
    if milestone || busy {
        Mood::Festival
    } else {
        Mood::Workday
    }
}
