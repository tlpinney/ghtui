//! GitHub transport and authentication exclusively through the installed `gh` CLI.
use super::runner::{CancellationToken, CommandFailure, CommandRunner, CommandSpec, FailureKind};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::time::Duration;
mod inbox_exact_v1;
mod inbox_rollup_v1;
pub(crate) use inbox_exact_v1::ExactInboxRepositoryMetadataV1;
use inbox_exact_v1::{ExactInboxState, InboxReadPolicy};
pub use inbox_rollup_v1::{
    inbox_rollup_args_v1, inbox_rollup_order_v1, inbox_rollup_query_v1,
    inbox_rollup_status_args_v1, inbox_rollup_status_query_v1, parse_inbox_rollup_v1,
    InboxReviewerV1, InboxRollupV1, ReviewerRoleV1, INBOX_COMMENTERS_LAST_V1,
    INBOX_REVIEWERS_FIRST_V1, INBOX_ROLLUP_BATCH_V1,
};
mod pr;
pub(crate) use pr::FreshInlineCommentObservationV14;
#[cfg(test)]
pub(crate) use pr::CONVERSATION_COMMENT_QUERY_V17;
pub use pr::*;
pub(crate) use pr::{
    conversation_post_caps_v17, conversation_post_caps_v19, ClosedConversationRefreshV19,
    ConversationDetailListV19, ConversationDetailSnapshotV19, ConversationHttpAckV17,
    ConversationHttpRequestV17, ConversationPageSupplementV61, ConversationPublicationFailureV17,
    ConversationRawSummaryV19, ConversationReceiptV17, ConversationRecoveryCurrentV17,
    ConversationRefreshCloseSealV19, ConversationRefreshPreparationV19,
    FreshConversationPublicationV17, IssueLinkRefreshV87, MembersRefreshV48,
    OriginalConversationCommentV17, OriginalConversationReactionsV17,
    OriginalConversationSubscriptionV17, StateRefreshV68,
};
mod actions;
pub(crate) mod workflow_capacity_v9;
pub use actions::*;
mod search;
pub use search::*;
mod discussions;
pub use discussions::*;
mod metrics;
pub use metrics::*;
pub(crate) use metrics::{
    ActualMembersPageContextV61, MembersMetadataHistoryReadV62, MembersWriteV48,
    MetadataMembersObservationV48, MetadataMembersReadPurposeV48,
    MetadataMembersRecoveryCurrentV48, OriginalMetadataMembersV48, PrMetadataMembersFailureV48,
    PrMetadataMembersReceiptV48, METADATA_MEMBERS_POST_CAPS_V48, METADATA_MEMBERS_REFRESH_CAPS_V48,
    METADATA_MEMBERS_TAB_POST_CAPS_V62, METADATA_MEMBERS_TAB_REFRESH_CAPS_V62,
};
use metrics::{MembersVisibilityV48, MetadataMembersContextV48, PreparedMembersRequestV48};
mod cli_requirements_v1;
pub use cli_requirements_v1::*;
mod capability_requirements_v1;
mod cli_preflight_v1;
pub use capability_requirements_v1::*;
mod thread_phase_issuer_v16;
pub(crate) use thread_phase_issuer_v16::ThreadPhaseIssuerV16;
mod repository_catalogue;
pub(crate) use repository_catalogue::notifications_v63;
pub(crate) use repository_catalogue::OriginalPrClosedAccessV13;
pub use repository_catalogue::*;
pub(crate) use repository_catalogue::{
    ClosedNotificationReadV63, NotificationActionPreviewV63, NotificationActorsStatusV63,
    NotificationInspectionV63, NotificationLatestActorV63, NotificationPrFollowIntentV63,
    NotificationPrSourceV63, NotificationPublicationStateV63, NotificationPublicationV63,
    NotificationSelectionV63, NotificationWriteReceiptV63, WorkflowCleanupSealV9,
};
pub(crate) use repository_catalogue::{
    ClosedStateSourceV80, StateSourcePreparedBindingV80, StateSourceReadV80,
};
use repository_catalogue::{
    MembersMetadataClosingInputV62, MembersMetadataContextInputV62, MembersMetadataRoleV62,
};
pub(crate) use repository_catalogue::{OriginalPrAccessSessionV13, OriginalPrMembershipRefV13};
pub(crate) use repository_catalogue::{StateSourceCaptureV80, StateSourceChargeV80};
pub(crate) use repository_catalogue::{ThreadReactionHttpAckV3, ThreadReactionHttpRequestV3};
pub(crate) use repository_catalogue::{
    WorkflowEditPhaseV9, WorkflowGitRoleV9, WorkflowPostCaptureV9,
};
mod remote_repository_v1;
pub use remote_repository_v1::*;
mod required_checks_v1;
pub(crate) use required_checks_v1::read_required_checks_v1;
pub use required_checks_v1::{
    context_matches, ObservedContextV1, RequiredChecksV1, RequiredContextV1, RequiredSourceV1,
    RequiredStateV1,
};
mod pr_create_v1;
pub(in crate::providers) mod pr_issue_links_v87;
mod pr_resources_v1;
pub(crate) use pr_create_v1::encode_component_v1;
pub(crate) use pr_issue_links_v87::read::{
    IssueLinksInspectionReadV87, IssueLinksOpenReadV87, IssueLinksSelectedReadV87,
};
pub(crate) use pr_issue_links_v87::refresh::{
    ActualIssueLinkPageContextV87, IssueLinkContextBindingV87,
};
pub(crate) use pr_issue_links_v87::{
    IssueCatalogueResultV102, IssueHostUnsupportedV102, IssueLinkCatalogueV87, IssueLinkChoiceV87,
    IssueLinkManualReadV94, IssueLinkPlanV87, IssueLinkPublicationFailureV87,
    IssueLinkReadPurposeV87, IssueLinkReceiptV87, IssueLinkRecoveryCurrentV87,
    IssueLinkRepositoryChoiceV87, IssueLinkWriteV87, IssueNoChangeV102, IssueRequestedV102,
    OriginalIssueLinksV87,
};
pub(crate) use pr_issue_links_v87::{
    ISSUE_LINK_METADATA_POST_CAPS_V87, ISSUE_LINK_METADATA_REFRESH_CAPS_V87,
    ISSUE_LINK_POST_CAPS_V87, ISSUE_LINK_REFRESH_CAPS_V87,
};
use pr_resources_v1::MetadataMembersPostDataV48;
#[cfg(test)]
pub(crate) use pr_resources_v1::METADATA_TEXT_CAPABILITY_QUERY_V25;
pub use pr_resources_v1::*;
pub(crate) use pr_resources_v1::{
    ActualMetadataTabContextV51, BaseCatalogueLimitsV32, BaseCatalogueV32, BaseCoverageStatusV32,
    BaseRecoveryCurrentV32, BaseSelectionV32, BaseTransitionV32, FileCommentBodyV75,
    MetadataRecoveryCurrentV25, OriginalMetadataBaseV32, OriginalMetadataTextV25,
    OriginalPrFileCommentV75, OriginalPrStateV68, PrMetadataBaseFailureV32, PrMetadataBaseReadV32,
    PrMetadataBaseReceiptV32, PrMetadataTextFailureV25, PrMetadataTextReadV25,
    PrMetadataTextReceiptV25, PrStateReadV68, StateCapabilityV68, StatePublicationFailureV68,
    StatePublishInputV68, StateReceiptV68, StateRecoveryCurrentV68, StateTransitionV68,
    FILE_COMMENT_POSTWRITE_CAPS_V75, METADATA_BASE_POST_CAPS_V32, METADATA_TAB_TEXT_POST_CAPS_V51,
    METADATA_TEXT_POST_CAPS_V25, STATE_POST_CAPS_V68,
};
#[cfg(test)]
pub(crate) use pr_resources_v1::{FileCommentReceiptV75, FILE_COMMENT_NODE_QUERY_V75};
#[cfg(test)]
#[path = "gh/identity_before_tests.rs"]
mod identity_before_tests;
#[cfg(test)]
#[path = "gh/identity_version_tests.rs"]
mod identity_version_tests;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostIdentity {
    pub host: String,
    pub login: String,
    /// Durable REST account identifier. Missing IDs must not unlock private caches.
    #[serde(default)]
    pub account_id: Option<u64>,
    pub gh_version: String,
}
impl HostIdentity {
    pub fn same_account(&self, other: &Self) -> bool {
        self.host == other.host
            && self.login.eq_ignore_ascii_case(&other.login)
            && self.account_id == other.account_id
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Actor {
    pub login: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub actor_type: String,
}
impl Actor {
    pub fn label(&self) -> String {
        if self.login.is_empty() {
            "unknown actor".into()
        } else {
            format!("@{}", self.login)
        }
    }
    fn from_json(v: &Value) -> Self {
        Self {
            login: text(v, "login"),
            display_name: v.get("name").and_then(Value::as_str).map(str::to_owned),
            avatar_url: v
                .get("avatar_url")
                .or_else(|| v.get("avatarUrl"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            actor_type: v
                .get("type")
                .or_else(|| v.get("__typename"))
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .unwrap_or("Unknown")
                .into(),
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Permissions {
    pub pull: bool,
    pub push: bool,
    pub maintain: bool,
    pub admin: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Repository {
    pub host: String,
    pub id: u64,
    pub full_name: String,
    pub description: String,
    pub default_branch: String,
    pub private: bool,
    pub archived: bool,
    pub permissions: Permissions,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Queue {
    Authored,
    Assigned,
    ReviewRequested,
}
impl Queue {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Authored => "Created by me",
            Self::Assigned => "Assigned to me",
            Self::ReviewRequested => "Needs my review",
        }
    }
    fn qualifier(&self) -> &'static str {
        match self {
            Self::Authored => "author",
            Self::Assigned => "assignee",
            Self::ReviewRequested => "review-requested",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullRequest {
    pub host: String,
    pub id: String,
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub body: String,
    pub url: String,
    pub author: Actor,
    pub state: String,
    pub draft: bool,
    pub created_at: String,
    pub updated_at: String,
    pub queues: BTreeSet<Queue>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Coverage {
    pub loaded: usize,
    pub total_matches: u64,
    pub complete: bool,
    pub reasons: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inbox {
    pub identity: HostIdentity,
    pub pull_requests: Vec<PullRequest>,
    pub coverage: Coverage,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConversationComment {
    pub id: u64,
    pub node_id: String,
    pub body: String,
    pub author: Actor,
    pub created_at: String,
    pub updated_at: String,
    pub url: String,
    /// The comment card's display facts from the same read (TASK-115); None when the read
    /// reported neither. Boxed so every comment-carrying enum keeps its size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_v115: Option<Box<CommentCardFactsV115>>,
}
/// Display-only facts of a Conversation comment card (TASK-115); never permission.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentCardFactsV115 {
    /// GitHub's `author_association` (OWNER, MEMBER, …) for the role badge.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author_association: String,
    /// Nonzero reaction counts as `(content, count)` in GitHub's order, from the read's
    /// `reactions` rollup; the reaction browser reads the actual reactors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<(String, u64)>,
}
impl ConversationComment {
    pub fn author_association(&self) -> &str {
        self.card_v115
            .as_ref()
            .map_or("", |facts| facts.author_association.as_str())
    }
    pub fn reactions(&self) -> &[(String, u64)] {
        self.card_v115
            .as_ref()
            .map_or(&[], |facts| facts.reactions.as_slice())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitSummary {
    pub sha: String,
    pub message: String,
    pub author: Actor,
    pub committer: Actor,
    pub author_name: String,
    pub committer_name: String,
    /// The Git emails recorded in the commit (TASK-34 profiles). Older cached pages lack them.
    #[serde(default)]
    pub author_email: String,
    #[serde(default)]
    pub committer_email: String,
    pub authored_at: String,
    pub parents: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChangedFile {
    pub path: String,
    pub previous_path: Option<String>,
    pub status: String,
    pub additions: u64,
    pub deletions: u64,
    pub patch: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrSummary {
    pub pr: PullRequest,
    pub head_sha: String,
    pub base_sha: String,
    pub head_ref: String,
    pub base_ref: String,
    pub merged: bool,
    pub auto_merge: bool,
    pub merge_commit_sha: Option<String>,
    pub mergeable: Option<bool>,
    pub mergeable_state: String,
    pub additions: u64,
    pub deletions: u64,
    pub changed_files: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrDetails {
    pub summary: PrSummary,
    pub comments: Vec<ConversationComment>,
    pub commits: Vec<CommitSummary>,
    pub files: Vec<ChangedFile>,
    pub reviews: Vec<Value>,
    pub timeline: Vec<Value>,
    pub coverage: Coverage,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrDiff {
    pub base_sha: String,
    pub head_sha: String,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Check {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: String,
    pub url: String,
    pub head_sha: String,
    pub kind: String,
    /// The GitHub App that reported a check run; commit statuses name none.
    #[serde(default)]
    pub app_id: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub state: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub workflow_id: u64,
    pub name: String,
    pub title: String,
    pub head_sha: String,
    pub head_branch: String,
    pub event: String,
    pub status: String,
    pub conclusion: String,
    pub attempt: u64,
    pub actor: Actor,
    pub triggering_actor: Actor,
    pub created_at: String,
    pub updated_at: String,
    pub url: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunDetails {
    pub run: WorkflowRun,
    pub jobs: Vec<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteTreeEntry {
    pub path: String,
    pub mode: String,
    pub object_type: String,
    pub sha: String,
    pub size: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteTree {
    pub sha: String,
    pub entries: Vec<RemoteTreeEntry>,
    pub truncated: bool,
    pub recursive: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub label: String,
    pub transport: String,
    pub permission: String,
    pub concurrency_guard: String,
    pub task: String,
    pub implemented: bool,
    pub reason: String,
    #[serde(default)]
    pub source_urls: Vec<String>,
}
#[derive(Clone, Debug)]
pub enum GithubErrorKind {
    MissingGh,
    Authentication,
    IdentityChanged,
    InvalidTarget,
    StaleHead,
    Permission,
    /// The selected read target is not accessible. This is not proof of deletion.
    NotFound,
    RateLimited,
    Unsupported,
    Transport,
    /// The runner rejected actual captured bytes, not an inaccessible target.
    OutputLimit,
    Json,
    AmbiguousWrite,
}
#[derive(Clone, Debug)]
pub struct GithubError {
    pub kind: GithubErrorKind,
    pub message: String,
}
impl std::fmt::Display for GithubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for GithubError {}
impl GithubError {
    /// Only use for an explicitly selected repository read after re-verifying
    /// its positive current principal. An inaccessible endpoint is not an
    /// authoritative remote-deletion receipt. Transport, auth and rate limits
    /// must not delete cached membership or imply complete empty coverage.
    pub fn repository_access_lost(&self) -> bool {
        matches!(
            self.kind,
            GithubErrorKind::Permission | GithubErrorKind::NotFound
        )
    }
}

/// Recognize gh's own exited-command status envelopes, never a bare number in
/// an argv, URL, JSON body or network diagnostic. gh api uses a trailing
/// `(HTTP NNN)`; native commands/go-gh also use a leading `HTTP NNN:`.
fn exited_http_status(stderr: &str) -> Option<u16> {
    // Prefer the status appended by gh api over status-looking server prose at
    // the start of its message (e.g. gh: HTTP 403: quoted body (HTTP 502)).
    let api_status = stderr.lines().rev().find_map(|line| {
        let detail = line.trim().strip_prefix("gh: ")?;
        let start = detail.rfind(" (HTTP ")?;
        let envelope = &detail[start + " (HTTP ".len()..];
        if envelope.len() == 4 && envelope.ends_with(')') {
            let digits = &envelope[..3];
            if digits.bytes().all(|byte| byte.is_ascii_digit()) {
                return digits.parse().ok();
            }
        }
        None
    });
    api_status.or_else(|| {
        stderr.lines().find_map(|line| {
            let line = line.trim();
            let detail = line.strip_prefix("gh: ").unwrap_or(line);
            if let Some(rest) = detail.strip_prefix("HTTP ") {
                let digits = rest.get(..3)?;
                let suffix = rest.get(3..)?;
                if digits.bytes().all(|byte| byte.is_ascii_digit())
                    && (suffix.is_empty() || suffix.starts_with(':') || suffix.starts_with(" ("))
                {
                    return digits.parse().ok();
                }
            }
            None
        })
    })
}
impl From<CommandFailure> for GithubError {
    fn from(e: CommandFailure) -> Self {
        let stderr = String::from_utf8_lossy(&e.stderr);
        let exited = e.kind == FailureKind::Exit;
        let status = exited.then(|| exited_http_status(&stderr)).flatten();
        let rate_limited = matches!(status, Some(403 | 429))
            && (status == Some(429)
                || stderr.to_ascii_lowercase().contains("rate limit")
                || stderr.to_ascii_lowercase().contains("abuse detection"));
        let kind = if e.outcome_unknown {
            GithubErrorKind::AmbiguousWrite
        } else if e.kind == FailureKind::OutputLimit {
            GithubErrorKind::OutputLimit
        } else if e.kind == FailureKind::MissingProgram {
            GithubErrorKind::MissingGh
        } else if status == Some(401)
            || exited && (stderr.contains("gh auth login") || stderr.contains("not logged in"))
        {
            GithubErrorKind::Authentication
        } else if rate_limited {
            GithubErrorKind::RateLimited
        } else if status == Some(403) {
            GithubErrorKind::Permission
        } else if status == Some(404) {
            GithubErrorKind::NotFound
        } else if e.message.contains("Unknown JSON field") || e.message.contains("unknown flag") {
            GithubErrorKind::Unsupported
        } else {
            GithubErrorKind::Transport
        };
        let message=match kind {GithubErrorKind::MissingGh=>"GitHub CLI is missing. Install gh and run gh auth login; local Git remains available.".into(),GithubErrorKind::Authentication=>format!("GitHub authentication failed. Run gh auth login --hostname for the selected host. {}",e.message),GithubErrorKind::OutputLimit=>"GitHub response exceeded its bounded capture; no response body admitted".into(),_=>e.message};
        Self { kind, message }
    }
}
pub type GithubResult<T> = Result<T, GithubError>;

#[derive(Clone)]
pub struct GithubProvider {
    host: String,
    program: OsString,
    runner: CommandRunner,
    env: BTreeMap<OsString, OsString>,
    timeout: Duration,
    scoped_read_window: Option<(std::time::Instant, usize)>,
    scoped_read_budget: Option<std::sync::Arc<super::runner::ScopedReadBudgetV1>>,
    workflow_environment_v9: Option<std::sync::Arc<super::runner::WorkflowProcessEnvironmentV9>>,
}
impl GithubProvider {
    pub fn new(host: impl Into<String>) -> GithubResult<Self> {
        let host = canonical_actions_host(&host.into())?;
        Ok(Self {
            host,
            program: OsString::from("gh"),
            runner: CommandRunner::default(),
            env: BTreeMap::new(),
            timeout: Duration::from_secs(30),
            scoped_read_window: None,
            scoped_read_budget: None,
            workflow_environment_v9: super::runner::WorkflowProcessEnvironmentV9::current_v9(),
        })
    }
    pub fn with_program(mut self, path: impl Into<OsString>) -> Self {
        self.program = path.into();
        self
    }
    pub fn with_env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    /// Application workers share one runner so recreating a client cannot bypass write guards.
    pub fn with_runner(mut self, runner: CommandRunner) -> Self {
        self.runner = runner;
        self
    }
    /// The original native worker uses this SAME runner for fixed optional
    /// local renderer commands; never its authenticated scoped read window.
    pub(crate) fn native_external_runner_v4(&self) -> &CommandRunner {
        &self.runner
    }
    pub fn host(&self) -> &str {
        &self.host
    }
    /// Bounds only, not an account/access receipt. Later timeout changes cannot
    /// extend the original absolute deadline or shared per-child capture cap.
    pub fn with_scoped_read_window(
        mut self,
        deadline: std::time::Instant,
        max_output_bytes: usize,
    ) -> GithubResult<Self> {
        if !(1..=1024 * 1024).contains(&max_output_bytes) {
            return Err(error(
                GithubErrorKind::InvalidTarget,
                "Scoped read capture must be1..1MiB",
            ));
        }
        self.scoped_read_window = Some((deadline, max_output_bytes));
        Ok(self)
    }
    /// Crate-only bounds clamp for the catalogue factory. Preserves its default
    /// 4 MiB page allowance and any EXISTING narrower read window. Not authority.
    pub(crate) fn with_catalogue_read_deadline(mut self, deadline: std::time::Instant) -> Self {
        self.scoped_read_window = Some(match self.scoped_read_window {
            Some((original, cap)) => (original.min(deadline), cap.min(4 * 1024 * 1024)),
            None => (deadline, 4 * 1024 * 1024),
        });
        self
    }
    /// Bounds only for the single closed Actions archive protocol. No public
    /// window maximum is raised and an original narrower capture stays narrow.
    pub(crate) fn with_actions_archive_read_deadline_v2(
        mut self,
        deadline: std::time::Instant,
    ) -> Self {
        self.scoped_read_window = Some(match self.scoped_read_window {
            Some((original, cap)) => (original.min(deadline), cap.min(4 * 1024 * 1024)),
            None => (deadline, 4 * 1024 * 1024),
        });
        self
    }
    /// GitHub owner/repository/organization names are case-insensitive, unlike Git refs
    /// and filesystem paths. Normalize only those identity segments of provider keys.
    /// The generic command runner intentionally keeps local Git resource keys intact.
    fn canonical_resource(&self, resource: &str) -> String {
        canonical_github_resource(&self.host, resource)
    }
    fn spec(&self, args: Vec<String>) -> CommandSpec {
        let mut spec = CommandSpec::new(self.program.clone())
            // gh credentials are external mutable state. Sharing responses solely
            // by argv/env may reuse old-account private data after a new account
            // check. Until reads have an immutable verified-principal namespace,
            // every GitHub observation launches its own bounded fresh child.
            // Installed Git/default runner reads continue to coalesce normally.
            .fresh_read()
            .args(args)
            .env("GH_HOST", &self.host)
            .env("GH_PROMPT_DISABLED", "1")
            .env("NO_COLOR", "1")
            .timeout(self.timeout);
        spec.env.extend(self.env.clone());
        // The caller cannot override transport provenance through auxiliary environment.
        spec.env.insert("GH_HOST".into(), self.host.clone().into());
        if let Some((deadline, cap)) = self.scoped_read_window {
            spec.absolute_deadline_v1 = Some(deadline);
            spec.timeout = spec
                .timeout
                .min(deadline.saturating_duration_since(std::time::Instant::now()));
            spec.output_limit = spec.output_limit.min(cap);
        }
        spec.scoped_read_budget = self.scoped_read_budget.clone();
        spec
    }
    fn execute(
        &self,
        args: Vec<String>,
        input: Option<Value>,
        write: Option<String>,
        cancel: &CancellationToken,
    ) -> GithubResult<Vec<u8>> {
        let mut spec = self.spec(args);
        if let Some(input) = input {
            spec.stdin = serde_json::to_vec(&input)
                .map_err(|e| error(GithubErrorKind::Json, e.to_string()))?;
        }
        if let Some(resource) = write {
            spec = spec.write(self.canonical_resource(&resource));
        }
        self.runner
            .run(spec, cancel)
            .map(|o| o.stdout)
            .map_err(Into::into)
    }
    /// TASK-107: a read whose memory holds only the last `output_limit` stdout bytes.
    /// Returns the kept tail and how many earlier bytes were dropped.
    fn execute_tail(
        &self,
        args: Vec<String>,
        cancel: &CancellationToken,
    ) -> GithubResult<(Vec<u8>, u64)> {
        let mut spec = self.spec(args);
        spec.stdout_sink_v107 = super::runner::StdoutSinkV107::Tail;
        self.runner
            .run(spec, cancel)
            .map(|o| (o.stdout, o.stdout_skipped_bytes))
            .map_err(Into::into)
    }
    /// TASK-107: a read streamed straight to `file` (nothing kept in memory), up to
    /// `disk_limit` bytes and within `timeout`.
    fn execute_to_file(
        &self,
        args: Vec<String>,
        file: std::fs::File,
        disk_limit: usize,
        timeout: std::time::Duration,
        cancel: &CancellationToken,
    ) -> GithubResult<()> {
        let mut spec = self.spec(args);
        if self.scoped_read_window.is_none() {
            spec.timeout = spec.timeout.max(timeout);
        }
        spec.output_limit = disk_limit;
        spec.stdout_sink_v107 = super::runner::StdoutSinkV107::File(std::sync::Arc::new(file));
        self.runner
            .run(spec, cancel)
            .map(|_| ())
            .map_err(Into::into)
    }
    /// A failed post-write observation is not proof that the mutation failed.
    fn after_write<T>(&self, resource: &str, observed: GithubResult<T>) -> GithubResult<T> {
        observed.map_err(|failure| {
            self.runner.mark_ambiguous(&self.canonical_resource(resource));
            error(GithubErrorKind::AmbiguousWrite, format!("The write may have succeeded, but its state could not be confirmed: {}. Reconcile the selected resource before retrying.", failure.message))
        })
    }
    /// Escape-bearing raw output is consumed as data, never written directly to a terminal.
    /// Older gh releases reject this flag before requesting data; only that safe read is retried.
    fn raw_spec(&self, mut spec: CommandSpec, cancel: &CancellationToken) -> GithubResult<Vec<u8>> {
        // Raw callers may apply legacy per-operation defaults AFTER spec().
        // Reapply an original scoped bound here rather than widening its cap
        // or replacing its absolute deadline (unscoped legacy stays unchanged).
        if let Some((deadline, cap)) = self.scoped_read_window {
            spec.output_limit = spec.output_limit.min(cap);
            spec.absolute_deadline_v1 = Some(
                spec.absolute_deadline_v1
                    .map_or(deadline, |original| original.min(deadline)),
            );
            spec.timeout = spec
                .timeout
                .min(deadline.saturating_duration_since(std::time::Instant::now()));
        }
        let mut modern = spec.clone();
        modern.args.push("--allow-escape-sequences".into());
        match self.runner.run(modern, cancel) {
            Ok(output) => Ok(output.stdout),
            Err(failure)
                if matches!(failure.kind, FailureKind::Exit)
                    && String::from_utf8_lossy(&failure.stderr).contains("unknown flag")
                    && String::from_utf8_lossy(&failure.stderr)
                        .contains("allow-escape-sequences") =>
            {
                self.runner
                    .run(spec, cancel)
                    .map(|output| output.stdout)
                    .map_err(Into::into)
            }
            Err(failure) => Err(failure.into()),
        }
    }
    fn api(
        &self,
        endpoint: &str,
        method: &str,
        input: Option<Value>,
        write: Option<String>,
        paginate: bool,
        cancel: &CancellationToken,
    ) -> GithubResult<Value> {
        let write = write.map(|resource| self.canonical_resource(&resource));
        let mut args = vec![
            "api".into(),
            endpoint.into(),
            "--hostname".into(),
            self.host.clone(),
            "--method".into(),
            method.into(),
        ];
        if input.is_some() {
            args.extend(["--input".into(), "-".into()]);
        }
        if paginate {
            args.extend(["--paginate".into(), "--slurp".into()]);
        }
        let data = self.execute(args, input, write.clone(), cancel)?;
        self.parse_api_response_v48(data, write)
    }
    /// SAME JSON/error/ambiguity parser for the original Value API and the
    /// closed preencoded Members request. No transport or authority is issued.
    fn parse_api_response_v48(&self, data: Vec<u8>, write: Option<String>) -> GithubResult<Value> {
        if data.is_empty() {
            return Ok(Value::Null);
        }
        let value: Value = serde_json::from_slice(&data).map_err(|e| {
            if let Some(resource) = &write {
                self.runner.mark_ambiguous(resource);
                error(
                    GithubErrorKind::AmbiguousWrite,
                    format!("Write response could not be decoded; reconcile before retry: {e}"),
                )
            } else {
                error(
                    GithubErrorKind::Json,
                    format!("gh returned invalid JSON: {e}"),
                )
            }
        })?;
        if value
            .get("errors")
            .and_then(Value::as_array)
            .is_some_and(|errors| !errors.is_empty())
        {
            let message = super::runner::sanitize_terminal(&value["errors"].to_string());
            if let Some(resource) = &write {
                self.runner.mark_ambiguous(resource);
                return Err(error(
                    GithubErrorKind::AmbiguousWrite,
                    format!("GraphQL write returned errors; reconcile before retry: {message}"),
                ));
            }
            return Err(error(
                GithubErrorKind::Unsupported,
                format!("Host GraphQL capability/permission error: {message}"),
            ));
        }
        Ok(value)
    }
    fn api_list(
        &self,
        endpoint: &str,
        key: Option<&str>,
        cancel: &CancellationToken,
    ) -> GithubResult<Vec<Value>> {
        let pages = self.api(endpoint, "GET", None, None, true, cancel)?;
        Self::api_list_pages_v19(pages, key)
    }
    /// SAME page-envelope conversion, now moving owned list values rather than
    /// cloning them. No endpoint, runner, read window or source is exposed.
    fn api_list_pages_v19(pages: Value, key: Option<&str>) -> GithubResult<Vec<Value>> {
        let mut all = Vec::new();
        let Value::Array(pages) = pages else {
            return Err(error(
                GithubErrorKind::Json,
                "Expected paginated JSON pages",
            ));
        };
        for mut page in pages {
            let items = if let Some(key) = key {
                page.get_mut(key).map(Value::take)
            } else {
                Some(page)
            };
            let Some(Value::Array(items)) = items else {
                return Err(error(GithubErrorKind::Json, "Expected list in gh API page"));
            };
            all.extend(items);
        }
        Ok(all)
    }
    pub fn identity(&self, cancel: &CancellationToken) -> GithubResult<HostIdentity> {
        self.identity_with_deadline(None, cancel)
    }
    /// Same gh argv and bounded version policy, but both fresh children consume
    /// the caller's original deadline. No minor-version floor or auth stack.
    pub(crate) fn identity_before(
        &self,
        deadline: std::time::Instant,
        cancel: &CancellationToken,
    ) -> GithubResult<HostIdentity> {
        self.identity_with_deadline(Some(deadline), cancel)
    }
    fn identity_with_deadline(
        &self,
        deadline: Option<std::time::Instant>,
        cancel: &CancellationToken,
    ) -> GithubResult<HostIdentity> {
        let current = || -> GithubResult<Self> {
            let mut provider = self.clone();
            if let Some(deadline) = deadline {
                let left = deadline.saturating_duration_since(std::time::Instant::now());
                if left.is_zero() || cancel.is_cancelled() {
                    return Err(error(
                        GithubErrorKind::Transport,
                        "Original identity deadline/cancellation reached before another gh child",
                    ));
                }
                provider.timeout = provider.timeout.min(left);
            }
            Ok(provider)
        };
        let mut version_spec = current()?.spec(vec!["--version".into()]);
        version_spec.output_limit = version_spec.output_limit.min(4096);
        let version = self.runner.run(version_spec, cancel)?.stdout;
        let version = Self::identity_version_line(&version)?;
        let user = current()?
            .api("user", "GET", None, None, false, cancel)
            .map_err(|mut e| {
                if matches!(e.kind, GithubErrorKind::Authentication) {
                    e.message = format!(
                        "GitHub authentication failed for {}. Run gh auth login --hostname {}. {}",
                        self.host, self.host, e.message
                    );
                }
                e
            })?;
        self.identity_from_user_v3(&user, version)
    }
    // One bounded syntax policy for ordinary and original-deadline identity.
    // This grants neither operation-flag nor host/permission authority.
    fn identity_version_line(version: &[u8]) -> GithubResult<String> {
        let parsed = GhCliVersion::parse_stdout(version).map_err(|failure| {
            error(
                GithubErrorKind::Unsupported,
                format!("GitHub CLI version could not be verified: {failure}. Check gh --version and install a reviewed gh 2.x build; local Git remains available."),
            )
        })?;
        if parsed.numbers().major != 2 {
            return Err(error(
                GithubErrorKind::Unsupported,
                "Unsupported GitHub CLI major version. Install a reviewed gh 2.x build; future majors require compatibility review. Local Git remains available.",
            ));
        }
        // Parsing validates the complete bounded raw output before any display
        // text is retained. A valid suffix is not proof of operation flags,
        // host support, security policy or current repository permissions.
        Ok(std::str::from_utf8(version)
            .expect("version parser validated UTF-8")
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned())
    }
    /// ONE scalar user parser shared by the unchanged legacy identity bracket
    /// and an already-prepaid final user observation. Not an auth fallback.
    fn identity_from_user_v3(&self, user: &Value, version: String) -> GithubResult<HostIdentity> {
        let login = text(user, "login").to_ascii_lowercase();
        if login.is_empty() {
            return Err(error(
                GithubErrorKind::Authentication,
                format!(
                    "No authenticated identity; run gh auth login --hostname {}",
                    self.host
                ),
            ));
        }
        Ok(HostIdentity {
            host: self.host.clone(),
            login,
            account_id: user.get("id").and_then(Value::as_u64).filter(|id| *id > 0),
            gh_version: version,
        })
    }
    pub fn repository(&self, name: &str, cancel: &CancellationToken) -> GithubResult<Repository> {
        validate_repo(name)?;
        let value = self.api(&format!("repos/{name}"), "GET", None, None, false, cancel)?;
        self.repo_from_json(&value)
    }
    fn repo_from_json(&self, v: &Value) -> GithubResult<Repository> {
        let full_name = text(v, "full_name");
        validate_repo(&full_name)?;
        Ok(Repository {
            host: self.host.clone(),
            id: number(v, "id"),
            full_name,
            description: text(v, "description"),
            default_branch: text(v, "default_branch"),
            private: boolean(v, "private"),
            archived: boolean(v, "archived"),
            permissions: serde_json::from_value(v.get("permissions").cloned().unwrap_or_else(
                || json!({"pull":false,"push":false,"maintain":false,"admin":false}),
            ))
            .unwrap_or_default(),
        })
    }
    pub fn organization_repositories(
        &self,
        org: &str,
        cancel: &CancellationToken,
    ) -> GithubResult<Vec<Repository>> {
        validate_name(org)?;
        self.api_list(
            &format!("orgs/{org}/repos?per_page=100&type=all"),
            None,
            cancel,
        )?
        .iter()
        .map(|v| self.repo_from_json(v))
        .collect()
    }
    pub fn personal_inbox(
        &self,
        identity: &HostIdentity,
        scope: &[String],
        cancel: &CancellationToken,
    ) -> GithubResult<Inbox> {
        self.personal_inbox_with_policy(identity, scope, cancel, InboxReadPolicy::Legacy, None)
    }
    /// Actual strict queue observations, not repository access or canonical PR authority.
    /// Only the original-bound exact workspace route chooses this policy.
    pub(crate) fn personal_inbox_exact_scope(
        &self,
        identity: &HostIdentity,
        scope: &[String],
        cancel: &CancellationToken,
    ) -> GithubResult<Inbox> {
        self.personal_inbox_with_policy(identity, scope, cancel, InboxReadPolicy::ExactScoped, None)
    }
    /// Optional metadata consistency from genuine pending reads; NOT access.
    pub(crate) fn personal_inbox_exact_scope_current(
        &self,
        identity: &HostIdentity,
        scope: &[String],
        metadata: &ExactInboxRepositoryMetadataV1,
        cancel: &CancellationToken,
    ) -> GithubResult<Inbox> {
        self.personal_inbox_with_policy(
            identity,
            scope,
            cancel,
            InboxReadPolicy::ExactScoped,
            Some(metadata),
        )
    }
    fn personal_inbox_with_policy(
        &self,
        identity: &HostIdentity,
        scope: &[String],
        cancel: &CancellationToken,
        policy: InboxReadPolicy,
        repository_metadata: Option<&ExactInboxRepositoryMetadataV1>,
    ) -> GithubResult<Inbox> {
        if identity.host != self.host {
            return Err(error(
                GithubErrorKind::IdentityChanged,
                "Account host does not match selected host",
            ));
        }
        validate_name(&identity.login)?;
        if let Some(metadata) = repository_metadata {
            metadata.validate_scope(identity, scope)?;
        }
        let mut records: BTreeMap<(String, u64), PullRequest> = BTreeMap::new();
        let mut coverage = Coverage {
            complete: true,
            ..Coverage::default()
        };
        let mut exact = if policy == InboxReadPolicy::ExactScoped {
            Some(ExactInboxState::new(&self.host, identity, scope)?)
        } else {
            None
        };
        let scopes: Vec<Option<&String>> = if scope.is_empty() {
            vec![None]
        } else {
            scope.iter().map(Some).collect()
        };
        for queue in [Queue::Authored, Queue::Assigned, Queue::ReviewRequested] {
            for repo in &scopes {
                let mut query = format!("is:pr is:open {}:{}", queue.qualifier(), identity.login);
                if let Some(repo) = repo {
                    validate_repo(repo)?;
                    query.push_str(&format!(" repo:{repo}"));
                }
                if let Some(exact) = &mut exact {
                    exact.begin_query(repo.ok_or_else(|| {
                        error(
                            GithubErrorKind::InvalidTarget,
                            "Exact queue cannot default to all",
                        )
                    })?);
                }
                let mut loaded = 0u64;
                for page in 1..=10 {
                    let args = vec![
                        "api".into(),
                        "search/issues".into(),
                        "--hostname".into(),
                        self.host.clone(),
                        "--method".into(),
                        "GET".into(),
                        "-f".into(),
                        format!("q={query}"),
                        "-f".into(),
                        "per_page=100".into(),
                        "-f".into(),
                        format!("page={page}"),
                    ];
                    let bytes = self.execute(args, None, None, cancel)?;
                    let result: Value = serde_json::from_slice(&bytes)
                        .map_err(|e| error(GithubErrorKind::Json, e.to_string()))?;
                    let (total, items) = if let Some(exact) = &mut exact {
                        let page = exact.page(&result, &mut coverage)?;
                        (page.total, page.items)
                    } else {
                        let items =
                            result
                                .get("items")
                                .and_then(Value::as_array)
                                .ok_or_else(|| {
                                    error(GithubErrorKind::Json, "Search response has no items")
                                })?;
                        (number(&result, "total_count"), items.as_slice())
                    };
                    if page == 1 {
                        if exact.is_none() {
                            coverage.total_matches += total;
                        }
                        if total > 1000 {
                            coverage.complete = false;
                            coverage.reasons.push(format!("{} has {total} matches; GitHub search exposes at most 1000. Select narrower repositories to continue.", queue.label()));
                        }
                    }
                    if exact.is_none() && boolean(&result, "incomplete_results") {
                        coverage.complete = false;
                        coverage
                            .reasons
                            .push(format!("{} search is incomplete", queue.label()));
                    }
                    if exact.is_none() {
                        loaded += items.len() as u64;
                    }
                    for item in items {
                        let mut pr = if let Some(exact) = &mut exact {
                            let (pr, unique) = exact.item(self, item, &mut coverage)?;
                            if let Some(metadata) = repository_metadata {
                                metadata.validate_optional_repository(&pr.repository, item)?;
                            }
                            if unique {
                                loaded = loaded.checked_add(1).ok_or_else(|| {
                                    error(
                                        GithubErrorKind::Json,
                                        "Exact queue unique count overflow",
                                    )
                                })?;
                            }
                            pr
                        } else {
                            self.pr_from_json(item, None)?
                        };
                        pr.queues.insert(queue.clone());
                        let key = if exact.is_some() {
                            (pr.repository.to_ascii_lowercase(), pr.number)
                        } else {
                            (pr.repository.clone(), pr.number)
                        };
                        records
                            .entry(key)
                            .and_modify(|old| {
                                old.queues.insert(queue.clone());
                            })
                            .or_insert(pr);
                    }
                    if let Some(exact) = &exact {
                        exact.check_loaded(loaded, total, &mut coverage);
                    }
                    if items.is_empty() && loaded < total {
                        coverage.complete = false;
                        coverage.reasons.push(format!("{} returned {loaded} of {total} reported matches; refresh or narrow the repository scope", queue.label()));
                    }
                    if loaded >= total || items.is_empty() {
                        break;
                    }
                    if page == 10 {
                        coverage.complete = false;
                        if total <= 1000 {
                            coverage.reasons.push(format!(
                                "{} pagination ended with {loaded} of {total} reported matches",
                                queue.label()
                            ));
                        }
                    }
                }
            }
        }
        let mut pull_requests: Vec<_> = records.into_values().collect();
        pull_requests.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        coverage.loaded = pull_requests.len();
        Ok(Inbox {
            identity: identity.clone(),
            pull_requests,
            coverage,
        })
    }
    fn pr_from_json(&self, v: &Value, repo: Option<&str>) -> GithubResult<PullRequest> {
        if number(v, "number") == 0 {
            return Err(error(
                GithubErrorKind::Json,
                "GitHub returned a PR without a stable positive number",
            ));
        }
        let repository = repo.map(str::to_owned).unwrap_or_else(|| {
            text(v, "repository_url")
                .split("/repos/")
                .nth(1)
                .unwrap_or_default()
                .to_owned()
        });
        validate_repo(&repository)?;
        let id = if text(v, "node_id").is_empty() {
            v.get("id").map(Value::to_string).unwrap_or_default()
        } else {
            text(v, "node_id")
        };
        Ok(PullRequest {
            host: self.host.clone(),
            id,
            repository,
            number: number(v, "number"),
            title: text(v, "title"),
            body: text(v, "body"),
            url: text(v, "html_url"),
            author: Actor::from_json(v.get("user").unwrap_or(&Value::Null)),
            state: if boolean(v, "merged") {
                "merged".into()
            } else {
                text(v, "state")
            },
            draft: boolean(v, "draft"),
            created_at: text(v, "created_at"),
            updated_at: text(v, "updated_at"),
            queues: BTreeSet::new(),
        })
    }
    pub fn pr_summary(
        &self,
        repo: &str,
        n: u64,
        cancel: &CancellationToken,
    ) -> GithubResult<PrSummary> {
        validate_pr(repo, n)?;
        let v = self.api(
            &format!("repos/{repo}/pulls/{n}"),
            "GET",
            None,
            None,
            false,
            cancel,
        )?;
        self.pr_summary_from_raw_v19(repo, n, &v)
    }
    /// Existing summary display conversion only. The V19 caller first verifies
    /// the SAME raw response with the genuine current-parent parser and its
    /// preclone bounds; this conversion never manufactures source authority.
    fn pr_summary_from_raw_v19(&self, repo: &str, n: u64, v: &Value) -> GithubResult<PrSummary> {
        if number(v, "number") != n || text(v, "node_id").is_empty() {
            return Err(error(
                GithubErrorKind::Json,
                "PR response does not identify the requested canonical PR number/node",
            ));
        }
        for observed_repository in [
            v["base"]["repo"]["full_name"].as_str(),
            v["repository_url"]
                .as_str()
                .and_then(|url| url.rsplit_once("/repos/").map(|(_, repo)| repo)),
        ]
        .into_iter()
        .flatten()
        {
            if !observed_repository.eq_ignore_ascii_case(repo) {
                return Err(error(
                    GithubErrorKind::InvalidTarget,
                    "PR response belongs to another or renamed repository; reload its canonical identity",
                ));
            }
        }
        if validate_sha(&text(&v["head"], "sha")).is_err()
            || validate_sha(&text(&v["base"], "sha")).is_err()
        {
            return Err(error(GithubErrorKind::Unsupported,"The selected host returned a PR without valid base/head commit identities. Refresh or check host API compatibility."));
        }
        Ok(PrSummary {
            pr: self.pr_from_json(v, Some(repo))?,
            head_sha: text(&v["head"], "sha"),
            base_sha: text(&v["base"], "sha"),
            head_ref: text(&v["head"], "ref"),
            base_ref: text(&v["base"], "ref"),
            merged: boolean(v, "merged"),
            auto_merge: !v.get("auto_merge").unwrap_or(&Value::Null).is_null(),
            merge_commit_sha: v
                .get("merge_commit_sha")
                .and_then(Value::as_str)
                .map(str::to_owned),
            mergeable: v.get("mergeable").and_then(Value::as_bool),
            mergeable_state: text(v, "mergeable_state"),
            additions: number(v, "additions"),
            deletions: number(v, "deletions"),
            changed_files: number(v, "changed_files"),
        })
    }
    pub fn pr_details(
        &self,
        repo: &str,
        n: u64,
        cancel: &CancellationToken,
    ) -> GithubResult<PrDetails> {
        let summary = self.pr_summary(repo, n, cancel)?;
        let comments = self
            .api_list(
                &format!("repos/{repo}/issues/{n}/comments?per_page=100"),
                None,
                cancel,
            )?
            .iter()
            .map(comment_from_json)
            .collect();
        let commits = self
            .api_list(
                &format!("repos/{repo}/pulls/{n}/commits?per_page=100"),
                None,
                cancel,
            )?
            .iter()
            .map(commit_from_json)
            .collect();
        let files: Vec<ChangedFile> = self
            .api_list(
                &format!("repos/{repo}/pulls/{n}/files?per_page=100"),
                None,
                cancel,
            )?
            .iter()
            .map(file_from_json)
            .collect();
        let reviews = self.api_list(
            &format!("repos/{repo}/pulls/{n}/reviews?per_page=100"),
            None,
            cancel,
        )?;
        let timeline = self.api_list(
            &format!("repos/{repo}/issues/{n}/timeline?per_page=100"),
            None,
            cancel,
        )?;
        let fresh = self.pr_summary(repo, n, cancel)?;
        if fresh.head_sha != summary.head_sha
            || fresh.base_sha != summary.base_sha
            || fresh.pr.id != summary.pr.id
            || fresh.pr.number != summary.pr.number
            || fresh.pr.host != summary.pr.host
            || !fresh
                .pr
                .repository
                .eq_ignore_ascii_case(&summary.pr.repository)
        {
            return Err(error(GithubErrorKind::StaleHead,"PR identity, base or head changed while loading details; refresh before reviewing these files."));
        }
        let complete = files.len() as u64 >= fresh.changed_files;
        let coverage = Coverage {
            loaded: files.len(),
            total_matches: fresh.changed_files,
            complete,
            reasons: if complete {
                vec![]
            } else {
                vec!["GitHub's changed-file API is capped at 3000 files; displayed coverage is partial".into()]
            },
        };
        Ok(PrDetails {
            summary: fresh,
            comments,
            commits,
            files,
            reviews,
            timeline,
            coverage,
        })
    }
    pub fn pr_diff(&self, repo: &str, n: u64, cancel: &CancellationToken) -> GithubResult<String> {
        validate_pr(repo, n)?;
        let data = self.execute(
            vec![
                "pr".into(),
                "diff".into(),
                n.to_string(),
                "--repo".into(),
                format!("{}/{repo}", self.host),
                "--color".into(),
                "never".into(),
            ],
            None,
            None,
            cancel,
        )?;
        Ok(String::from_utf8_lossy(&data).into_owned())
    }
    /// Review callers retain immutable coordinates and reject a diff fetched across a head change.
    pub fn pr_diff_at(
        &self,
        target: &PrSummary,
        cancel: &CancellationToken,
    ) -> GithubResult<PrDiff> {
        let text = self.pr_diff(&target.pr.repository, target.pr.number, cancel)?;
        let fresh = self.pr_summary(&target.pr.repository, target.pr.number, cancel)?;
        if fresh.head_sha != target.head_sha || fresh.base_sha != target.base_sha {
            return Err(error(GithubErrorKind::StaleHead,"PR changed while loading the diff; refresh its base/head before selecting review lines."));
        }
        Ok(PrDiff {
            base_sha: target.base_sha.clone(),
            head_sha: target.head_sha.clone(),
            text,
        })
    }
    pub fn checks(
        &self,
        repo: &str,
        sha: &str,
        cancel: &CancellationToken,
    ) -> GithubResult<Vec<Check>> {
        validate_repo(repo)?;
        validate_sha(sha)?;
        let mut checks: Vec<Check> = self
            .api_list(
                &format!("repos/{repo}/commits/{sha}/check-runs?per_page=100"),
                Some("check_runs"),
                cancel,
            )?
            .iter()
            .map(|v| Check {
                id: number(v, "id"),
                name: text(v, "name"),
                status: text(v, "status"),
                conclusion: text(v, "conclusion"),
                url: text(v, "details_url"),
                head_sha: text(v, "head_sha"),
                kind: "check".into(),
                app_id: v
                    .get("app")
                    .and_then(|app| app.get("id"))
                    .and_then(Value::as_u64)
                    .filter(|id| *id > 0),
            })
            .collect();
        if checks.iter().any(|check| {
            check.id == 0
                || check.name.is_empty()
                // The exact observed head only: a similarly prefixed SHA is another commit.
                || !check.head_sha.eq_ignore_ascii_case(sha)
                || check.status.is_empty()
        }) {
            return Err(error(
                GithubErrorKind::Json,
                "Check runs are missing or mismatch the selected check/commit identity",
            ));
        }
        let mut status_contexts = BTreeSet::new();
        for v in self.api_list(
            &format!("repos/{repo}/commits/{sha}/statuses?per_page=100"),
            None,
            cancel,
        )? {
            let context = text(&v, "context");
            if context.is_empty() || number(&v, "id") == 0 {
                return Err(error(
                    GithubErrorKind::Json,
                    "Commit status has no context/ID",
                ));
            }
            // GitHub lists statuses newest first. Old results in one context are history,
            // not additional currently-failing checks beside the latest successful result.
            if !status_contexts.insert(context.to_lowercase()) {
                continue;
            }
            let state = text(&v, "state");
            checks.push(Check {
                id: number(&v, "id"),
                name: context,
                status: if state == "pending" {
                    "pending".into()
                } else {
                    "completed".into()
                },
                conclusion: if state == "pending" {
                    String::new()
                } else {
                    state
                },
                url: text(&v, "target_url"),
                head_sha: sha.into(),
                kind: "status".into(),
                app_id: None,
            });
        }
        Ok(checks)
    }
    fn resource(&self, repo: &str, n: u64) -> String {
        self.canonical_resource(&format!("{}/{repo}/pull/{n}", self.host))
    }
    fn guard(
        &self,
        identity: &HostIdentity,
        repo: &str,
        n: u64,
        sha: &str,
        cancel: &CancellationToken,
    ) -> GithubResult<PrSummary> {
        validate_pr(repo, n)?;
        validate_sha(sha)?;
        let current = self.identity(cancel)?;
        if !current.same_account(identity) {
            return Err(error(
                GithubErrorKind::IdentityChanged,
                "GitHub account changed. Refresh the selected account before publishing.",
            ));
        }
        let summary = self.pr_summary(repo, n, cancel)?;
        if summary.head_sha != sha {
            return Err(error(
                GithubErrorKind::StaleHead,
                "PR head changed. Refresh and review the new changes before publishing.",
            ));
        }
        Ok(summary)
    }
    pub fn comment(
        &self,
        identity: &HostIdentity,
        repo: &str,
        n: u64,
        sha: &str,
        body: &str,
        cancel: &CancellationToken,
    ) -> GithubResult<ConversationComment> {
        if body.trim().is_empty() {
            return Err(error(
                GithubErrorKind::InvalidTarget,
                "Comment cannot be empty",
            ));
        }
        self.guard(identity, repo, n, sha, cancel)?;
        let value = self.api(
            &format!("repos/{repo}/issues/{n}/comments"),
            "POST",
            Some(json!({"body":body})),
            Some(self.resource(repo, n)),
            false,
            cancel,
        )?;
        let comment = comment_from_json(&value);
        if comment.id == 0
            || comment.body != body
            || !comment.author.login.eq_ignore_ascii_case(&identity.login)
        {
            self.runner.mark_ambiguous(&self.resource(repo, n));
            return Err(error(GithubErrorKind::AmbiguousWrite,"Comment response did not confirm the requested author/text/identity; reconcile before retry"));
        }
        Ok(comment)
    }
    pub fn review(
        &self,
        identity: &HostIdentity,
        target: &PrSummary,
        event: ReviewEvent,
        body: &str,
        cancel: &CancellationToken,
    ) -> GithubResult<Value> {
        if !matches!(event, ReviewEvent::Approve) && body.trim().is_empty() {
            return Err(error(
                GithubErrorKind::InvalidTarget,
                "Comment and change-request reviews need a summary",
            ));
        }
        if !matches!(event, ReviewEvent::Comment)
            && target.pr.author.login.eq_ignore_ascii_case(&identity.login)
        {
            return Err(error(
                GithubErrorKind::Permission,
                "You cannot approve or request changes on your own PR",
            ));
        }
        let repo = &target.pr.repository;
        let n = target.pr.number;
        self.guard_target(identity, target, cancel)?;
        let result = self.api(
            &format!("repos/{repo}/pulls/{n}/reviews"),
            "POST",
            Some(json!({"event":event.api_value(),"body":body,"commit_id":target.head_sha})),
            Some(self.resource(repo, n)),
            false,
            cancel,
        )?;
        let state = match event {
            ReviewEvent::Approve => "APPROVED",
            ReviewEvent::RequestChanges => "CHANGES_REQUESTED",
            ReviewEvent::Comment => "COMMENTED",
        };
        if number(&result, "id") == 0
            || text(&result, "state") != state
            || text(&result, "commit_id") != target.head_sha
        {
            self.runner.mark_ambiguous(&self.resource(repo, n));
            return Err(error(GithubErrorKind::AmbiguousWrite,"Review response did not confirm the requested decision/head; reconcile before retry"));
        }
        Ok(result)
    }
    pub fn lifecycle(
        &self,
        identity: &HostIdentity,
        target: &PrSummary,
        action: PrLifecycle,
        cancel: &CancellationToken,
    ) -> GithubResult<PrSummary> {
        let repo = &target.pr.repository;
        let n = target.pr.number;
        self.guard_target(identity, target, cancel)?;
        let verb = match action {
            PrLifecycle::Close => "close",
            PrLifecycle::Reopen => "reopen",
            PrLifecycle::Ready => "ready",
            PrLifecycle::Draft => "ready",
        };
        let mut args = vec![
            "pr".into(),
            verb.into(),
            n.to_string(),
            "--repo".into(),
            format!("{}/{repo}", self.host),
        ];
        if matches!(action, PrLifecycle::Draft) {
            args.push("--undo".into());
        }
        self.execute(args, None, Some(self.resource(repo, n)), cancel)?;
        let fresh = self.after_write(&self.resource(repo, n), self.pr_summary(repo, n, cancel))?;
        let confirmed = match action {
            PrLifecycle::Close => fresh.pr.state == "closed",
            PrLifecycle::Reopen => fresh.pr.state == "open",
            PrLifecycle::Ready => !fresh.pr.draft,
            PrLifecycle::Draft => fresh.pr.draft,
        };
        if !confirmed {
            self.runner.mark_ambiguous(&self.resource(repo, n));
            // Partial outcome: the command ran, but the re-fetched PR does
            // not show the effect. Say what GitHub reports now.
            return Err(error(
                GithubErrorKind::AmbiguousWrite,
                format!(
                    "PR lifecycle state was not confirmed: GitHub re-read the PR as {}{} at head {}; reconcile before retrying",
                    if fresh.merged { "merged" } else { fresh.pr.state.as_str() },
                    if fresh.pr.draft { " (draft)" } else { "" },
                    fresh.head_sha.get(..7).unwrap_or(&fresh.head_sha),
                ),
            ));
        }
        Ok(fresh)
    }
    pub fn merge(
        &self,
        identity: &HostIdentity,
        target: &PrSummary,
        method: MergeMethod,
        auto: bool,
        cancel: &CancellationToken,
    ) -> GithubResult<PrSummary> {
        let repo = &target.pr.repository;
        let n = target.pr.number;
        let fresh = self.guard_target(identity, target, cancel)?;
        if fresh.pr.draft || fresh.pr.state != "open" {
            return Err(error(
                GithubErrorKind::InvalidTarget,
                "Only an open, ready PR can be merged",
            ));
        }
        let mut args = vec![
            "pr".into(),
            "merge".into(),
            n.to_string(),
            "--repo".into(),
            format!("{}/{repo}", self.host),
            method.flag().into(),
            "--match-head-commit".into(),
            target.head_sha.clone(),
        ];
        if auto {
            args.push("--auto".into());
        }
        self.execute(args, None, Some(self.resource(repo, n)), cancel)?;
        self.after_write(&self.resource(repo, n), self.pr_summary(repo, n, cancel))
    }
    /// Turn off a pending auto-merge request; read-back must show it gone.
    pub fn disable_auto_merge_request(
        &self,
        identity: &HostIdentity,
        target: &PrSummary,
        cancel: &CancellationToken,
    ) -> GithubResult<PrSummary> {
        let repo = &target.pr.repository;
        let n = target.pr.number;
        let fresh = self.guard_target(identity, target, cancel)?;
        if !fresh.auto_merge {
            return Err(error(
                GithubErrorKind::InvalidTarget,
                "Auto-merge is not enabled",
            ));
        }
        let args = vec![
            "pr".into(),
            "merge".into(),
            n.to_string(),
            "--repo".into(),
            format!("{}/{repo}", self.host),
            "--disable-auto".into(),
        ];
        self.execute(args, None, Some(self.resource(repo, n)), cancel)?;
        let fresh = self.after_write(&self.resource(repo, n), self.pr_summary(repo, n, cancel))?;
        if fresh.auto_merge {
            self.runner.mark_ambiguous(&self.resource(repo, n));
            return Err(error(
                GithubErrorKind::AmbiguousWrite,
                "Auto-merge still appears enabled after the request; reconcile before retrying",
            ));
        }
        Ok(fresh)
    }
    /// Rebase the PR head onto its base (`gh pr update-branch --rebase`).
    /// The merge-based update uses the head-pinned REST endpoint instead.
    pub fn rebase_pr_branch(
        &self,
        identity: &HostIdentity,
        target: &PrSummary,
        cancel: &CancellationToken,
    ) -> GithubResult<PrSummary> {
        let repo = &target.pr.repository;
        let n = target.pr.number;
        self.guard_target(identity, target, cancel)?;
        let args = vec![
            "pr".into(),
            "update-branch".into(),
            n.to_string(),
            "--repo".into(),
            format!("{}/{repo}", self.host),
            "--rebase".into(),
        ];
        self.execute(args, None, Some(self.resource(repo, n)), cancel)?;
        self.after_write(&self.resource(repo, n), self.pr_summary(repo, n, cancel))
    }
    pub fn reconcile_pr(
        &self,
        repo: &str,
        n: u64,
        cancel: &CancellationToken,
    ) -> GithubResult<PrDetails> {
        // Ordinary refresh only reads effects. Clearing an uncertain write requires
        // a provider-produced observation and explicit generation-stamped acknowledgement.
        self.pr_details(repo, n, cancel)
    }
    /// Browse one immutable tree. When recursive output is truncated, fetch subtrees individually.
    pub fn remote_tree(
        &self,
        repo: &str,
        sha: &str,
        recursive: bool,
        cancel: &CancellationToken,
    ) -> GithubResult<RemoteTree> {
        validate_repo(repo)?;
        validate_sha(sha)?;
        let mut endpoint = format!("repos/{repo}/git/trees/{sha}");
        if recursive {
            endpoint.push_str("?recursive=1");
        }
        let value = self.api(&endpoint, "GET", None, None, false, cancel)?;
        let entries = value
            .get("tree")
            .and_then(Value::as_array)
            .ok_or_else(|| error(GithubErrorKind::Json, "Remote tree response has no entries"))?
            .iter()
            .map(|v| RemoteTreeEntry {
                path: text(v, "path"),
                mode: text(v, "mode"),
                object_type: text(v, "type"),
                sha: text(v, "sha"),
                size: v.get("size").and_then(Value::as_u64),
            })
            .collect();
        Ok(RemoteTree {
            sha: text(&value, "sha"),
            entries,
            truncated: boolean(&value, "truncated"),
            recursive,
        })
    }
    /// Raw bytes stay bounded by the runner and are never executed or checked out.
    pub fn remote_blob(
        &self,
        repo: &str,
        sha: &str,
        cancel: &CancellationToken,
    ) -> GithubResult<Vec<u8>> {
        validate_repo(repo)?;
        validate_sha(sha)?;
        self.raw_spec(
            self.spec(vec![
                "api".into(),
                format!("repos/{repo}/git/blobs/{sha}"),
                "--hostname".into(),
                self.host.clone(),
                "--method".into(),
                "GET".into(),
                "--header".into(),
                "Accept: application/vnd.github.raw+json".into(),
            ]),
            cancel,
        )
    }
    pub fn workflows(&self, repo: &str, cancel: &CancellationToken) -> GithubResult<Vec<Workflow>> {
        validate_repo(repo)?;
        Ok(self
            .api_list(
                &format!("repos/{repo}/actions/workflows?per_page=100"),
                Some("workflows"),
                cancel,
            )?
            .iter()
            .map(actions::workflow_from_json_v2)
            .collect())
    }
    pub fn runs(
        &self,
        repo: &str,
        head_sha: Option<&str>,
        cancel: &CancellationToken,
    ) -> GithubResult<Vec<WorkflowRun>> {
        validate_repo(repo)?;
        let mut endpoint = format!("repos/{repo}/actions/runs?per_page=100");
        if let Some(sha) = head_sha {
            validate_sha(sha)?;
            endpoint.push_str(&format!("&head_sha={sha}"));
        }
        // Newest page only (see run_catalog): full run history exceeds capture.
        let page = self.api(&endpoint, "GET", None, None, false, cancel)?;
        let runs = Self::api_list_pages_v19(Value::Array(vec![page]), Some("workflow_runs"))?
            .iter()
            .map(run_from_json)
            .collect::<Vec<_>>();
        if runs.iter().any(|run| {
            run.id == 0
                || run.attempt == 0
                || validate_sha(&run.head_sha).is_err()
                || head_sha.is_some_and(|sha| run.head_sha != sha)
        }) {
            return Err(error(GithubErrorKind::Json,"Run catalog contains an incomplete or mismatched ID/attempt/commit; no empty/deleted state is inferred"));
        }
        Ok(runs)
    }
    pub fn run_details(
        &self,
        repo: &str,
        id: u64,
        cancel: &CancellationToken,
    ) -> GithubResult<RunDetails> {
        validate_pr(repo, id)?;
        let run = self.api(
            &format!("repos/{repo}/actions/runs/{id}"),
            "GET",
            None,
            None,
            false,
            cancel,
        )?;
        let attempt = number(&run, "run_attempt");
        if attempt == 0 {
            return Err(error(GithubErrorKind::Unsupported,"The selected host did not provide the run attempt; attempt-specific jobs cannot be safely targeted."));
        }
        let jobs = self.api_list(
            &format!("repos/{repo}/actions/runs/{id}/attempts/{attempt}/jobs?per_page=100"),
            Some("jobs"),
            cancel,
        )?;
        Ok(RunDetails {
            run: run_from_json(&run),
            jobs,
        })
    }
    pub fn run_logs(
        &self,
        repo: &str,
        id: u64,
        job: Option<u64>,
        cancel: &CancellationToken,
    ) -> GithubResult<String> {
        validate_pr(repo, id)?;
        let mut args = vec![
            "run".into(),
            "view".into(),
            id.to_string(),
            "--repo".into(),
            format!("{}/{repo}", self.host),
            "--log".into(),
        ];
        if let Some(job) = job {
            args.extend(["--job".into(), job.to_string()]);
        }
        self.execute(args, None, None, cancel).map(|v| {
            super::runner::sanitize_terminal(&actions::readable_run_log(&String::from_utf8_lossy(
                &v,
            )))
        })
    }
    pub fn run_control(
        &self,
        identity: &HostIdentity,
        repo: &str,
        id: u64,
        action: RunControl,
        cancel: &CancellationToken,
    ) -> GithubResult<WorkflowRun> {
        validate_pr(repo, id)?;
        let current = self.identity(cancel)?;
        if !current.same_account(identity) {
            return Err(error(
                GithubErrorKind::IdentityChanged,
                "GitHub account changed; refresh before changing a run",
            ));
        }
        let observed = self.api(
            &format!("repos/{repo}/actions/runs/{id}"),
            "GET",
            None,
            None,
            false,
            cancel,
        )?;
        if matches!(action, RunControl::Cancel) && text(&observed, "status") == "completed" {
            return Err(error(
                GithubErrorKind::InvalidTarget,
                "Run already completed; it cannot be cancelled",
            ));
        }
        let verb = if matches!(action, RunControl::Cancel) {
            "cancel"
        } else {
            "rerun"
        };
        let mut args = vec![
            "run".into(),
            verb.into(),
            id.to_string(),
            "--repo".into(),
            format!("{}/{repo}", self.host),
        ];
        if matches!(action, RunControl::RerunFailed) {
            args.push("--failed".into());
        }
        self.execute(
            args,
            None,
            Some(format!("{}/{repo}/run/{id}", self.host)),
            cancel,
        )?;
        self.api(
            &format!("repos/{repo}/actions/runs/{id}"),
            "GET",
            None,
            None,
            false,
            cancel,
        )
        .map(|v| run_from_json(&v))
    }
    pub fn reconcile_run(
        &self,
        repo: &str,
        id: u64,
        cancel: &CancellationToken,
    ) -> GithubResult<RunDetails> {
        let details = self.run_details(repo, id, cancel)?;
        self.runner
            .mark_reconciled(&self.canonical_resource(&format!("{}/{repo}/run/{id}", self.host)));
        Ok(details)
    }
    pub fn capabilities(&self) -> Vec<Capability> {
        capability_inventory()
    }
}
#[derive(Clone, Copy, Debug)]
pub enum ReviewEvent {
    Comment,
    Approve,
    RequestChanges,
}
impl ReviewEvent {
    fn api_value(self) -> &'static str {
        match self {
            Self::Comment => "COMMENT",
            Self::Approve => "APPROVE",
            Self::RequestChanges => "REQUEST_CHANGES",
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum PrLifecycle {
    Close,
    Reopen,
    Ready,
    Draft,
}
#[derive(Clone, Copy, Debug)]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}
impl MergeMethod {
    fn flag(self) -> &'static str {
        match self {
            Self::Merge => "--merge",
            Self::Squash => "--squash",
            Self::Rebase => "--rebase",
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum RunControl {
    Cancel,
    RerunAll,
    RerunFailed,
}
fn error(kind: GithubErrorKind, message: impl Into<String>) -> GithubError {
    GithubError {
        kind,
        message: message.into(),
    }
}
fn text(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}
fn number(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or_default()
}
fn boolean(v: &Value, key: &str) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or_default()
}
fn validate_name(name: &str) -> GithubResult<()> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.starts_with('-')
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        Err(error(
            GithubErrorKind::InvalidTarget,
            "Invalid repository or organization name",
        ))
    } else {
        Ok(())
    }
}
fn validate_repo(repo: &str) -> GithubResult<()> {
    let parts: Vec<_> = repo.split('/').collect();
    if parts.len() != 2 {
        return Err(error(
            GithubErrorKind::InvalidTarget,
            "Repository must be owner/name",
        ));
    }
    for part in parts {
        validate_name(part)?;
    }
    Ok(())
}
fn validate_pr(repo: &str, n: u64) -> GithubResult<()> {
    validate_repo(repo)?;
    if n == 0 {
        Err(error(
            GithubErrorKind::InvalidTarget,
            "Resource number must be positive",
        ))
    } else {
        Ok(())
    }
}
fn validate_sha(sha: &str) -> GithubResult<()> {
    if !(7..=64).contains(&sha.len()) || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        Err(error(
            GithubErrorKind::InvalidTarget,
            "Commit identity must be a hexadecimal SHA",
        ))
    } else {
        Ok(())
    }
}
fn comment_from_json(v: &Value) -> ConversationComment {
    ConversationComment {
        id: number(v, "id"),
        node_id: text(v, "node_id"),
        body: text(v, "body"),
        author: Actor::from_json(&v["user"]),
        created_at: text(v, "created_at"),
        updated_at: text(v, "updated_at"),
        url: text(v, "html_url"),
        card_v115: comment_card_facts_v115(v),
    }
}
fn comment_card_facts_v115(v: &Value) -> Option<Box<CommentCardFactsV115>> {
    let facts = CommentCardFactsV115 {
        author_association: text(v, "author_association"),
        reactions: [
            "+1", "-1", "laugh", "hooray", "confused", "heart", "rocket", "eyes",
        ]
        .into_iter()
        .filter_map(|content| {
            let count = v["reactions"][content]
                .as_u64()
                .filter(|count| *count > 0)?;
            Some((content.to_owned(), count))
        })
        .collect(),
    };
    (facts != CommentCardFactsV115::default()).then(|| Box::new(facts))
}
fn commit_from_json(v: &Value) -> CommitSummary {
    CommitSummary {
        sha: text(v, "sha"),
        message: text(&v["commit"], "message"),
        author: Actor::from_json(&v["author"]),
        committer: Actor::from_json(&v["committer"]),
        author_name: text(&v["commit"]["author"], "name"),
        committer_name: text(&v["commit"]["committer"], "name"),
        author_email: text(&v["commit"]["author"], "email"),
        committer_email: text(&v["commit"]["committer"], "email"),
        authored_at: text(&v["commit"]["author"], "date"),
        parents: v
            .get("parents")
            .and_then(Value::as_array)
            .map(|p| p.iter().map(|v| text(v, "sha")).collect())
            .unwrap_or_default(),
    }
}
fn file_from_json(v: &Value) -> ChangedFile {
    ChangedFile {
        path: text(v, "filename"),
        previous_path: v
            .get("previous_filename")
            .and_then(Value::as_str)
            .map(str::to_owned),
        status: text(v, "status"),
        additions: number(v, "additions"),
        deletions: number(v, "deletions"),
        patch: v.get("patch").and_then(Value::as_str).map(str::to_owned),
    }
}
fn run_from_json(v: &Value) -> WorkflowRun {
    WorkflowRun {
        id: number(v, "id"),
        workflow_id: number(v, "workflow_id"),
        name: text(v, "name"),
        title: text(v, "display_title"),
        head_sha: text(v, "head_sha"),
        head_branch: text(v, "head_branch"),
        event: text(v, "event"),
        status: text(v, "status"),
        conclusion: text(v, "conclusion"),
        attempt: number(v, "run_attempt"),
        actor: Actor::from_json(&v["actor"]),
        triggering_actor: Actor::from_json(&v["triggering_actor"]),
        created_at: text(v, "created_at"),
        updated_at: text(v, "updated_at"),
        url: text(v, "html_url"),
    }
}

/// Inventory describes implemented transports, not live server permission evidence.
pub fn capability_inventory() -> Vec<Capability> {
    let rows = [
        (
            "personal-queues",
            "Personal PR queues",
            "gh api search/issues",
            "read",
            "12",
            true,
        ),
        (
            "pr-details",
            "PR summary, timeline, commits, files and reviews",
            "gh api pulls/issues",
            "read",
            "13",
            true,
        ),
        (
            "pr-checks",
            "Checks, status, run jobs and logs",
            "gh api check-runs/statuses; gh run view",
            "read",
            "14",
            true,
        ),
        (
            "pr-create",
            "Create PR, choose forks/head/base",
            "gh pr create",
            "push",
            "26",
            false,
        ),
        (
            "pr-metadata",
            "Title, body, base, labels, assignees, reviewers and linked items",
            "gh pr edit; gh api",
            "triage/write",
            "18",
            false,
        ),
        (
            "pr-lifecycle",
            "Draft, ready, close and reopen",
            "gh pr ready/close/reopen",
            "author/write",
            "25",
            true,
        ),
        (
            "pr-lock-update-revert",
            "Lock, update branch, delete branch and revert",
            "gh pr lock/update-branch; git",
            "write",
            "25",
            false,
        ),
        (
            "pr-comments",
            "Create conversation comment",
            "gh api issues/comments",
            "comment",
            "17",
            true,
        ),
        (
            "pr-comment-edit-delete-react",
            "Edit/delete comments, reactions and subscription",
            "gh api",
            "author/write",
            "17",
            false,
        ),
        (
            "pr-inline",
            "Inline/multiline comments, replies and edits",
            "gh api pulls/comments",
            "comment",
            "19",
            false,
        ),
        (
            "pr-reviews",
            "Submit approval, change request or comment review",
            "gh api pulls/reviews",
            "review",
            "20",
            true,
        ),
        (
            "pr-pending-reviews",
            "Persistent pending reviews, discard and dismissal",
            "gh api pulls/reviews",
            "review/maintain",
            "20",
            false,
        ),
        (
            "pr-threads",
            "Resolve/reopen review threads",
            "gh api graphql",
            "write",
            "21",
            false,
        ),
        (
            "pr-viewed",
            "Viewed/unviewed files",
            "gh api graphql",
            "read",
            "22",
            false,
        ),
        (
            "pr-suggestions",
            "Preview and apply suggested changes",
            "isolated Git worktree",
            "push",
            "23",
            false,
        ),
        (
            "pr-conflicts",
            "Edit PR files and resolve conflicts",
            "isolated Git worktree",
            "push",
            "27",
            false,
        ),
        (
            "pr-merge",
            "Merge, squash, rebase and auto-merge",
            "gh pr merge --match-head-commit",
            "merge",
            "24",
            true,
        ),
        (
            "pr-queue",
            "Merge queue status and cancellation",
            "gh api graphql",
            "merge",
            "24",
            false,
        ),
        (
            "pr-navigation",
            "Copy, open and export",
            "native navigation; gh browse",
            "read",
            "10",
            false,
        ),
        (
            "actions-workflows",
            "Workflow catalog and SHA-related runs",
            "gh api actions/workflows/runs",
            "read",
            "52",
            true,
        ),
        (
            "actions-details",
            "Run attempts, matrix jobs, steps",
            "gh api actions/runs/jobs",
            "read",
            "53",
            true,
        ),
        (
            "actions-logs",
            "Bounded run/job logs",
            "gh run view --log",
            "read",
            "53",
            true,
        ),
        (
            "actions-dispatch",
            "Workflow YAML, inputs, enable/disable and dispatch",
            "gh workflow; gh api contents",
            "write",
            "54",
            false,
        ),
        (
            "actions-control",
            "Rerun all/failed and cancel runs",
            "gh run rerun/cancel",
            "write",
            "55",
            true,
        ),
        (
            "actions-delete",
            "Delete runs/logs, debug/specific-job rerun and force cancel",
            "gh api actions",
            "write",
            "55",
            false,
        ),
        (
            "actions-artifacts",
            "Artifact inventory, preview, download and delete",
            "gh api artifacts; gh run download",
            "read/write",
            "56",
            false,
        ),
        (
            "actions-approval",
            "Fork-run and environment approvals",
            "gh api pending_deployments",
            "write",
            "57",
            false,
        ),
        (
            "actions-cache",
            "Cache inventory, quota and deletion",
            "gh cache; gh api",
            "read/write",
            "58",
            false,
        ),
        (
            "actions-secrets-variables",
            "Secret metadata and variables across scopes",
            "gh secret/variable",
            "admin",
            "59",
            false,
        ),
        (
            "actions-policy",
            "Actions policies and workflow token permissions",
            "gh api actions/permissions",
            "admin",
            "60",
            false,
        ),
        (
            "actions-environments",
            "Environments and protection rules",
            "gh api environments",
            "admin",
            "60",
            false,
        ),
        (
            "actions-runners",
            "Runner/group status, labels and associations",
            "gh api runners",
            "admin",
            "60",
            false,
        ),
        (
            "actions-activity",
            "Cross-repository filters and dashboard drilldown",
            "native shared queries",
            "read",
            "62",
            false,
        ),
    ];
    rows.into_iter().map(|(id,label,transport,permission,task,implemented)|Capability {id:id.into(),label:label.into(),transport:transport.into(),permission:permission.into(),concurrency_guard:if permission=="read" {"Read only; bounded paging".into()}else{"Current account and resource state; explicit confirmation; serialize; reconcile ambiguity".into()},task:format!("TASK-{task}"),implemented,reason:if implemented {"Transport implemented; availability still depends on repository permissions, host features and current state. Offline fixtures do not prove live host compatibility.".into()}else{format!("Native workflow pending TASK-{task}; no browser handoff is counted as support.")},source_urls:vec!["https://cli.github.com/manual/gh_api".into(),if id.starts_with("actions") {"https://docs.github.com/en/rest/actions".into()} else {"https://docs.github.com/en/rest/pulls".into()}]}).collect()
}

#[cfg(test)]
mod resource_key_tests {
    use super::*;

    #[test]
    fn github_alias_keys_fold_only_documented_case_insensitive_identities() {
        let gh = GithubProvider::new("GITHUB.INVALID").unwrap();
        for (input, expected) in [
            (
                "GITHUB.INVALID/EXAMPLE/REPO/pull/7",
                "github.invalid/example/repo/pull/7",
            ),
            (
                "github.invalid/repos/EXAMPLE/REPO/actions/variable/mode",
                "github.invalid/repos/example/repo/actions/variable/MODE",
            ),
            (
                "github.invalid/orgs/EXAMPLE/actions/secrets/deploy_token/repositories",
                "github.invalid/orgs/example/actions/secrets/DEPLOY_TOKEN/repositories",
            ),
            (
                "github.invalid/repos/EXAMPLE/REPO/environments/PROD/secret/deploy_token",
                "github.invalid/repos/example/repo/environments/prod/secret/DEPLOY_TOKEN",
            ),
            (
                "github.invalid/EXAMPLE/REPO/create/owner:Feature/SECRET/Main",
                "github.invalid/example/repo/create/owner:Feature/SECRET/Main",
            ),
            ("git:/CaseSensitive/Checkout", "git:/CaseSensitive/Checkout"),
        ] {
            assert_eq!(gh.canonical_resource(input), expected);
        }
    }
}
