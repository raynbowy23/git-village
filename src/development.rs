//! Commit-derived growth and weekly upkeep.
use crate::snapshot::{Snapshot, monday};
use chrono::Duration;
use std::collections::BTreeMap;
/// Inclusive growth thresholds.
pub const STAGES: [(&str, u64); 6] = [
    ("Outpost", 0),
    ("Hamlet", 100),
    ("Village", 1500),
    ("Town", 5000),
    ("City", 20000),
    ("Metropolis", 100000),
];
/// Visual signals derived from commits.
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
/// Derive all traits without a clock.
pub fn derive(snapshot: &Snapshot) -> Development {
    let total = snapshot.total_commits;
    // Log interpolation preserves diminishing height growth within each stage, and past the last anchor the same slope continues.
    let anchors = [
        (0, 5),
        (100, 7),
        (1500, 12),
        (5000, 22),
        (20000, 43),
        (100000, 74),
        (500000, 80),
    ];
    let log = |commits: u64| ((commits as f64) + 1.0).ln();
    let (lower, height, upper, next_height) = anchors
        .windows(2)
        .find(|pair| total <= pair[1].0)
        .map_or((100000, 74, 500000, 80), |pair| {
            (pair[0].0, pair[0].1, pair[1].0, pair[1].1)
        });
    let fraction = (log(total) - log(lower)) / (log(upper) - log(lower));
    let max_height = height + (f64::from(next_height - height) * fraction) as i32;
    // Deliberately slow: 1,000 commits is still a handful of buildings.
    let building_count = ((total as f64 / 60.0).powf(0.7).round() as usize).max(1);
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
