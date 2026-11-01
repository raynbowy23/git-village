//! Commit-derived growth and weekly upkeep.
use crate::snapshot::{Snapshot, monday};
use chrono::Duration;
use std::collections::BTreeMap;
/// Inclusive growth thresholds.
pub const STAGES: [(&str, u64); 6] = [
    ("Outpost", 0),
    ("Hamlet", 25),
    ("Village", 250),
    ("Town", 1000),
    ("City", 5000),
    ("Metropolis", 20000),
];
/// Bounded visual signals.
#[derive(Clone, Debug)]
pub struct Development {
    /// Growth stage index.
    pub stage: usize,
    /// Number of claimed plots.
    pub building_count: usize,
    /// Maximum building height.
    pub max_height: i32,
    /// Commits during the last 365 days.
    pub population: u64,
    /// Active weekly run, allowing the current incomplete week.
    pub current_streak: usize,
    /// Window and lamp upkeep, from zero to seven, fading for four weeks after a streak ends.
    pub upkeep: usize,
    /// Longest active weekly run.
    pub longest_streak: usize,
    /// Latest seven days of commits.
    pub recent: u64,
}
/// Aggregate Monday-Sunday weeks.
pub fn weeks(snapshot: &Snapshot) -> BTreeMap<chrono::NaiveDate, u64> {
    let mut weeks = BTreeMap::<_, u64>::new();
    for (&day, &count) in &snapshot.daily_commits {
        let entry = weeks.entry(monday(day)).or_default();
        *entry = entry.saturating_add(count)
    }
    weeks
}
/// Derive all bounded traits without a clock.
pub fn derive(snapshot: &Snapshot) -> Development {
    let total = snapshot.total_commits;
    // Log interpolation preserves diminishing growth within each visually distinct stage.
    let anchors = [
        (0, 1, 5),
        (25, 3, 7),
        (250, 9, 12),
        (1000, 22, 22),
        (5000, 55, 43),
        (20000, 96, 74),
        (100000, 96, 80),
    ];
    let mut building_count = 96;
    let mut max_height = 80;
    for pair in anchors.windows(2) {
        let (lower, count, height) = pair[0];
        let (upper, next_count, next_height) = pair[1];
        if total <= upper {
            let fraction = (((total + 1) as f64).ln() - ((lower + 1) as f64).ln())
                / (((upper + 1) as f64).ln() - ((lower + 1) as f64).ln());
            building_count = count + ((next_count - count) as f64 * fraction) as usize;
            max_height = height + ((next_height - height) as f64 * fraction) as i32;
            break;
        }
    }
    let weeks = weeks(snapshot);
    let mut longest = 0;
    let mut run = 0;
    let mut previous = None;
    for (&week, &count) in &weeks {
        if count == 0 {
            continue;
        }
        run = if previous == Some(week - Duration::days(7)) {
            run + 1
        } else {
            1
        };
        longest = longest.max(run);
        previous = Some(week)
    }
    let mut week = monday(snapshot.observed_on);
    if weeks.get(&week).copied().unwrap_or(0) == 0 {
        week -= Duration::days(7)
    }
    let mut current = 0;
    while weeks.get(&week).copied().unwrap_or(0) > 0 {
        current += 1;
        week -= Duration::days(7)
    }
    Development {
        stage: STAGES
            .iter()
            .rposition(|(_, threshold)| total >= *threshold)
            .unwrap(),
        building_count,
        max_height,
        population: snapshot.recent(365),
        current_streak: current,
        upkeep: if current > 0 {
            current.min(7)
        } else {
            weeks
                .iter()
                .rev()
                .find(|(_, count)| **count > 0)
                .map_or(0, |(last, _)| {
                    let elapsed = (snapshot.observed_on - (*last + Duration::days(14)))
                        .num_days()
                        .max(0);
                    (7 * (28 - elapsed).max(0) / 28) as usize
                })
        },
        longest_streak: longest,
        recent: snapshot.recent(7),
    }
}
