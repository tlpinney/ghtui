//! TASK-12 AC2: one bounded GraphQL read of the review decision and the head commit's
//! status-check rollup for the PRs an Inbox load just returned.
//!
//! The queue read (`search/issues`) carries neither, so the ordinary Inbox load follows it
//! with this read. It is read-only, takes its query in argv (no stdin), and sends one call per
//! [`INBOX_ROLLUP_BATCH_V1`] PRs in a fixed (repository, number) order. A PR GitHub does not
//! return (deleted, inaccessible, or a host without GraphQL) simply has no rollup, and the
//! caller keeps its session memo as the fallback.
//!
//! The same read also lists the PR's reviewers for the Inbox preview: requested reviewers,
//! the latest review of each reviewer, and recent commenters (bots included).
use super::{error, validate_repo, Actor, GithubErrorKind, GithubProvider, GithubResult};
use crate::providers::runner::{CancellationToken, CommandFailure, FailureKind};
use serde_json::Value;

/// PRs per GraphQL call. Fifty aliased `repository.pullRequest` lookups stay far inside
/// GitHub's node limit and the request size of an ordinary read.
pub const INBOX_ROLLUP_BATCH_V1: usize = 50;

/// Requested reviewers and latest reviews read per PR; GitHub rarely lists more.
pub const INBOX_REVIEWERS_FIRST_V1: usize = 10;
/// Most recent conversation comments scanned for commenters per PR.
pub const INBOX_COMMENTERS_LAST_V1: usize = 20;

/// Where one actor stands on a PR, in the order the preview lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReviewerRoleV1 {
    ChangesRequested,
    Approved,
    Reviewed,
    Dismissed,
    Requested,
    Commented,
}

impl ReviewerRoleV1 {
    pub fn label(self) -> &'static str {
        match self {
            Self::ChangesRequested => "changes requested",
            Self::Approved => "approved",
            Self::Reviewed => "reviewed",
            Self::Dismissed => "review dismissed",
            Self::Requested => "requested",
            Self::Commented => "commented",
        }
    }
}

/// One user, team or bot on a PR's reviewer list. A team's `login` is its `org/slug`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboxReviewerV1 {
    pub actor: Actor,
    pub role: ReviewerRoleV1,
}

/// What GitHub reports for one PR. `None` fields mean GitHub returned null for them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboxRollupV1 {
    pub repository: String,
    pub number: u64,
    /// `APPROVED`, `CHANGES_REQUESTED`, `REVIEW_REQUIRED`, or `None` when no review rule
    /// applies.
    pub review_decision: Option<String>,
    /// `SUCCESS`, `FAILURE`, `ERROR`, `PENDING`, `EXPECTED`, or `None` without any check.
    pub check_state: Option<String>,
    pub head_oid: String,
    /// Reviews first, then pending requests, then commenters; one entry per login. `None`
    /// when the status-only fallback answered and reviewers were not read.
    pub reviewers: Option<Vec<InboxReviewerV1>>,
}

/// The exact GraphQL document for `targets`, in the order given.
pub fn inbox_rollup_query_v1(targets: &[(&str, u64)]) -> String {
    rollup_query(targets, true)
}

/// The same document without the reviewer fields: the fallback for a server whose schema
/// rejects them (an older GitHub Enterprise Server), so the status column still loads.
pub fn inbox_rollup_status_query_v1(targets: &[(&str, u64)]) -> String {
    rollup_query(targets, false)
}

fn rollup_query(targets: &[(&str, u64)], with_reviewers: bool) -> String {
    // `...on Actor` rather than one fragment per type: servers differ in which actor types
    // `RequestedReviewer` includes (GHES 3.9 and 3.10 have no `Bot`).
    let fields = if !with_reviewers {
        String::new()
    } else {
        format!(
            " reviewRequests(first:{INBOX_REVIEWERS_FIRST_V1}){{nodes{{requestedReviewer{{__typename ...on Actor{{login}} ...on Team{{combinedSlug}}}}}}}} latestReviews(first:{INBOX_REVIEWERS_FIRST_V1}){{nodes{{state author{{__typename login}}}}}} comments(last:{INBOX_COMMENTERS_LAST_V1}){{nodes{{author{{__typename login}}}}}}"
        )
    };
    let mut query = String::from("query{");
    for (index, (repository, number)) in targets.iter().enumerate() {
        let (owner, name) = repository.split_once('/').unwrap_or((repository, ""));
        if index > 0 {
            query.push(' ');
        }
        query.push_str(&format!(
            "p{index}:repository(owner:\"{owner}\",name:\"{name}\"){{pullRequest(number:{number}){{reviewDecision commits(last:1){{nodes{{commit{{oid statusCheckRollup{{state}}}}}}}}{fields}}}}}"
        ));
    }
    query.push('}');
    query
}

/// The exact `gh` argv for one batch.
pub fn inbox_rollup_args_v1(host: &str, targets: &[(&str, u64)]) -> Vec<String> {
    graphql_args(host, inbox_rollup_query_v1(targets))
}

/// The exact `gh` argv for one batch's status-only fallback.
pub fn inbox_rollup_status_args_v1(host: &str, targets: &[(&str, u64)]) -> Vec<String> {
    graphql_args(host, inbox_rollup_status_query_v1(targets))
}

fn graphql_args(host: &str, query: String) -> Vec<String> {
    vec![
        "api".into(),
        "graphql".into(),
        "--hostname".into(),
        host.into(),
        "-f".into(),
        format!("query={query}"),
    ]
}

/// Sorts and de-duplicates targets the way the Inbox load sends them.
pub fn inbox_rollup_order_v1(targets: &mut Vec<(String, u64)>) {
    targets.sort_by(|a, b| {
        a.0.to_ascii_lowercase()
            .cmp(&b.0.to_ascii_lowercase())
            .then(a.1.cmp(&b.1))
    });
    targets.dedup_by(|a, b| a.0.eq_ignore_ascii_case(&b.0) && a.1 == b.1);
}

fn text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

fn nodes<'a>(pr: &'a Value, field: &str) -> impl Iterator<Item = &'a Value> {
    pr.pointer(&format!("/{field}/nodes"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

/// Reviews, requests and commenters, one entry per login (a review outranks a request, and
/// both outrank a comment). Entries without a login (deleted users, unknown types) drop out.
fn reviewers(pr: &Value) -> Vec<InboxReviewerV1> {
    let mut entries: Vec<InboxReviewerV1> = Vec::new();
    let mut add = |actor: Actor, role: ReviewerRoleV1| {
        if actor.login.is_empty() {
            return;
        }
        match entries
            .iter_mut()
            .find(|entry| entry.actor.login.eq_ignore_ascii_case(&actor.login))
        {
            Some(entry) if role < entry.role => entry.role = role,
            Some(_) => {}
            None => entries.push(InboxReviewerV1 { actor, role }),
        }
    };
    for review in nodes(pr, "latestReviews") {
        let role = match review.get("state").and_then(Value::as_str) {
            Some("APPROVED") => ReviewerRoleV1::Approved,
            Some("CHANGES_REQUESTED") => ReviewerRoleV1::ChangesRequested,
            Some("COMMENTED") => ReviewerRoleV1::Reviewed,
            Some("DISMISSED") => ReviewerRoleV1::Dismissed,
            // PENDING is the viewer's own unsubmitted review.
            _ => continue,
        };
        if let Some(author) = review.get("author").filter(|a| a.is_object()) {
            add(Actor::from_json(author), role);
        }
    }
    for request in nodes(pr, "reviewRequests") {
        let Some(reviewer) = request.get("requestedReviewer").filter(|r| r.is_object()) else {
            continue;
        };
        let mut actor = Actor::from_json(reviewer);
        if let Some(slug) = text(reviewer.get("combinedSlug")) {
            actor.login = slug;
        }
        add(actor, ReviewerRoleV1::Requested);
    }
    for comment in nodes(pr, "comments") {
        if let Some(author) = comment.get("author").filter(|a| a.is_object()) {
            add(Actor::from_json(author), ReviewerRoleV1::Commented);
        }
    }
    entries.sort_by(|a, b| a.role.cmp(&b.role));
    entries
}

/// Whether `gh` exited because GitHub answered the query with GraphQL errors and no data,
/// as a schema without one of the requested fields or types does.
fn graphql_rejection(failure: &CommandFailure) -> bool {
    failure.kind == FailureKind::Exit
        && serde_json::from_slice::<Value>(&failure.stdout).is_ok_and(|body| {
            body.get("errors")
                .and_then(Value::as_array)
                .is_some_and(|errors| !errors.is_empty())
                && body.get("data").is_none_or(Value::is_null)
        })
}

/// Reads one response. A null alias (PR missing or not visible) yields no row.
pub fn parse_inbox_rollup_v1(
    targets: &[(&str, u64)],
    response: &Value,
) -> GithubResult<Vec<InboxRollupV1>> {
    parse_rollup(targets, response, true)
}

fn parse_rollup(
    targets: &[(&str, u64)],
    response: &Value,
    with_reviewers: bool,
) -> GithubResult<Vec<InboxRollupV1>> {
    let data = response
        .get("data")
        .filter(|data| data.is_object())
        .ok_or_else(|| error(GithubErrorKind::Json, "Inbox status response has no data"))?;
    let mut rows = Vec::new();
    for (index, (repository, number)) in targets.iter().enumerate() {
        let Some(pr) = data
            .get(format!("p{index}"))
            .and_then(|repo| repo.get("pullRequest"))
            .filter(|pr| pr.is_object())
        else {
            continue;
        };
        let commit = pr.pointer("/commits/nodes/0/commit");
        rows.push(InboxRollupV1 {
            repository: (*repository).to_owned(),
            number: *number,
            review_decision: text(pr.get("reviewDecision")),
            check_state: text(commit.and_then(|c| c.pointer("/statusCheckRollup/state"))),
            head_oid: text(commit.and_then(|c| c.get("oid"))).unwrap_or_default(),
            reviewers: with_reviewers.then(|| reviewers(pr)),
        });
    }
    Ok(rows)
}

impl GithubProvider {
    /// One batch (at most [`INBOX_ROLLUP_BATCH_V1`] PRs). Read-only. If the server rejects
    /// the document's fields, one status-only retry keeps the review and check states loading.
    pub fn inbox_rollup_v1(
        &self,
        targets: &[(&str, u64)],
        cancel: &CancellationToken,
    ) -> GithubResult<Vec<InboxRollupV1>> {
        if targets.is_empty() {
            return Ok(Vec::new());
        }
        if targets.len() > INBOX_ROLLUP_BATCH_V1 {
            return Err(error(
                GithubErrorKind::InvalidTarget,
                "Inbox status batch is too large",
            ));
        }
        for (repository, number) in targets {
            validate_repo(repository)?;
            if *number == 0 {
                return Err(error(
                    GithubErrorKind::InvalidTarget,
                    "Resource number must be positive",
                ));
            }
        }
        let parse = |bytes: &[u8], with_reviewers: bool| {
            let response: Value = serde_json::from_slice(bytes)
                .map_err(|e| error(GithubErrorKind::Json, e.to_string()))?;
            parse_rollup(targets, &response, with_reviewers)
        };
        match self
            .runner
            .run(self.spec(inbox_rollup_args_v1(&self.host, targets)), cancel)
        {
            Ok(output) => parse(&output.stdout, true),
            // Only GitHub rejecting the document itself (gh exits with a GraphQL `errors`
            // body and no data) is retried; network, HTTP and timeout failures are not.
            Err(failure) if graphql_rejection(&failure) && !cancel.is_cancelled() => {
                let bytes = self.execute(
                    inbox_rollup_status_args_v1(&self.host, targets),
                    None,
                    None,
                    cancel,
                )?;
                parse(&bytes, false)
            }
            Err(failure) => Err(failure.into()),
        }
    }
}

#[cfg(test)]
#[path = "inbox_rollup_v1_tests.rs"]
mod tests;
