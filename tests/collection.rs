#[path = "fixtures/support.rs"]
mod support;
use chrono::{Duration, NaiveDate};
use git_village::{
    collect::{CollectionError, GraphqlClient, collect_account_snapshot},
    snapshot::Snapshot,
};
use serde_json::{Value, json};
use std::cell::RefCell;
use support::synthetic;
struct Mock {
    responses: RefCell<Vec<Value>>,
    variables: RefCell<Vec<Vec<(String, String)>>>,
}
impl GraphqlClient for Mock {
    fn run_query(&self, query: &str, variables: &[(&str, &str)]) -> Result<Value, CollectionError> {
        assert!(!query.contains("totalCommitContributions"));
        assert!(!query.contains("restrictedContributionsCount"));
        if variables.len() > 1 {
            assert!(query.contains("visibility"));
        }
        self.variables.borrow_mut().push(
            variables
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
        );
        Ok(self.responses.borrow_mut().remove(0))
    }
}
fn client(created: &str, windows: Vec<Value>) -> Mock {
    let mut responses =
        vec![json!({"user":{"databaseId":42,"createdAt":format!("{created}T00:00:00Z")}})];
    responses.extend(windows);
    Mock {
        responses: RefCell::new(responses),
        variables: RefCell::new(Vec::new()),
    }
}
fn date(value: &str) -> NaiveDate {
    value.parse().unwrap()
}
fn window(repositories: Vec<Value>) -> Value {
    json!({"user":{"contributionsCollection":{"commitContributionsByRepository":repositories}}})
}
fn repository(day: &str, count: u64) -> Value {
    json!({"repository":{"visibility":"PUBLIC","primaryLanguage":{"name":"Rust","color":null}},"contributions":{"pageInfo":{"hasNextPage":false},"nodes":[{"occurredAt":format!("{day}T07:00:00Z"),"commitCount":count}]}})
}
#[test]
fn attributes_dates_languages_nulls_and_excludes_all_nonpublic_work() {
    let fixture = serde_json::from_str(include_str!("fixtures/window.json")).unwrap();
    let snapshot = collect_account_snapshot(
        &client("2026-10-01", vec![fixture]),
        "person",
        date("2026-11-01"),
    )
    .unwrap();
    assert_eq!(snapshot.total_commits, 7);
    assert_eq!(snapshot.daily_commits[&date("2026-10-21")], 7);
    assert_eq!(snapshot.languages.len(), 1);
    assert_eq!(snapshot.languages[0].commits, 5);
    assert_eq!(snapshot.languages[0].color, None);
    let serialized = serde_json::to_string(&snapshot).unwrap();
    assert!(!serialized.contains("repository"));
    assert!(!serialized.contains("synthetic-public-only"));
    assert!(!serialized.contains("Excluded"));
    assert_eq!(
        Snapshot::from_json(serialized.as_bytes())
            .unwrap()
            .total_commits,
        7
    );
}
#[test]
fn windows_are_ninety_days_or_less_and_do_not_overlap() {
    let empty = window(vec![]);
    let mock = client("2026-01-01", vec![empty.clone(), empty.clone(), empty]);
    let snapshot = collect_account_snapshot(&mock, "person", date("2026-07-01")).unwrap();
    assert_eq!(snapshot.total_commits, 0);
    let calls = mock.variables.borrow();
    let mut expected = date("2026-01-01");
    let mut windows: Vec<_> = calls.iter().skip(1).collect();
    windows.sort_by_key(|call| &call[1].1);
    for call in windows {
        assert_eq!(call[1].1, format!("{expected}T00:00:00Z"));
        let end = (expected + Duration::days(89)).min(snapshot.observed_on);
        assert_eq!(call[2].1, format!("{end}T23:59:59Z"));
        expected = end + Duration::days(1);
    }
    assert_eq!(expected, date("2026-07-01"));
}
#[test]
fn full_repository_windows_are_split_and_parent_counts_discarded() {
    let full = window(vec![repository("2026-10-01", 999); 100]);
    let mock = client(
        "2026-10-01",
        vec![
            full,
            window(vec![repository("2026-10-01", 2)]),
            window(vec![repository("2026-10-02", 3)]),
        ],
    );
    let snapshot = collect_account_snapshot(&mock, "person", date("2026-10-03")).unwrap();
    assert_eq!(snapshot.total_commits, 5);
    assert!(!snapshot.detail_truncated);
    let calls = mock.variables.borrow();
    assert_eq!(calls[2][2].1, "2026-10-01T23:59:59Z");
    assert_eq!(calls[3][1].1, "2026-10-02T00:00:00Z");
}
#[test]
fn single_day_at_repository_cap_is_explicitly_truncated() {
    let mock = client(
        "2026-10-01",
        vec![window(vec![repository("2026-10-01", 1); 100])],
    );
    let snapshot = collect_account_snapshot(&mock, "person", date("2026-10-02")).unwrap();
    assert!(snapshot.detail_truncated);
    assert_eq!(snapshot.total_commits, 100);
}
#[test]
fn paging_unknown_login_and_repository_subject_fail_with_repairs() {
    let mut repo = repository("2026-10-01", 1);
    repo["contributions"]["pageInfo"]["hasNextPage"] = json!(true);
    let error = collect_account_snapshot(
        &client("2026-10-01", vec![window(vec![repo])]),
        "person",
        date("2026-10-02"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("hasNextPage"));
    let mock = Mock {
        responses: RefCell::new(vec![json!({"user":null})]),
        variables: RefCell::new(vec![]),
    };
    assert!(matches!(
        collect_account_snapshot(&mock, "missing", date("2026-11-01")),
        Err(CollectionError::AccountNotFound(_))
    ));
    assert!(
        collect_account_snapshot(&mock, "owner/repo", date("2026-11-01"))
            .unwrap_err()
            .to_string()
            .contains("one person's account")
    );
}
#[test]
fn snapshot_schema_and_count_validation() {
    assert!(
        Snapshot::from_json(br#"{"schema_version":9}"#)
            .unwrap_err()
            .contains("upgrade")
    );
    let mut snapshot = synthetic(0);
    snapshot.daily_commits.insert(snapshot.observed_on, 1);
    assert!(snapshot.validate().is_err());
    snapshot.detail_truncated = true;
    assert!(snapshot.validate().is_ok());
}

#[test]
fn account_created_today_has_no_complete_day_and_no_requests_for_commits() {
    let mock = client("2026-11-01", vec![]);
    let snapshot = collect_account_snapshot(&mock, "new-person", date("2026-11-01")).unwrap();
    assert_eq!(snapshot.total_commits, 0);
    assert!(snapshot.languages.is_empty());
    assert_eq!(mock.variables.borrow().len(), 1);
    assert_eq!(snapshot.observed_on, date("2026-10-31"));
}

#[test]
fn arm_threshold_uses_sorted_daily_language_counts_including_split_windows() {
    let mut repo = repository("2026-10-01", 10);
    repo["contributions"]["nodes"] = json!([
        {"occurredAt":"2026-10-03T07:00:00Z","commitCount":20},
        {"occurredAt":"2026-10-01T07:00:00Z","commitCount":10},
        {"occurredAt":"2026-10-02T07:00:00Z","commitCount":15}
    ]);
    let snapshot = collect_account_snapshot(
        &client("2026-10-01", vec![window(vec![repo])]),
        "person",
        date("2026-10-04"),
    )
    .unwrap();
    assert_eq!(
        snapshot.languages[0].arm_earned_on,
        Some(date("2026-10-02"))
    );
    let full = window(vec![repository("2026-10-01", 999); 100]);
    let snapshot = collect_account_snapshot(
        &client(
            "2026-10-01",
            vec![
                full,
                window(vec![repository("2026-10-01", 24)]),
                window(vec![repository("2026-10-02", 1)]),
            ],
        ),
        "person",
        date("2026-10-03"),
    )
    .unwrap();
    assert_eq!(
        snapshot.languages[0].arm_earned_on,
        Some(date("2026-10-02"))
    );
    let snapshot = collect_account_snapshot(
        &client(
            "2026-10-01",
            vec![window(vec![repository("2026-10-01", 24)])],
        ),
        "person",
        date("2026-10-02"),
    )
    .unwrap();
    assert_eq!(snapshot.languages[0].arm_earned_on, None);
}

#[test]
fn language_threshold_survives_newest_window_fetched_first() {
    let mock = client(
        "2026-01-01",
        vec![
            window(vec![repository("2026-04-01", 10)]),
            window(vec![repository("2026-02-01", 24)]),
        ],
    );
    let snapshot = collect_account_snapshot(&mock, "person", date("2026-04-02")).unwrap();
    assert_eq!(mock.variables.borrow()[1][1].1, "2026-04-01T00:00:00Z");
    assert_eq!(
        snapshot.languages[0].arm_earned_on,
        Some(date("2026-04-01"))
    );
}
