//! TASK-12 AC2: Inbox rows show the review and check state of PR pages already read, never
//! an invented status, and only to the account that read them.
use super::*;
use crate::app::{App, View};
use crate::github_workspace::{GithubWorkspace, PrTab};
use crate::providers::gh::{Coverage, HostIdentity, PrDetails, PrSummary, Queue};
use ratatui::{backend::TestBackend, layout::Rect, Terminal};
use serde_json::json;
use std::collections::BTreeSet;

fn identity(account: Option<u64>) -> HostIdentity {
    HostIdentity {
        host: "github.invalid".into(),
        login: "current-user".into(),
        account_id: account,
        gh_version: "2.99.0".into(),
    }
}

fn pull_request(number: u64, updated_at: &str) -> PullRequest {
    PullRequest {
        host: "github.invalid".into(),
        id: format!("PR_{number}"),
        repository: "Example/Actual".into(),
        number,
        title: format!("Row {number}"),
        body: String::new(),
        url: String::new(),
        author: Default::default(),
        state: "open".into(),
        draft: false,
        created_at: "2026-10-01T00:00:00Z".into(),
        updated_at: updated_at.into(),
        queues: BTreeSet::from([Queue::Authored]),
    }
}

fn review(login: &str, state: &str) -> Value {
    json!({"state": state, "user": {"login": login}})
}

fn check(status: &str, conclusion: &str) -> Check {
    Check {
        id: 1,
        name: "ci".into(),
        status: status.into(),
        conclusion: conclusion.into(),
        url: String::new(),
        head_sha: "a".repeat(40),
        kind: "check".into(),
        app_id: None,
    }
}

fn page(
    number: u64,
    head: char,
    updated_at: &str,
    reviews: Vec<Value>,
    checks: Option<Vec<Check>>,
) -> PrPage {
    let mut pr = pull_request(number, updated_at);
    // The page's canonical repository may differ in case from the queue's.
    pr.repository = "example/actual".into();
    PrPage {
        details: PrDetails {
            summary: PrSummary {
                pr,
                head_sha: head.to_string().repeat(40),
                base_sha: "b".repeat(40),
                head_ref: "feature".into(),
                base_ref: "main".into(),
                merged: false,
                auto_merge: false,
                merge_commit_sha: None,
                mergeable: None,
                mergeable_state: "unknown".into(),
                additions: 0,
                deletions: 0,
                changed_files: 0,
            },
            comments: vec![],
            commits: vec![],
            files: vec![],
            reviews,
            timeline: vec![],
            coverage: Coverage {
                complete: true,
                ..Default::default()
            },
        },
        tab: if checks.is_some() {
            PrTab::Checks
        } else {
            PrTab::Summary
        },
        diff: None,
        checks,
        required_checks: None,
        rows: vec![],
        context: None,
        metadata_display_v51: None,
        review_progress: None,
    }
}

fn workspace(account: Option<u64>) -> GithubWorkspace {
    let mut github = GithubWorkspace::empty(identity(account), vec![], BTreeSet::new());
    github.inbox.pull_requests = vec![
        pull_request(1, "2026-10-02T00:00:00Z"),
        pull_request(2, "2026-10-02T00:00:00Z"),
    ];
    github.selected = github.inbox.pull_requests.first().map(Into::into);
    github
}

#[test]
fn inbox_status_v1_review_state_uses_each_reviewers_latest_decision() {
    assert_eq!(review_state(&[]), ReviewState::NoReviews);
    assert_eq!(
        review_state(&[review("a", "COMMENTED"), review("b", "PENDING")]),
        ReviewState::Reviewed
    );
    assert_eq!(
        review_state(&[review("a", "PENDING")]),
        ReviewState::NoReviews
    );
    assert_eq!(
        review_state(&[review("a", "CHANGES_REQUESTED"), review("a", "APPROVED")]),
        ReviewState::Approved
    );
    assert_eq!(
        review_state(&[review("a", "APPROVED"), review("B", "CHANGES_REQUESTED")]),
        ReviewState::ChangesRequested
    );
    assert_eq!(
        review_state(&[review("a", "CHANGES_REQUESTED"), review("A", "DISMISSED")]),
        ReviewState::Reviewed
    );
}

#[test]
fn inbox_status_v1_check_state_fails_first_then_pending_then_passing() {
    assert_eq!(check_state(&[]), CheckState::NoChecks);
    assert_eq!(
        check_state(&[check("completed", "success"), check("completed", "skipped")]),
        CheckState::Passing
    );
    assert_eq!(
        check_state(&[check("completed", "success"), check("in_progress", "")]),
        CheckState::Pending
    );
    for failed in [
        "failure",
        "error",
        "timed_out",
        "cancelled",
        "action_required",
    ] {
        assert_eq!(
            check_state(&[check("in_progress", ""), check("completed", failed)]),
            CheckState::Failing,
            "{failed}"
        );
    }
}

#[test]
fn inbox_status_v1_memo_labels_read_pages_keeps_checks_per_head_and_marks_stale() {
    let mut github = workspace(Some(42));
    let mut memo = InboxStatusMemo::default();
    let rows = github.inbox.pull_requests.clone();
    // Nothing read yet: the row says so instead of inventing a status.
    memo.observe(&github);
    let unknown = memo.label(&github, &rows[0]);
    assert_eq!(unknown.text, NOT_LOADED);
    assert!(!unknown.known);
    // Opening the PR reads its reviews; checks wait for the Checks tab.
    github.pr = Some(page(
        1,
        'a',
        "2026-10-02T00:00:00Z",
        vec![review("alice", "APPROVED")],
        None,
    ));
    memo.observe(&github);
    assert_eq!(
        memo.label(&github, &rows[0]).text,
        "Approved, checks not loaded"
    );
    github.pr = Some(page(
        1,
        'a',
        "2026-10-02T00:00:00Z",
        vec![review("alice", "APPROVED")],
        Some(vec![check("completed", "success")]),
    ));
    memo.observe(&github);
    let passed = memo.label(&github, &rows[0]);
    assert_eq!(passed.text, "Approved, checks pass");
    assert_eq!(passed.role, Role::Success);
    // Back on the Summary tab of the same head, the checks read still holds.
    github.pr = Some(page(
        1,
        'a',
        "2026-10-02T00:00:00Z",
        vec![review("alice", "APPROVED")],
        None,
    ));
    memo.observe(&github);
    assert_eq!(memo.label(&github, &rows[0]).text, "Approved, checks pass");
    // A new head drops the old head's checks.
    github.pr = Some(page(
        1,
        'c',
        "2026-10-02T00:00:00Z",
        vec![review("bob", "CHANGES_REQUESTED")],
        None,
    ));
    memo.observe(&github);
    let changes = memo.label(&github, &rows[0]);
    assert_eq!(changes.text, "Changes requested, checks not loaded");
    assert_eq!(changes.role, Role::Failure);
    // Other rows stay unknown.
    assert_eq!(memo.label(&github, &rows[1]).text, NOT_LOADED);
    // A newer queue update than the page read marks the state stale.
    github.pr = None;
    let mut newer = rows[0].clone();
    newer.updated_at = "2026-10-03T00:00:00Z".into();
    let stale = memo.label(&github, &newer);
    assert_eq!(stale.text, "Changes requested, checks not loaded (stale)");
    assert_eq!(stale.role, Role::TextMuted);
}

#[test]
fn inbox_status_v1_memo_is_private_to_the_reading_account() {
    let mut github = workspace(Some(42));
    let rows = github.inbox.pull_requests.clone();
    let mut memo = InboxStatusMemo::default();
    github.pr = Some(page(
        1,
        'a',
        "2026-10-02T00:00:00Z",
        vec![review("alice", "APPROVED")],
        None,
    ));
    memo.observe(&github);
    assert!(memo.label(&github, &rows[0]).known);
    // Hidden identity: nothing shown, and the memo is cleared.
    let mut hidden = workspace(None);
    assert_eq!(memo.label(&hidden, &rows[0]).text, NOT_LOADED);
    hidden.pr = None;
    memo.observe(&hidden);
    assert!(!memo.label(&github, &rows[0]).known);
    // Another account never sees the first account's reads.
    github.pr = Some(page(
        1,
        'a',
        "2026-10-02T00:00:00Z",
        vec![review("alice", "APPROVED")],
        None,
    ));
    memo.observe(&github);
    let other = workspace(Some(7));
    assert_eq!(memo.label(&other, &rows[0]).text, NOT_LOADED);
}

fn paint(app: &mut App, width: u16, height: u16) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    app.hits.clear();
    terminal
        .draw(|frame| crate::github_ui::draw(frame, app, Rect::new(0, 0, width, height)))
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
        .collect()
}

#[test]
fn inbox_status_v1_rows_paint_status_column_when_wide_and_detail_line_when_narrow() {
    let mut github = workspace(Some(42));
    github.pr = Some(page(
        1,
        'a',
        "2026-10-02T00:00:00Z",
        vec![review("alice", "APPROVED")],
        Some(vec![check("completed", "failure")]),
    ));
    let mut app = App {
        view: View::Inbox,
        github: Some(github),
        github_content_focused: true,
        ..Default::default()
    };
    // Painting the open PR page records its state; Back returns to the queue.
    paint(&mut app, 160, 30);
    app.github.as_mut().unwrap().pr = None;
    let lines = paint(&mut app, 160, 30);
    let screen = lines.join("\n");
    let header = lines
        .iter()
        .find(|line| line.contains("TITLE") && line.contains("STATUS"))
        .unwrap_or_else(|| panic!("STATUS column at 160:\n{screen}"));
    let first = lines
        .iter()
        .find(|line| line.contains("Example/Actual#1 "))
        .unwrap_or_else(|| panic!("row 1:\n{screen}"));
    let second = lines
        .iter()
        .find(|line| line.contains("Example/Actual#2 "))
        .unwrap_or_else(|| panic!("row 2:\n{screen}"));
    assert!(first.contains("Approved, checks fail"), "{screen}");
    let column = |line: &str, needle: &str| line[..line.find(needle).unwrap()].chars().count();
    assert_eq!(
        column(first, "Approved"),
        column(header, "STATUS"),
        "status under its title:\n{screen}"
    );
    assert!(second.contains(NOT_LOADED), "{screen}");
    // With a long title STATUS leaves before the title is cut; the detail line carries it.
    app.github.as_mut().unwrap().inbox.pull_requests[0].title =
        "Keep long Inbox titles whole before the review and check status column".into();
    let lines = paint(&mut app, 100, 24);
    let screen = lines.join("\n");
    assert!(!screen.contains("STATUS"), "{screen}");
    let row = lines
        .iter()
        .position(|line| line.contains("Example/Actual#1"))
        .unwrap_or_else(|| panic!("row 1 at 100:\n{screen}"));
    assert!(lines[row + 1].contains("Approved, checks fail"), "{screen}");
}

fn rollup(
    review: Option<&str>,
    checks: Option<&str>,
    head: char,
) -> crate::providers::gh::InboxRollupV1 {
    crate::providers::gh::InboxRollupV1 {
        repository: "example/actual".into(),
        number: 1,
        review_decision: review.map(str::to_owned),
        check_state: checks.map(str::to_owned),
        head_oid: head.to_string().repeat(40),
        reviewers: Vec::new(),
    }
}

#[test]
fn inbox_status_v1_load_rollup_labels_unopened_rows_and_yields_to_newer_pages() {
    let mut github = workspace(Some(42));
    let rows = github.inbox.pull_requests.clone();
    let key = row_key("github.invalid", "Example/Actual", 1);
    let mut memo = InboxStatusMemo::default();
    memo.observe(&github);
    for (review, checks, text, role) in [
        (
            Some("APPROVED"),
            Some("SUCCESS"),
            "Approved, checks pass",
            Role::Success,
        ),
        (
            Some("CHANGES_REQUESTED"),
            Some("SUCCESS"),
            "Changes requested, checks pass",
            Role::Failure,
        ),
        (
            Some("REVIEW_REQUIRED"),
            Some("PENDING"),
            "Review required, checks pending",
            Role::Warning,
        ),
        (
            Some("REVIEW_REQUIRED"),
            Some("EXPECTED"),
            "Review required, checks pending",
            Role::Warning,
        ),
        (
            None,
            Some("ERROR"),
            "No review decision, checks fail",
            Role::Failure,
        ),
        (None, None, "No review decision, no checks", Role::Text),
    ] {
        github.inbox_rollup_v1.insert(
            key.clone(),
            RowStatus::from_rollup_v1(&rollup(review, checks, 'a'), "2026-10-02T00:00:00Z"),
        );
        let label = memo.label(&github, &rows[0]);
        assert_eq!(label.text, text);
        assert_eq!(label.role, role);
        assert!(label.known);
    }
    // The other row was not in the rollup and was never opened.
    assert_eq!(memo.label(&github, &rows[1]).text, NOT_LOADED);
    // A page read after the load wins; its missing checks come from the same head's rollup.
    github.inbox_rollup_v1.insert(
        key.clone(),
        RowStatus::from_rollup_v1(
            &rollup(Some("REVIEW_REQUIRED"), Some("SUCCESS"), 'a'),
            "2026-10-02T00:00:00Z",
        ),
    );
    github.pr = Some(page(
        1,
        'a',
        "2026-10-02T00:00:00Z",
        vec![review("bob", "APPROVED")],
        None,
    ));
    memo.observe(&github);
    assert_eq!(memo.label(&github, &rows[0]).text, "Approved, checks pass");
    // A different head never borrows the rollup's checks.
    github.pr = Some(page(
        1,
        'c',
        "2026-10-02T00:00:00Z",
        vec![review("bob", "APPROVED")],
        None,
    ));
    memo.observe(&github);
    assert_eq!(
        memo.label(&github, &rows[0]).text,
        "Approved, checks not loaded"
    );
    // A newer queue update than the rollup marks it stale.
    let mut newer = rows[0].clone();
    newer.updated_at = "2026-10-03T00:00:00Z".into();
    github.pr = None;
    let mut fresh_memo = InboxStatusMemo::default();
    fresh_memo.observe(&github);
    let stale = fresh_memo.label(&github, &newer);
    assert_eq!(stale.text, "Review required, checks pass (stale)");
    assert_eq!(stale.role, Role::TextMuted);
    // A hidden identity never sees the rollup.
    github.identity.account_id = None;
    assert_eq!(fresh_memo.label(&github, &rows[0]).text, NOT_LOADED);
}
