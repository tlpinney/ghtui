//! The Inbox Merged tab: pull requests merged in the last [`RECENTLY_MERGED_DAYS_V1`] days
//! that involve the signed-in user (authored, assigned, reviewed, commented or mentioned).
//!
//! One read-only `search/issues` call, sent only when the tab is chosen or refreshed. The
//! ordinary Inbox load never sends it, so the open queues' reads stay exactly as they were.
use super::{
    error, validate_name, validate_repo, GithubErrorKind, GithubProvider, GithubResult,
    HostIdentity, PullRequest,
};
use crate::providers::runner::CancellationToken;
use serde_json::Value;

/// How far back the Merged tab looks.
pub const RECENTLY_MERGED_DAYS_V1: u64 = 7;
/// Most merged PRs one read lists (one search page).
pub const RECENTLY_MERGED_LIMIT_V1: usize = 50;

/// One merged pull request and when it merged (GitHub's `merged_at`, UTC).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedPrV1 {
    pub pr: PullRequest,
    pub merged_at: String,
}

/// `YYYY-MM-DD` for `RECENTLY_MERGED_DAYS_V1` days before `now` (Unix seconds, UTC).
pub fn recently_merged_since_v1(now: u64) -> String {
    let since = now.saturating_sub(RECENTLY_MERGED_DAYS_V1 * 86_400);
    crate::metrics_workspace::utc_label(since)
        .get(..10)
        .unwrap_or("1970-01-01")
        .to_owned()
}

/// The search query; a repository scope narrows it with one `repo:` per repository.
pub fn recently_merged_query_v1(login: &str, since: &str, repositories: &[String]) -> String {
    let mut query = format!("is:pr is:merged involves:{login} merged:>={since}");
    for repository in repositories {
        query.push_str(&format!(" repo:{repository}"));
    }
    query
}

/// The exact `gh` argv for the read.
pub fn recently_merged_args_v1(host: &str, query: &str) -> Vec<String> {
    vec![
        "api".into(),
        "search/issues".into(),
        "--hostname".into(),
        host.into(),
        "--method".into(),
        "GET".into(),
        "-f".into(),
        format!("q={query}"),
        "-f".into(),
        "sort=updated".into(),
        "-f".into(),
        "order=desc".into(),
        "-f".into(),
        format!("per_page={RECENTLY_MERGED_LIMIT_V1}"),
    ]
}

impl GithubProvider {
    /// Merged PRs involving `identity` since `since`, newest merge first. Read-only.
    pub fn recently_merged_v1(
        &self,
        identity: &HostIdentity,
        repositories: &[String],
        since: &str,
        cancel: &CancellationToken,
    ) -> GithubResult<Vec<MergedPrV1>> {
        validate_name(&identity.login)?;
        for repository in repositories {
            validate_repo(repository)?;
        }
        let query = recently_merged_query_v1(&identity.login, since, repositories);
        let bytes = self.execute(
            recently_merged_args_v1(&self.host, &query),
            None,
            None,
            cancel,
        )?;
        let response: Value = serde_json::from_slice(&bytes)
            .map_err(|e| error(GithubErrorKind::Json, e.to_string()))?;
        self.parse_recently_merged_v1(&response)
    }

    /// Search items with a `pull_request.merged_at`; anything else (an unmerged close, an
    /// issue) is left out rather than shown as merged.
    pub fn parse_recently_merged_v1(&self, response: &Value) -> GithubResult<Vec<MergedPrV1>> {
        let items = response
            .get("items")
            .and_then(Value::as_array)
            .ok_or_else(|| error(GithubErrorKind::Json, "Search response has no items"))?;
        let mut merged = Vec::new();
        for item in items {
            let Some(merged_at) = item
                .pointer("/pull_request/merged_at")
                .and_then(Value::as_str)
                .filter(|at| !at.is_empty())
            else {
                continue;
            };
            let mut pr = self.pr_from_json(item, None)?;
            pr.state = "merged".into();
            merged.push(MergedPrV1 {
                pr,
                merged_at: merged_at.to_owned(),
            });
        }
        merged.sort_by(|a, b| b.merged_at.cmp(&a.merged_at));
        Ok(merged)
    }
}

#[cfg(test)]
#[path = "recently_merged_v1_tests.rs"]
mod tests;
