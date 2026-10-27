use super::*;
use crate::test_support::{Fixture, GhStep};
use serde_json::json;
use std::fs;

fn provider(fixture: &Fixture) -> GithubProvider {
    GithubProvider::new("github.invalid")
        .unwrap()
        .with_program(fixture.root.join("bin/gh"))
        .with_env("GH_FIXTURE_DIR", fixture.root.join("gh-fixture"))
}

#[test]
fn inbox_rollup_v1_query_is_exact_and_ordered() {
    assert_eq!(
        inbox_rollup_query_v1(&[("org/alpha", 7), ("org/beta", 9)]),
        "query{p0:repository(owner:\"org\",name:\"alpha\"){pullRequest(number:7){reviewDecision commits(last:1){nodes{commit{oid statusCheckRollup{state}}}} reviewRequests(first:10){nodes{requestedReviewer{__typename ...on Actor{login} ...on Team{combinedSlug}}}} latestReviews(first:10){nodes{state author{__typename login}}} comments(last:20){nodes{author{__typename login}}}}} p1:repository(owner:\"org\",name:\"beta\"){pullRequest(number:9){reviewDecision commits(last:1){nodes{commit{oid statusCheckRollup{state}}}} reviewRequests(first:10){nodes{requestedReviewer{__typename ...on Actor{login} ...on Team{combinedSlug}}}} latestReviews(first:10){nodes{state author{__typename login}}} comments(last:20){nodes{author{__typename login}}}}}}"
    );
    let args = inbox_rollup_args_v1("github.invalid", &[("org/alpha", 7)]);
    assert_eq!(
        &args[..5],
        &["api", "graphql", "--hostname", "github.invalid", "-f"]
    );
    assert!(args[5].starts_with("query=query{p0:repository("));
    let mut targets = vec![
        ("org/Beta".to_owned(), 2),
        ("org/alpha".to_owned(), 9),
        ("org/beta".to_owned(), 2),
        ("org/alpha".to_owned(), 7),
    ];
    inbox_rollup_order_v1(&mut targets);
    assert_eq!(
        targets,
        vec![
            ("org/alpha".to_owned(), 7),
            ("org/alpha".to_owned(), 9),
            ("org/Beta".to_owned(), 2)
        ]
    );
}

#[test]
fn inbox_rollup_v1_parses_states_and_skips_missing_prs() {
    let targets = [("org/alpha", 7), ("org/beta", 7), ("org/gamma", 9)];
    let response = json!({"data":{
        "p0":{"pullRequest":{"reviewDecision":"CHANGES_REQUESTED","commits":{"nodes":[{"commit":{"oid":"abc","statusCheckRollup":{"state":"SUCCESS"}}}]}}},
        "p1":null,
        "p2":{"pullRequest":{"reviewDecision":null,"commits":{"nodes":[{"commit":{"oid":"def","statusCheckRollup":null}}]}}}
    }});
    let rows = parse_inbox_rollup_v1(&targets, &response).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].repository, "org/alpha");
    assert_eq!(
        rows[0].review_decision.as_deref(),
        Some("CHANGES_REQUESTED")
    );
    assert_eq!(rows[0].check_state.as_deref(), Some("SUCCESS"));
    assert_eq!(rows[0].head_oid, "abc");
    assert_eq!(rows[1].repository, "org/gamma");
    assert_eq!(rows[1].review_decision, None);
    assert_eq!(rows[1].check_state, None);
    assert!(parse_inbox_rollup_v1(&targets, &json!({"errors":[{"message":"no"}]})).is_err());
}

#[test]
fn inbox_rollup_v1_lists_reviewers_requests_and_commenters_once_each() {
    let targets = [("org/alpha", 7), ("org/beta", 8)];
    let response = json!({"data":{
        "p0":{"pullRequest":{"reviewDecision":"CHANGES_REQUESTED",
            "commits":{"nodes":[{"commit":{"oid":"abc","statusCheckRollup":null}}]},
            "latestReviews":{"nodes":[
                {"state":"APPROVED","author":{"__typename":"User","login":"alice"}},
                {"state":"COMMENTED","author":{"__typename":"Bot","login":"coderabbitai"}},
                {"state":"CHANGES_REQUESTED","author":{"__typename":"User","login":"carol"}},
                {"state":"PENDING","author":{"__typename":"User","login":"viewer"}},
                {"state":"APPROVED","author":null}
            ]},
            "reviewRequests":{"nodes":[
                {"requestedReviewer":{"__typename":"User","login":"bob"}},
                {"requestedReviewer":{"__typename":"User","login":"Alice"}},
                {"requestedReviewer":{"__typename":"Team","combinedSlug":"org/core"}},
                {"requestedReviewer":null}
            ]},
            "comments":{"nodes":[
                {"author":{"__typename":"Bot","login":"coderabbitai"}},
                {"author":{"__typename":"User","login":"dave"}},
                {"author":{"__typename":"User","login":"dave"}},
                {"author":null}
            ]}}},
        "p1":{"pullRequest":{"reviewDecision":null,
            "commits":{"nodes":[{"commit":{"oid":"def","statusCheckRollup":null}}]}}}
    }});
    let rows = parse_inbox_rollup_v1(&targets, &response).unwrap();
    let seen: Vec<(&str, &str, ReviewerRoleV1)> = rows[0]
        .reviewers
        .as_ref()
        .unwrap()
        .iter()
        .map(|r| (r.actor.login.as_str(), r.actor.actor_type.as_str(), r.role))
        .collect();
    assert_eq!(
        seen,
        vec![
            ("carol", "User", ReviewerRoleV1::ChangesRequested),
            ("alice", "User", ReviewerRoleV1::Approved),
            ("coderabbitai", "Bot", ReviewerRoleV1::Reviewed),
            ("bob", "User", ReviewerRoleV1::Requested),
            ("org/core", "Team", ReviewerRoleV1::Requested),
            ("dave", "User", ReviewerRoleV1::Commented),
        ]
    );
    // A PR answered without the reviewer fields (older fixtures) simply has none.
    assert_eq!(rows[1].reviewers, Some(Vec::new()));
}

#[test]
fn inbox_rollup_v1_falls_back_to_status_only_when_the_server_rejects_reviewer_fields() {
    let fixture = Fixture::new();
    let targets = [("org/alpha", 7)];
    let full = inbox_rollup_args_v1("github.invalid", &targets);
    let full: Vec<&str> = full.iter().map(String::as_str).collect();
    let status = inbox_rollup_status_args_v1("github.invalid", &targets);
    let status: Vec<&str> = status.iter().map(String::as_str).collect();
    // The status-only document is the pre-reviewer query, byte for byte.
    assert_eq!(
        inbox_rollup_status_query_v1(&targets),
        "query{p0:repository(owner:\"org\",name:\"alpha\"){pullRequest(number:7){reviewDecision commits(last:1){nodes{commit{oid statusCheckRollup{state}}}}}}}"
    );
    let mut rejected = GhStep::read(&full, "");
    rejected.status = 1;
    rejected.stderr = b"gh: Fragment on Bot can't be spread inside RequestedReviewer\n".to_vec();
    fixture.scripted_gh(&[
        rejected,
        GhStep::read(
            &status,
            r#"{"data":{"p0":{"pullRequest":{"reviewDecision":"APPROVED","commits":{"nodes":[{"commit":{"oid":"abc","statusCheckRollup":{"state":"SUCCESS"}}}]}}}}}"#,
        ),
    ]);
    let rows = provider(&fixture)
        .inbox_rollup_v1(&targets, &CancellationToken::default())
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].review_decision.as_deref(), Some("APPROVED"));
    assert_eq!(rows[0].check_state.as_deref(), Some("SUCCESS"));
    // Reviewers were not read, which is not the same as "nobody".
    assert_eq!(rows[0].reviewers, None);
    assert_eq!(journal_lines(&fixture), 2);
}

#[test]
fn inbox_rollup_v1_sends_one_strict_read_and_refuses_bad_targets() {
    let fixture = Fixture::new();
    let targets = [("org/alpha", 7)];
    let args = inbox_rollup_args_v1("github.invalid", &targets);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    fixture.scripted_gh(&[GhStep::read(
        &args,
        r#"{"data":{"p0":{"pullRequest":{"reviewDecision":"APPROVED","commits":{"nodes":[{"commit":{"oid":"abc","statusCheckRollup":{"state":"PENDING"}}}]}}}}}"#,
    )]);
    let provider = provider(&fixture);
    let cancel = CancellationToken::default();
    let rows = provider.inbox_rollup_v1(&targets, &cancel).unwrap();
    assert_eq!(rows[0].review_decision.as_deref(), Some("APPROVED"));
    assert_eq!(rows[0].check_state.as_deref(), Some("PENDING"));
    assert!(provider
        .inbox_rollup_v1(&[("bad name", 1)], &cancel)
        .is_err());
    assert!(provider
        .inbox_rollup_v1(&[("org/alpha", 0)], &cancel)
        .is_err());
    let too_many: Vec<(&str, u64)> = (1..=INBOX_ROLLUP_BATCH_V1 as u64 + 1)
        .map(|n| ("org/alpha", n))
        .collect();
    assert!(provider.inbox_rollup_v1(&too_many, &cancel).is_err());
    let journal = fs::read_to_string(fixture.root.join("gh-fixture/commands.jsonl")).unwrap();
    assert_eq!(journal.lines().count(), 1, "refused targets send nothing");
    assert!(journal.contains("\"matched\":true"));
    assert!(journal.contains("\"stdin_hex\":\"\""));
}

fn queue_row(repository: &str, number: u64) -> crate::providers::gh::PullRequest {
    crate::providers::gh::PullRequest {
        host: "github.invalid".into(),
        id: format!("PR_{number}"),
        repository: repository.into(),
        number,
        title: format!("Row {number}"),
        body: String::new(),
        url: String::new(),
        author: Default::default(),
        state: "open".into(),
        draft: false,
        created_at: "2026-10-01T00:00:00Z".into(),
        updated_at: "2026-10-02T00:00:00Z".into(),
        queues: Default::default(),
    }
}

fn loaded(account: Option<u64>, rows: usize) -> crate::github_workspace::GithubWorkspace {
    use crate::github_workspace::{Freshness, GithubWorkspace};
    let identity = crate::providers::gh::HostIdentity {
        host: "github.invalid".into(),
        login: "octocat".into(),
        account_id: account,
        gh_version: "2.99.0".into(),
    };
    let mut github = GithubWorkspace::empty(identity, vec![], Default::default());
    github.inbox.pull_requests = (1..=rows as u64)
        .map(|n| queue_row(if n % 2 == 0 { "org/Beta" } else { "org/alpha" }, n))
        .collect();
    github.freshness = Freshness::Fresh { fetched_at: 1 };
    github
}

fn journal_lines(fixture: &Fixture) -> usize {
    fs::read_to_string(fixture.root.join("gh-fixture/commands.jsonl"))
        .unwrap_or_default()
        .lines()
        .count()
}

#[test]
fn inbox_rollup_v1_workspace_load_batches_every_row_and_keeps_failures_unknown() {
    use crate::inbox_status_v1::{row_key, CheckState, ReviewState};
    // 51 rows: two batches (50 + 1); the second batch fails and its row stays unknown.
    let fixture = Fixture::new();
    let mut github = loaded(Some(42), 51);
    let mut targets: Vec<(String, u64)> = github
        .inbox
        .pull_requests
        .iter()
        .map(|pr| (pr.repository.clone(), pr.number))
        .collect();
    inbox_rollup_order_v1(&mut targets);
    let first: Vec<(&str, u64)> = targets[..50]
        .iter()
        .map(|(r, n)| (r.as_str(), *n))
        .collect();
    let last: Vec<(&str, u64)> = targets[50..]
        .iter()
        .map(|(r, n)| (r.as_str(), *n))
        .collect();
    let mut data = serde_json::Map::new();
    for (index, _) in first.iter().enumerate() {
        data.insert(
            format!("p{index}"),
            json!({"pullRequest":{"reviewDecision":"APPROVED","commits":{"nodes":[{"commit":{"oid":"a","statusCheckRollup":{"state":"FAILURE"}}}]}}}),
        );
    }
    let args = inbox_rollup_args_v1("github.invalid", &first);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let last_args = inbox_rollup_args_v1("github.invalid", &last);
    let last_args: Vec<&str> = last_args.iter().map(String::as_str).collect();
    let mut failed = GhStep::read(&last_args, "");
    failed.status = 1;
    failed.stderr = b"HTTP 502: fixture GraphQL unavailable\n".to_vec();
    fixture.scripted_gh(&[
        GhStep::read(&args, &json!({ "data": data }).to_string()),
        failed,
    ]);
    let cancel = CancellationToken::default();
    github.load_inbox_rollup_v1(&provider(&fixture), &cancel);
    assert_eq!(journal_lines(&fixture), 2);
    assert_eq!(github.inbox_rollup_v1.len(), 50);
    let (missing_repo, missing_number) = last[0];
    assert!(!github.inbox_rollup_v1.contains_key(&row_key(
        "github.invalid",
        missing_repo,
        missing_number
    )));
    let any = github.inbox_rollup_v1.values().next().unwrap();
    assert_eq!(any.review, ReviewState::Approved);
    assert_eq!(any.checks, Some(CheckState::Failing));
    assert_eq!(any.updated_at, "2026-10-02T00:00:00Z");
    // A queue that dropped a row also drops its rollup on the next load.
    github.inbox.pull_requests.truncate(1);
    let only = github.inbox.pull_requests[0].clone();
    let fixture = Fixture::new();
    let args = inbox_rollup_args_v1("github.invalid", &[(only.repository.as_str(), only.number)]);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    fixture.scripted_gh(&[GhStep::read(&args, r#"{"data":{"p0":null}}"#)]);
    github.load_inbox_rollup_v1(&provider(&fixture), &cancel);
    assert_eq!(journal_lines(&fixture), 1);
    assert_eq!(
        github.inbox_rollup_v1.keys().cloned().collect::<Vec<_>>(),
        vec![row_key("github.invalid", &only.repository, only.number)],
        "a null answer keeps the row's older rollup, which reads as stale once updated"
    );
}

#[test]
fn inbox_rollup_v1_workspace_load_sends_nothing_without_account_fresh_rows_or_any_row() {
    use crate::github_workspace::Freshness;
    let cancel = CancellationToken::default();
    for (account, rows, freshness) in [
        (None, 2, Freshness::Fresh { fetched_at: 1 }),
        (Some(42), 0, Freshness::Fresh { fetched_at: 1 }),
        (Some(42), 2, Freshness::Cached { fetched_at: 1 }),
        (
            Some(42),
            2,
            Freshness::Offline {
                fetched_at: 1,
                error: "offline".into(),
            },
        ),
    ] {
        let fixture = Fixture::new();
        fixture.scripted_gh(&[]);
        let mut github = loaded(account, rows);
        github.freshness = freshness;
        github.load_inbox_rollup_v1(&provider(&fixture), &cancel);
        assert_eq!(journal_lines(&fixture), 0);
        assert!(github.inbox_rollup_v1.is_empty());
    }
    // Cancelled before the batch: nothing is sent.
    let fixture = Fixture::new();
    fixture.scripted_gh(&[]);
    let mut github = loaded(Some(42), 2);
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    github.load_inbox_rollup_v1(&provider(&fixture), &cancelled);
    assert_eq!(journal_lines(&fixture), 0);
}
