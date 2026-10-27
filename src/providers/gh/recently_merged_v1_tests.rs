use super::*;
use crate::providers::gh::{inbox_rollup_args_v1, ReviewerRoleV1};
use crate::test_support::{Fixture, GhStep};
use serde_json::json;
use std::fs;

fn provider(fixture: &Fixture) -> GithubProvider {
    GithubProvider::new("github.invalid")
        .unwrap()
        .with_program(fixture.root.join("bin/gh"))
        .with_env("GH_FIXTURE_DIR", fixture.root.join("gh-fixture"))
}

fn identity(account: Option<u64>) -> HostIdentity {
    HostIdentity {
        host: "github.invalid".into(),
        login: "octocat".into(),
        account_id: account,
        gh_version: "2.99.0".into(),
    }
}

fn item(repository: &str, number: u64, merged_at: Option<&str>) -> serde_json::Value {
    json!({
        "number": number,
        "node_id": format!("PR_{number}"),
        "title": format!("Change {number}"),
        "body": "",
        "html_url": format!("https://github.invalid/{repository}/pull/{number}"),
        "repository_url": format!("https://api.github.invalid/repos/{repository}"),
        "user": {"login": "alice", "type": "User"},
        "state": "closed",
        "draft": false,
        "created_at": "2026-10-20T00:00:00Z",
        "updated_at": "2026-10-26T00:00:00Z",
        "pull_request": {"merged_at": merged_at},
    })
}

#[test]
fn recently_merged_v1_query_and_args_are_exact() {
    // 2026-10-27T12:00:00Z minus seven days.
    assert_eq!(recently_merged_since_v1(1_793_102_400), "2026-10-20");
    assert_eq!(
        recently_merged_query_v1("octocat", "2026-10-20", &[]),
        "is:pr is:merged involves:octocat merged:>=2026-10-20"
    );
    assert_eq!(
        recently_merged_query_v1(
            "octocat",
            "2026-10-20",
            &["org/a".to_owned(), "org/b".to_owned()]
        ),
        "is:pr is:merged involves:octocat merged:>=2026-10-20 repo:org/a repo:org/b"
    );
    assert_eq!(
        recently_merged_args_v1("github.invalid", "q1"),
        [
            "api",
            "search/issues",
            "--hostname",
            "github.invalid",
            "--method",
            "GET",
            "-f",
            "q=q1",
            "-f",
            "sort=updated",
            "-f",
            "order=desc",
            "-f",
            "per_page=50"
        ]
    );
}

#[test]
fn recently_merged_v1_keeps_only_merged_items_newest_merge_first() {
    let fixture = Fixture::new();
    let response = json!({"total_count": 3, "items": [
        item("org/a", 1, Some("2026-10-25T10:00:00Z")),
        item("org/a", 2, None),
        item("org/b", 3, Some("2026-10-26T10:00:00Z")),
    ]});
    let merged = provider(&fixture)
        .parse_recently_merged_v1(&response)
        .unwrap();
    let seen: Vec<(&str, u64, &str, &str)> = merged
        .iter()
        .map(|m| {
            (
                m.pr.repository.as_str(),
                m.pr.number,
                m.pr.state.as_str(),
                m.merged_at.as_str(),
            )
        })
        .collect();
    assert_eq!(
        seen,
        vec![
            ("org/b", 3, "merged", "2026-10-26T10:00:00Z"),
            ("org/a", 1, "merged", "2026-10-25T10:00:00Z"),
        ]
    );
    assert!(provider(&fixture)
        .parse_recently_merged_v1(&json!({"message": "no"}))
        .is_err());
}

#[test]
fn recently_merged_v1_workspace_load_reads_search_then_reviewers_only_for_merged() {
    use crate::github_workspace::{GithubWorkspace, InboxFilter, PrKey};
    let fixture = Fixture::new();
    let since = recently_merged_since_v1(crate::storage::now());
    let query = recently_merged_query_v1("octocat", &since, &[]);
    let search = recently_merged_args_v1("github.invalid", &query);
    let search: Vec<&str> = search.iter().map(String::as_str).collect();
    let rollup = inbox_rollup_args_v1("github.invalid", &[("org/a", 1)]);
    let rollup: Vec<&str> = rollup.iter().map(String::as_str).collect();
    let response =
        json!({"total_count": 1, "items": [item("org/a", 1, Some("2026-10-26T10:00:00Z"))]});
    fixture.scripted_gh(&[
        GhStep::read(&search, &response.to_string()),
        GhStep::read(
            &rollup,
            r#"{"data":{"p0":{"pullRequest":{"reviewDecision":"APPROVED","commits":{"nodes":[]},"latestReviews":{"nodes":[{"state":"APPROVED","author":{"__typename":"Bot","login":"coderabbitai"}}]}}}}}"#,
        ),
    ]);
    let mut github = GithubWorkspace::empty(identity(Some(42)), vec![], Default::default());
    github.set_filter(InboxFilter::Merged);
    let cancel = CancellationToken::default();
    let github = github
        .load_recently_merged_v1(&provider(&fixture), &cancel)
        .unwrap();
    let journal = fs::read_to_string(fixture.root.join("gh-fixture/commands.jsonl")).unwrap();
    assert_eq!(journal.lines().count(), 2, "{journal}");
    assert!(!journal.contains("\"matched\":false"), "{journal}");
    assert_eq!(github.merged_error_v1, None);
    let visible = github.visible();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].state, "merged");
    assert_eq!(
        github
            .merged_at_v1
            .get(&PrKey::from(visible[0]))
            .map(String::as_str),
        Some("2026-10-26T10:00:00Z")
    );
    let key = crate::inbox_status_v1::row_key("github.invalid", "org/a", 1);
    let reviewers = &github.inbox_reviewers_v1[&key];
    assert_eq!(reviewers[0].actor.login, "coderabbitai");
    assert_eq!(reviewers[0].role, ReviewerRoleV1::Approved);
    // The open queues are untouched and do not count merged PRs.
    assert_eq!(github.queue_counts(), [0, 0, 0, 0]);
    assert!(github.merged_summary_v1().contains("1 involving you"));
}

#[test]
fn recently_merged_v1_workspace_load_sends_nothing_without_account_and_reports_failures() {
    use crate::github_workspace::GithubWorkspace;
    let fixture = Fixture::new();
    fixture.scripted_gh(&[]);
    let cancel = CancellationToken::default();
    let github = GithubWorkspace::empty(identity(None), vec![], Default::default())
        .load_recently_merged_v1(&provider(&fixture), &cancel)
        .unwrap();
    assert!(github.merged_v1.is_none());
    assert!(github.merged_error_v1.is_some());
    let journal =
        fs::read_to_string(fixture.root.join("gh-fixture/commands.jsonl")).unwrap_or_default();
    assert_eq!(journal.lines().count(), 0, "{journal}");

    // An unscripted (failed) read keeps no list and says why instead of "none merged".
    let github = GithubWorkspace::empty(identity(Some(42)), vec![], Default::default())
        .load_recently_merged_v1(&provider(&fixture), &cancel)
        .unwrap();
    assert!(github.merged_v1.is_none());
    assert!(github
        .merged_error_v1
        .as_deref()
        .is_some_and(|error| error.starts_with("Merged pull requests unavailable")));
}
