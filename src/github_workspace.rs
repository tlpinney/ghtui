//! Prepared GitHub pages. All I/O happens in workers; renderers use this model.
use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::providers::gh::{
    Check, Coverage, GithubProvider, HostIdentity, Inbox, PrDetails, PrDiff, PullRequest, Queue,
};
use crate::providers::runner::CancellationToken;
use crate::storage::{now, Cache, Record, Scope};

#[path = "github_legacy_startup_v80.rs"]
mod legacy_startup_v80;
#[path = "github_timeline_projection_v1.rs"]
pub(crate) mod timeline_projection_v1;
pub(crate) use legacy_startup_v80::LegacyStartupQueueV80;
pub(crate) use legacy_startup_v80::PreparedStateWorkspaceV80;
use legacy_startup_v80::StateSourceRouteHintV80;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PrKey {
    pub host: String,
    pub repository: String,
    pub number: u64,
}
impl From<&PullRequest> for PrKey {
    fn from(pr: &PullRequest) -> Self {
        Self {
            host: pr.host.clone(),
            repository: pr.repository.clone(),
            number: pr.number,
        }
    }
}
impl PrKey {
    pub fn label(&self) -> String {
        format!("{}#{}", self.repository, self.number)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InboxFilter {
    #[default]
    All,
    Authored,
    Assigned,
    ReviewRequested,
}
impl InboxFilter {
    pub const ALL: [(Self, &'static str, char); 4] = [
        (Self::All, "All", 'a'),
        (Self::Authored, "Created by me", 'c'),
        (Self::Assigned, "Assigned to me", 'm'),
        (Self::ReviewRequested, "Needs my review", 'v'),
    ];
    pub fn accepts(self, pr: &PullRequest) -> bool {
        match self {
            Self::All => true,
            Self::Authored => pr.queues.contains(&Queue::Authored),
            Self::Assigned => pr.queues.contains(&Queue::Assigned),
            Self::ReviewRequested => pr.queues.contains(&Queue::ReviewRequested),
        }
    }
}

/// "Queue: <current> • All N · Created by me N · ..." for the loaded inbox.
pub fn queue_summary_v1(current: &str, counts: [usize; 4]) -> String {
    let counts = InboxFilter::ALL
        .iter()
        .zip(counts)
        .map(|((_, label, _), count)| format!("{label} {count}"))
        .collect::<Vec<_>>()
        .join(" · ");
    format!("Queue: {current} • {counts}")
}
/// Empty-queue text that points at a populated queue instead of a dead end.
pub fn empty_queue_hint_v1(filter: InboxFilter, counts: [usize; 4]) -> String {
    let base = "No pull requests in this queue / scope.";
    if filter == InboxFilter::All {
        return format!("{base} Press r to refresh.");
    }
    match InboxFilter::ALL
        .iter()
        .zip(counts)
        .find(|((_, _, _), count)| *count > 0)
    {
        Some(((_, label, key), count)) => {
            format!("{base} {label} has {count}: press {key} or click {label} above.")
        }
        None => format!("{base} Press r to refresh."),
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PrTab {
    #[default]
    Summary,
    Conversation,
    Commits,
    Files,
    Checks,
    Timeline,
    Metadata,
    Progress,
}
impl PrTab {
    pub const ALL: [(Self, &'static str, char); 8] = [
        (Self::Summary, "Summary", 'o'),
        (Self::Conversation, "Conversation", 'c'),
        (Self::Commits, "Commits", 'h'),
        (Self::Files, "Files", 'f'),
        (Self::Checks, "Checks", 'k'),
        (Self::Timeline, "Timeline", 't'),
        (Self::Metadata, "Metadata", 'u'),
        (Self::Progress, "Review progress", 'v'),
    ];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GithubAction {
    FileCommentV75,
    ThreadHistoryV1,
    ReviewOperationHistoryV3,
    ReviewThreadsV4,
    ActionsHistoryV5,
    /// The selected row's GitHub actor profile (TASK-34); read-only.
    ActorProfile,
    Filter(InboxFilter),
    Open(PrKey),
    Back,
    PrTab(PrTab),
    Resource(crate::scoped_pr_observed_tabs_v1::resources::PrResourceUiActionV1),
    ClearCache,
    Actions,
    Compose(crate::forms::PrFormKind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Freshness {
    Empty,
    Cached { fetched_at: u64 },
    Fresh { fetched_at: u64 },
    Offline { fetched_at: u64, error: String },
}
fn fetched(seconds: u64) -> String {
    if seconds == 0 {
        "never".into()
    } else {
        crate::metrics_workspace::utc_label(seconds)
    }
}
impl Freshness {
    pub fn label(&self) -> String {
        match self {
            Self::Empty => "No loaded records".into(),
            // Cached is an observation, not proof that a refresh worker exists.
            // Actual pending work is shown by the application loading state.
            Self::Cached { fetched_at } => format!("Cached • fetched {}", fetched(*fetched_at)),
            Self::Fresh { fetched_at } => format!("Fresh • fetched {}", fetched(*fetched_at)),
            Self::Offline { fetched_at, error } => {
                format!("Offline/stale • {error} • fetched {}", fetched(*fetched_at))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct PrPage {
    pub details: PrDetails,
    pub tab: PrTab,
    pub diff: Option<PrDiff>,
    pub checks: Option<Vec<Check>>,
    /// The base branch's required checks, read beside `checks` (TASK-14 AC #1).
    /// None until the Checks tab loads; `Unavailable` never marks a check optional.
    pub required_checks: Option<crate::providers::gh::RequiredChecksV1>,
    pub rows: Vec<PrRow>,
    pub context: Option<crate::providers::gh::PrContext>,
    /// Actual Metadata-tab display availability, never source or permission.
    /// None preserves legacy contexts; only the genuine bounded producer can
    /// create the opaque facts carried here.
    pub metadata_display_v51: Option<crate::providers::gh::MetadataDisplayFactsV51>,
    pub review_progress: Option<crate::pr_progress::ReviewProgress>,
}
#[derive(Clone, Debug)]
pub struct PrCommentSelection {
    pub kind: crate::providers::gh::CommentKind,
    pub observed: crate::providers::gh::ConversationComment,
}
#[derive(Clone, Debug)]
pub struct PrRow {
    pub label: String,
    pub role: crate::theme::SemanticToken,
    pub actor: Option<crate::providers::gh::Actor>,
    pub anchor: Option<crate::repository_view::DiffAnchor>,
    pub comment: Option<std::sync::Arc<PrCommentSelection>>,
    /// Table cells for the tab's columns (`PrPage::columns`); `label` keeps the full prose
    /// (full SHA, UTC time) for palette, plans and logs. None paints `label` as a prose row.
    pub cells: Option<PrCells>,
    /// A Timeline item's own record in another loaded tab (TASK-13). Navigation only:
    /// kept only when that exact comment, review or commit is in this PR's details.
    pub link: Option<PrRowLinkV13>,
    /// How a Summary/Conversation row is painted (TASK-114/115); None paints `label`.
    pub card: Option<PrCardV115>,
}
/// The painted part of a PR body or Conversation card (TASK-114/115). `label` keeps the
/// row's plain text for palette, plans and logs.
#[derive(Clone, Debug)]
pub enum PrCardV115 {
    /// One rendered Markdown line of a description, comment or review body.
    Markdown(std::sync::Arc<crate::markdown_v114::MdLine>),
    /// The rule that opens a card.
    Top,
    /// Reaction counts under a comment body (click: browse its reactions). Chrome rows are
    /// not list stops and carry no comment: they act on the comment rows just above them.
    Reactions,
    /// The per-comment buttons that close a comment card.
    Buttons,
}
/// What a Timeline row points at: a conversation comment, a review or a commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrRowLinkV13 {
    Comment(u64),
    Review(u64),
    Commit(String),
}
/// A PR list row painted as table cells (TASK-91): Commits, Files and Checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrCells {
    /// The column titles, inline after the tab's intro row.
    Header,
    /// One cell per column; the actor column's cell is the name shown without a username.
    Row(Vec<String>),
}
/// Commits tab: short SHA, first message line, author chip and authored age. Author and age
/// share the message priority so a long message is cut before they leave.
pub const COMMIT_COLUMNS: [crate::ui_table::Column; 4] = [
    crate::ui_table::Column::new("SHA", 7).priority(0),
    crate::ui_table::Column::new("MESSAGE", 10)
        .flex(1)
        .priority(2),
    crate::ui_table::Column::new("AUTHOR", 6)
        .priority(2)
        .actor(),
    crate::ui_table::Column::new("AGE", 3).priority(2).right(),
];
/// Files tab: change kind, path (with its old path), added and deleted lines; a long path is
/// cut before any of them leaves.
pub const FILE_COLUMNS: [crate::ui_table::Column; 4] = [
    crate::ui_table::Column::new("STATUS", 6).priority(2),
    crate::ui_table::Column::new("PATH", 10).flex(1).priority(2),
    crate::ui_table::Column::new("ADD", 3).priority(1).right(),
    crate::ui_table::Column::new("DEL", 3).priority(1).right(),
];
/// Checks tab (status glyph in the glyph slot): name, state and the head commit (the commit
/// leaves first: the exact head is painted above the table).
pub const CHECK_COLUMNS: [crate::ui_table::Column; 3] = [
    crate::ui_table::Column::new("CHECK", 10)
        .flex(1)
        .priority(1),
    crate::ui_table::Column::new("STATUS", 6).priority(1),
    crate::ui_table::Column::new("COMMIT", 7).priority(3),
];
impl PrRow {
    fn text(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            role: crate::theme::SemanticToken::Normal,
            actor: None,
            anchor: None,
            comment: None,
            cells: None,
            link: None,
            card: None,
        }
    }
    fn with_link(mut self, link: Option<PrRowLinkV13>) -> Self {
        self.link = link;
        self
    }
    fn actor(label: impl Into<String>, actor: crate::providers::gh::Actor) -> Self {
        Self {
            actor: Some(actor),
            ..Self::text(label)
        }
    }
    /// A card chrome row (rule, reactions, buttons).
    fn card(card: PrCardV115) -> Self {
        Self {
            card: Some(card),
            ..Self::text("")
        }
    }
    /// The rendered Markdown lines of a body, each one row; a comment's lines carry it.
    fn markdown(body: &str, selection: Option<&std::sync::Arc<PrCommentSelection>>) -> Vec<Self> {
        crate::markdown_v114::render(body)
            .into_iter()
            .map(|line| Self {
                label: line.plain(),
                comment: selection.cloned(),
                card: Some(PrCardV115::Markdown(std::sync::Arc::new(line))),
                ..Self::text("")
            })
            .collect()
    }
    fn with_cells(mut self, cells: PrCells) -> Self {
        self.cells = Some(cells);
        self
    }
    fn with_comment(mut self, selection: &std::sync::Arc<PrCommentSelection>) -> Self {
        self.comment = Some(selection.clone());
        self
    }
}
/// Latest decision per reviewer, in first-review order. A later plain
/// comment does not replace an approval or change request (GitHub semantics).
pub fn review_decisions(reviews: &[serde_json::Value]) -> String {
    // (login key, shared identity label, latest decision word)
    let mut latest: Vec<(String, String, &str)> = Vec::new();
    for review in reviews {
        // The shared identity label, read as the Conversation tab reads review authors: a
        // deleted or missing reviewer is never shown as a username, and a bot or organization
        // keeps its kind.
        let actor = value_actor(&review["user"]);
        let label = crate::identity::ActorIdentity::github("", &actor).label();
        let login = actor.login.as_str();
        let state = review["state"].as_str().unwrap_or("");
        let word = match state {
            "APPROVED" => "approved",
            "CHANGES_REQUESTED" => "requested changes",
            "COMMENTED" => "commented",
            "DISMISSED" => "review dismissed",
            "PENDING" => "pending review (not submitted)",
            _ => continue,
        };
        // Reviews without an account are never merged into one another.
        match latest
            .iter_mut()
            .find(|(who, _, _)| !login.is_empty() && who.eq_ignore_ascii_case(login))
        {
            Some(entry)
                if matches!(state, "COMMENTED" | "PENDING")
                    && matches!(
                        entry.2,
                        "approved" | "requested changes" | "review dismissed"
                    ) => {}
            Some(entry) => entry.2 = word,
            None => latest.push((login.to_owned(), label, word)),
        }
    }
    if latest.is_empty() {
        return "Reviews: none yet".into();
    }
    let parts: Vec<String> = latest
        .iter()
        .map(|(_, label, word)| format!("{label} {word}"))
        .collect();
    format!("Reviews: {}", parts.join(" • "))
}
impl PrPage {
    /// Checks rows are prepared one-to-one from this exact original vector.
    /// A display label or details URL is never parsed into a target identity.
    pub fn check_at(&self, index: usize) -> Option<&Check> {
        (self.tab == PrTab::Checks)
            .then(|| self.checks.as_ref()?.get(index))
            .flatten()
    }
    /// The table columns of this tab's cell rows and whether they carry a status glyph.
    pub fn columns(&self) -> Option<(&'static [crate::ui_table::Column], bool)> {
        match self.tab {
            PrTab::Commits => Some((&COMMIT_COLUMNS, false)),
            PrTab::Files => Some((&FILE_COLUMNS, false)),
            PrTab::Checks => Some((&CHECK_COLUMNS, true)),
            _ => None,
        }
    }
    pub(crate) fn prepare_rows(&mut self) -> Result<()> {
        use crate::theme::SemanticToken;
        let summary = &self.details.summary;
        self.rows = match self.tab {
            PrTab::Summary => {
                let mut rows = vec![
                    PrRow::actor(
                        format!("opened {}", summary.pr.created_at),
                        summary.pr.author.clone(),
                    ),
                    PrRow::text(format!(
                        "{} → {} • +{} −{} • {} changed files",
                        summary.head_ref,
                        summary.base_ref,
                        summary.additions,
                        summary.deletions,
                        summary.changed_files
                    )),
                    PrRow::text(format!("Head {}", summary.head_sha)),
                    PrRow::text(format!("Base {}", summary.base_sha)),
                    PrRow::text(format!(
                        "Mergeable: {} • {} • auto-merge {}",
                        summary.mergeable.map_or("not yet known", |yes| if yes {
                            "yes"
                        } else {
                            "no"
                        }),
                        summary.mergeable_state,
                        if summary.auto_merge {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    )),
                    PrRow::text(format!(
                        "{} comments • {} reviews • {} commits",
                        self.details.comments.len(),
                        self.details.reviews.len(),
                        self.details.commits.len()
                    )),
                    PrRow::text(review_decisions(&self.details.reviews)),
                ];
                // TASK-13: closing-keyword issues from the loaded Description;
                // Metadata (u) lists them all with permissions, requested reviewers
                // and fork origin. At most five here, so the row stays inside the
                // fixed per-row slack the Summary presentation preflight charges.
                let mut closing = closing_keyword_issues_v1(&summary.pr.body);
                if closing.len() > 5 {
                    let more = closing.len() - 5;
                    closing.truncate(5);
                    closing.push(format!("+{more} more in u Metadata"));
                }
                if !closing.is_empty() {
                    rows.push(PrRow::text(linked_issues_row_v13(&closing)));
                }
                rows.extend([
                    PrRow::text(format!("Source: {}", summary.pr.url)),
                    PrRow::text(""),
                ]);
                // The description reads as GitHub renders it (TASK-114).
                rows.extend(PrRow::markdown(&summary.pr.body, None));
                rows
            }
            PrTab::Conversation => {
                // One card per description, comment and review (TASK-115): a rule opens it,
                // the author header and rendered body follow, and a comment closes with its
                // reactions and buttons. Only headers and body lines are list stops.
                let pr_author = summary.pr.author.login.as_str();
                let mut rows = vec![
                    PrRow::card(PrCardV115::Top),
                    PrRow::actor("PR description", summary.pr.author.clone()),
                ];
                rows.extend(PrRow::markdown(&summary.pr.body, None));
                for comment in &self.details.comments {
                    let selection = crate::pr_comment_selection_shared_v1::conversation_selection(comment);
                    rows.push(PrRow::card(PrCardV115::Top));
                    rows.push(
                        PrRow::actor(
                            crate::pr_conversation_cards_v115::comment_header(comment, pr_author),
                            comment.author.clone(),
                        )
                        .with_comment(&selection),
                    );
                    rows.extend(PrRow::markdown(&comment.body, Some(&selection)));
                    if let Some(reactions) =
                        crate::pr_conversation_cards_v115::reaction_summary(comment.reactions())
                    {
                        rows.push(PrRow {
                            role: SemanticToken::Muted,
                            label: reactions,
                            ..PrRow::card(PrCardV115::Reactions)
                        });
                    }
                    rows.push(PrRow::card(PrCardV115::Buttons));
                }
                for review in &self.details.reviews {
                    rows.push(PrRow::card(PrCardV115::Top));
                    rows.push(PrRow::actor(
                        crate::pr_conversation_cards_v115::review_header(review, pr_author),
                        value_actor(&review["user"]),
                    ));
                    rows.extend(PrRow::markdown(
                        review["body"].as_str().unwrap_or_default(),
                        None,
                    ));
                }
                rows
            }
            PrTab::Commits => {
                let mut rows = vec![PrRow::text(
                    "Commit = a saved version • author writes, committer records it",
                )];
                if !self.details.commits.is_empty() {
                    rows.push(PrRow::text("").with_cells(PrCells::Header));
                }
                for commit in &self.details.commits {
                    let subject = commit.message.lines().next().unwrap_or_default();
                    let mut row = PrRow::text(format!("{} {subject}", commit.sha)).with_cells(
                        PrCells::Row(vec![
                            crate::ui_table::short_sha(&commit.sha).to_owned(),
                            subject.to_owned(),
                            commit.author_name.clone(),
                            crate::ui_table::relative_age(&commit.authored_at),
                        ]),
                    );
                    if !commit.author.login.is_empty() {
                        row.actor = Some(commit.author.clone());
                    }
                    rows.push(row);
                    if commit.author.login.is_empty() {
                        rows.push(PrRow::text(format!(
                            "author {} [local Git; no associated username]",
                            commit.author_name
                        )));
                    } else {
                        rows.push(PrRow::actor(
                            format!("author {} • {}", commit.author_name, commit.authored_at),
                            commit.author.clone(),
                        ));
                    }
                    if commit.committer.login.is_empty() {
                        rows.push(PrRow::text(format!(
                            "committer {} [local Git; no associated username]",
                            commit.committer_name
                        )));
                    } else {
                        rows.push(PrRow::actor(
                            format!("committer {}", commit.committer_name),
                            commit.committer.clone(),
                        ));
                    }
                    // The full SHA stays visible and copyable under the short one, and the
                    // full parent SHAs get their own row so 80 columns never cut them.
                    rows.push(PrRow::text(format!("Commit {}", commit.sha)));
                    rows.push(PrRow::text(format!(
                        "Parents: {}",
                        if commit.parents.is_empty() {
                            "Parent links unavailable or none in this response".into()
                        } else {
                            commit.parents.join(", ")
                        }
                    )));
                }
                rows
            }
            PrTab::Files => {
                let mut rows = vec![PrRow::text(format!(
                    "Canonical diff • base {} • head {}",
                    summary.base_sha, summary.head_sha
                ))];
                if !self.details.files.is_empty() {
                    rows.push(PrRow::text("").with_cells(PrCells::Header));
                }
                for file in &self.details.files {
                    let was = file
                        .previous_path
                        .as_ref()
                        .map_or(String::new(), |old| format!(" (was {old})"));
                    rows.push(
                        PrRow::text(format!(
                            "{} {} +{} −{}{was}",
                            file.status, file.path, file.additions, file.deletions,
                        ))
                        .with_cells(PrCells::Row(vec![
                            file.status.clone(),
                            format!("{}{was}", file.path),
                            format!("+{}", file.additions),
                            format!("−{}", file.deletions),
                        ])),
                    );
                }
                if let Some(diff) = &self.diff {
                    let mut canonical = crate::diff::Diff::parse(diff.text.as_bytes())
                        .map_err(anyhow::Error::msg)?;
                    canonical.base = Some(diff.base_sha.clone());
                    canonical.head = Some(diff.head_sha.clone());
                    for row in crate::repository_view::diff_rows(
                        &canonical,
                        crate::repository_view::DiffLayout::Unified,
                        true,
                    ) {
                        rows.push(PrRow {
                            label: row.left.label,
                            role: row.left.role.into(),
                            actor: None,
                            anchor: row.left.anchor,
                            comment: None,
                            cells: None,
                            link: None,
                            card: None,
                        });
                    }
                }
                rows
            }
            PrTab::Checks => self.checks.as_ref().map_or_else(
                || vec![PrRow::text("Checks have not loaded")],
                |checks| {
                    let required = self.required_checks.as_ref();
                    // Required contexts no check reported for this head. They follow the
                    // check rows, so `check_at` stays one-to-one with `checks`.
                    let missing = required.map_or_else(Vec::new, |required| {
                        required
                            .states(checks)
                            .into_iter()
                            .filter_map(|(context, state)| match state {
                                crate::providers::gh::RequiredStateV1::NotReported => {
                                    Some((context, "expected, not reported"))
                                }
                                crate::providers::gh::RequiredStateV1::Unverified => {
                                    Some((context, "reported without the pinned app"))
                                }
                                _ => None,
                            })
                            .map(|(context, why)| PrRow {
                                label: format!(
                                    "{} {context} • required • {why} for head {}",
                                    SemanticToken::Pending.symbol(),
                                    summary.head_sha
                                ),
                                role: SemanticToken::Pending,
                                actor: None,
                                anchor: None,
                                comment: None,
                                link: None,
                                card: None,
                                cells: Some(PrCells::Row(vec![
                                    format!("{context} (required)"),
                                    "expected".into(),
                                    crate::ui_table::short_sha(&summary.head_sha).to_owned(),
                                ])),
                            })
                            .collect::<Vec<_>>()
                    });
                    if checks.is_empty() {
                        let mut rows = vec![PrRow::text(
                            "No checks/statuses reported for this exact head SHA",
                        )];
                        rows.extend(missing);
                        rows
                    } else {
                        let mut rows: Vec<PrRow> = checks
                            .iter()
                            .map(|check| {
                                let need = required.and_then(|r| r.required(check));
                                let marker = match need {
                                    Some(true) => " • required",
                                    Some(false) => " • not required",
                                    None if required.is_some() => " • required unknown",
                                    None => "",
                                };
                                // The shared CI mapping: an errored commit status fails and a
                                // skipped check is muted here exactly as in Actions and Inbox.
                                let role =
                                    SemanticToken::ci_state(&check.status, &check.conclusion);
                                PrRow {
                                    label: format!(
                                        "{} {} • {} / {} • {}{marker}",
                                        role.symbol(),
                                        check.name,
                                        check.status,
                                        if check.conclusion.is_empty() {
                                            "not concluded"
                                        } else {
                                            &check.conclusion
                                        },
                                        check.head_sha
                                    ),
                                    role,
                                    actor: None,
                                    anchor: None,
                                    comment: None,
                                    cells: Some(PrCells::Row(vec![
                                        if need == Some(true) {
                                            format!("{} (required)", check.name)
                                        } else {
                                            check.name.clone()
                                        },
                                        if check.conclusion.is_empty() {
                                            check.status.clone()
                                        } else {
                                            check.conclusion.clone()
                                        },
                                        crate::ui_table::short_sha(&check.head_sha).to_owned(),
                                    ])),
                                    link: None,
                                    card: None,
                                }
                            })
                            .collect();
                        rows.extend(missing);
                        rows
                    }
                },
            ),
            PrTab::Timeline => {
                let mut rows = timeline_projection_v1::prepare(&self.details.timeline)?;
                // A link survives only when its exact record is in these details.
                for row in &mut rows {
                    if row.link.as_ref().is_some_and(|link| !self.details_contain_v13(link)) {
                        row.link = None;
                    }
                }
                rows
            }
            PrTab::Progress => self.review_progress.as_ref().map_or_else(
                || vec![PrRow { role: SemanticToken::Pending, ..PrRow::text("Review progress UNAVAILABLE • no viewed state inferred. Refresh this exact PR, then open Review progress again.") }],
                |progress| progress.rows().into_iter().map(|(label,role)|PrRow {role, ..PrRow::text(label)}).collect(),
            ),
            PrTab::Metadata => metadata_rows_v51(
                self.context.as_ref().context("PR metadata has not loaded")?,
                self.metadata_display_v51.as_ref(),
            ),
        };
        if self.rows.is_empty() {
            self.rows
                .push(PrRow::text("No records reported for this section."));
        }
        if !self.details.coverage.complete {
            self.rows.insert(
                0,
                PrRow {
                    role: SemanticToken::Pending,
                    ..PrRow::text(self.details.coverage.reasons.join(" • "))
                },
            );
        }
        Ok(())
    }
    /// The Files tab's changed file at `row`: a file table row, or the file of a diff line
    /// (TASK-99.3 File history).
    pub fn file_path_at(&self, row: usize) -> Option<crate::git::RawPath> {
        if self.tab != PrTab::Files {
            return None;
        }
        let item = self.rows.get(row)?;
        if let Some(anchor) = &item.anchor {
            return anchor.new_path.clone().or_else(|| anchor.old_path.clone());
        }
        if !matches!(item.cells, Some(PrCells::Row(_))) {
            return None;
        }
        let index = self.rows[..row]
            .iter()
            .filter(|row| matches!(row.cells, Some(PrCells::Row(_))))
            .count();
        let file = self.details.files.get(index)?;
        Some(crate::git::RawPath(file.path.as_bytes().to_vec()))
    }
    pub fn selected_progress_file(&self, row: usize) -> Option<crate::pr_progress::FileProgress> {
        self.progress_file_at(row).cloned()
    }
    pub fn progress_file_at(&self, row: usize) -> Option<&crate::pr_progress::FileProgress> {
        if self.tab != PrTab::Progress {
            return None;
        }
        self.review_progress
            .as_ref()?
            .file_at(row.checked_sub(usize::from(!self.details.coverage.complete))?)
    }
    pub fn progress_selection(&self, path: &str) -> Option<usize> {
        if self.tab != PrTab::Progress {
            return None;
        }
        self.review_progress
            .as_ref()?
            .selected_row(path)
            .map(|row| row + usize::from(!self.details.coverage.complete))
    }
    /// Whether this PR's loaded details hold the exact record a Timeline link names.
    fn details_contain_v13(&self, link: &PrRowLinkV13) -> bool {
        match link {
            PrRowLinkV13::Comment(id) => self.details.comments.iter().any(|c| c.id == *id),
            PrRowLinkV13::Review(id) => self
                .details
                .reviews
                .iter()
                .any(|review| review["id"].as_u64() == Some(*id)),
            PrRowLinkV13::Commit(sha) => {
                *sha == self.details.summary.head_sha
                    || self.details.commits.iter().any(|commit| commit.sha == *sha)
            }
        }
    }
    /// The Timeline link on `row`, only while the Timeline tab is shown.
    pub fn timeline_link_at(&self, row: usize) -> Option<&PrRowLinkV13> {
        (self.tab == PrTab::Timeline)
            .then(|| self.rows.get(row)?.link.as_ref())
            .flatten()
    }
    /// The tab that shows a Timeline link's record: comments and reviews in Conversation,
    /// the current head commit in Checks (they ran on it), any other commit in Commits.
    pub fn timeline_target_v13(&self, link: &PrRowLinkV13) -> PrTab {
        match link {
            PrRowLinkV13::Comment(_) | PrRowLinkV13::Review(_) => PrTab::Conversation,
            PrRowLinkV13::Commit(sha) if *sha == self.details.summary.head_sha => PrTab::Checks,
            PrRowLinkV13::Commit(_) => PrTab::Commits,
        }
    }
    /// The row of the current tab that shows a Timeline link's record.
    pub fn link_row_v13(&self, link: &PrRowLinkV13) -> Option<usize> {
        match (self.tab, link) {
            (PrTab::Conversation, PrRowLinkV13::Comment(id)) => self.rows.iter().position(|row| {
                row.actor.is_some()
                    && row
                        .comment
                        .as_ref()
                        .is_some_and(|comment| comment.observed.id == *id)
            }),
            (PrTab::Conversation, PrRowLinkV13::Review(id)) => {
                let label = format!("review {id} ");
                self.rows
                    .iter()
                    .position(|row| row.actor.is_some() && row.label.starts_with(&label))
            }
            (PrTab::Commits, PrRowLinkV13::Commit(sha)) => self.rows.iter().position(|row| {
                matches!(row.cells, Some(PrCells::Row(_)))
                    && row.label.split(' ').next() == Some(sha.as_str())
            }),
            (PrTab::Checks, PrRowLinkV13::Commit(_)) => {
                (0..self.rows.len()).find(|row| self.check_at(*row).is_some())
            }
            _ => None,
        }
    }
}
/// Issue references after GitHub's closing keywords (`Fixes #12`,
/// `closes owner/repo#3`), in order and without duplicates. Read from the
/// already loaded Description only; nothing else is inferred.
pub(crate) fn closing_keyword_issues_v1(body: &str) -> Vec<String> {
    const KEYWORDS: [&str; 9] = [
        "close", "closes", "closed", "fix", "fixes", "fixed", "resolve", "resolves", "resolved",
    ];
    let valid = |reference: &str| {
        let Some((repository, number)) = reference.rsplit_once('#') else {
            return false;
        };
        let name = |part: &str| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        };
        !number.is_empty()
            && number.len() <= 10
            && number.bytes().all(|b| b.is_ascii_digit())
            && (repository.is_empty()
                || repository
                    .split_once('/')
                    .is_some_and(|(owner, repo)| name(owner) && name(repo)))
    };
    let words: Vec<&str> = body.split_whitespace().collect();
    let mut found: Vec<String> = Vec::new();
    for pair in words.windows(2) {
        let keyword = pair[0].trim_end_matches(':').to_ascii_lowercase();
        if !KEYWORDS.contains(&keyword.as_str()) {
            continue;
        }
        let reference = pair[1].trim_end_matches(['.', ',', ';', ':', ')', '!', '?']);
        if valid(reference) && !found.iter().any(|known| known == reference) {
            found.push(reference.to_owned());
        }
    }
    found
}
pub(crate) fn metadata_rows_v51(
    context: &crate::providers::gh::PrContext,
    observed: Option<&crate::providers::gh::MetadataDisplayFactsV51>,
) -> Vec<PrRow> {
    let permissions = &context.repository.permissions;
    let flag = |value: Option<bool>| match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "unavailable",
    };
    // Three-state: observed facts first, then the raw repository flags; only
    // an old context without either falls back to its boolean permissions.
    let permission_flags = observed
        .map(|facts| facts.permissions())
        .or(context.permission_flags_v13);
    let mut rows = vec![
        PrRow::text(format!(
            "Your repository permissions: read {} • write {} • maintain {} • admin {}",
            flag(permission_flags.map_or(Some(permissions.pull), |v| v[0])),
            flag(permission_flags.map_or(Some(permissions.push), |v| v[1])),
            flag(permission_flags.map_or(Some(permissions.maintain), |v| v[2])),
            flag(permission_flags.map_or(Some(permissions.admin), |v| v[3])),
        )),
        PrRow::text(fork_origin_v13(context)),
        PrRow::text(format!(
            "Base repository: {} • cross-repository {} • maintainer edits {}",
            context.base_repository.full_name,
            context.is_cross_repository,
            flag(observed.map_or(Some(context.maintainer_can_modify), |f| f.maintainer()))
        )),
        PrRow::text(format!(
            "Locked conversation: {}",
            flag(observed.map_or(
                Some(context.issue_metadata["locked"].as_bool().unwrap_or(false)),
                |f| f.locked()
            ))
        )),
        PrRow::text(format!(
            "Milestone: {}",
            if observed.is_some_and(|f| !f.milestone_available()) {
                "unavailable"
            } else {
                context.issue_metadata["milestone"]["title"]
                    .as_str()
                    .unwrap_or("none")
            }
        )),
    ];
    if let Some(facts) = observed {
        for (available, label) in [
            (facts.labels_available(), "Labels"),
            (facts.assignees_available(), "Assignees"),
            (facts.reviewers_available(), "Requested reviewers"),
            (facts.teams_available(), "Requested teams"),
        ] {
            if !available {
                rows.push(PrRow::text(format!(
                    "{label}: unavailable • no empty set inferred"
                )));
            }
        }
    }
    // Say "none" for an observed empty set so users can tell it apart from a
    // section that failed to load (reported as unavailable above).
    let empty = |key: &str| {
        context.issue_metadata[key]
            .as_array()
            .is_some_and(|items| items.is_empty())
    };
    if empty("labels") && observed.is_none_or(|f| f.labels_available()) {
        rows.push(PrRow::text("Labels: none"));
    }
    if empty("assignees") && observed.is_none_or(|f| f.assignees_available()) {
        rows.push(PrRow::text("Assignees: none"));
    }
    if context.requested_reviewers.is_empty()
        && context.requested_teams.is_empty()
        && observed.is_none_or(|f| f.reviewers_available() && f.teams_available())
    {
        rows.push(PrRow::text("Requested reviewers: none"));
    }
    for label in context.issue_metadata["labels"]
        .as_array()
        .into_iter()
        .flatten()
    {
        rows.push(PrRow::text(format!(
            "Label: {}",
            label["name"].as_str().unwrap_or("unknown")
        )));
    }
    for assignee in context.issue_metadata["assignees"]
        .as_array()
        .into_iter()
        .flatten()
    {
        rows.push(PrRow::actor("assigned", value_actor(assignee)));
    }
    for reviewer in &context.requested_reviewers {
        rows.push(PrRow::actor("review requested", reviewer.clone()));
    }
    // A team has no avatar: its row names the team and its @org/slug mention.
    let organization = context
        .base_repository
        .full_name
        .split_once('/')
        .map_or("", |(owner, _)| owner);
    for team in &context.requested_teams {
        rows.push(PrRow::text(
            if organization.is_empty() || team.slug.is_empty() {
                format!("Review team: {} ({})", team.name, team.slug)
            } else {
                format!(
                    "Review team: {} ({}) • @{organization}/{}",
                    team.name, team.slug, team.slug
                )
            },
        ));
    }
    let closing = closing_keyword_issues_v1(context.issue_metadata["body"].as_str().unwrap_or(""));
    rows.push(PrRow::text(if context.issue_metadata.get("body").is_none() {
        "Linked issues: inspect closing keywords in Description; GitHub resolves them on the default branch.".to_owned()
    } else if closing.is_empty() {
        "Linked issues: none via closing keywords in Description (issues linked in GitHub's sidebar are not listed here).".to_owned()
    } else {
        linked_issues_row_v13(&closing)
    }));
    for url in &context.source_urls {
        rows.push(PrRow::text(format!("Source: {url}")));
    }
    rows.push(PrRow::text(metadata_access_v1(permission_flags.unwrap_or(
        [
            Some(permissions.pull),
            Some(permissions.push),
            Some(permissions.maintain),
            Some(permissions.admin),
        ],
    ))));
    rows
}
/// TASK-13: the closing-keyword issues first, so a narrow pane still shows them.
pub(crate) fn linked_issues_row_v13(closing: &[String]) -> String {
    format!(
        "Linked issues: {} • closing keywords in Description; closed on merge to the default branch",
        closing.join(", ")
    )
}
/// TASK-13: where the head branch lives. A fork names its repository and the
/// base it targets; a same-repository PR says it is not from a fork.
pub(crate) fn fork_origin_v13(context: &crate::providers::gh::PrContext) -> String {
    match &context.head_repository {
        None => {
            "Fork origin: deleted or unavailable • the head repository is gone or hidden".into()
        }
        Some(head) if context.is_cross_repository => format!(
            "Fork origin: {} • fork of {}",
            head.full_name, context.base_repository.full_name
        ),
        Some(head) => format!("Fork origin: none • head branch is in {}", head.full_name),
    }
}
/// Plain-language summary of what the observed repository permissions allow
/// for PR metadata. GitHub still checks access when a change is previewed.
pub(crate) fn metadata_access_v1(flags: [Option<bool>; 4]) -> &'static str {
    let [_, push, maintain, admin] = flags;
    if [push, maintain, admin].contains(&Some(true)) {
        "You can change labels, assignees, reviewers and the milestone here."
    } else if [push, maintain, admin].iter().all(|v| *v == Some(false)) {
        "View only: changing labels, assignees, reviewers or the milestone needs triage or write access."
    } else {
        "Edit access unknown: GitHub checks it when you preview a change."
    }
}
fn value_actor(value: &serde_json::Value) -> crate::providers::gh::Actor {
    crate::providers::gh::Actor {
        login: value["login"].as_str().unwrap_or_default().into(),
        display_name: value["name"].as_str().map(str::to_owned),
        avatar_url: value["avatar_url"].as_str().map(str::to_owned),
        actor_type: value["type"].as_str().unwrap_or("User").into(),
    }
}
/// GitHub timestamps use UTC; display an age without making malformed data a zero.
pub fn age_label(timestamp: &str) -> String {
    let Some(created) = timestamp_seconds(timestamp) else {
        return "age unavailable".into();
    };
    format!(
        "{} old",
        crate::ui_table::age_from_seconds(now().saturating_sub(created))
    )
}
pub(crate) fn timestamp_seconds(value: &str) -> Option<u64> {
    if value.len() != 20
        || !value.is_ascii()
        || &value[4..5] != "-"
        || &value[7..8] != "-"
        || &value[10..11] != "T"
        || &value[13..14] != ":"
        || &value[16..17] != ":"
        || &value[19..] != "Z"
    {
        return None;
    }
    let year: i64 = value.get(..4)?.parse().ok()?;
    let month: i64 = value.get(5..7)?.parse().ok()?;
    let day: i64 = value.get(8..10)?.parse().ok()?;
    let hour: u64 = value.get(11..13)?.parse().ok()?;
    let minute: u64 = value.get(14..16)?.parse().ok()?;
    let second: u64 = value.get(17..19)?.parse().ok()?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&month)
        || day < 1
        || day > days[(month - 1) as usize]
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let adjusted = year - i64::from(month <= 2);
    let era = adjusted.div_euclid(400);
    let yoe = adjusted - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let days =
        era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * mp + 2) / 5 + day - 1 - 719468;
    u64::try_from(days)
        .ok()?
        .checked_mul(86400)?
        .checked_add(hour * 3600 + minute * 60 + second)
}
#[derive(Clone, Debug)]
pub struct GithubWorkspace {
    pub identity: HostIdentity,
    pub repositories: Vec<String>,
    pub exclusions: BTreeSet<String>,
    pub inbox: Inbox,
    pub filter: InboxFilter,
    pub selected: Option<PrKey>,
    pub freshness: Freshness,
    pub pr: Option<PrPage>,
    /// Actual page-install epoch only; never authentication/source authority.
    pub(crate) personal_pr_page_ticket_v13:
        Option<std::sync::Arc<crate::personal_pr_entry_v13::PersonalPrPageTicketV13>>,
    pub(crate) notification_pr_page_ticket_v63:
        Option<std::sync::Arc<crate::notification_pr_v63::NotificationPrPageTicketV63>>,
    /// Navigation only; never authentication, permission or restored approval.
    pub scoped_pr_navigation:
        Option<std::sync::Arc<crate::scoped_pr_observed_tabs_v1::ScopedPrNavigationV1>>,
    /// Closed observed resource sidecar; never permissions or a write permit.
    pub scoped_pr_resource: Option<
        std::sync::Arc<crate::scoped_pr_observed_tabs_v1::resources::ScopedPrResourceViewV1>,
    >,
    pub needs_refresh: bool,
    pub generation: u64,
    // Negative display state only: a fresh exact PR page does not certify the
    // old aggregate queue/filter membership or counts. Never access authority.
    state_membership_unavailable_v68: bool,
    // Producer-issued read intent only. Neither missing scope fields nor a
    // decoded/cached queue can manufacture this actual startup lineage.
    legacy_startup_queue_v80: Option<std::sync::Arc<LegacyStartupQueueV80>>,
    state_source_route_v80: Option<std::sync::Arc<StateSourceRouteHintV80>>,
    /// TASK-12 AC2: review decision and check rollup GitHub reported for the rows of the
    /// last ordinary Inbox load, keyed like the session memo. Display only.
    pub inbox_rollup_v1: std::collections::BTreeMap<PrKey, crate::inbox_status_v1::RowStatus>,
    /// Reviewers and commenters from the same Inbox read, for the preview pane.
    pub inbox_reviewers_v1:
        std::collections::BTreeMap<PrKey, Vec<crate::providers::gh::InboxReviewerV1>>,
}
impl GithubWorkspace {
    /// SAME borrowed target predicate for painting, mouse hit regions and key
    /// traversal. Metadata/notices do not become targets through a cursor.
    pub fn pr_row_is_interactive(&self, index: usize) -> bool {
        let Some(page) = self.pr.as_ref() else {
            return false;
        };
        let Some(row) = page.rows.get(index) else {
            return false;
        };
        row.comment.is_some()
            || page.timeline_link_at(index).is_some() && self.scoped_pr_navigation.is_none()
            || page.progress_file_at(index).is_some()
            || page.check_at(index).is_some()
            || self
                .scoped_pr_resource
                .as_ref()
                .and_then(|view| view.row(index))
                .is_some_and(|target| {
                    !matches!(
                        target,
                        crate::scoped_pr_observed_tabs_v1::resources::PrResourceRowV1::Notice
                    )
                })
    }
    /// A direction chooses only existing interactive rows. At the edge retain
    /// a real target; a metadata/out-of-range starting cursor clamps to the
    /// nearest real boundary. None leaves read-only detail scrolling intact.
    pub fn pr_row_navigation_target(&self, from: usize, down: bool) -> Option<usize> {
        let length = self.pr.as_ref()?.rows.len();
        let from = from.min(length.checked_sub(1)?);
        let next = if down {
            (from.saturating_add(1)..length).find(|index| self.pr_row_is_interactive(*index))
        } else {
            (0..from)
                .rev()
                .find(|index| self.pr_row_is_interactive(*index))
        };
        next.or_else(|| self.pr_row_is_interactive(from).then_some(from))
            .or_else(|| {
                if down {
                    (0..=from)
                        .rev()
                        .find(|index| self.pr_row_is_interactive(*index))
                } else {
                    (from..length).find(|index| self.pr_row_is_interactive(*index))
                }
            })
    }
    pub fn empty(
        identity: HostIdentity,
        repositories: Vec<String>,
        exclusions: BTreeSet<String>,
    ) -> Self {
        Self {
            inbox: Inbox {
                identity: identity.clone(),
                pull_requests: vec![],
                coverage: Coverage::default(),
            },
            identity,
            repositories,
            exclusions,
            filter: InboxFilter::All,
            selected: None,
            freshness: Freshness::Empty,
            pr: None,
            personal_pr_page_ticket_v13: None,
            notification_pr_page_ticket_v63: None,
            scoped_pr_navigation: None,
            scoped_pr_resource: None,
            needs_refresh: true,
            generation: 0,
            state_membership_unavailable_v68: false,
            legacy_startup_queue_v80: None,
            state_source_route_v80: None,
            inbox_rollup_v1: Default::default(),
            inbox_reviewers_v1: Default::default(),
        }
    }
    pub fn scope(&self) -> Result<Scope> {
        Scope::github(&self.identity.host, self.identity.account_id)
    }
    pub fn source(&self) -> String {
        let mut repositories = self.repositories.clone();
        repositories.sort();
        repositories.dedup();
        format!(
            "personal-inbox:{}",
            json!({"repositories":repositories,"excluded":self.exclusions})
        )
    }
    pub fn visible(&self) -> Vec<&PullRequest> {
        self.inbox
            .pull_requests
            .iter()
            .filter(|pr| self.filter.accepts(pr) && !self.exclusions.contains(&pr.repository))
            .collect()
    }
    /// Loaded PRs per queue (same order as `InboxFilter::ALL`), after exclusions.
    pub fn queue_counts(&self) -> [usize; 4] {
        InboxFilter::ALL.map(|(filter, _, _)| {
            self.inbox
                .pull_requests
                .iter()
                .filter(|pr| filter.accepts(pr) && !self.exclusions.contains(&pr.repository))
                .count()
        })
    }
    pub fn selected_index(&self) -> usize {
        self.visible()
            .iter()
            .position(|pr| self.selected.as_ref() == Some(&PrKey::from(*pr)))
            .unwrap_or(0)
    }
    pub fn select_index(&mut self, index: usize) {
        self.selected = self.visible().get(index).map(|pr| PrKey::from(*pr));
    }
    pub fn set_filter(&mut self, filter: InboxFilter) {
        self.filter = filter;
        self.preserve_selection();
    }
    fn preserve_selection(&mut self) {
        if !self
            .visible()
            .iter()
            .any(|pr| self.selected.as_ref() == Some(&PrKey::from(*pr)))
        {
            self.select_index(0);
        }
    }
    pub fn context(&self) -> String {
        if self.identity.account_id.is_none() {
            return format!(
                "{} / identity unavailable • private data hidden",
                self.identity.host
            );
        }
        let context = format!(
            "{} / @{} / {}",
            self.identity.host,
            self.identity.login,
            if self.repositories.is_empty() {
                "all repositories on host".into()
            } else {
                self.repositories.join(", ")
            }
        );
        self.pr.as_ref().map_or(context.clone(), |page| {
            format!(
                "{context} / {}#{} / {}",
                page.details.summary.pr.repository,
                page.details.summary.pr.number,
                page.details.summary.head_sha
            )
        })
    }
    pub fn coverage_label(&self) -> String {
        if self.state_membership_unavailable_v68 {
            return "PR queue membership/counts unavailable • refresh this scope explicitly".into();
        }
        format!(
            "{} unique PRs across {} queue matches • {}{}",
            self.inbox.pull_requests.len(),
            self.inbox.coverage.total_matches,
            if self.inbox.coverage.complete {
                "complete"
            } else {
                "PARTIAL"
            },
            if self.exclusions.is_empty() {
                String::new()
            } else {
                format!(" • {} excluded repos", self.exclusions.len())
            }
        )
    }
    pub(crate) fn state_membership_unavailable_v68(&self) -> bool {
        self.state_membership_unavailable_v68
    }
    pub(crate) fn retire_state_membership_v68(&mut self) {
        self.state_membership_unavailable_v68 = true;
        self.inbox.pull_requests.clear();
        self.inbox.coverage.loaded = 0;
        self.inbox.coverage.complete = false;
        self.inbox.coverage.reasons = vec![
            "PR lifecycle changed • old filter membership/counts are unavailable, not empty".into(),
        ];
        self.needs_refresh = true;
    }
    pub fn actors(&self) -> Vec<crate::providers::gh::Actor> {
        crate::actor_avatar_projection_v1::github(self, None).unwrap_or_default()
    }
    pub fn cached(mut self, path: &Path) -> Result<Self> {
        if let Err(error) = self.scope() {
            self.inbox.pull_requests.clear();
            self.pr = None;
            self.personal_pr_page_ticket_v13 = None;
            self.notification_pr_page_ticket_v63 = None;
            self.scoped_pr_navigation = None;
            self.scoped_pr_resource = None;
            self.needs_refresh = false;
            self.freshness = Freshness::Offline {
                fetched_at: 0,
                error: error.to_string(),
            };
            return Ok(self);
        }
        let cache = Cache::open(path)?;
        if let Some(frame) = cache.sync_frame(&self.scope()?, &self.source())? {
            let meta = frame
                .records
                .iter()
                .find(|record| record.kind == "inbox-meta");
            if let Some(meta) = meta {
                self.inbox.coverage = serde_json::from_value(meta.payload["coverage"].clone())?;
            }
            for record in frame.records.iter().filter(|record| record.kind == "pr") {
                let pr: PullRequest = serde_json::from_value(record.payload.clone())?;
                if pr.host != self.identity.host
                    || (!self.repositories.is_empty()
                        && !self.repositories.contains(&pr.repository))
                {
                    bail!("Cached PR provenance does not match the selected scope");
                }
                if !self.exclusions.contains(&pr.repository) {
                    self.inbox.pull_requests.push(pr);
                }
            }
            self.inbox.coverage.loaded = self.inbox.pull_requests.len();
            self.inbox.coverage.complete &= frame.complete;
            if !frame.complete {
                self.inbox
                    .coverage
                    .reasons
                    .push("Cached source is partial or access was revoked".into());
            }
            self.generation = frame.generation;
            self.freshness = Freshness::Cached {
                fetched_at: frame.fetched_at,
            };
            self.sort();
            self.preserve_selection();
        }
        Ok(self)
    }
    pub fn refresh(
        self,
        provider: &GithubProvider,
        path: &Path,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        self.refresh_with_scope(None, None, provider, path, cancel)
    }
    /// Exact navigation never seeds name-only cached frames or rebases an
    /// original positive account. Current repository access and closing proof
    /// are still the caller's responsibility.
    pub fn refresh_exact_scope(
        self,
        expected_scope: &Scope,
        provider: &GithubProvider,
        path: &Path,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        self.refresh_with_scope(Some(expected_scope), None, provider, path, cancel)
    }
    /// Same exact worker, with non-authoritative optional metadata consistency.
    pub(crate) fn refresh_exact_scope_current(
        self,
        expected_scope: &Scope,
        metadata: &crate::providers::gh::ExactInboxRepositoryMetadataV1,
        provider: &GithubProvider,
        path: &Path,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        self.refresh_with_scope(Some(expected_scope), Some(metadata), provider, path, cancel)
    }
    fn refresh_with_scope(
        mut self,
        expected_scope: Option<&Scope>,
        repository_metadata: Option<&crate::providers::gh::ExactInboxRepositoryMetadataV1>,
        provider: &GithubProvider,
        path: &Path,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        self.needs_refresh = false;
        let identity = match provider.identity(cancel) {
            Ok(identity) => identity,
            Err(error) if expected_scope.is_some() => return Err(error.into()),
            Err(error) => return Ok(self.hide_private(&error.to_string())),
        };
        if let Some(expected_scope) = expected_scope {
            if Scope::github(&identity.host, identity.account_id)? != *expected_scope {
                bail!("Exact inbox scope changed; previous private pages must remain hidden");
            }
            if self.repositories.is_empty() {
                bail!(
                    "Exact inbox scope has no repository; personal all-scope queries are forbidden"
                );
            }
            self = Self::empty(identity, self.repositories, self.exclusions);
            self.needs_refresh = false;
        } else if identity.host != self.identity.host
            || identity.login != self.identity.login
            || identity.account_id != self.identity.account_id
        {
            // Deliberately discard old private pages before loading another account.
            self = Self::empty(identity, self.repositories, self.exclusions).cached(path)?;
            self.needs_refresh = false;
        }
        if self.identity.account_id.is_none() {
            return self.cached(path);
        }
        if let Some(metadata) = repository_metadata {
            metadata.validate_scope(&self.identity, &self.repositories)?;
        }
        let mut cache = Cache::open(path)?;
        let source = self.source();
        let scope = self.scope()?;
        let generation = if expected_scope.is_some() {
            cache
                .sync_checkpoint(&scope, &source)?
                .map_or(1, |(generation, _)| generation + 1)
        } else {
            cache
                .sync_frame(&scope, &source)?
                .map_or(1, |frame| frame.generation + 1)
        };
        let inbox = if expected_scope.is_some() {
            match repository_metadata {
                Some(metadata) => provider.personal_inbox_exact_scope_current(
                    &self.identity,
                    &self.repositories,
                    metadata,
                    cancel,
                ),
                None => {
                    provider.personal_inbox_exact_scope(&self.identity, &self.repositories, cancel)
                }
            }
        } else {
            provider.personal_inbox(&self.identity, &self.repositories, cancel)
        };
        match inbox {
            Ok(mut inbox) => {
                if expected_scope.is_some()
                    && inbox.pull_requests.iter().any(|pr| {
                        pr.host != self.identity.host
                            || !self
                                .repositories
                                .iter()
                                .any(|repository| repository.eq_ignore_ascii_case(&pr.repository))
                    })
                {
                    bail!(
                        "Exact inbox response contains a PR outside the selected host/repositories"
                    );
                }
                inbox
                    .pull_requests
                    .retain(|pr| !self.exclusions.contains(&pr.repository));
                inbox.coverage.loaded = inbox.pull_requests.len();
                let fetched_at = now();
                let mut records = vec![Record::new(
                    "inbox-meta",
                    &source,
                    "",
                    json!({"coverage":inbox.coverage}),
                )];
                records[0].complete = inbox.coverage.complete;
                for pr in &inbox.pull_requests {
                    let mut record = Record::new(
                        "pr",
                        PrKey::from(pr).label(),
                        &pr.repository,
                        serde_json::to_value(pr)?,
                    );
                    record.source_updated_at = Some(pr.updated_at.clone());
                    record.complete = true;
                    record.fetched_at = fetched_at;
                    records.push(record);
                }
                if cancel.is_cancelled() {
                    bail!("Inbox refresh cancelled");
                }
                if !cache.commit_sync(
                    &scope,
                    &source,
                    generation,
                    &records,
                    inbox.coverage.complete,
                )? {
                    bail!("A newer inbox generation already finished; refresh again");
                }
                self.generation = generation;
                self.inbox = inbox;
                self.sort();
                self.preserve_selection();
                self.freshness = Freshness::Fresh { fetched_at };
                if expected_scope.is_none() {
                    // A bounded State recovery stamp is optional; its refusal
                    // must not change the old ordinary refresh result.
                    if self.install_legacy_queue_v80(None).is_err() {
                        self.legacy_startup_queue_v80 = None;
                    }
                }
            }
            Err(error) if expected_scope.is_some() => return Err(error.into()),
            Err(error) if !cancel.is_cancelled() => {
                let fetched_at = match self.freshness {
                    Freshness::Cached { fetched_at }
                    | Freshness::Fresh { fetched_at }
                    | Freshness::Offline { fetched_at, .. } => fetched_at,
                    Freshness::Empty => 0,
                };
                self.freshness = Freshness::Offline {
                    fetched_at,
                    error: error.to_string(),
                };
            }
            Err(error) => return Err(error.into()),
        }
        Ok(self)
    }
    fn sort(&mut self) {
        self.inbox.pull_requests.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then_with(|| a.repository.cmp(&b.repository))
                .then_with(|| a.number.cmp(&b.number))
        });
    }
    pub fn open(
        mut self,
        key: &PrKey,
        provider: &GithubProvider,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        let already_open = self
            .pr
            .as_ref()
            .is_some_and(|page| PrKey::from(&page.details.summary.pr) == *key);
        if key.host != self.identity.host
            || (!already_open && !self.visible().iter().any(|pr| PrKey::from(*pr) == *key))
        {
            bail!("PR target is outside the loaded account/scope");
        }
        let details = provider.pr_details(&key.repository, key.number, cancel)?;
        self.install_pr_details(key, details)?;
        Ok(self)
    }

    /// Open an explicitly scoped discovery result without inventing personal queue membership.
    /// The caller verifies the current principal before and after the read, just as for search.
    pub fn open_search_result(
        mut self,
        result: &crate::providers::gh::RemoteSearchItem,
        provider: &GithubProvider,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        if result.kind != crate::providers::gh::RemoteSearchKind::PullRequests
            || result.host != self.identity.host
            || result.id.is_empty()
            || self.identity.account_id.is_none_or(|id| id == 0)
            || result.number.is_none_or(|number| number == 0)
            || self
                .exclusions
                .iter()
                .any(|repo| repo.eq_ignore_ascii_case(&result.repository))
            || !self
                .repositories
                .iter()
                .any(|repo| repo.eq_ignore_ascii_case(&result.repository))
        {
            bail!("Discovered PR is outside the verified explicit repository scope");
        }
        let key = PrKey {
            host: result.host.clone(),
            repository: result.repository.clone(),
            number: result.number.unwrap(),
        };
        let details = provider.pr_details(&key.repository, key.number, cancel)?;
        self.install_pr_details(&key, details)?;
        self.needs_refresh = false;
        Ok(self)
    }

    /// Native metric drill without invented personal queue memberships. Root
    /// brackets these legacy detail reads with positive numeric repository/PR
    /// identity, current principal and original journal proofs.
    pub fn open_metric_pr(
        mut self,
        original: &crate::providers::gh::MetricsPrObservation,
        provider: &GithubProvider,
        cancel: &CancellationToken,
    ) -> crate::providers::gh::GithubResult<Self> {
        use crate::providers::gh::{GithubError, GithubErrorKind};
        let refuse = |message: &str| GithubError {
            kind: GithubErrorKind::InvalidTarget,
            message: message.into(),
        };
        if original.repository.host != self.identity.host
            || self.identity.account_id.is_none_or(|id| id == 0)
            || original.repository.id == 0
            || original.id == 0
            || original.number == 0
            || original.node_id.is_empty()
            || original
                .head_sha
                .as_ref()
                .is_none_or(|head| !crate::search::model::valid_sha(head))
            || !self
                .repositories
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&original.repository.full_name))
            || self
                .exclusions
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&original.repository.full_name))
        {
            return Err(refuse(
                "Metric PR is outside its verified explicit canonical scope",
            ));
        }
        let key = PrKey {
            host: self.identity.host.clone(),
            repository: original.repository.full_name.clone(),
            number: original.number,
        };
        let details = provider.pr_details(&key.repository, key.number, cancel)?;
        if details.summary.pr.id != original.node_id
            || details.summary.head_sha.as_str() != original.head_sha.as_deref().unwrap_or_default()
        {
            return Err(GithubError {kind:GithubErrorKind::StaleHead,message:"Metric PR node/head changed while loading native details; refresh the original scope".into()});
        }
        self.install_pr_details(&key, details)
            .map_err(|error| GithubError {
                kind: GithubErrorKind::Json,
                message: error.to_string(),
            })?;
        self.needs_refresh = false;
        self.freshness = Freshness::Fresh { fetched_at: now() };
        Ok(self)
    }

    fn install_pr_details(&mut self, key: &PrKey, details: PrDetails) -> Result<()> {
        let canonical = &details.summary.pr;
        if canonical.host != key.host
            || !canonical.repository.eq_ignore_ascii_case(&key.repository)
            || canonical.number != key.number
            || canonical.id.is_empty()
        {
            bail!("Fetched PR details do not match the exact selected host/repository/number");
        }
        let mut page = PrPage {
            details,
            tab: PrTab::Summary,
            diff: None,
            checks: None,
            required_checks: None,
            rows: vec![],
            context: None,
            metadata_display_v51: None,
            review_progress: None,
        };
        page.prepare_rows()?;
        self.selected = Some(key.clone());
        self.pr = Some(page);
        self.personal_pr_page_ticket_v13 =
            Some(crate::personal_pr_entry_v13::PersonalPrPageTicketV13::new());
        self.notification_pr_page_ticket_v63 = None;
        self.scoped_pr_navigation = None;
        self.scoped_pr_resource = None;
        Ok(())
    }

    /// Install fresh, identity-bound provider details and rebuild rows without I/O or acknowledgement.
    pub fn replace_pr_details(&mut self, details: PrDetails) -> Result<()> {
        let current = self.pr.as_ref().context("No exact PR is open")?;
        let previous = &current.details.summary.pr;
        let fresh = &details.summary.pr;
        if previous.host != fresh.host
            || previous.id != fresh.id
            || previous.number != fresh.number
            || !previous.repository.eq_ignore_ascii_case(&fresh.repository)
        {
            bail!("Fresh details belong to a different PR; no selected action target was replaced");
        }
        let tab = current.tab;
        let key = PrKey::from(previous);
        self.install_pr_details(&key, details)?;
        let page = self.pr.as_mut().unwrap();
        page.tab = tab;
        page.prepare_rows()?;
        self.freshness = Freshness::Fresh { fetched_at: now() };
        Ok(())
    }

    /// Worker-only replacement of freshly inspected details. Metadata cannot
    /// reuse its previous context: title/base edits and independent metadata
    /// changes must be read from the new exact target before restoring that tab.
    /// Other tabs retain the existing provider-free replacement semantics.
    pub fn replace_pr_details_with_current_context(
        mut self,
        details: PrDetails,
        provider: &GithubProvider,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        let metadata = self.pr.as_ref().context("No exact PR is open")?.tab == PrTab::Metadata;
        if metadata {
            // install_pr_details discards context. Build the safe Summary first;
            // only a successful canonical context read may restore Metadata.
            self.pr.as_mut().unwrap().tab = PrTab::Summary;
        }
        self.replace_pr_details(details)?;
        if metadata {
            self = self.load_tab(PrTab::Metadata, provider, cancel)?;
            let current = provider.identity(cancel)?;
            if !self.identity.same_account(&current) {
                bail!("Account changed while restoring fresh PR metadata; original publication remains confirmed; no resend");
            }
            self.identity = current;
        }
        Ok(self)
    }
    pub fn load_tab(
        mut self,
        tab: PrTab,
        provider: &GithubProvider,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        if tab == PrTab::Progress {
            let target = self
                .pr
                .as_ref()
                .context("No PR is open")?
                .details
                .summary
                .clone();
            match crate::pr_progress::read(provider, &self.identity, &target, cancel) {
                Ok(progress) => self.pr.as_mut().unwrap().review_progress = Some(progress),
                Err(error) if crate::pr_ui::account_failure(&error) => {
                    return Ok(self.hide_private(&error))
                }
                Err(error) => {
                    self.pr.as_mut().unwrap().review_progress = None;
                    self.freshness = Freshness::Offline {
                        fetched_at: now(),
                        error,
                    };
                }
            }
        }
        let page = self.pr.as_mut().context("No PR is open")?;
        let target = &page.details.summary;
        match tab {
            PrTab::Files if page.diff.is_none() => {
                page.diff = Some(provider.pr_diff_at(target, cancel)?)
            }
            PrTab::Checks if page.checks.is_none() => {
                let checks = provider.checks(&target.pr.repository, &target.head_sha, cancel)?;
                let required =
                    provider.required_checks(&target.pr.repository, &target.base_ref, cancel)?;
                page.checks = Some(checks);
                page.required_checks = Some(required);
            }
            PrTab::Metadata if page.context.is_none() => {
                page.context = Some(provider.pr_context(target, cancel)?)
            }
            _ => {}
        }
        page.tab = tab;
        page.prepare_rows()?;
        self.personal_pr_page_ticket_v13 =
            Some(crate::personal_pr_entry_v13::PersonalPrPageTicketV13::new());
        self.notification_pr_page_ticket_v63 = None;
        Ok(self)
    }
    /// An explicit PR refresh inspects the resource before releasing an unknown
    /// write outcome. Ordinary inbox refresh/cache clearing cannot release it.
    pub fn reconcile_page(
        mut self,
        provider: &GithubProvider,
        path: &Path,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        let identity = match provider.identity(cancel) {
            Ok(identity) => identity,
            Err(error) => return Ok(self.hide_private(&error.to_string())),
        };
        if identity.host != self.identity.host
            || identity.login != self.identity.login
            || identity.account_id != self.identity.account_id
        {
            return Self::empty(identity, self.repositories, self.exclusions)
                .cached(path)?
                .refresh(provider, path, cancel);
        }
        let current = self.pr.as_ref().context("No PR is open")?;
        let tab = current.tab;
        let details = provider.reconcile_pr(
            &current.details.summary.pr.repository,
            current.details.summary.pr.number,
            cancel,
        )?;
        self.scoped_pr_navigation = None;
        self.scoped_pr_resource = None;
        self.pr = Some(PrPage {
            details,
            tab: PrTab::Summary,
            diff: None,
            checks: None,
            required_checks: None,
            rows: vec![],
            context: None,
            metadata_display_v51: None,
            review_progress: None,
        });
        self.freshness = Freshness::Fresh { fetched_at: now() };
        self.load_tab(tab, provider, cancel)
    }
    pub fn clear_cache(mut self, path: &Path) -> Result<Self> {
        let state_read_intent = self.state_source_sync_required_v80();
        Cache::open(path)?.clear_records(&self.scope()?)?;
        if !state_read_intent {
            self.legacy_startup_queue_v80 = None;
        }
        self.inbox.pull_requests.clear();
        self.inbox.coverage = Coverage::default();
        self.selected = None;
        self.pr = None;
        self.scoped_pr_navigation = None;
        self.scoped_pr_resource = None;
        self.freshness = Freshness::Empty;
        self.personal_pr_page_ticket_v13 = None;
        self.notification_pr_page_ticket_v63 = None;
        self.needs_refresh = false;
        Ok(self)
    }
    /// The Inbox view's load follows a fresh ordinary queue read with this: one read-only
    /// GraphQL batch per [`crate::providers::gh::INBOX_ROLLUP_BATCH_V1`] rows. `refresh`
    /// itself stays the queue read only. Nothing is sent without a verified account, a fresh
    /// queue or any row. A refused or failed batch leaves those rows to the session memo
    /// (older rollups for them stay, and read as stale once the queue shows a newer update).
    pub fn load_inbox_rollup_v1(&mut self, provider: &GithubProvider, cancel: &CancellationToken) {
        use crate::inbox_status_v1::RowStatus;
        use crate::providers::gh::{inbox_rollup_order_v1, INBOX_ROLLUP_BATCH_V1};
        if self.identity.account_id.is_none() {
            self.inbox_rollup_v1.clear();
            self.inbox_reviewers_v1.clear();
            return;
        }
        if !matches!(self.freshness, Freshness::Fresh { .. }) {
            return;
        }
        let current: std::collections::BTreeMap<PrKey, String> = self
            .inbox
            .pull_requests
            .iter()
            .map(|pr| {
                (
                    crate::inbox_status_v1::row_key(&pr.host, &pr.repository, pr.number),
                    pr.updated_at.clone(),
                )
            })
            .collect();
        self.inbox_rollup_v1
            .retain(|key, _| current.contains_key(key));
        self.inbox_reviewers_v1
            .retain(|key, _| current.contains_key(key));
        let mut targets: Vec<(String, u64)> = self
            .inbox
            .pull_requests
            .iter()
            .map(|pr| (pr.repository.clone(), pr.number))
            .collect();
        inbox_rollup_order_v1(&mut targets);
        for chunk in targets.chunks(INBOX_ROLLUP_BATCH_V1) {
            if cancel.is_cancelled() {
                return;
            }
            let batch: Vec<(&str, u64)> = chunk.iter().map(|(r, n)| (r.as_str(), *n)).collect();
            let Ok(rows) = provider.inbox_rollup_v1(&batch, cancel) else {
                continue;
            };
            for row in rows {
                let key = crate::inbox_status_v1::row_key(
                    &self.identity.host,
                    &row.repository,
                    row.number,
                );
                let Some(updated_at) = current.get(&key) else {
                    continue;
                };
                self.inbox_rollup_v1
                    .insert(key.clone(), RowStatus::from_rollup_v1(&row, updated_at));
                match row.reviewers {
                    Some(reviewers) => self.inbox_reviewers_v1.insert(key, reviewers),
                    None => self.inbox_reviewers_v1.remove(&key),
                };
            }
        }
    }
    fn hide_private(mut self, error: &str) -> Self {
        self.inbox_rollup_v1.clear();
        self.inbox_reviewers_v1.clear();
        self.legacy_startup_queue_v80 = None;
        self.state_source_route_v80 = None;
        self.identity.account_id = None;
        self.inbox.pull_requests.clear();
        self.inbox.coverage.complete = false;
        self.inbox.coverage.reasons =
            vec!["Identity verification failed; private pages hidden".into()];
        self.pr = None;
        self.scoped_pr_navigation = None;
        self.scoped_pr_resource = None;
        self.selected = None;
        self.personal_pr_page_ticket_v13 = None;
        self.notification_pr_page_ticket_v63 = None;
        self.needs_refresh = false;
        self.freshness = Freshness::Offline {
            fetched_at: 0,
            error: format!("Identity unavailable • private data hidden: {error}"),
        };
        self
    }
}

#[cfg(test)]
#[path = "github_row_navigation_tests_v1.rs"]
mod row_navigation_tests_v1;
#[cfg(test)]
mod tests {
    use super::*;
    fn workspace() -> GithubWorkspace {
        GithubWorkspace::empty(
            HostIdentity {
                host: "github.com".into(),
                login: "alice".into(),
                gh_version: "gh version 2.99.0".into(),
                account_id: Some(42),
            },
            vec![],
            BTreeSet::new(),
        )
    }
    fn pr(repo: &str, number: u64, queues: &[Queue]) -> PullRequest {
        PullRequest {
            host: "github.com".into(),
            id: format!("{repo}#{number}"),
            repository: repo.into(),
            number,
            title: "A change".into(),
            body: String::new(),
            url: String::new(),
            author: Default::default(),
            state: "open".into(),
            draft: false,
            created_at: String::new(),
            updated_at: String::new(),
            queues: queues.iter().cloned().collect(),
        }
    }
    #[test]
    fn personal_inbox_overlapping_filters_and_cross_repo_selection() {
        let mut workspace = workspace();
        workspace.inbox.pull_requests = vec![
            pr("org/a", 1, &[Queue::Authored, Queue::Assigned]),
            pr("org/b", 1, &[Queue::ReviewRequested]),
        ];
        workspace.select_index(1);
        workspace.inbox.pull_requests.reverse();
        assert_eq!(workspace.selected_index(), 0);
        workspace.set_filter(InboxFilter::Assigned);
        assert_eq!(workspace.visible().len(), 1);
        assert_eq!(workspace.selected.as_ref().unwrap().repository, "org/a");
        workspace.set_filter(InboxFilter::Authored);
        assert_eq!(workspace.selected_index(), 0);
    }
    #[test]
    fn cache_storage_workspace_provenance_restart_revocation_and_private_drafts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("private/cache.sqlite3");
        let workspace = workspace();
        let scope = workspace.scope().unwrap();
        let source = workspace.source();
        let mut cache = Cache::open(&path).unwrap();
        cache.put_draft(&scope, "review", "Keep draft").unwrap();
        cache.commit_sync(&scope,&source,1,&[Record::new("pr","org/a#1","org/a",serde_json::to_value(pr("org/a",1,&[Queue::Assigned])).unwrap()),Record::new("inbox-meta",&source,"",json!({"coverage":Coverage{loaded:1,total_matches:1,complete:true,reasons:vec![]}}))],true).unwrap();
        let cached = workspace.clone().cached(&path).unwrap();
        assert_eq!(cached.visible().len(), 1);
        assert!(matches!(cached.freshness, Freshness::Cached { .. }));
        let mut other = workspace.clone();
        other.identity.login = "bob".into();
        other.identity.account_id = Some(43);
        assert!(other.cached(&path).unwrap().visible().is_empty());
        cache.revoke_repository(&scope, "org/a").unwrap();
        let revoked = workspace.cached(&path).unwrap();
        assert!(revoked.visible().is_empty());
        assert!(!revoked.inbox.coverage.complete);
        revoked.clear_cache(&path).unwrap();
        assert_eq!(
            cache.draft(&scope, "review").unwrap().as_deref(),
            Some("Keep draft")
        );
    }
}

#[cfg(test)]
#[path = "github_exact_scope_tests.rs"]
mod exact_scope_tests;

#[cfg(test)]
mod v1_pr_reviews_tests {
    use super::review_decisions;
    use crate::forms::{PrFormKind, PrIntent};
    use crate::providers::gh::{HostIdentity, PrSummary};
    use crate::theme::SemanticToken;
    use serde_json::json;

    fn summary(author: &str, state: &str, merged: bool) -> PrSummary {
        serde_json::from_value(json!({"pr":{"host":"github.invalid","id":"PR_7","repository":"example/repo","number":7,"title":"T","body":"","url":"https://github.invalid/example/repo/pull/7","author":{"login":author,"actor_type":"User"},"state":state,"draft":false,"created_at":"t0","updated_at":"t0","queues":[]},"head_sha":"a".repeat(40),"base_sha":"b".repeat(40),"head_ref":"feature","base_ref":"main","merged":merged,"auto_merge":false,"merge_commit_sha":null,"mergeable":true,"mergeable_state":"clean","additions":0,"deletions":0,"changed_files":0})).unwrap()
    }
    fn review(login: &str, state: &str) -> serde_json::Value {
        json!({"user":{"login":login},"state":state})
    }

    #[test]
    fn v1_pr_reviews_decisions_keep_latest_decision_per_reviewer() {
        assert_eq!(review_decisions(&[]), "Reviews: none yet");
        let reviews = [
            review("hubot", "CHANGES_REQUESTED"),
            review("octocat", "COMMENTED"),
            review("hubot", "APPROVED"),
            review("hubot", "COMMENTED"),
            review("monalisa", "PENDING"),
            review("octocat", "CHANGES_REQUESTED"),
        ];
        assert_eq!(
            review_decisions(&reviews),
            "Reviews: @hubot approved • @octocat requested changes • @monalisa pending review (not submitted)"
        );
    }

    #[test]
    fn audit_v64_review_decisions_use_the_shared_identity_label() {
        // TASK-64 AC3: bots, deleted (ghost) and missing reviewers read as the shared identity
        // fallback says, never as a bare `@login` and never as a fake `@unknown` account.
        let reviews = [
            json!({"user":{"login":"ci-helper","type":"Bot"},"state":"APPROVED"}),
            json!({"user":null,"state":"COMMENTED"}),
            json!({"user":null,"state":"CHANGES_REQUESTED"}),
            json!({"user":{"login":"ghost","type":"User"},"state":"DISMISSED"}),
            json!({"user":{"login":"acme","type":"Organization"},"state":"COMMENTED"}),
        ];
        let line = review_decisions(&reviews);
        assert_eq!(
            line,
            "Reviews: @ci-helper [bot] approved • unknown actor [unknown] commented • \
             unknown actor [unknown] requested changes • @ghost [deleted] review dismissed • \
             @acme [organization] commented"
        );
        assert!(!line.contains("@unknown"), "{line}");
    }

    #[test]
    fn audit_v64_pr_checks_rows_use_the_shared_ci_state_tokens() {
        // An errored commit status fails and a skipped check is muted, as in Actions/Inbox.
        for (status, conclusion, token) in [
            ("completed", "error", SemanticToken::Failure),
            ("completed", "startup_failure", SemanticToken::Failure),
            ("completed", "skipped", SemanticToken::Muted),
            ("completed", "neutral", SemanticToken::Muted),
            ("completed", "success", SemanticToken::Success),
            ("completed", "cancelled", SemanticToken::Cancelled),
            ("in_progress", "", SemanticToken::Running),
            ("queued", "", SemanticToken::Pending),
            ("pending", "", SemanticToken::Pending),
        ] {
            assert_eq!(SemanticToken::ci_state(status, conclusion), token);
            assert_eq!(
                crate::actions_workspace::status_role(status, conclusion),
                token,
                "Actions and PR Checks share one mapping"
            );
        }
    }

    #[test]
    fn v1_pr_reviews_disabled_reasons_and_effect_sentence() {
        let other = summary("hubot", "open", false);
        let own = summary("octocat", "open", false);
        assert_eq!(
            PrFormKind::Approve.review_disabled_reason(&other, "octocat"),
            None
        );
        assert_eq!(
            PrFormKind::RequestChanges.review_disabled_reason(&own, "OctoCat"),
            Some("You cannot approve or request changes on your own PR.".into())
        );
        // A comment review on your own PR is allowed.
        assert_eq!(
            PrFormKind::ReviewComment.review_disabled_reason(&own, "octocat"),
            None
        );
        assert!(PrFormKind::ReviewComment
            .review_disabled_reason(&summary("hubot", "closed", true), "octocat")
            .is_some());
        assert_eq!(
            PrFormKind::Title.review_disabled_reason(&own, "octocat"),
            None
        );
        let intent = |kind| PrIntent {
            kind,
            target: other.clone(),
            identity: HostIdentity {
                host: "github.invalid".into(),
                login: "octocat".into(),
                account_id: Some(42),
                gh_version: "2.101.0".into(),
            },
        };
        assert_eq!(
            intent(PrFormKind::Approve).review_effect().unwrap(),
            "Approve example/repo#7 by @hubot at head aaaaaaa as @octocat • summary optional"
        );
        assert!(intent(PrFormKind::RequestChanges)
            .review_effect()
            .unwrap()
            .starts_with("Request changes on example/repo#7 by @hubot"));
        assert!(intent(PrFormKind::Title).review_effect().is_none());
    }
}
