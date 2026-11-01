use chrono::{Duration, NaiveDate};
use git_village::snapshot::{Language, Snapshot};
/// Synthetic observations for tests and the review example.
pub fn synthetic(total: u64) -> Snapshot {
    let observed_on = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
    let mut remaining = total;
    let daily_commits = (0..420)
        .filter_map(|offset| {
            let count = remaining.min(if offset % 4 == 0 { 0 } else { 3 });
            remaining -= count;
            (count > 0).then_some((observed_on - Duration::days(offset), count))
        })
        .collect();
    Snapshot {
        schema_version: 1,
        account_id: 42,
        login: "pixel-city".into(),
        created_on: observed_on - Duration::days(2000),
        observed_on,
        total_commits: total,
        daily_commits,
        languages: [
            ("Rust", "#dea584"),
            ("Python", "#3572a5"),
            ("TypeScript", "#3178c6"),
            ("Go", "#00add8"),
            ("Ruby", "#701516"),
        ]
        .into_iter()
        .map(|(name, color)| Language {
            arm_earned_on: (total / 5 >= 25).then(|| "2022-01-01".parse().unwrap()),
            name: name.into(),
            color: Some(color.into()),
            commits: total / 5,
        })
        .collect(),
        detail_truncated: false,
    }
}
