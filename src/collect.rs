//! GitHub commit collection behind a mockable transport.
use crate::snapshot::{Language, Snapshot};
use chrono::{DateTime, Duration, NaiveDate};
use serde_json::Value;
use std::{collections::BTreeMap, fmt, process::Command};
/// Collection failures include a repair action.
#[derive(Debug)]
pub enum CollectionError {
    /// GitHub CLI or transport failure.
    Transport(String),
    /// Missing account.
    AccountNotFound(String),
    /// Incomplete or malformed response.
    Response(String),
}
impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(message) => write!(
                f,
                "GitHub request failed: {message}; install gh, run gh auth login, and retry"
            ),
            Self::AccountNotFound(login) => {
                write!(f, "account {login} not found; check the login and token")
            }
            Self::Response(message) => write!(
                f,
                "invalid GitHub response: {message}; retry collection or upgrade {}",
                crate::DISPLAY_NAME
            ),
        }
    }
}
impl std::error::Error for CollectionError {}
/// Runs a GraphQL query and returns its data object.
pub trait GraphqlClient {
    /// Send string variables without numeric coercion.
    fn run_query(&self, query: &str, variables: &[(&str, &str)]) -> Result<Value, CollectionError>;
}
/// Uses gh's authenticated token, including GH_TOKEN.
pub struct GhCli;
impl GraphqlClient for GhCli {
    fn run_query(&self, query: &str, variables: &[(&str, &str)]) -> Result<Value, CollectionError> {
        let mut command = Command::new("gh");
        command
            .args(["api", "graphql", "-f"])
            .arg(format!("query={query}"));
        for (name, value) in variables {
            command.arg("-f").arg(format!("{name}={value}"));
        }
        let output = command
            .output()
            .map_err(|e| CollectionError::Transport(e.to_string()))?;
        let parsed: Result<Value, _> = serde_json::from_slice(&output.stdout);
        // GraphQL may return a nonzero gh exit status alongside a null user; preserve the account-not-found distinction.
        if let Ok(response) = &parsed {
            if response
                .get("data")
                .and_then(|data| data.get("user"))
                .is_some_and(Value::is_null)
            {
                return Ok(response["data"].clone());
            }
        }
        if !output.status.success() {
            return Err(CollectionError::Transport(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        let response = parsed.map_err(|error| CollectionError::Response(error.to_string()))?;
        if response.get("errors").is_some() {
            return Err(CollectionError::Response(response["errors"].to_string()));
        }
        Ok(response["data"].clone())
    }
}
const ACCOUNT_QUERY: &str = "query($login:String!){user(login:$login){databaseId createdAt}}";
const COMMITS_QUERY: &str = "query($login:String!,$from:DateTime!,$to:DateTime!){user(login:$login){contributionsCollection(from:$from,to:$to){commitContributionsByRepository(maxRepositories:100){repository{visibility primaryLanguage{name color}} contributions(first:100){pageInfo{hasNextPage} nodes{occurredAt commitCount}}}}}}";
fn field<'a>(value: &'a Value, name: &str) -> Result<&'a Value, CollectionError> {
    value
        .get(name)
        .ok_or_else(|| CollectionError::Response(format!("missing {name}")))
}
fn number(value: &Value, name: &str) -> Result<u64, CollectionError> {
    field(value, name)?
        .as_u64()
        .ok_or_else(|| CollectionError::Response(format!("invalid {name}")))
}
fn string<'a>(value: &'a Value, name: &str) -> Result<&'a str, CollectionError> {
    field(value, name)?
        .as_str()
        .ok_or_else(|| CollectionError::Response(format!("invalid {name}")))
}
/// Collect inclusive, nonoverlapping windows of at most ninety complete UTC days.
pub fn collect_account_snapshot(
    client: &impl GraphqlClient,
    login: &str,
    today: NaiveDate,
) -> Result<Snapshot, CollectionError> {
    if login.contains('/') {
        return Err(CollectionError::Response(format!(
            "{} draws one person's account; supply a login",
            crate::DISPLAY_NAME
        )));
    }
    let data = client.run_query(ACCOUNT_QUERY, &[("login", login)])?;
    let user = field(&data, "user")?;
    if user.is_null() {
        return Err(CollectionError::AccountNotFound(login.into()));
    }
    let created_on = DateTime::parse_from_rfc3339(string(user, "createdAt")?)
        .map_err(|e| CollectionError::Response(e.to_string()))?
        .date_naive();
    let observed_on = today - Duration::days(1);
    let mut snapshot = Snapshot {
        schema_version: 1,
        account_id: number(user, "databaseId")?,
        login: login.into(),
        created_on,
        observed_on,
        total_commits: 0,
        daily_commits: BTreeMap::new(),
        languages: Vec::new(),
        detail_truncated: false,
    };
    let mut languages = BTreeMap::<String, Language>::new();
    let mut language_days = BTreeMap::<String, BTreeMap<NaiveDate, u64>>::new();
    let mut windows = Vec::new();
    let mut start = created_on;
    while start <= observed_on {
        let end = (start + Duration::days(89)).min(observed_on);
        windows.push((start, end));
        start = end + Duration::days(1);
    }
    while let Some((start, end)) = windows.pop() {
        let from = format!("{start}T00:00:00Z");
        let to = format!("{end}T23:59:59Z");
        let data = client.run_query(
            COMMITS_QUERY,
            &[("login", login), ("from", &from), ("to", &to)],
        )?;
        if data["user"].is_null() {
            return Err(CollectionError::AccountNotFound(login.into()));
        }
        let collection = field(field(&data, "user")?, "contributionsCollection")?;
        let repositories = field(collection, "commitContributionsByRepository")?
            .as_array()
            .ok_or_else(|| CollectionError::Response("invalid repository detail".into()))?;
        for repository in repositories {
            if field(
                field(field(repository, "contributions")?, "pageInfo")?,
                "hasNextPage",
            )?
            .as_bool()
                != Some(false)
            {
                return Err(CollectionError::Response("unexpected hasNextPage in a 90-day window; collection cannot safely skip pagination".into()));
            }
        }
        if repositories.len() >= 100 {
            if start < end {
                let middle = start + Duration::days((end - start).num_days() / 2);
                windows.push((middle + Duration::days(1), end));
                windows.push((start, middle));
                continue;
            }
            snapshot.detail_truncated = true;
        }
        for repository in repositories {
            // Filter before reading any counts or language: tokens may see private work.
            if string(field(repository, "repository")?, "visibility")? != "PUBLIC" {
                continue;
            }
            let contributions = field(repository, "contributions")?;
            let mut commits = 0u64;
            let mut days = Vec::new();
            for node in field(contributions, "nodes")?
                .as_array()
                .ok_or_else(|| CollectionError::Response("invalid nodes".into()))?
            {
                let timestamp = DateTime::parse_from_rfc3339(string(node, "occurredAt")?)
                    .map_err(|e| CollectionError::Response(e.to_string()))?;
                // Verified live on 2026-11-01: occurredAt was fixed at 07:00:00Z for accounts in both the US and China (Pacific-time midnight, likely 08:00 in winter). The UTC timestamp date is the calendar day, and no daily commit count exceeded the contribution calendar count for that date.
                let day = timestamp.date_naive();
                if day < start || day > end {
                    return Err(CollectionError::Response(
                        "commit date outside requested window".into(),
                    ));
                }
                let count = number(node, "commitCount")?;
                days.push((day, count));
                let entry = snapshot.daily_commits.entry(day).or_default();
                *entry = entry
                    .checked_add(count)
                    .ok_or_else(|| CollectionError::Response("daily overflow".into()))?;
                commits = commits
                    .checked_add(count)
                    .ok_or_else(|| CollectionError::Response("language overflow".into()))?;
            }
            snapshot.total_commits = snapshot
                .total_commits
                .checked_add(commits)
                .ok_or_else(|| CollectionError::Response("total overflow".into()))?;
            let language = field(field(repository, "repository")?, "primaryLanguage")?;
            if !language.is_null() {
                let name = string(language, "name")?.to_owned();
                let color = field(language, "color")?.as_str().map(str::to_owned);
                let entry = languages.entry(name.clone()).or_insert(Language {
                    name,
                    color,
                    commits: 0,
                    arm_earned_on: None,
                });
                let daily = language_days.entry(entry.name.clone()).or_default();
                for (day, count) in days {
                    let total = daily.entry(day).or_default();
                    *total = total.checked_add(count).ok_or_else(|| {
                        CollectionError::Response("language daily overflow".into())
                    })?;
                }
                entry.commits = entry
                    .commits
                    .checked_add(commits)
                    .ok_or_else(|| CollectionError::Response("language overflow".into()))?;
            }
        }
    }
    for language in languages.values_mut() {
        let mut cumulative = 0u64;
        language.arm_earned_on = language_days[&language.name]
            .iter()
            .find_map(|(&day, &count)| {
                cumulative = cumulative.saturating_add(count);
                (cumulative >= 25).then_some(day)
            });
    }
    snapshot.languages = languages.into_values().collect();
    snapshot.validate().map_err(CollectionError::Response)?;
    Ok(snapshot)
}
