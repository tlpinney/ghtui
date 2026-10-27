use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::layout::Rect;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Inbox,
    Repository,
    Search,
    Actions,
    Dashboard,
}

impl View {
    pub const ALL: [Self; 5] = [
        Self::Inbox,
        Self::Repository,
        Self::Search,
        Self::Actions,
        Self::Dashboard,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::Repository => "Repository",
            Self::Search => "Search",
            Self::Actions => "Actions",
            Self::Dashboard => "Dashboard",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|v| *v == self).unwrap_or(0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    NotificationsV63(crate::notifications_ui_v63::NotificationActionV63),
    ScopedReviewsV2(crate::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::browser_v2::ReviewBrowserActionV2),
    OpenScopedReviewsV2,
    StashRecoveryV1(crate::repository_stash_recovery_v1::StashRecoveryActionV1),
    View(View),
    Help,
    Palette,
    PaletteInput(crate::command_palette::PaletteAction),
    HelpScroll(i32),
    Navigation(crate::navigation_trail::NavigationAction),
    Capabilities,
    Inspector(crate::capability_inspector::InspectorAction),
    Close,
    Quit,
    Refresh,
    Cancel,
    Settings,
    SettingsField(usize),
    SettingsTheme,
    SettingsDensity,
    SettingsSave,
    RepositoryMode(crate::workspace::RepositoryMode),
    RepositoryRow(crate::workspace::RowAction),
    /// TASK-99.3: History with a `path:` chip for a file chosen in Search or PR Files.
    FileHistoryV993(crate::history::entry::FileHistoryTargetV993),
    OriginalDiffSidesV1(crate::repository_diff_sides_v1::OriginalDiffActionV1),
    NativeDiffV1(crate::repository_native_diff_ui_v1::NativeDiffActionV1),
    NativeMutableDiffV2(
        crate::repository_native_diff_ui_v1::NativeDiffActionV1<crate::git::MutableDiffSourceV2>,
    ),
    NativeMutablePlanV2(crate::repository_native_mutable_plan_native_v2::NativeMutablePlanActionV2),
    OriginalTreeV1(crate::repository_tree_jump_v1::OriginalTreeJumpActionV1),
    OriginalTreeViewportV2(crate::repository_tree_viewport_v2::TreeViewportActionV2),
    GraphRereadV3(crate::repository_graph_reread_native_v3::GraphRereadActionV3),
    LocalActor(crate::repository_commit_actor_native_v1::LocalActorActionV1),
    RelatedPrV1(crate::repository_related_pr_v1::RelatedPrActionV1),
    Scroll(i32),
    Select(usize),
    Github(crate::github_workspace::GithubAction),
    GithubRow(usize),
    /// Show the selected Timeline item's comment, review or commit in its own tab.
    PrTimelineGoV13,
    PrChecks(crate::pr_checks_native::PrChecksUiAction),
    PrCheckChoice(crate::pr_checks_native::scoped::choices::PrCheckChoiceUiV1),
    PrCheckContinueV2(
        crate::pr_checks_native::scoped::choices::continuations_v2::ui::PrCheckContinueUiV2,
    ),
    Catalogue(crate::repository_catalogue_native::CatalogueUiAction),
    Pr(PrUiAction),
    Form(crate::forms::FormAction),
    LocalMenu,
    LocalCommand(crate::local_actions::LocalCommand),
    LocalQuick(crate::local_actions::QuickLocal),
    /// Conversation card buttons and the PR action bar (TASK-115).
    PrCardsV115(crate::pr_conversation_cards_v115::CardsActionV115),
    /// Repository panels (TASK-93): the untracked toggle.
    RepositoryPanelsV93(crate::repository_panels_v93::PanelActionV93),
    /// Worktrees (TASK-118): the way back, and the stale `core.worktree` screen's choices.
    WorktreeV118(crate::repository_worktrees_v118::WorktreeActionV118),
    /// History commit detail (TASK-99.4): select or open an item, show the SHA to copy.
    HistoryDetailV994(crate::history::detail::DetailUiActionV994),
    MenuScroll(i32),
    Search(crate::search::workspace::SearchAction),
    SearchBlend(crate::search::fusion::ui::BlendAction),
    SearchRankDetails,
    RepositoryHunk,
    /// The History filter row: search, matches, hide, branch scope, chips (TASK-99.3).
    HistoryFilterV993(crate::repository_ui::history_filter::HistoryFilterActionV993),
    RepositorySource,
    Actions(crate::actions_workspace::ActionsAction),
    Explain(String),
    NoticeScroll(i32),
    Compare,
    Corpus,
    SearchSource(crate::search_sources::SourceAction),
    PrReactionScroll(i32),
    Metrics(crate::metrics_workspace::MetricsWorkspaceAction),
    PrFileImage(
        crate::scoped_pr_observed_tabs_v1::resources::immutable_images::ui::PrImageUiActionV1,
    ),
    /// TASK-27: edit a PR branch file or resolve its conflicts in an isolated revision.
    PrBranchEditV27(crate::pr_branch_edit_v27::BranchEditActionV27),
    /// TASK-120: copy the focused item, or release/restore mouse capture.
    Clipboard(crate::clipboard::ClipboardAction),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrUiAction {
    Command(crate::pr_actions::PrCommand),
    StateV68(crate::pr_state_model::PrStateCommand),
    StateHistoryV74(crate::pr_state_model::PrStateCommand),
    MetadataTextV25(crate::providers::gh::PrMetadataTextFieldV25),
    MetadataHistoryV25(crate::providers::gh::PrMetadataTextFieldV25),
    MetadataBaseV32,
    MetadataMembersV48(crate::pr_metadata_actions::PrMetadataCommand),
    MetadataMembersHistoryV48,
    MetadataBaseHistoryV32,
    IssueLinksV87(crate::pr_issue_links_v87::IssueLinkCommandV87),
    IssueLinkHistoryV87(crate::pr_issue_links_v87::IssueLinkCommandV87),
    Metadata(crate::pr_metadata_actions::PrMetadataCommand),
    Inspect,
    Acknowledge,
    BrowseReactions,
}
pub type SharedPrInspection =
    std::sync::Arc<std::sync::Mutex<Option<crate::pr_write_journal::PrJournalInspection>>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlay {
    NotificationsV63,
    Help,
    Palette,
    Capabilities,
    Settings,
    Form,
    PrActions,
    LocalActions,
    Notice,
    PrReactions,
    SearchSources,
    SearchContext,
    SearchBlend,
    SearchRankDetails,
    RepositorySource,
    RepositoryCatalogue,
    PrFileImages,
    ScopedReviewsV2,
    PrCheckChoices,
    PrThreadsV1,
    PrThreadHistoryV1,
    PrCheckContinuationV2,
    ScopedActionsReadV2,
    GraphRereadV3,
}

#[derive(Clone, Debug)]
pub struct HitRegion {
    pub rect: Rect,
    pub action: Action,
}

#[derive(Clone, Debug)]
pub struct DiffSelection {
    pub start: crate::repository_view::DiffAnchor,
    pub end: crate::repository_view::DiffAnchor,
}
impl DiffSelection {
    fn same_file(&self, anchor: &crate::repository_view::DiffAnchor) -> bool {
        self.start.file_index == anchor.file_index
            && self.start.base == anchor.base
            && self.start.head == anchor.head
            && self.start.old_path == anchor.old_path
            && self.start.new_path == anchor.new_path
    }
    fn contains(&self, anchor: &crate::repository_view::DiffAnchor) -> bool {
        let first = (self.start.hunk_index, self.start.line_index);
        let last = (self.end.hunk_index, self.end.line_index);
        let current = (anchor.hunk_index, anchor.line_index);
        self.same_file(anchor) && current >= first.min(last) && current <= first.max(last)
    }
}

/// Last painted legacy pane only. Presentation geometry is not source,
/// selection, account, or publication authority; native widgets own theirs.
#[derive(Default)]
pub(crate) struct PointerRegion {
    owner: Option<(View, Option<Overlay>)>,
    area: Rect,
}
impl PointerRegion {
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
    pub(crate) fn capture(&mut self, view: View, overlay: Option<Overlay>, area: Rect) {
        self.owner = Some((view, overlay));
        self.area = area;
    }
    fn contains(&self, view: View, overlay: Option<Overlay>, column: u16, row: u16) -> bool {
        self.owner == Some((view, overlay)) && self.area.contains((column, row).into())
    }
}

/// One state owner; drawing reads prepared content and never invokes a provider.
pub struct App {
    /// TASK-120: drag selection, pending OSC 52 copy and mouse-capture release.
    pub copy_v120: crate::clipboard::CopyStateV120,
    pub notifications_v63: Option<crate::notifications_ui_v63::NotificationBrowserV63>,
    pub notification_pr_source_v63: Option<crate::notification_pr_v63::NotificationPrStateV63>,
    pub notifications_work_v63: Option<Arc<crate::notifications_v63::NotificationWorkV63>>,
    pub notifications_owner_v63: Option<Arc<crate::notifications_v63::NotificationWorkV63>>,
    pub notifications_cancel_v63: bool,
    /// The configured refresh interval (`refresh_seconds`). A notification
    /// watch never polls faster than this (at least 15 seconds) nor faster
    /// than the server's `X-Poll-Interval`/`Retry-After`.
    pub notifications_watch_seconds_v63: u64,
    pub scoped_actions_v1: Option<crate::scoped_actions_native_v1::ScopedActionsStateV1>,
    pub scoped_actions_work_v1: Option<Arc<crate::scoped_actions_v1::ScopedActionsWorkV1>>,
    pub scoped_actions_owner_v1: Option<Arc<crate::scoped_actions_native_v1::ScopedActionsOwnerV1>>,
    pub scoped_actions_cancel_v1: bool,
    /// Mechanical process-lifecycle fence only; never source/account authority.
    pub native_external_admission_v4: Arc<crate::providers::owned_pty_viewer_v4::ViewerAdmissionV4>,
    pub pr_native_diff_v1: Option<crate::scoped_pr_native_diff_native_v1::PrNativeStateV1>,
    pub pr_native_diff_layout_v1: crate::scoped_pr_native_diff_native_v1::PrNativeLayoutV1,
    pub pr_native_diff_work_v1: Option<Arc<crate::scoped_pr_native_diff_native_v1::PrNativeWorkV1>>,
    pub pr_native_diff_owner_v1: Option<Arc<crate::scoped_pr_native_diff_native_v1::PrNativeOwnerV1>>,
    pub pr_native_diff_cancel_v1: bool,
    pub scoped_threads_work_v1: Option<Arc<crate::scoped_pr_observed_tabs_v1::resources::threads_v1::native_v1::ScopedThreadsOwnerV1>>,
    pub scoped_threads_owner_v1: Option<Arc<crate::scoped_pr_observed_tabs_v1::resources::threads_v1::native_v1::ScopedThreadsOwnerV1>>,
    pub scoped_threads_cancel_v1: bool,
    pub scoped_threads_v1: Option<crate::scoped_pr_observed_tabs_v1::resources::threads_v1::ui_v1::ThreadBrowserV1>,
    /// TASK-27: the PR branch edit or conflict resolution in its isolated revision.
    pub pr_branch_edit_v27: Option<crate::pr_branch_edit_v27::PrBranchEditViewV27>,
    pub scoped_thread_code_return_v4: Option<crate::scoped_pr_observed_tabs_v1::resources::threads_v1::code_v4::ThreadCodeReturnV4>,
    pub scoped_thread_history_v1:Option<crate::scoped_pr_observed_tabs_v1::resources::threads_v1::history_v1::ui::ThreadHistoryBrowserV1>,
    pub scoped_inline_work_v1: Option<Arc<crate::scoped_pr_observed_tabs_v1::resources::immutable_images::inline_v1::native_v1::ScopedInlineWorkV1>>,
    pub scoped_inline_owner_v1: Option<Arc<crate::scoped_pr_observed_tabs_v1::resources::immutable_images::inline_v1::native_v1::ScopedInlineOwnerV1>>,
    pub scoped_inline_cancel_v1: bool,
    pub scoped_pr_collaboration_work_v17: Option<Arc<crate::scoped_pr_collaboration_native_v17::ScopedPrCollaborationWorkV17>>,
    pub scoped_pr_collaboration_return_v19: Option<crate::scoped_pr_collaboration_refresh_v19::ConversationReturnStateV19>,
    pub scoped_pr_collaboration_owner_v17: Option<Arc<crate::scoped_pr_collaboration_native_v17::ScopedPrCollaborationOwnerV17>>,
    pub scoped_pr_collaboration_cancel_v17: bool,
    pub(crate) closed_conversation_draft_v20: Option<crate::scoped_pr_collaboration_ui_v17::pure_close_v20::ClosedConversationDraftV20>,
    pub scoped_reviews_work_v2: Option<Arc<crate::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::native_v2::ScopedReviewsWorkV2>>,
    pub scoped_reviews_owner_v2: Option<Arc<crate::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::native_v2::ScopedReviewsOwnerV2>>,
    pub scoped_reviews_cancel_v2: bool,
    pub scoped_reviews_v2: Option<crate::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::browser_v2::ReviewBrowserV2>,
    pub scoped_review_history_v3: Option<crate::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::operations_v3::history_v3::ui::ReviewOperationHistoryViewV3>,
    pub original_stash_recovery_request_v1:
        Option<std::sync::Arc<crate::repository_stash_recovery_v1::OriginalStashRecoveryIntentV1>>,
    pub original_stash_recovery_cancel_requested_v1: bool,
    pub original_stash_recovery_v1:
        Option<crate::repository_stash_recovery_v1::StashRecoveryStateV1>,
    pub view: View,
    pub overlay: Option<Overlay>,
    pub quit: bool,
    pub status: String,
    pub context: String,
    pub content: [Vec<String>; 5],
    pub hits: Vec<HitRegion>,
    pub focus: usize,
    /// The Inbox (GitHub view) list: queue or open PR page rows.
    pub inbox_list: crate::ui_table::ListState,
    /// The Repository list (Overview, Graph, Tree, … display rows).
    pub repository_list: crate::ui_table::ListState,
    /// The plain content scroll of the remaining views (Search, Actions, Dashboard).
    pub content_list: crate::ui_table::ListState,
    /// Each view's list state while another view is shown, so switching away and back keeps
    /// the user's place (TASK-10). Search, Actions and Dashboard share `content_list`.
    pub view_lists_v10: [crate::ui_table::ListState; 5],
    pub palette_index: usize,
    pub command_palette: Option<crate::command_palette::CommandPalette>,
    pub command_palette_context: Option<crate::navigation_catalogue::CatalogueContext>,
    pub command_source_epoch: u64,
    pub command_context_exhausted: bool,
    pub context_help: Option<crate::context_help::ContextHelp>,
    pub context_help_scroll: usize,
    pub context_help_rows: usize,
    pub context_help_area: Rect,
    /// Keep presentation geometry outside the already-wide inline App layout.
    pub(crate) help_scrollbar: Box<crate::scrollbar::Scrollbar>,
    pub(crate) pointer_region: Box<PointerRegion>,
    pub navigation_trail: crate::navigation_trail::NavigationTrail,
    pub original_saved_visits: crate::navigation_local_source_v1::OriginalSavedVisitsV1,
    pub original_diff_picker_v1: Option<crate::repository_diff_sides_v1::OriginalDiffPickerV1>,
    pub native_diff_v1: Option<crate::repository_native_diff_v1::NativeDiffStateV1>,
    pub native_diff_layout_v1: crate::repository_native_diff_ui_v1::NativeDiffLayoutV1,
    pub native_mutable_diff_v2: Option<
        crate::repository_native_diff_v1::NativeDiffStateV1<crate::git::MutableDiffSourceV2>,
    >,
    pub native_mutable_diff_layout_v2:
        crate::repository_native_diff_ui_v1::NativeDiffLayoutV1<crate::git::MutableDiffSourceV2>,
    pub native_mutable_plan_v2:
        Option<crate::repository_native_mutable_plan_native_v2::NativeMutablePlanUiV2>,
    pub original_diff_work_v1:
        Option<Arc<crate::repository_diff_sides_worker_v1::OriginalDiffWorkV1>>,
    pub original_diff_layout_v1: crate::repository_diff_sides_ui_v1::OriginalDiffLayoutV1,
    pub original_diff_cancel_v1: bool,
    pub original_tree_jump_v1: Option<crate::repository_tree_jump_v1::OriginalTreeJumpV1>,
    pub original_tree_work_v1: Option<Arc<crate::repository_tree_worker_v1::OriginalTreeWorkV1>>,
    pub original_tree_layout_v1: crate::repository_tree_jump_ui_v1::OriginalTreeLayoutV1,
    pub original_tree_cancel_v1: bool,
    pub original_tree_viewport_v2: crate::repository_tree_viewport_native_v2::TreeViewportNativeV2,
    pub graph_reread_dialog_v3:
        Option<crate::repository_graph_reread_native_v3::GraphRereadDialogV3>,
    pub graph_reread_request_v3:
        Option<Arc<crate::repository_graph_reread_v3::GraphRereadPreviewV3>>,
    pub navigation_restore: Option<crate::navigation_state::NavigationRestore>,
    pub navigation_restore_inflight: bool,
    pub navigation_restore_focus: Option<String>,
    pub capability_inspector: Option<crate::capability_inspector::CapabilityInspector>,
    pub capability_hits: Vec<crate::capability_inspector::InspectorHit>,
    pub no_color: bool,
    pub refresh_requested: bool,
    pub cancel_requested: bool,
    pub scoped_pr_tab_cancel_requested: bool,
    pub loading: [bool; 5],
    pub settings_host: String,
    pub settings_set: String,
    pub settings_theme: usize,
    /// List density (TASK-96): applied live like the theme, saved with Save settings.
    pub density: crate::config::Density,
    pub settings_cursor: usize,
    pub settings_save_requested: bool,
    pub bindings: BTreeMap<String, String>,
    pub user_navigated: bool,
    pub workspace: Option<crate::workspace::Workspace>,
    pub local_commit_actors: crate::repository_commit_actor_native_v1::LocalCommitActorNativeV1,
    pub local_actor_layout: crate::repository_commit_actor_ui_v1::LocalActorLayoutV1,
    pub local_actor_requested: bool,
    pub local_actor_cancel_requested: bool,
    pub related_pr_v1: Option<crate::repository_related_pr_v1::RelatedPrStateV1>,
    pub related_pr_requested_v1: bool,
    pub related_pr_cancel_v1: bool,
    pub repository_source_identity:
        Option<std::sync::Arc<crate::repository_source_identity::PreparedRepositorySourceIdentity>>,
    pub repository_source_overlay:
        Option<std::sync::Arc<crate::repository_source_identity::PreparedRepositorySourceIdentity>>,
    pub repository_source_scroll: usize,
    pub repository_source_area: Rect,
    pub repository_source_rows: usize,
    pub repository_source_return_focus: Option<(usize, bool)>,
    pub repository_mode: crate::workspace::RepositoryMode,
    pub repository_load_requested: bool,
    pub repository_action: Option<crate::workspace::RowAction>,
    /// Another listed worktree to load read-only in place of the current one (TASK-41).
    pub repository_directory_request_v41: Option<std::path::PathBuf>,
    /// The worktree shown before another one was opened from the stack (TASK-118).
    pub repository_home_v118: Option<std::path::PathBuf>,
    /// The stale `core.worktree` screen, while the Repository read reports one (TASK-118).
    pub core_worktree_v118: Option<crate::repository_worktrees_v118::MismatchStateV118>,
    /// A confirmed unset of the previewed stale `core.worktree`, for the worker.
    pub core_worktree_unset_v118: Option<crate::git::CoreWorktreeMismatchV118>,
    /// The History query while it has keyboard focus (TASK-99.3).
    pub history_filter_input_v993: Option<String>,
    /// `owner/name` of the launch checkout's GitHub remote: PR files open File history only
    /// when the PR is in this repository (TASK-99.3).
    pub local_github_repository_v993: Option<String>,
    pub repository_content_focused: bool,
    pub repository_visible_rows: usize,
    /// Repository panels (TASK-93): panel scroll offsets and the untracked toggle.
    pub repository_panels_v93: crate::repository_panels_v93::PanelsV93,
    /// The pane and shell click targets around an open Repository viewer (TASK-93).
    pub takeover_v93: crate::repository_panels_v93::takeover::TakeoverChromeV93,
    /// History commit detail (TASK-99.4): item cursor, scroll and changed-files cache.
    pub history_detail_v994: crate::history::detail::HistoryDetailStateV994,
    /// The commit an arrow key last chose in History while its read runs (TASK-99): a bar
    /// moved further during that read stays where the person put it.
    pub history_bar_choice_v99: Option<String>,
    /// Where History stood when its next page was asked for (TASK-109): the page that comes
    /// back keeps the bar and the viewport on the same commits.
    pub history_place_v109: Option<crate::history::place::HistoryPlaceV109>,
    pub github:Option<crate::github_workspace::GithubWorkspace>,
    pub github_action: Option<crate::github_workspace::GithubAction>,
    pub github_content_focused: bool,
    /// Which pane of a list plus preview split owns the content focus (TASK-92).
    pub content_pane: crate::list_preview_v92::Pane,
    /// The painted preview pane, when the current view shows one beside its list.
    pub preview_area_v92: Option<ratatui::layout::Rect>,
    /// Cells of lanes the History GRAPH column has grown to; reset when History closes.
    pub history_graph_width_v99: usize,
    /// Lines the preview pane is scrolled by, and the item it was scrolled on.
    pub preview_scroll_v92: usize,
    pub preview_item_v92: String,
    pub github_visible_rows: usize,
    /// Review and check state of PR pages read this session, shown on Inbox rows (TASK-12).
    pub inbox_status_v1: crate::inbox_status_v1::InboxStatusMemo,
    pub pr_checks_origin: Option<std::sync::Arc<crate::pr_checks_native::PrChecksOrigin>>,
    pub pr_checks_route: Option<std::sync::Arc<crate::pr_checks_native::PrChecksRoute>>,
    pub pr_checks_request: bool,
    pub pr_check_choice_picker:
        Option<crate::pr_checks_native::scoped::choices::PrCheckChoicePickerV1>,
    pub pr_check_choice_request:
        Option<crate::pr_checks_native::scoped::choices::PrCheckChoiceWorkV1>,
    pub pr_check_continuation_request_v2: Option<
        crate::pr_checks_native::scoped::choices::continuations_v2::PrCheckContinuationWorkV2,
    >,
    pub pr_check_continuation_owner_v2: Option<
        std::sync::Arc<
            crate::pr_checks_native::scoped::choices::continuations_v2::PrCheckContinuationOwnerV2,
        >,
    >,
    pub pr_check_watch_v2: Option<
        std::sync::Arc<crate::pr_checks_native::scoped::choices::continuations_v2::PrCheckWatchV2>,
    >,
    pub pr_check_log_matches_v2:
        Option<crate::pr_checks_native::scoped::choices::continuations_v2::LoadedLogMatchesV2>,
    pub pr_check_log_editor_v2:
        Option<crate::pr_checks_native::scoped::choices::continuations_v2::ui::PrCheckLogEditorV2>,
    pub pr_check_annotations_detail_v2:
        Option<crate::pr_checks_native::scoped::choices::continuations_v2::ui::AnnotationDetailV2>,
    pub pr_check_attempt_advance_v2:
        Option<std::sync::Arc<crate::providers::gh::PrCheckAttemptAdvanceV2>>,
    pub pr_check_continuation_cancel_v2: bool,
    pub pr_checks_cancel_actions: bool,
    /// A Timeline item opened in another tab and the Timeline place to return to (TASK-13).
    pub pr_timeline_return_v13: Option<crate::pr_timeline_links_v13::TimelineReturnV13>,
    pub catalogue: Option<crate::repository_catalogue_native::CatalogueModal>,
    pub remote_repository: Option<crate::remote_repository_native_v1::RemoteRepositoryNativeV1>,
    pub remote_repository_cancel_requested: bool,
    pub catalogue_cancel_requested: bool,
    pub remote_scope: Option<std::sync::Arc<crate::active_remote_scope_v1::ActiveRemoteScopeV1>>,
    pub remote_scope_source: Option<std::sync::Arc<crate::storage::CatalogueIntentSource>>,
    /// TASK-105: ticket for a Personal scope fenced by a confirmed review write.
    pub personal_readmission_v105:
        Option<crate::personal_readmission_v105::PersonalReadmissionV105>,
    /// TASK-105: configured host + repository scope of Personal startup reads.
    pub personal_repository_scope_v105:
        crate::personal_readmission_v105::PersonalRepositoryScopeV105,
    pub personal_pr_entry_work_v13:
        Option<std::sync::Arc<crate::personal_pr_entry_v13::PersonalPrEntryWorkV13>>,
    pub personal_pr_entry_owner_v13:
        Option<std::sync::Arc<crate::personal_pr_entry_v13::PersonalPrEntryOwnerV13>>,
    pub personal_pr_entry_cancel_v13: bool,
    pub personal_pr_source_v13: Option<crate::personal_pr_entry_v13::PersonalPrEntryStateV13>,
    pub avatar_states: crate::actor_avatar_states_v1::ActorAvatarStatesV1,
    pub form: Option<crate::forms::Form>,
    pub pr_action_index: usize,
    pub pr_menu_rows: usize,
    pub pr_action: Option<PrUiAction>,
    pub pr_inspection: Option<SharedPrInspection>,
    pub pr_pending_message: String,
    pub pr_file_images:
        Option<crate::scoped_pr_observed_tabs_v1::resources::immutable_images::ui::PrImageViewerV1>,
    pub pr_line_comment_target_v19: Option<
        crate::scoped_pr_observed_tabs_v1::resources::immutable_images::line_comment_v19::LineCommentTargetV19,
    >,
    pub pr_reactions: Option<crate::pr_actions::PrReactionPage>,
    pub pr_reaction_display: Vec<crate::pr_ui::PrReactionDisplayRow>,
    /// A Quote reply waiting for its New comment composer to open (TASK-115).
    pub quote_reply_v115: Option<String>,
    pub pr_reaction_scroll: usize,
    pub pr_reaction_rows: usize,
    pub color_mode: crate::theme::ColorMode,
    pub local_command: Option<crate::local_actions::LocalCommand>,
    /// Prepare the staging preview with the form (direct + / - shortcuts).
    pub local_command_preview: bool,
    pub local_action_index: usize,
    pub form_dismissed: bool,
    pub repository_reconcile_requested: bool,
    pub search: Option<crate::search::workspace::SearchWorkspace>,
    pub search_controls: crate::search_ui::SearchControls,
    pub search_action: Option<crate::search::workspace::SearchAction>,
    pub search_load_requested: bool,
    pub repository_selection: Option<DiffSelection>,
    pub actions: Option<crate::actions_workspace::ActionsWorkspace>,
    pub actions_inspection: Option<crate::actions_recovery::SharedActionsInspection>,
    pub actions_action: Option<crate::actions_workspace::ActionsAction>,
    pub actions_load_requested: bool,
    pub actions_content_focused: bool,
    pub actions_visible_rows: usize,
    pub notice: String,
    pub notice_scroll: usize,
    pub compare_requested: bool,
    pub corpus_requested: bool,
    pub source_controls: crate::search_sources::SourceControls,
    pub source_action: Option<crate::search_sources::SourceAction>,
    pub source_context: Option<crate::search_sources::context::SourceContext>,
    pub source_context_scroll: usize,
    pub source_context_visible: usize,
    pub metrics: Option<crate::metrics_workspace::MetricsWorkspace>,
    pub metrics_action: Option<crate::metrics::MetricsAction>,
    pub metrics_load_requested: bool,
    pub metrics_content_focused: bool,
    /// Search content owns focus after entering the view, until Tab moves to a control.
    pub search_content_focused: bool,
    pub metrics_content: Rect,
    pub metrics_visible_rows: usize,
    pub metrics_selected_repository: Option<String>,
    pub metrics_days: u64,
    pub github_origin: Option<View>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            copy_v120: Default::default(),
            native_external_admission_v4:
                crate::providers::owned_pty_viewer_v4::ViewerAdmissionV4::new(),
            pr_native_diff_v1: None,
            scoped_actions_v1: None,
            scoped_actions_work_v1: None,
            scoped_actions_owner_v1: None,
            scoped_actions_cancel_v1: false,
            pr_native_diff_layout_v1: Default::default(),
            pr_native_diff_work_v1: None,
            pr_native_diff_owner_v1: None,
            pr_native_diff_cancel_v1: false,
            notifications_v63: None,
            notification_pr_source_v63: None,
            notifications_work_v63: None,
            notifications_owner_v63: None,
            notifications_cancel_v63: false,
            notifications_watch_seconds_v63: 60,
            scoped_threads_work_v1: None,
            scoped_threads_owner_v1: None,
            scoped_threads_cancel_v1: false,
            scoped_threads_v1: None,
            pr_branch_edit_v27: None,
            scoped_thread_code_return_v4: None,
            scoped_thread_history_v1: None,
            scoped_inline_work_v1: None,
            scoped_inline_owner_v1: None,
            scoped_inline_cancel_v1: false,
            scoped_pr_collaboration_work_v17: None,
            scoped_pr_collaboration_return_v19: None,
            scoped_pr_collaboration_owner_v17: None,
            scoped_pr_collaboration_cancel_v17: false,
            closed_conversation_draft_v20: None,
            scoped_reviews_work_v2: None,
            scoped_reviews_owner_v2: None,
            scoped_reviews_cancel_v2: false,
            scoped_reviews_v2: None,
            scoped_review_history_v3: None,
            original_stash_recovery_request_v1: None,
            original_stash_recovery_cancel_requested_v1: false,
            original_stash_recovery_v1: None,
            view: View::Inbox,
            overlay: None,
            quit: false,
            status: "Ready • gh handles GitHub authentication".into(),
            context: "github.com / personal queues / no repository selected".into(),
            content: [
                vec![
                    "Your pull requests across repositories".into(),
                    "Created by me • Assigned to me • Needs my review".into(),
                    "Use Refresh to load your authenticated GitHub inbox.".into(),
                    "Press r (or click Refresh below) to load your PRs • : all commands • ? help"
                        .into(),
                ],
                vec![
                    "Repository workspace".into(),
                    "Commit = a saved version. Branch = a name pointing to a commit.".into(),
                    "Local repository discovery will show your working files and history.".into(),
                ],
                vec![
                    "Search repositories, code and conversations".into(),
                    "Choose a visible scope and search mode before querying.".into(),
                ],
                vec![
                    "GitHub Actions".into(),
                    "Workflow runs, jobs, logs and artifacts appear here.".into(),
                ],
                vec![
                    "Dashboard".into(),
                    "No loaded records. Missing data is not zero.".into(),
                ],
            ],
            hits: Vec::new(),
            focus: 0,
            inbox_list: Default::default(),
            repository_list: Default::default(),
            content_list: Default::default(),
            view_lists_v10: Default::default(),
            palette_index: 0,
            command_palette: None,
            command_palette_context: None,
            command_source_epoch: 0,
            command_context_exhausted: false,
            context_help: None,
            context_help_scroll: 0,
            context_help_rows: 1,
            context_help_area: Rect::default(),
            help_scrollbar: Default::default(),
            pointer_region: Default::default(),
            navigation_trail: Default::default(),
            original_saved_visits: Default::default(),
            original_diff_picker_v1: None,
            native_diff_v1: None,
            native_diff_layout_v1: Default::default(),
            native_mutable_diff_v2: None,
            native_mutable_plan_v2: None,
            native_mutable_diff_layout_v2: Default::default(),
            original_diff_work_v1: None,
            original_diff_layout_v1: Default::default(),
            original_diff_cancel_v1: false,
            original_tree_jump_v1: None,
            original_tree_work_v1: None,
            original_tree_layout_v1: Default::default(),
            original_tree_cancel_v1: false,
            original_tree_viewport_v2: Default::default(),
            graph_reread_dialog_v3: None,
            graph_reread_request_v3: None,
            navigation_restore: None,
            navigation_restore_inflight: false,
            navigation_restore_focus: None,
            capability_inspector: None,
            capability_hits: vec![],
            no_color: std::env::var_os("NO_COLOR").is_some(),
            refresh_requested: false,
            cancel_requested: false,
            scoped_pr_tab_cancel_requested: false,
            loading: [false; 5],
            settings_host: "github.com".into(),
            settings_set: String::new(),
            settings_theme: 0,
            density: crate::config::Density::Comfortable,
            settings_cursor: 0,
            settings_save_requested: false,
            bindings: crate::config::default_bindings(),
            user_navigated: false,
            workspace: None,
            local_commit_actors: Default::default(),
            local_actor_layout: Default::default(),
            local_actor_requested: false,
            local_actor_cancel_requested: false,
            related_pr_v1: None,
            related_pr_requested_v1: false,
            related_pr_cancel_v1: false,
            repository_source_identity: None,
            repository_source_overlay: None,
            repository_source_scroll: 0,
            repository_source_area: Rect::default(),
            repository_source_rows: 1,
            repository_source_return_focus: None,
            repository_mode: crate::workspace::RepositoryMode::Overview,
            repository_load_requested: false,
            repository_action: None,
            repository_directory_request_v41: None,
            repository_home_v118: None,
            core_worktree_v118: None,
            core_worktree_unset_v118: None,
            history_filter_input_v993: None,
            local_github_repository_v993: None,
            repository_content_focused: true,
            repository_visible_rows: 20,
            repository_panels_v93: Default::default(),
            takeover_v93: Default::default(),
            history_detail_v994: Default::default(),
            history_bar_choice_v99: None,
            history_place_v109: None,
            github: None,
            github_action: None,
            github_content_focused: true,
            content_pane: Default::default(),
            preview_area_v92: None,
            history_graph_width_v99: 0,
            preview_scroll_v92: 0,
            preview_item_v92: String::new(),
            github_visible_rows: 10,
            inbox_status_v1: Default::default(),
            pr_checks_origin: None,
            pr_checks_route: None,
            pr_checks_request: false,
            pr_check_choice_picker: None,
            pr_check_choice_request: None,
            pr_check_continuation_request_v2: None,
            pr_check_continuation_owner_v2: None,
            pr_check_watch_v2: None,
            pr_check_log_matches_v2: None,
            pr_check_log_editor_v2: None,
            pr_check_annotations_detail_v2: None,
            pr_check_attempt_advance_v2: None,
            pr_check_continuation_cancel_v2: false,
            pr_checks_cancel_actions: false,
            pr_timeline_return_v13: None,
            catalogue: None,
            remote_repository: None,
            remote_repository_cancel_requested: false,
            catalogue_cancel_requested: false,
            remote_scope: None,
            remote_scope_source: None,
            personal_readmission_v105: None,
            personal_repository_scope_v105: Default::default(),
            personal_pr_entry_work_v13: None,
            personal_pr_entry_owner_v13: None,
            personal_pr_entry_cancel_v13: false,
            personal_pr_source_v13: None,
            avatar_states: Default::default(),
            form: None,
            pr_action_index: 0,
            pr_menu_rows: 8,
            pr_action: None,
            pr_inspection: None,
            pr_pending_message: String::new(),
            pr_file_images: None,
            pr_line_comment_target_v19: None,
            pr_reactions: None,
            pr_reaction_display: Vec::new(),
            quote_reply_v115: None,
            pr_reaction_scroll: 0,
            pr_reaction_rows: 8,
            color_mode: crate::theme::ColorMode::TrueColor,
            local_command: None,
            local_command_preview: false,
            local_action_index: 0,
            form_dismissed: false,
            repository_reconcile_requested: false,
            search: None,
            search_controls: Default::default(),
            search_action: None,
            search_load_requested: false,
            repository_selection: None,
            actions: None,
            actions_inspection: None,
            actions_action: None,
            actions_load_requested: false,
            actions_content_focused: true,
            actions_visible_rows: 10,
            notice: String::new(),
            notice_scroll: 0,
            compare_requested: false,
            corpus_requested: false,
            source_controls: Default::default(),
            source_action: None,
            source_context: None,
            source_context_scroll: 0,
            source_context_visible: 1,
            metrics: None,
            metrics_action: None,
            metrics_load_requested: false,
            metrics_content_focused: true,
            search_content_focused: false,
            metrics_content: Default::default(),
            metrics_visible_rows: 1,
            metrics_selected_repository: None,
            metrics_days: 7,
            github_origin: None,
        }
    }
}

impl App {
    /// The list state each view keeps (Inbox and Repository own theirs; the remaining
    /// views share the plain content scroll).
    pub fn list_for(&self, view: View) -> &crate::ui_table::ListState {
        match view {
            View::Inbox => &self.inbox_list,
            View::Repository => &self.repository_list,
            _ => &self.content_list,
        }
    }
    pub fn list_for_mut(&mut self, view: View) -> &mut crate::ui_table::ListState {
        match view {
            View::Inbox => &mut self.inbox_list,
            View::Repository => &mut self.repository_list,
            _ => &mut self.content_list,
        }
    }
    /// The current view's list state.
    pub fn view_list(&self) -> &crate::ui_table::ListState {
        self.list_for(self.view)
    }
    pub fn view_list_mut(&mut self) -> &mut crate::ui_table::ListState {
        self.list_for_mut(self.view)
    }
    /// Every view's list back to its first row (a view switch or a cleared scope).
    pub fn reset_lists_v92(&mut self) {
        self.inbox_list = Default::default();
        self.repository_list = Default::default();
        self.content_list = Default::default();
    }
    /// Start with the library-owned empty lifecycle, including private draft
    /// protection. Callers may then configure the ordinary public UI fields.
    pub fn new() -> Self {
        Self::default()
    }

    /// Public inventory and already prepared diagnostics only. Nothing here
    /// probes a host, extracts credentials or authorizes any operation.
    pub fn palette_controls(&self) -> Vec<(String, Action)> {
        let mut entries = View::ALL
            .into_iter()
            .map(|view| (format!("Open {}", view.label()), Action::View(view)))
            .collect::<Vec<_>>();
        entries.push(("Inspect capabilities".into(), Action::Capabilities));
        entries
    }
    pub(crate) fn clear_capability_inspector(&mut self) {
        self.capability_inspector = None;
        self.capability_hits.clear();
        if self.overlay == Some(Overlay::Capabilities) {
            self.overlay = None;
            self.hits.clear();
        }
    }
    pub fn avatar_source(
        &self,
    ) -> Option<(
        crate::providers::gh::HostIdentity,
        String,
        Vec<crate::providers::gh::Actor>,
    )> {
        crate::actor_avatar_projection_v1::active(self)
    }
    pub fn pr_context(&self) -> Result<crate::pr_actions::PrActionContext, String> {
        let github = self
            .github
            .as_ref()
            .ok_or("No verified GitHub workspace is open")?;
        let selected = github
            .pr
            .as_ref()
            .and_then(|page| page.rows.get(self.inbox_list.selected))
            .and_then(|row| row.comment.as_deref().cloned());
        let mut context = crate::pr_actions::PrActionContext::from_workspace(github, selected)?;
        context.selected_file = github
            .pr
            .as_ref()
            .and_then(|page| page.selected_progress_file(self.inbox_list.selected));
        Ok(context)
    }
    /// Render-time controls borrow prepared rows only; no large workspace cloning or I/O.
    pub fn pr_controls(&self) -> Vec<(String, Action)> {
        self.pr_controls_at(self.inbox_list.selected)
    }
    /// The PR actions menu as it reads with row `selected` of the PR page selected; a
    /// Conversation card asks for its own comment's buttons this way (TASK-115).
    pub fn pr_controls_at(&self, selected: usize) -> Vec<(String, Action)> {
        use crate::pr_actions::PrCommand;
        let unavailable = self
            .github
            .as_ref()
            .and_then(|github| {
                if github.identity.account_id.is_none_or(|id| id == 0) {
                    Some("Verify the current numeric account before changing GitHub.".to_string())
                } else if github.pr.is_none() {
                    Some("Open an exact PR before choosing a PR action.".to_string())
                } else {
                    None
                }
            })
            .or_else(|| {
                self.github
                    .is_none()
                    .then(|| "Open a verified GitHub PR first.".into())
            });
        let mut controls = crate::forms::PrFormKind::ALL
            .iter()
            .map(|(kind, label)| {
                let review_reason = self.github.as_ref().and_then(|github| {
                    let page = github.pr.as_ref()?;
                    kind.review_disabled_reason(&page.details.summary, &github.identity.login)
                });
                let action = if let Some(reason) = &unavailable {
                    Action::Explain(reason.clone())
                } else if let Some(reason) = review_reason {
                    Action::Explain(reason)
                } else if self.pr_pending_message.is_empty() {
                    match kind {
                        crate::forms::PrFormKind::Title => Action::Pr(PrUiAction::MetadataTextV25(
                            crate::providers::gh::PrMetadataTextFieldV25::Title,
                        )),
                        crate::forms::PrFormKind::Description => {
                            Action::Pr(PrUiAction::MetadataTextV25(
                                crate::providers::gh::PrMetadataTextFieldV25::Description,
                            ))
                        }
                        crate::forms::PrFormKind::Base => Action::Pr(PrUiAction::MetadataBaseV32),
                        _ => Action::Github(crate::github_workspace::GithubAction::Compose(*kind)),
                    }
                } else {
                    Action::Explain(self.pr_pending_message.clone())
                };
                (label.to_string(), action)
            })
            .collect::<Vec<_>>();
        let page = self.github.as_ref().and_then(|github| github.pr.as_ref());
        // Merge sits with the review actions so it is visible without paging.
        let merge_reason = unavailable
            .clone()
            .or_else(|| {
                (!self.pr_pending_message.is_empty()).then(|| self.pr_pending_message.clone())
            })
            .or_else(|| {
                page.and_then(|page| {
                    crate::pr_actions::merge_disabled_reason(&page.details.summary)
                })
            });
        controls.insert(
            controls.len().min(4),
            (
                PrCommand::Merge.label().into(),
                merge_reason.map_or(
                    Action::Pr(PrUiAction::Command(PrCommand::Merge)),
                    Action::Explain,
                ),
            ),
        );
        // Pending reviews, dismissal and re-requests live in the Reviews
        // browser, which needs the loaded Files page; say how to get there.
        controls.insert(
            controls.len().min(4),
            (
                "Pending reviews, dismissal, re-request".into(),
                Action::Explain(
                    "Open Files (f), then F10 Reviews: resume, submit or discard a pending review, dismiss a review, or re-request a reviewer.".into(),
                ),
            ),
        );
        // Lifecycle actions work from any PR tab; the form re-reads the PR.
        let lifecycle_action = |command: PrCommand| {
            unavailable
                .clone()
                .or_else(|| {
                    (!self.pr_pending_message.is_empty()).then(|| self.pr_pending_message.clone())
                })
                .or_else(|| {
                    page.and_then(|page| {
                        crate::pr_actions::lifecycle_disabled_reason(command, &page.details.summary)
                    })
                })
                .map_or(Action::Pr(PrUiAction::Command(command)), Action::Explain)
        };
        let summary = page.map(|page| &page.details.summary);
        let lifecycle = [
            if summary.is_some_and(|s| s.pr.draft) {
                PrCommand::Ready
            } else {
                PrCommand::Draft
            },
            if summary.is_some_and(|s| s.auto_merge) {
                PrCommand::DisableAutoMerge
            } else {
                PrCommand::EnableAutoMerge
            },
            PrCommand::UpdateBranch,
        ];
        let at = controls.len().min(5);
        for (offset, command) in lifecycle.into_iter().enumerate() {
            controls.insert(
                at + offset,
                (command.label().into(), lifecycle_action(command)),
            );
        }
        let original_conversation = page.is_some_and(|p| {
            matches!(
                p.tab,
                crate::github_workspace::PrTab::Summary
                    | crate::github_workspace::PrTab::Conversation
            )
        }) && self.github.as_ref().is_some_and(|g| {
            g.scoped_pr_navigation.is_some() || g.personal_pr_page_ticket_v13.is_some()
        })
            && !crate::scoped_pr_collaboration_native_v17::retired_presentation(self);
        let original_metadata_text_v51 = original_conversation
            || page.is_some_and(|p| p.tab == crate::github_workspace::PrTab::Metadata)
                && self.github.as_ref().is_some_and(|g| {
                    g.scoped_pr_navigation.is_some() || g.personal_pr_page_ticket_v13.is_some()
                })
                && !crate::scoped_pr_collaboration_native_v17::retired_presentation(self);
        for command in [
            crate::pr_state_model::PrStateCommand::Close,
            crate::pr_state_model::PrStateCommand::Reopen,
        ] {
            let action = if unavailable.is_none() && original_conversation {
                // Cached state/permissions never authorize this effect. The
                // SAME new original Core checks exact capability/old state;
                // a known protected original routes to readonly History.
                Action::Pr(PrUiAction::StateV68(command))
            } else if unavailable.is_none() && page.is_some() {
                // Other tabs use the menu lifecycle path with fresh checks.
                lifecycle_action(if command == crate::pr_state_model::PrStateCommand::Close {
                    PrCommand::Close
                } else {
                    PrCommand::Reopen
                })
            } else {
                Action::Explain(unavailable.clone().unwrap_or_else(||
                    "Open the actual Summary or Conversation before a State action; no source fallback".into()))
            };
            controls.push((command.label().into(), action));
            controls.push((
                format!("Inspect {} history", command.label()),
                if unavailable.is_none() && original_conversation {
                    Action::Pr(PrUiAction::StateHistoryV74(command))
                } else {
                    Action::Explain(unavailable.clone().unwrap_or_else(|| {
                        "Original Summary or Conversation required to inspect State history".into()
                    }))
                },
            ));
        }
        for field in [
            crate::providers::gh::PrMetadataTextFieldV25::Title,
            crate::providers::gh::PrMetadataTextFieldV25::Description,
        ] {
            controls.push((format!("Inspect {} history", field.label()),
                if unavailable.is_none() && original_metadata_text_v51 { Action::Pr(PrUiAction::MetadataHistoryV25(field)) }
                else { Action::Explain(unavailable.clone().unwrap_or_else(|| "Open the actual original Summary, Conversation or Metadata to inspect protected metadata history".into())) }));
        }
        controls.push(("Inspect Base history".into(),if unavailable.is_none()&&original_metadata_text_v51 {Action::Pr(PrUiAction::MetadataBaseHistoryV32)}else{Action::Explain(unavailable.clone().unwrap_or_else(||"Open the actual original Summary, Conversation or Metadata to inspect protected Base history".into()))}));
        let original_members_v62 = original_conversation
            || page.is_some_and(|p| p.tab == crate::github_workspace::PrTab::Metadata)
                && self.github.as_ref().is_some_and(|g| {
                    g.scoped_pr_navigation.is_some() || g.personal_pr_page_ticket_v13.is_some()
                })
                && !crate::scoped_pr_collaboration_native_v17::retired_presentation(self);
        controls.push(("Inspect member metadata history".into(),if unavailable.is_none()&&original_members_v62 {Action::Pr(PrUiAction::MetadataMembersHistoryV48)}else{Action::Explain(unavailable.clone().unwrap_or_else(||"Open the actual original Summary, Conversation or Metadata to inspect protected member history".into()))}));
        for command in crate::pr_metadata_actions::PrMetadataCommand::ALL {
            controls.push((
                command.label().into(),
                if let Some(reason)=&unavailable {Action::Explain(reason.clone())}
                else if original_members_v62 {Action::Pr(PrUiAction::MetadataMembersV48(command))}
                else {Action::Explain("Open the actual original Summary, Conversation or Metadata before selecting a member action".into())},
            ));
        }
        // These entries start a closed original READ, never grant mutation
        // capability. The worker freshly probes the exact host contract and
        // selected pair before it can prepare one useful confirmation.
        let original_issue_links =
            original_members_v62 && self.view == View::Inbox && !self.loading[View::Inbox.index()];
        for command in [
            crate::pr_issue_links_v87::IssueLinkCommandV87::LinkIssue,
            crate::pr_issue_links_v87::IssueLinkCommandV87::UnlinkManualIssue,
        ] {
            for (label, action) in [
                (
                    command.label().to_string(),
                    PrUiAction::IssueLinksV87(command),
                ),
                (
                    format!("Inspect {} history", command.label()),
                    PrUiAction::IssueLinkHistoryV87(command),
                ),
            ] {
                let action = if let Some(reason) = &unavailable {
                    Action::Explain(reason.clone())
                } else if !self.pr_pending_message.is_empty() {
                    Action::Explain(self.pr_pending_message.clone())
                } else if original_issue_links {
                    Action::Pr(action)
                } else {
                    Action::Explain("Open the idle actual original Summary, Conversation or Metadata before manual Issue links; no source fallback".into())
                };
                controls.push((label, action));
            }
        }
        let comment = page
            .and_then(|page| page.rows.get(selected))
            .and_then(|row| row.comment.as_ref());
        for (command, label) in &PrCommand::ALL {
            if *command == PrCommand::Merge || PrCommand::LIFECYCLE.contains(command) {
                continue; // Listed beside the review actions or as State rows.
            }
            let reason = if let Some(reason) = &unavailable {
                Some(reason.clone())
            } else if !self.pr_pending_message.is_empty() {
                Some(self.pr_pending_message.clone())
            } else if PrCommand::GOVERNED.contains(command) {
                // The form reads current queue, lock and permission state.
                page.and_then(|page| {
                    crate::pr_actions::governed_disabled_reason(*command, &page.details.summary)
                })
            } else if matches!(command, PrCommand::MarkViewed | PrCommand::MarkUnviewed) {
                let file = page.and_then(|page| page.selected_progress_file(selected));
                match (self.github.as_ref(), page, file) {
                    (Some(github), Some(page), Some(file))
                        if page.review_progress.as_ref().is_some_and(|progress| {
                            progress.matches(&github.identity, &page.details.summary)
                        }) => crate::pr_progress::file_disabled_reason(
                            &file,
                            *command == PrCommand::MarkViewed,
                        ),
                    _ => Some("Open Review progress and select an exact current file before marking viewed/unviewed.".into()),
                }
            } else if matches!(
                command,
                PrCommand::EditComment
                    | PrCommand::DeleteComment
                    | PrCommand::AddReaction
                    | PrCommand::RemoveReaction
            ) && comment.is_none()
            {
                Some(
                    "Select an exact comment header or body first, using arrows or a mouse click."
                        .into(),
                )
            } else if original_conversation
                && comment.is_some_and(|comment| {
                    comment.kind != crate::providers::gh::CommentKind::Conversation
                })
                && !matches!(command, PrCommand::NewComment | PrCommand::Subscription)
            {
                Some("Selected review comment belongs to the original Threads workflow, not the Conversation issue-comment endpoint.".into())
            } else if matches!(command, PrCommand::EditComment | PrCommand::DeleteComment)
                && !original_conversation
                && comment.is_some_and(|comment| {
                    self.github.as_ref().is_none_or(|github| {
                        !comment
                            .observed
                            .author
                            .login
                            .eq_ignore_ascii_case(&github.identity.login)
                    }) && page
                        .and_then(|page| page.context.as_ref())
                        .is_none_or(|context| {
                            !(context.repository.permissions.push
                                || context.repository.permissions.maintain
                                || context.repository.permissions.admin)
                        })
                })
            {
                Some("This is another actor's comment. Load Metadata to verify collaborator permissions before editing or deleting.".into())
            } else {
                None
            };
            let action = reason.map_or(Action::Pr(PrUiAction::Command(*command)), Action::Explain);
            controls.push((
                if original_conversation && *command == PrCommand::NewComment {
                    "New comment • eligibility UNKNOWN/server-enforced".into()
                } else {
                    label.to_string()
                },
                action,
            ));
        }
        controls.push((
            "Inspect pending result".into(),
            Action::Pr(PrUiAction::Inspect),
        ));
        controls.push(("Acknowledge inspected result".into(),if self.pr_inspection.as_ref().is_some_and(|slot|slot.try_lock().is_ok_and(|inspection|inspection.is_some())) {Action::Pr(PrUiAction::Acknowledge)} else {Action::Explain("Inspect this exact PR's pending outcome first. Refresh is not an acknowledgement.".into())}));
        controls.push((
            "Browse selected comment reactions".into(),
            unavailable
                .or_else(|| {
                    comment
                        .is_none()
                        .then(|| "Select an exact conversation or review comment first.".into())
                })
                .map_or(Action::Pr(PrUiAction::BrowseReactions), Action::Explain),
        ));
        controls
    }
    pub fn dispatch(&mut self, action: Action) {
        if let Action::PrBranchEditV27(input) = &action {
            crate::pr_branch_edit_v27::dispatch(self, *input);
            return;
        }
        if let Action::Clipboard(input) = &action {
            crate::clipboard::dispatch(self, *input);
            return;
        }
        if let Action::NotificationsV63(input) = &action {
            crate::notifications_ui_v63::dispatch(self, *input);
            return;
        }
        if self.overlay == Some(Overlay::NotificationsV63) {
            if matches!(action, Action::Close | Action::Cancel | Action::Quit) {
                crate::notifications_native_v63::close(self);
                if action == Action::Quit {
                    self.quit = true;
                }
            }
            return;
        }
        if action
            == Action::Github(crate::github_workspace::GithubAction::Resource(
                crate::scoped_pr_observed_tabs_v1::resources::PrResourceUiActionV1::CommentLine,
            ))
        {
            crate::scoped_pr_observed_tabs_v1::resources::immutable_images::line_comment_v19::start(
                self,
            );
            return;
        }
        if let Action::Github(crate::github_workspace::GithubAction::Resource(_)) = action {
            self.pr_line_comment_target_v19 = None;
        }
        if crate::personal_pr_entry_v13::intercept_action(self, &action) {
            return;
        }
        if crate::notification_pr_v63::intercept_action(self, &action) {
            return;
        }
        if action == Action::Github(crate::github_workspace::GithubAction::FileCommentV75) {
            crate::scoped_pr_observed_tabs_v1::resources::threads_v1::native_v1::open_file_comment_v75(self);
            return;
        }
        if action == Action::Github(crate::github_workspace::GithubAction::ActionsHistoryV5) {
            crate::scoped_actions_native_v1::journal_v5::open_summary(self);
            return;
        }
        if action == Action::Github(crate::github_workspace::GithubAction::ReviewOperationHistoryV3)
        {
            if self.github.as_ref().is_some_and(|github| {
                self.remote_scope.as_ref().is_none_or(|scope| {
                    matches!(
                        scope.consumer(&github.identity),
                        crate::active_remote_scope_v1::RemoteScopeConsumerV1::DefaultPersonal
                    )
                })
            }) {
                self.github_action =
                    Some(crate::github_workspace::GithubAction::ReviewOperationHistoryV3);
                return;
            }
            crate::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::native_v2::open_history_v3(self);
            return;
        }
        if action == Action::Github(crate::github_workspace::GithubAction::ReviewThreadsV4) {
            crate::scoped_pr_observed_tabs_v1::resources::threads_v1::native_v1::open_detail_v4(
                self,
            );
            return;
        }
        if action == Action::Github(crate::github_workspace::GithubAction::ThreadHistoryV1) {
            if self
                .github
                .as_ref()
                .is_some_and(|github| github.scoped_pr_navigation.is_none())
            {
                // Main captures the genuine cached Summary + actual DB before
                // the SAME Inbox queue. This is not a legacy fallback action.
                self.github_action = Some(crate::github_workspace::GithubAction::ThreadHistoryV1);
                return;
            }
            crate::scoped_pr_observed_tabs_v1::resources::threads_v1::native_v1::open_history(self);
            return;
        }
        if action
            == Action::Github(crate::github_workspace::GithubAction::Resource(
                crate::scoped_pr_observed_tabs_v1::resources::PrResourceUiActionV1::Threads,
            ))
        {
            crate::scoped_pr_observed_tabs_v1::resources::threads_v1::native_v1::open(self);
            return;
        }
        if matches!(
            self.overlay,
            Some(Overlay::PrThreadsV1 | Overlay::PrThreadHistoryV1)
        ) {
            if matches!(action, Action::Close | Action::Cancel | Action::Quit) {
                crate::scoped_pr_observed_tabs_v1::resources::threads_v1::native_v1::close(self);
                if action == Action::Quit {
                    self.quit = true;
                }
            }
            return;
        }
        if let Action::StashRecoveryV1(input) = &action {
            crate::repository_stash_recovery_v1::dispatch(self, input.clone());
            return;
        }
        if action == Action::Form(crate::forms::FormAction::InspectStashV1) {
            crate::repository_stash_recovery_v1::dispatch(
                self,
                crate::repository_stash_recovery_v1::StashRecoveryActionV1::Inspect,
            );
            return;
        }
        if self.original_stash_recovery_v1.is_some() {
            match action {
                Action::Quit => {
                    crate::repository_stash_recovery_v1::dispatch(
                        self,
                        crate::repository_stash_recovery_v1::StashRecoveryActionV1::Back,
                    );
                    self.quit = true;
                }
                Action::Close | Action::Cancel => crate::repository_stash_recovery_v1::dispatch(
                    self,
                    crate::repository_stash_recovery_v1::StashRecoveryActionV1::Back,
                ),
                _ => {}
            }
            return;
        }
        if let Action::RelatedPrV1(input) = &action {
            crate::repository_related_pr_v1::dispatch(self, input.clone());
            return;
        }
        if self.related_pr_v1.is_some() {
            match action {
                Action::Quit => {
                    self.related_pr_v1 = None;
                    self.related_pr_requested_v1 = false;
                    self.related_pr_cancel_v1 = true;
                }
                Action::Close | Action::Cancel => {
                    crate::repository_related_pr_v1::dispatch(
                        self,
                        crate::repository_related_pr_v1::RelatedPrActionV1::Back,
                    );
                    return;
                }
                _ => return,
            }
        }
        if self.overlay == Some(Overlay::GraphRereadV3) {
            match &action {
                Action::GraphRereadV3(_) => {}
                Action::Close => {
                    crate::repository_graph_reread_native_v3::dispatch(
                        self,
                        crate::repository_graph_reread_native_v3::GraphRereadActionV3::Back,
                    );
                    return;
                }
                Action::Quit => crate::repository_graph_reread_native_v3::retire(self),
                _ => return, // No background click/key/control fall-through.
            }
        }
        if crate::navigation_state::handle_retention_action_v2(self, &action) {
            return;
        }
        if let Action::Scroll(delta) = &action {
            if let Some(graph) =
                crate::repository_graph_ui_v1::navigation_v2::boundary_scroll_v2(self, *delta)
            {
                self.dispatch(Action::RepositoryRow(crate::workspace::RowAction::Graph(
                    graph,
                )));
                return;
            }
        }
        crate::scoped_pr_observed_tabs_v1::resources::immutable_images::native::retire_invalid(
            self,
        );
        if let Action::PrFileImage(input) = &action {
            crate::scoped_pr_observed_tabs_v1::resources::immutable_images::native::action(
                self, *input,
            );
            return;
        }
        self.observe_navigation();
        if action == Action::Refresh
            && crate::scoped_pr_collaboration_refresh_v19::return_v19::forward_refresh(self)
        {
            return;
        }
        crate::pr_checks_native::before_action(self, &action);
        // Global/footer/palette/custom Refresh uses the SAME admitted typed
        // scoped action before any generic actor/context/navigation effects.
        // Legacy None keeps its existing later explanation/refusal.
        if action == Action::Refresh
            && crate::pr_checks_native::scoped::choices::continuations_v2::corrective_v3::forward_global_refresh(self)
        {
            return;
        }
        if let Action::RepositoryRow(crate::workspace::RowAction::Actor(role)) = &action {
            let same_profile =
                self.local_commit_actors
                    .profile()
                    .zip(self.workspace.as_ref())
                    .is_some_and(|(profile, workspace)| {
                        profile.role() == *role
                            && workspace.actor_profile == Some(*role)
                            && workspace
                                .selected_commit
                                .as_ref()
                                .is_some_and(|commit| commit.id == profile.sha())
                            && workspace.actor_origin.as_ref().is_some_and(|origin| {
                                std::sync::Arc::ptr_eq(origin, profile.origin())
                            })
                    });
            if same_profile {
                self.repository_content_focused = true;
                self.user_navigated = true;
                return; // Genuine same-role preparation; no Background replacement/cancel.
            }
        }
        let actor_was_pending = self.local_commit_actors.is_pending();
        if matches!(&action, Action::View(view) if *view != View::Repository)
            || matches!(
                &action,
                Action::RepositoryMode(_)
                    | Action::Navigation(_)
                    | Action::Refresh
                    | Action::Settings
                    | Action::SettingsSave
                    | Action::Quit
            )
            || matches!(&action, Action::RepositoryRow(row) if !matches!(row, crate::workspace::RowAction::Actor(_)))
        {
            self.local_actor_requested = false;
            self.local_actor_cancel_requested |= actor_was_pending;
            if actor_was_pending {
                self.loading[View::Repository.index()] = false;
            }
            self.local_commit_actors.retire_private();
            self.local_actor_layout.clear();
            self.avatar_states.clear();
        } else if matches!(
            &action,
            Action::RepositoryRow(crate::workspace::RowAction::Actor(_))
        ) {
            self.local_actor_requested = false;
            self.local_actor_cancel_requested |= actor_was_pending;
            if actor_was_pending {
                self.loading[View::Repository.index()] = false;
            }
            self.local_commit_actors.supersede_pending();
            self.local_actor_layout.clear();
        } else if action == Action::Cancel {
            self.local_actor_requested = false;
            self.local_actor_cancel_requested |= actor_was_pending;
            if actor_was_pending {
                self.loading[View::Repository.index()] = false;
            }
            self.local_commit_actors.clear_host_report();
            self.local_actor_layout.clear();
            self.avatar_states.clear();
        }
        if self.pr_checks_route.is_some() && self.view == View::Actions && action == Action::Refresh
        {
            self.dispatch(Action::Explain(
                crate::pr_checks_native::unavailable_action(
                    &crate::actions_workspace::ActionsAction::Refresh,
                )
                .unwrap(),
            ));
            return;
        }
        if matches!(
            &action,
            Action::View(_)
                | Action::RepositoryMode(_)
                | Action::RepositoryRow(_)
                | Action::Cancel
                | Action::Refresh
                | Action::Settings
        ) {
            self.abandon_navigation_restore();
        }
        if !matches!(
            &action,
            Action::PaletteInput(_) | Action::HelpScroll(_) | Action::Palette
        ) {
            self.retire_command_context();
        }
        if matches!(
            &action,
            Action::View(_)
                | Action::RepositoryMode(_)
                | Action::RepositoryRow(_)
                | Action::Cancel
                | Action::Refresh
                | Action::Navigation(_)
        ) {
            crate::repository_tree_native_v1::retire(self);
            crate::repository_diff_sides_native_v1::retire(self);
            crate::repository_native_diff_native_v1::retire(self);
        }
        match action {
            Action::NotificationsV63(_) => {}
            Action::StashRecoveryV1(_) => {}
            Action::OpenScopedReviewsV2=>crate::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::native_v2::open(self,false),
            Action::ScopedReviewsV2(action)=>crate::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::native_v2::browser_action(self,action),
            Action::NativeDiffV1(action) => {
                crate::repository_native_diff_native_v1::dispatch(self, action)
            }
            Action::NativeMutableDiffV2(action) => {
                crate::repository_native_diff_native_v1::dispatch_mutable(self, action)
            }
            Action::NativeMutablePlanV2(action) => {
                crate::repository_native_mutable_plan_native_v2::dispatch(self, action)
            }
            Action::RelatedPrV1(_) => (), // Handled before global dispatch.
            Action::OriginalTreeViewportV2(action) => {
                crate::repository_tree_viewport_native_v2::dispatch(self, action)
            }
            Action::PrChecks(action) => crate::pr_checks_native::dispatch(self, action),
            Action::PrCheckContinueV2(action) => {
                crate::pr_checks_native::scoped::choices::continuations_v2::ui::dispatch(
                    self, action,
                )
            }
            Action::PrCheckChoice(action) => {
                crate::pr_checks_native::scoped::choices::dispatch(self, action)
            }
            Action::Catalogue(action) => crate::repository_catalogue_native::dispatch(self, action),
            Action::LocalActor(action) => {
                use crate::repository_commit_actor_native_v1::LocalActorNativeIntentV1;
                if self.view != View::Repository {
                    return;
                }
                let actor_was_pending = self.local_commit_actors.is_pending();
                if self.loading[View::Repository.index()] && !actor_was_pending
                    && matches!(&action,
                        crate::repository_commit_actor_native_v1::LocalActorActionV1::ChooseRepository
                        | crate::repository_commit_actor_native_v1::LocalActorActionV1::Verify)
                { self.dispatch(Action::Explain("Wait for the genuine selected repository/profile read, or cancel it first".into())); return; }
                match self.local_commit_actors.apply(action) {
                    Ok(Some(LocalActorNativeIntentV1::RelatedPrs)) => {
                        crate::repository_related_pr_v1::dispatch(
                            self,
                            crate::repository_related_pr_v1::RelatedPrActionV1::Discover,
                        );
                    }
                    Ok(Some(LocalActorNativeIntentV1::ChooseRepository(purpose))) => {
                        self.local_actor_cancel_requested |= actor_was_pending;
                        if actor_was_pending {
                            self.loading[View::Repository.index()] = false;
                        }
                        self.avatar_states.clear();
                        crate::repository_catalogue_native::dispatch(
                            self,
                            crate::repository_catalogue_native::CatalogueUiAction::Open,
                        );
                        if let Some(modal) = &mut self.catalogue {
                            modal.local_actor_purpose = Some(purpose);
                            modal.status =
                                "Choose repository for saved commit • navigation intent only"
                                    .into();
                        }
                    }
                    Ok(Some(LocalActorNativeIntentV1::Verify)) => {
                        self.avatar_states.clear();
                        self.local_actor_layout.clear();
                        self.local_actor_requested = true;
                    }
                    Ok(None) => {
                        if actor_was_pending && !self.local_commit_actors.is_pending() {
                            self.local_actor_cancel_requested = true;
                            self.loading[View::Repository.index()] = false;
                        }
                    }
                    Err(reason) => {
                        self.dispatch(Action::Explain(reason));
                        return;
                    }
                }
                self.hits.clear();
                if let Some(status) = self.local_commit_actors.status {
                    self.status = status.into();
                }
            }
            Action::RepositorySource => crate::repository_source_ui::open(self),
            Action::Navigation(action) => self.request_navigation(action),
            Action::PaletteInput(input) => crate::navigation_ui::palette_input(self, input),
            Action::HelpScroll(delta) => {
                if self.overlay == Some(Overlay::Help) {
                    self.context_help_scroll = self
                        .context_help_scroll
                        .saturating_add_signed(delta as isize)
                        .min(4096);
                }
            }
            Action::Capabilities => {
                let identity = self
                    .github
                    .as_ref()
                    .map(|workspace| &workspace.identity)
                    .or_else(|| self.actions.as_ref().map(|workspace| &workspace.identity))
                    .or_else(|| {
                        self.search
                            .as_ref()
                            .and_then(|workspace| workspace.remote_identity.as_ref())
                    });
                let diagnostic = self
                    .github
                    .as_ref()
                    .and_then(|workspace| match &workspace.freshness {
                        crate::github_workspace::Freshness::Offline { .. }
                        | crate::github_workspace::Freshness::Cached { .. }
                        | crate::github_workspace::Freshness::Empty => {
                            Some(workspace.freshness.label())
                        }
                        crate::github_workspace::Freshness::Fresh { .. } => None,
                    })
                    .unwrap_or_else(|| {
                        format!(
                            "Last application diagnostic (not current permission evidence): {}",
                            self.status
                        )
                    });
                let prepared = crate::capability_inspector::CapabilityInspector::new(
                    identity,
                    Some(&diagnostic),
                );
                self.capability_inspector = None;
                self.hits.clear();
                self.capability_hits.clear();
                match prepared {
                    Ok(inspector) => {
                        self.capability_inspector = Some(inspector);
                        self.overlay = Some(Overlay::Capabilities);
                    }
                    Err(reason) => self.dispatch(Action::Explain(reason)),
                }
            }
            Action::Inspector(action) => {
                if self.overlay == Some(Overlay::Capabilities)
                    && self
                        .capability_inspector
                        .as_mut()
                        .is_some_and(|inspector| inspector.dispatch(action))
                {
                    self.clear_capability_inspector();
                }
            }
            Action::Metrics(action) => {
                self.metrics_content_focused = true;
                let result = self
                    .metrics
                    .as_mut()
                    .ok_or_else(|| {
                        "Load the explicit Dashboard scope before using its records".to_string()
                    })
                    .and_then(|metrics| metrics.apply(action));
                match result {
                    Ok(Some(request)) => {
                        if let Some(metrics) = &mut self.metrics {
                            metrics.pending = Some(request.clone());
                        }
                        self.metrics_action = Some(request);
                        self.status = "Preparing Dashboard • previous values hidden".into();
                    }
                    Ok(None) => {}
                    Err(reason) => self.dispatch(Action::Explain(reason)),
                }
            }
            Action::Corpus => {
                self.corpus_requested = true;
                self.search_controls.editing = None;
                self.search_controls.corpus = false;
                self.form_dismissed = false;
                self.overlay = None;
            }
            Action::PrFileImage(_) => unreachable!("Immutable image action dispatched above"),
            Action::PrBranchEditV27(_) => unreachable!("Branch edit action dispatched above"),
            Action::Clipboard(_) => unreachable!("Clipboard action dispatched above"),
            Action::View(view) => {
                if view != View::Inbox { self.closed_conversation_draft_v20 = None; }
                self.clear_capability_inspector();
                self.repository_selection = None;
                self.user_navigated = true;
                let previous = self.view;
                self.view = view;
                self.content_pane = crate::list_preview_v92::Pane::List;
                self.repository_content_focused = view == View::Repository;
                self.github_content_focused = view == View::Inbox;
                self.overlay = None;
                if previous == view {
                    self.reset_lists_v92();
                } else {
                    // Keep the place in the view being left; restore the destination's.
                    self.view_lists_v10[previous.index()] = *self.list_for(previous);
                    let restored = std::mem::take(&mut self.view_lists_v10[view.index()]);
                    self.reset_lists_v92();
                    *self.list_for_mut(view) = restored;
                }
                self.status = format!("{} • focused view", view.label());
                self.search_controls.editing = None;
                self.actions_content_focused = view == View::Actions;
                self.metrics_content_focused = view == View::Dashboard;
                self.search_content_focused = view == View::Search;
                if view == View::Dashboard {
                    if let Some(metrics) = &self.metrics {
                        self.context = format!(
                            "{} / @{} / Dashboard / {}",
                            metrics.snapshot.scope.host(),
                            metrics.snapshot.scope.login(),
                            metrics.scope_label
                        );
                    } else {
                        self.metrics_load_requested = true;
                    }
                }
                if view == View::Actions {
                    if let Some(actions) = &self.actions {
                        self.context = actions.context();
                    } else {
                        self.actions_load_requested = true;
                    }
                }
                if view == View::Search {
                    if let Some(search) = &self.search {
                        self.context = search.context();
                    } else {
                        self.search_load_requested = true;
                    }
                }
                if view == View::Inbox {
                    if let Some(github) = &self.github {
                        self.context = github.context();
                    }
                }
                if view == View::Repository {
                    if let Some(workspace) = &self.workspace {
                        self.context = format!(
                            "Local {} / {} / {}",
                            workspace.directory.display(),
                            workspace
                                .status
                                .branch
                                .as_deref()
                                .unwrap_or("detached HEAD"),
                            workspace.revision
                        );
                    } else if !self.loading[view.index()] {
                        self.repository_load_requested = true;
                    }
                }
            }
            Action::Help => crate::navigation_ui::open_help(self),
            Action::Compare => {
                if self
                    .workspace
                    .as_ref()
                    .is_some_and(|workspace| workspace.status.head.is_none())
                {
                    self.dispatch(Action::Explain(
                        "This repository has no saved commits yet. Compare needs two saved versions. You can still inspect Working and Staged changes, then create the first commit through Local operations.".into(),
                    ));
                } else if self.workspace.is_some() {
                    self.compare_requested = true;
                    self.form_dismissed = false;
                    self.overlay = None;
                } else {
                    self.dispatch(Action::Explain(
                        "Open a local Git repository before comparing saved versions.".into(),
                    ));
                }
            }
            Action::Explain(message) => {
                self.notice = safe_text(&message).chars().take(16000).collect();
                self.notice_scroll = 0;
                self.overlay = Some(Overlay::Notice);
            }
            Action::NoticeScroll(delta) => {
                if self.overlay == Some(Overlay::RepositorySource) {
                    self.repository_source_scroll = self
                        .repository_source_scroll
                        .saturating_add_signed(delta as isize)
                        .min(4096);
                    return;
                }
                self.notice_scroll = self
                    .notice_scroll
                    .saturating_add_signed(delta as isize)
                    .min(4096);
            }
            Action::PrReactionScroll(delta) => {
                self.pr_reaction_scroll = self
                    .pr_reaction_scroll
                    .saturating_add_signed(delta as isize);
            }
            Action::Palette => {
                crate::navigation_ui::open_palette(self);
            }
            Action::Close => {
                if self.overlay == Some(Overlay::PrCheckContinuationV2) {
                    crate::pr_checks_native::scoped::choices::continuations_v2::ui::dispatch(self,crate::pr_checks_native::scoped::choices::continuations_v2::ui::PrCheckContinueUiV2::Close);
                    return;
                }
                if self.overlay == Some(Overlay::RepositorySource) {
                    crate::repository_source_ui::close(self);
                }
                self.clear_capability_inspector();
                self.context_help = None;
                self.context_help_area = Rect::default();
                self.help_scrollbar.clear();
                if self.overlay == Some(Overlay::Form) {
                    if let Some(form) = &mut self.form {
                        form.observe_presentation_input_v4();
                    }
                    self.form_dismissed = true;
                }
                if self.overlay == Some(Overlay::PrReactions) {
                    self.pr_reactions = None;
                    self.pr_reaction_display.clear();
                    self.pr_reaction_scroll = 0;
                }
                self.overlay = None;
            }
            Action::Quit => {
                crate::pr_checks_native::scoped::choices::continuations_v2::retire(self);
                self.quit = true;
            }
            Action::Refresh => {
                if self.view == View::Actions
                    && self.pr_checks_route.as_ref().is_some_and(|r| r.scoped_v1())
                {
                    self.dispatch(Action::Actions(
                        crate::actions_workspace::ActionsAction::Refresh,
                    ));
                    return;
                }
                if self.view == View::Dashboard {
                    if self.metrics.is_some() {
                        self.dispatch(Action::Metrics(
                            crate::metrics_workspace::MetricsWorkspaceAction::Metric(
                                crate::metrics::MetricsAction::Refresh,
                            ),
                        ));
                        return;
                    }
                    if self.loading[View::Dashboard.index()] {
                        self.dispatch(Action::Explain(
                            "Dashboard is preparing; cancel the current read before refreshing"
                                .into(),
                        ));
                        return;
                    }
                }
                if self.view == View::Inbox {
                    self.pr_inspection = None;
                    self.pr_reactions = None;
                    self.pr_reaction_display.clear();
                }
                self.refresh_requested = true;
                self.status = "Refreshing…".into();
            }
            Action::Cancel => {
                if self.view == View::Inbox
                    && self.github.as_ref().is_some_and(|github| {
                        github.pr.is_none() && github.scoped_pr_navigation.is_some()
                    })
                {
                    self.dispatch(Action::Github(crate::github_workspace::GithubAction::Back));
                } else {
                    self.cancel_requested = true;
                }
            }
            Action::Settings => {
                self.overlay = Some(Overlay::Settings);
                self.focus = 0;
                self.settings_cursor = self.settings_host.chars().count();
            }
            Action::SettingsField(index) => {
                self.focus = index;
                self.settings_cursor = self
                    .settings_field()
                    .map_or(0, |value| value.chars().count());
            }
            Action::SettingsTheme => {
                self.settings_theme =
                    (self.settings_theme + 1) % crate::config::ThemeName::ALL.len()
            }
            Action::SettingsDensity => {
                self.density = self.density.next();
                self.status = format!("Density: {} • Save settings keeps it", self.density.label());
            }
            Action::SettingsSave => self.settings_save_requested = true,
            Action::GraphRereadV3(action) => {
                crate::repository_graph_reread_native_v3::dispatch(self, action)
            }
            Action::OriginalTreeV1(action) => {
                crate::repository_tree_native_v1::dispatch(self, action)
            }
            Action::OriginalDiffSidesV1(action) => {
                crate::repository_diff_sides_native_v1::dispatch(self, action)
            }
            Action::RepositoryRow(crate::workspace::RowAction::OriginalDiffReturn(ticket)) => {
                crate::repository_diff_sides_native_v1::back(self, ticket)
            }
            Action::RepositoryMode(mode) => {
                self.sync_history_cursor_revision_v99();
                self.repository_selection = None;
                self.repository_mode = mode;
                self.repository_load_requested = true;
                self.repository_list.selected = 0;
                self.repository_list.top = 0;
                self.user_navigated = true;
                self.repository_content_focused = true;
            }
            Action::FileHistoryV993(target) => crate::history::entry::open(self, target),
            Action::RepositoryRow(crate::workspace::RowAction::OpenWorktree(path)) => {
                // Read-only: a fresh Overview load of the chosen worktree; nothing is checked out.
                crate::repository_worktrees_v118::open(self, path);
            }
            Action::RepositoryRow(crate::workspace::RowAction::OverviewV119(link)) => {
                use crate::{
                    repository_panels_v93::PanelActionV93, workspace::OverviewLinkV119,
                };
                match link {
                    OverviewLinkV119::Mode(mode) => self.dispatch(Action::RepositoryMode(mode)),
                    OverviewLinkV119::ToggleWorktrees => {
                        self.dispatch(Action::RepositoryPanelsV93(PanelActionV93::ToggleWorktrees))
                    }
                    OverviewLinkV119::ToggleRecovery => {
                        self.dispatch(Action::RepositoryPanelsV93(PanelActionV93::ToggleRecovery))
                    }
                }
            }
            Action::RepositoryRow(action) => {
                if let crate::workspace::RowAction::Tree(tree) = action {
                    crate::repository_tree_native_v1::row(self, tree);
                    return;
                }
                if let crate::workspace::RowAction::Graph(graph) = &action {
                    // A History detail action is a new row-level intent, like choosing a row:
                    // it replaces a running read instead of waiting for it (TASK-99.4).
                    let row_level = matches!(
                        graph,
                        crate::repository_graph_v1::GraphActionV1::Detail { .. }
                            | crate::repository_graph_v1::GraphActionV1::OpenPath { .. }
                    );
                    let reason = if self.loading[View::Repository.index()] && !row_level {
                        Some("Wait for the original graph read, or cancel it first".into())
                    } else {
                        self.workspace.as_ref().map_or_else(
                            || Some("Open an original local graph first".into()),
                            |workspace| workspace.graph_action_reason_v1(graph),
                        )
                    };
                    if let Some(reason) = reason {
                        self.dispatch(Action::Explain(reason));
                        return;
                    }
                }
                if let crate::workspace::RowAction::FileProvenance(file_action) = &action {
                    let reason = if self.loading[View::Repository.index()] {
                        Some("Wait for the current repository read, or cancel it first".into())
                    } else {
                        self.workspace.as_ref().map_or_else(
                            || Some("Open a saved file in a local Git repository first".into()),
                            |workspace| workspace.provenance_available(file_action).err(),
                        )
                    };
                    if let Some(reason) = reason {
                        self.dispatch(Action::Explain(reason));
                        return;
                    }
                }
                if let Some(workspace) = &self.workspace {
                    let index = if workspace.graph_visible_v1() {
                        workspace.graph_action_index_v1(&action)
                    } else {
                        workspace
                            .display_rows_v1()
                            .iter()
                            .position(|row| row.action.as_ref() == Some(&action))
                    };
                    if let Some(index) = index {
                        self.repository_list.selected = index;
                    }
                }
                if let crate::workspace::RowAction::DiffLine(anchor) = action {
                    self.repository_selection = Some(DiffSelection {
                        start: anchor.clone(),
                        end: anchor,
                    });
                    self.repository_content_focused = true;
                    self.user_navigated = true;
                    self.status =
                        "Selected change line • + Stage / - Unstage this line • drag or Shift↑↓ for a range • h selects hunk"
                            .into();
                    return;
                }
                self.repository_selection = None;
                // Choosing a History row or a parent link keeps the table's window; the bar
                // moves within it (TASK-99.4).
                let keep_viewport = matches!(
                    &action,
                    crate::workspace::RowAction::Graph(
                        crate::repository_graph_v1::GraphActionV1::More
                            | crate::repository_graph_v1::GraphActionV1::Horizontal(_)
                            | crate::repository_graph_v1::GraphActionV1::Filter(_)
                            | crate::repository_graph_v1::GraphActionV1::Detail {
                                act: crate::repository_graph_v1::HistoryDetailActV994::Parent(_),
                                ..
                            }
                    )
                ) || (matches!(&action, crate::workspace::RowAction::Commit(_))
                    && self
                        .workspace
                        .as_ref()
                        .is_some_and(crate::workspace::Workspace::graph_visible_v1));
                if !matches!(
                    &action,
                    crate::workspace::RowAction::Graph(
                        crate::repository_graph_v1::GraphActionV1::Filter(_)
                    )
                ) {
                    // A row, a click or another graph action takes the focus from the query.
                    self.history_filter_input_v993 = None;
                }
                // The next History page keeps the reader's place (TASK-109): its window opens
                // on the commit under the bar, and the bar and viewport stay on their commits.
                self.history_place_v109 = if matches!(
                    &action,
                    crate::workspace::RowAction::Graph(
                        crate::repository_graph_v1::GraphActionV1::More
                    )
                ) {
                    self.sync_history_cursor_revision_v99();
                    crate::history::place::capture(self)
                } else {
                    None
                };
                self.repository_action = Some(action);
                if !keep_viewport {
                    self.repository_list.top = 0;
                }
                self.user_navigated = true;
                self.repository_content_focused = true;
            }
            Action::Scroll(delta) => {
                // Repository panels scroll the panel that holds the selection.
                if self.overlay.is_none()
                    && crate::repository_panels_v93::scroll_selected(self, delta as isize)
                {
                    return;
                }
                // No observed page in this body while its sole page prepares.
                // Never clamp its prior viewport against the cached Inbox.
                if self.view == View::Inbox
                    && self.overlay.is_none()
                    && self.github.as_ref().is_some_and(|github| {
                        github.pr.is_none() && github.scoped_pr_navigation.is_some()
                    })
                {
                    return;
                }
                let max = if let (View::Inbox, Some(github)) = (self.view, self.github.as_ref()) {
                    github
                        .pr
                        .as_ref()
                        .map_or_else(|| github.visible().len(), |page| page.rows.len())
                } else if self.view == View::Repository {
                    self.workspace
                        .as_ref()
                        .map_or(self.content[self.view.index()].len(), |workspace| {
                            workspace.display_rows_v1().len()
                        })
                } else {
                    self.content[self.view.index()].len()
                }
                .saturating_sub(1);
                let top = self.view_list().top.saturating_add_signed(delta as isize).min(max);
                self.view_list_mut().top = top;
            }
            Action::Select(index) => self.view_list_mut().selected = index,
            Action::PrTimelineGoV13 => crate::pr_timeline_links_v13::go(self),
            Action::GithubRow(index) => {
                if let Some(page) = self.github.as_ref().and_then(|github| github.pr.as_ref()) {
                    self.inbox_list.selected = index.min(page.rows.len().saturating_sub(1));
                    self.github_content_focused = true;
                    let visible = self.github_visible_rows.max(1);
                    if self.inbox_list.selected < self.inbox_list.top {
                        self.inbox_list.top = self.inbox_list.selected;
                    }
                    if self.inbox_list.selected >= self.inbox_list.top + visible {
                        self.inbox_list.top = self.inbox_list.selected + 1 - visible;
                    }
                    if let Some(comment) = page
                        .rows
                        .get(self.inbox_list.selected)
                        .and_then(|row| row.comment.as_ref())
                    {
                        self.status = format!(
                            "Selected comment #{} by {} • p PR actions",
                            comment.observed.id,
                            comment.observed.author.label()
                        );
                    } else if let Some(link) = page.timeline_link_at(self.inbox_list.selected) {
                        self.status = crate::pr_timeline_links_v13::selected_status(page, link);
                    } else if let Some(file) = page.progress_file_at(self.inbox_list.selected) {
                        self.status = format!(
                            "Selected file {} • {} • p PR actions",
                            safe_text(&file.path),
                            file.state.label()
                        );
                    }
                }
            }
            Action::Pr(action) => {
                self.pr_reactions = None;
                self.pr_reaction_display.clear();
                self.pr_reaction_scroll = 0;
                self.pr_action = Some(action);
                self.overlay = None;
                self.form_dismissed = false;
            }
            Action::Github(action) => {
                if crate::scoped_pr_collaboration_native_v17::retired_presentation(self)
                    && !matches!(action, crate::github_workspace::GithubAction::Back) {
                    self.status = "Original Conversation hidden • use Back/Refresh; no new action from retired page".into();
                    return;
                }
                if action == crate::github_workspace::GithubAction::Back && crate::notification_pr_v63::back(self) {
                    return;
                }
                if action == GithubAction::Resource(crate::scoped_pr_observed_tabs_v1::resources::PrResourceUiActionV1::NativeDiff) && self.remote_scope.is_some() {
                    crate::scoped_pr_native_diff_native_v1::open_files(self);
                    return;
                }
                use crate::github_workspace::GithubAction;
                if action == GithubAction::Back
                    && self
                        .github
                        .as_ref()
                        .is_some_and(|github| github.scoped_pr_navigation.is_some())
                {
                    self.notice.clear();
                    self.notice_scroll = 0;
                    if self.overlay == Some(Overlay::Notice) {
                        self.overlay = None;
                    }
                    self.scoped_pr_tab_cancel_requested = true;
                    self.loading[View::Inbox.index()] = false;
                    self.github_action = None;
                    self.status =
                        "Previous cached Inbox • no new reads • observed section closed".into();
                }
                if action == GithubAction::Actions
                    && self
                        .github
                        .as_ref()
                        .is_some_and(|github| github.scoped_pr_navigation.is_some()
                            && github.pr.as_ref().is_none_or(|p| !(matches!(p.tab, crate::github_workspace::PrTab::Summary | crate::github_workspace::PrTab::Conversation)
                                ||p.tab==crate::github_workspace::PrTab::Metadata&&p.metadata_display_v51.is_some())))
                {
                    self.status =
                        "Observed sections only • PR actions require their own fresh scoped read"
                            .into();
                    return;
                }
                if action == GithubAction::Back && self.github_origin == Some(View::Dashboard) {
                    self.github_origin = None;
                    self.dispatch(Action::View(View::Dashboard));
                    return;
                }
                if action == GithubAction::ActorProfile {
                    crate::pr_actor_profile_v1::open(self);
                    return;
                }
                if action == GithubAction::Actions {
                    self.overlay = Some(Overlay::PrActions);
                    self.pr_action_index = 0;
                    return;
                }
                if matches!(action, GithubAction::Open(_) | GithubAction::Back) {
                    self.pr_timeline_return_v13 = None;
                    self.pr_file_images = None;
                    self.pr_inspection = None;
                    self.pr_pending_message.clear();
                    self.pr_reactions = None;
                    self.pr_reaction_display.clear();
                }
                if let Some(github) = &mut self.github {
                    let preserve_scoped_view =
                        matches!(&action, GithubAction::PrTab(_) | GithubAction::Resource(_))
                            && github.scoped_pr_navigation.is_some()
                            ||matches!(&action,GithubAction::PrTab(crate::github_workspace::PrTab::Metadata))
                                &&github.pr.as_ref().is_some_and(|p|matches!(p.tab,crate::github_workspace::PrTab::Summary|crate::github_workspace::PrTab::Conversation));
                    match &action {
                        GithubAction::Filter(filter) => {
                            github.set_filter(*filter);
                            // The Merged tab reads on demand, the first time it is chosen.
                            if *filter == crate::github_workspace::InboxFilter::Merged
                                && github.merged_v1.is_none()
                            {
                                self.github_action = Some(action.clone());
                            }
                        }
                        GithubAction::Back => {
                            // State changed queue membership/counts. Closing
                            // this actual direct PR retires ONLY its stale
                            // page tickets, never a Source/owner/Return or the
                            // negative freshness flag. Existing early return
                            // routes and ordinary Back keep their semantics.
                            if github.pr.is_some() && github.state_membership_unavailable_v68() {
                                github.personal_pr_page_ticket_v13 = None;
                                github.notification_pr_page_ticket_v63 = None;
                                self.status="State queue membership unavailable • Refresh explicitly • no reads or resend".into();
                            }
                            github.pr = None;
                            github.scoped_pr_navigation = None;
                            github.scoped_pr_resource = None;
                        }
                        GithubAction::Actions => self.overlay = Some(Overlay::PrActions),
                        GithubAction::Compose(_) => {
                            self.form_dismissed = false;
                            self.overlay = None;
                            self.github_action = Some(action);
                        }
                        _ => self.github_action = Some(action),
                    }
                    if !preserve_scoped_view {
                        self.inbox_list.selected = github.selected_index();
                        self.inbox_list.top = 0;
                        self.github_content_focused = true;
                    }
                    self.context = github.context();
                }
            }
            Action::Form(action) => {
                if action == crate::forms::FormAction::Close
                    && crate::scoped_pr_collaboration_native_v17::intercept_close(self) {
                    return;
                }
                if action == crate::forms::FormAction::Close && self.form.as_ref().is_some_and(|form|matches!(form.purpose,crate::forms::FormPurpose::ScopedActionsV3(_) | crate::forms::FormPurpose::ScopedActionsWorkflowV6(_) | crate::forms::FormPurpose::ScopedActionsDispatchV7(_) | crate::forms::FormPurpose::ScopedWorkflowEditV9(_)) && form.submitting) {
                    crate::scoped_actions_native_v1::cancel_publication_v3(self);
                    return;
                }
                if action == crate::forms::FormAction::Close && self.form.as_ref().is_some_and(|form|matches!(form.purpose,crate::forms::FormPurpose::PrReviewV2(_)|crate::forms::FormPurpose::PrReviewOperationV3(_)) && form.submitting) {
                    self.scoped_reviews_cancel_v2=true;
                    self.status="Canceling original review operation • awaiting owned cleanup; effects may exist".into();return;
                }
                if action == crate::forms::FormAction::Close
                    && self.form.as_ref().is_some_and(|form| {
                        matches!(form.purpose, crate::forms::FormPurpose::PrThread(_) | crate::forms::FormPurpose::PrThreadCommentV2(_) | crate::forms::FormPurpose::PrThreadReactionV3(_) | crate::forms::FormPurpose::PrFileCommentV75(_))
                            && form.submitting
                    })
                {
                    self.scoped_threads_cancel_v1 = true;
                    self.status = "Canceling original thread operation • awaiting owned cleanup; effects may exist".into();
                    return;
                }
                if action == crate::forms::FormAction::Close
                    && self.form.as_ref().is_some_and(|form| {
                        matches!(form.purpose, crate::forms::FormPurpose::PrInline(_))
                            && form.submitting
                    })
                {
                    self.scoped_inline_cancel_v1 = true;
                    self.status="Canceling original inline operation • awaiting owned cleanup; effects may exist".into();
                    return;
                }
                if action == crate::forms::FormAction::Close
                    && self.form.as_ref().is_some_and(|form| form.submitting)
                {
                    self.cancel_requested = true;
                    self.status =
                        "Cancellation requested • waiting for cleanup; an effect may already exist"
                            .into();
                    return;
                }
                let retain_closed_conversation_v20 = action == crate::forms::FormAction::Close
                    && crate::scoped_pr_collaboration_ui_v17::pure_close_v20::can_close(self);
                let closed_history_v296 = if action == crate::forms::FormAction::Close {
                    crate::scoped_pr_collaboration_ui_v17::history_close_v296::capture(self)
                } else {
                    None
                };
                if let Some(form) = &mut self.form {
                    form.dispatch(&action);
                }
                if action == crate::forms::FormAction::Close {
                    if let Some(form) = &mut self.form {
                        for field in &mut form.fields {
                            if field.secret {
                                field.editor.clear();
                            }
                        }
                        form.invalidate_plan();
                        form.base_selection = None;
                        form.metadata_picker = None;
                    }
                    self.form_dismissed = true;
                    self.overlay = None;
                    self.status = "Composer closed • draft kept locally".into();
                    crate::scoped_pr_observed_tabs_v1::resources::immutable_images::inline_v1::native_entry_v1::form_closed(self);
                    if retain_closed_conversation_v20 {
                        crate::scoped_pr_collaboration_ui_v17::pure_close_v20::form_closed(self);
                    }
                    if let Some(proof) = closed_history_v296 {
                        crate::scoped_pr_collaboration_ui_v17::history_close_v296::form_closed(self, proof);
                    }
                }
            }
            Action::LocalMenu => {
                if self.workspace.is_some() {
                    self.overlay = Some(Overlay::LocalActions);
                    self.local_action_index = 0;
                }
            }
            Action::LocalCommand(command) => {
                if let Some(workspace) = &self.workspace {
                    if let Some(reason) =
                        crate::local_availability::LocalAvailability::from_workspace(
                            workspace,
                            self.repository_list.selected,
                            self.repository_selection
                                .as_ref()
                                .map(|selection| &selection.start),
                        )
                        .disabled_reason(command)
                    {
                        self.status = reason;
                    } else {
                        self.local_command = Some(command);
                        self.overlay = None;
                        self.form_dismissed = false;
                    }
                }
            }
            Action::MenuScroll(delta) => {
                if self.overlay == Some(Overlay::LocalActions) {
                    self.local_action_index = self
                        .local_action_index
                        .saturating_add_signed(delta as isize)
                        .min(crate::local_actions::LocalCommand::ALL.len() - 1);
                }
                if self.overlay == Some(Overlay::PrActions) {
                    self.pr_action_index = self
                        .pr_action_index
                        .saturating_add_signed(delta as isize)
                        .min(self.pr_controls().len().saturating_sub(1));
                }
            }
            Action::Search(action) => crate::search_ui::dispatch(self, action),
            Action::SearchBlend(action) => crate::search_blend::dispatch(self, action),
            Action::SearchRankDetails => {
                let details=self.search.as_ref().and_then(|search|search.blend_evidence.as_ref())
                    .map(|evidence|format!("Independent rank lists • RRF k=60 uses source positions, not mixed raw engine scores. Provider snapshot completeness is not full GitHub history or a relevance-quality promise.\n\n{}",evidence.rows.join("\n\n")));
                if let Some(details) = details {
                    self.dispatch(Action::Explain(details));
                    self.overlay = Some(Overlay::SearchRankDetails);
                } else {
                    self.dispatch(Action::Explain("Run an explicitly enabled Ranked blend query first. No remote query is sent by opening this explanation.".into()));
                }
            }
            Action::SearchSource(action) => crate::search_sources::ui::dispatch(self, action),
            Action::RepositoryHunk => self.select_repository_hunk(),
            Action::HistoryFilterV993(action) => {
                crate::repository_ui::history_filter::dispatch(self, action)
            }
            Action::LocalQuick(quick) => self.local_quick(quick),
            Action::PrCardsV115(action) => crate::pr_conversation_cards_v115::dispatch(self, action),
            Action::RepositoryPanelsV93(action) => {
                crate::repository_panels_v93::dispatch(self, action)
            }
            Action::WorktreeV118(action) => crate::repository_worktrees_v118::dispatch(self, action),
            Action::HistoryDetailV994(action) => crate::history::detail::dispatch(self, action),
            Action::Actions(action) => {
                if action == crate::actions_workspace::ActionsAction::ActorProfile {
                    // Read-only (TASK-34): no worker, gh read, SQL or Git effect.
                    crate::pr_actor_profile_v1::open_run(self);
                    return;
                }
                if crate::scoped_actions_native_v1::intercept(self,&action){return;}
                if crate::pr_checks_native::scoped::choices::continuations_v2::intercept(
                    self, &action,
                ) {
                    return;
                }
                if crate::pr_checks_native::scoped::choices::pending(self)
                    && !matches!(
                        action,
                        crate::actions_workspace::ActionsAction::Back
                            | crate::actions_workspace::ActionsAction::Select(_)
                            | crate::actions_workspace::ActionsAction::Unavailable(_)
                    )
                {
                    self.dispatch(Action::Explain("Original scoped run/member read preparing; Back remains available; nothing else queried".into()));
                    return;
                }
                if self.pr_checks_route.is_some() {
                    if let Some(reason) = self.pr_checks_route.as_ref().and_then(|route| {
                        crate::pr_checks_native::scoped::scoped_unavailable_v1(route, &action)
                    }) {
                        self.dispatch(Action::Explain(reason));
                        return;
                    }
                }
                if self.pr_checks_route.is_some()
                    && action == crate::actions_workspace::ActionsAction::Back
                {
                    self.dispatch(Action::PrChecks(
                        crate::pr_checks_native::PrChecksUiAction::Back,
                    ));
                    return;
                }
                if let crate::actions_workspace::ActionsAction::Unavailable(message) = action {
                    self.dispatch(Action::Explain(message));
                } else if matches!(action, crate::actions_workspace::ActionsAction::Watch) {
                    if let Some(Err(reason)) = self.actions.as_mut().map(|a| a.toggle_watch()) {
                        self.dispatch(Action::Explain(reason));
                    }
                } else if matches!(action, crate::actions_workspace::ActionsAction::Follow) {
                    // TASK-117: local only; the next live update keeps the newest line in view.
                    if let Some(actions) = &mut self.actions {
                        actions.toggle_follow_v117();
                    }
                } else if let crate::actions_workspace::ActionsAction::Select(index) = action {
                    if let Some(actions) = &mut self.actions {
                        actions.select(index);
                        if let Some(route) = &self.pr_checks_route {
                            crate::pr_checks_native::scoped::preserve_controls_v1(route, actions);
                        }
                        if actions.selected < actions.top {
                            actions.top = actions.selected;
                        } else if actions.selected >= actions.top + self.actions_visible_rows {
                            actions.top = actions.selected + 1 - self.actions_visible_rows.max(1);
                        }
                    }
                } else {
                    if matches!(action, crate::actions_workspace::ActionsAction::Compose(_)) {
                        self.form_dismissed = false;
                    }
                    self.actions_action = Some(action);
                }
                self.actions_content_focused = true;
            }
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        if matches!(&event, Event::Resize(_, _)) {
            self.pointer_region.clear();
        }
        let old_notification_pr = self.notification_pr_source_v63.is_some();
        crate::notification_pr_v63::retire_invalid(self);
        if old_notification_pr && self.notification_pr_source_v63.is_none() && self.github.is_none()
        {
            return;
        }
        // TASK-27: the open branch edit owns every key and click.
        if crate::pr_branch_edit_v27::event(self, &event) {
            return;
        }
        if crate::notifications_ui_v63::event(self, &event) {
            return;
        }
        // Only this already-owned modal captures here. Reject oversized Paste
        // and invalidate Resize geometry BEFORE any unrelated handler clones
        // the event; no source/work/provider lifetime is created or renewed.
        if self.overlay == Some(Overlay::ScopedActionsReadV2)
            && crate::scoped_actions_native_v1::read_ui_v2::event(self, &event)
        {
            return;
        }
        if crate::scoped_pr_observed_tabs_v1::resources::threads_v1::history_v1::ui::event(
            self, &event,
        ) {
            return;
        }
        if crate::scoped_pr_observed_tabs_v1::resources::threads_v1::ui_v1::event(self, &event) {
            return;
        }
        crate::scoped_pr_observed_tabs_v1::resources::immutable_images::inline_v1::ui_v1::retire_invalid_form(self);
        if crate::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::ui_v2::retire_invalid_form(self) {return;}
        if crate::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::operations_v3::ui_v3::retire_invalid_form(self){return;}
        if crate::repository_stash_recovery_v1::event(self, event.clone()) {
            return;
        }
        if let Event::Resize(width, _) = &event {
            crate::scoped_actions_native_v1::resize(self);
            if let Some(form) = &mut self.form {
                form.clear_preview_display_geometry_v1();
                crate::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::ui_v2::clear_layout(form);
                crate::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::operations_v3::ui_v3::clear_layout(form);
            }
            crate::repository_tree_viewport_native_v2::resize(self, *width);
        }
        // Native capture borrows the event before other handlers clone it,
        // including rejected oversized bracketed-paste payloads.
        if crate::scoped_actions_native_v1::read_ui_v2::event(self, &event) {
            return;
        }
        if crate::scoped_pr_observed_tabs_v1::resources::immutable_images::inline_v1::native_entry_v1::event(self,&event){return;}
        // The shell around an open Repository viewer keeps its header and key bar clicks.
        if crate::repository_panels_v93::takeover::event(self, &event) {
            return;
        }
        if crate::repository_native_diff_native_v1::capture(self, &event) {
            return;
        }
        if crate::repository_related_pr_v1::event(self, event.clone()) {
            return;
        }
        if crate::repository_tree_viewport_native_v2::folder_event(self, event.clone()) {
            return;
        }
        if crate::repository_graph_reread_native_v3::event(self, event.clone()) {
            return;
        }
        if crate::pr_checks_native::scoped::choices::continuations_v2::ui::event(
            self,
            event.clone(),
        ) {
            return;
        }
        if crate::pr_checks_native::scoped::choices::ui::event(self, event.clone()) {
            return;
        }
        if self.remote_repository.is_some() {
            crate::remote_repository_native_v1::event(self, event);
            return;
        }
        crate::scoped_pr_observed_tabs_v1::resources::immutable_images::native::retire_invalid(
            self,
        );
        if self.overlay == Some(Overlay::PrFileImages) {
            crate::scoped_pr_observed_tabs_v1::resources::immutable_images::native::event(
                self, event,
            );
            return;
        }
        if self.overlay == Some(Overlay::ScopedReviewsV2) {
            crate::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::native_v2::event(self,event);
            return;
        }
        if self.overlay.is_none() && crate::repository_tree_native_v1::capture(self, event.clone())
        {
            return;
        }
        if self.overlay.is_none()
            && crate::repository_diff_sides_native_v1::capture(self, event.clone())
        {
            return;
        }
        if matches!(&event, Event::Resize(_, _)) {
            self.hits.clear();
            self.local_actor_layout.clear();
        }
        if self.view == View::Repository
            && self.overlay.is_none()
            && self.local_commit_actors.details_open
        {
            let action = match &event {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        self.dispatch(Action::Quit);
                        return;
                    }
                    if key.code == KeyCode::Enter {
                        self.local_actor_layout
                            .focused_action(self.local_commit_actors.detail_focus)
                    } else {
                        crate::repository_commit_actor_ui_v1::detail_key(key.code)
                    }
                }
                Event::Mouse(mouse) => {
                    self.local_actor_layout
                        .mouse(mouse.kind, mouse.column, mouse.row)
                }
                _ => None,
            };
            if let Some(action) = action {
                self.dispatch(Action::LocalActor(action));
            }
            return;
        }
        if self.overlay == Some(Overlay::RepositoryCatalogue) {
            crate::repository_catalogue_native::event(self, event);
            return;
        }
        if self.overlay == Some(Overlay::RepositorySource) {
            crate::repository_source_ui::event(self, event);
            return;
        }
        if matches!(self.overlay, Some(Overlay::Palette | Overlay::Help)) {
            crate::navigation_ui::event(self, event);
            return;
        }
        if self.overlay.is_none()
            && self.navigation_restore_inflight
            && matches!(&event, Event::Key(key) if key.kind != KeyEventKind::Release && key.code == KeyCode::Esc)
        {
            self.dispatch(Action::Cancel);
            return;
        }
        if self.overlay == Some(Overlay::Capabilities) {
            let action = match &event {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        self.dispatch(Action::Quit);
                        return;
                    }
                    self.capability_inspector
                        .as_ref()
                        .and_then(|inspector| inspector.key(key.code, key.modifiers))
                }
                Event::Paste(text) => Some(crate::capability_inspector::InspectorAction::Insert(
                    text.clone(),
                )),
                Event::Mouse(mouse) => self.capability_inspector.as_ref().and_then(|inspector| {
                    inspector.mouse(mouse.kind, mouse.column, mouse.row, &self.capability_hits)
                }),
                Event::Resize(_, _) => {
                    self.hits.clear();
                    self.capability_hits.clear();
                    if let Some(inspector) = &mut self.capability_inspector {
                        inspector.clear_layout();
                    }
                    None
                }
                _ => None,
            };
            if let Some(action) = action {
                self.dispatch(Action::Inspector(action));
            }
            return;
        }
        if crate::search_blend::event(self, &event) {
            return;
        }
        if matches!(
            self.overlay,
            Some(Overlay::SearchSources | Overlay::SearchContext)
        ) {
            crate::search_sources::ui::handle_event(self, &event);
            return;
        }
        if self.view == View::Repository && self.overlay.is_none() {
            if let Event::Mouse(mouse) = &event {
                if mouse.kind == MouseEventKind::Drag(MouseButton::Left) {
                    let anchor = self
                        .hits
                        .iter()
                        .rev()
                        .find(|hit| hit.rect.contains((mouse.column, mouse.row).into()))
                        .and_then(|hit| {
                            if let Action::RepositoryRow(crate::workspace::RowAction::DiffLine(
                                anchor,
                            )) = &hit.action
                            {
                                Some(anchor.clone())
                            } else {
                                None
                            }
                        });
                    if let Some(anchor) = anchor {
                        self.extend_repository_selection(anchor);
                    }
                    return;
                }
            }
        }
        if self.overlay.is_none()
            && self.view == View::Search
            && crate::search_ui::handle_event(self, &event)
        {
            return;
        }
        if self.overlay == Some(Overlay::Form) {
            if self.form.as_ref().is_some_and(|form| {
                crate::scoped_pr_collaboration_ui_v17::issue_link_v87::picker(form).is_some()
            }) {
                let form = self.form.as_mut().unwrap();
                let picker =
                    crate::scoped_pr_collaboration_ui_v17::issue_link_v87::picker(form).unwrap();
                let action = match &event {
                    Event::Key(key) if key.kind != KeyEventKind::Release => {
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c')
                        {
                            self.dispatch(Action::Quit);
                            return;
                        }
                        picker.key(key.code, key.modifiers)
                    }
                    Event::Paste(text) => Some(
                        crate::pr_metadata_picker::PrMetadataPickerAction::Paste(text.clone()),
                    ),
                    Event::Mouse(mouse) => picker.mouse(
                        mouse.kind,
                        mouse.column,
                        mouse.row,
                        &form.metadata_picker_hits,
                    ),
                    Event::Resize(_, _) => {
                        form.metadata_picker_hits.clear();
                        None
                    }
                    _ => None,
                };
                if let Some(action) = action {
                    self.dispatch(Action::Form(crate::forms::FormAction::MetadataPicker(
                        action,
                    )));
                }
                return;
            }
            if self.form.as_ref().is_some_and(|form| {
                crate::scoped_pr_collaboration_ui_v17::metadata_base_v32::picker(form).is_some()
            }) {
                let form = self.form.as_mut().unwrap();
                let picker =
                    crate::scoped_pr_collaboration_ui_v17::metadata_base_v32::picker(form).unwrap();
                let action = match &event {
                    Event::Key(key) if key.kind != KeyEventKind::Release => {
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c')
                        {
                            self.dispatch(Action::Quit);
                            return;
                        }
                        picker.key(key.code, key.modifiers)
                    }
                    Event::Paste(text) => Some(
                        crate::pr_metadata_picker::PrMetadataPickerAction::Paste(text.clone()),
                    ),
                    Event::Mouse(mouse) => picker.mouse(
                        mouse.kind,
                        mouse.column,
                        mouse.row,
                        &form.metadata_picker_hits,
                    ),
                    Event::Resize(_, _) => {
                        form.metadata_picker_hits.clear();
                        None
                    }
                    _ => None,
                };
                if let Some(action) = action {
                    self.dispatch(Action::Form(crate::forms::FormAction::MetadataPicker(
                        action,
                    )));
                }
                return;
            }
            if self
                .form
                .as_ref()
                .is_some_and(|form| form.metadata_picker.is_some())
            {
                let form = self.form.as_mut().unwrap();
                let picker = form.metadata_picker.as_ref().unwrap();
                let action = match &event {
                    Event::Key(key) if key.kind != KeyEventKind::Release => {
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c')
                        {
                            self.dispatch(Action::Quit);
                            return;
                        }
                        picker.key(key.code, key.modifiers)
                    }
                    Event::Paste(text) => Some(
                        crate::pr_metadata_picker::PrMetadataPickerAction::Paste(text.clone()),
                    ),
                    Event::Mouse(mouse) => picker.mouse(
                        mouse.kind,
                        mouse.column,
                        mouse.row,
                        &form.metadata_picker_hits,
                    ),
                    Event::Resize(_, _) => {
                        form.metadata_picker_hits.clear();
                        None
                    }
                    _ => None,
                };
                if let Some(action) = action {
                    self.dispatch(Action::Form(crate::forms::FormAction::MetadataPicker(
                        action,
                    )));
                }
                return;
            }
            if self.form.as_ref().is_some_and(|form| form.picker.is_some()) {
                let action = match &event {
                    Event::Key(key) if key.kind != KeyEventKind::Release => {
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c')
                        {
                            self.dispatch(Action::Quit);
                            return;
                        }
                        crate::local_picker::LocalPicker::key(*key)
                    }
                    Event::Paste(text) => {
                        Some(crate::local_picker::PickerAction::Insert(text.clone()))
                    }
                    Event::Mouse(mouse)
                        if mouse.kind == MouseEventKind::Down(MouseButton::Left) =>
                    {
                        self.hits
                            .iter()
                            .rev()
                            .find(|hit| hit.rect.contains((mouse.column, mouse.row).into()))
                            .and_then(|hit| {
                                if let Action::Form(crate::forms::FormAction::Picker(action)) =
                                    &hit.action
                                {
                                    Some(action.clone())
                                } else {
                                    None
                                }
                            })
                    }
                    Event::Mouse(mouse)
                        if matches!(
                            mouse.kind,
                            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
                        ) =>
                    {
                        Some(crate::local_picker::PickerAction::Move(
                            if mouse.kind == MouseEventKind::ScrollDown {
                                3
                            } else {
                                -3
                            },
                        ))
                    }
                    _ => None,
                };
                if let Some(action) = action {
                    self.dispatch(Action::Form(crate::forms::FormAction::Picker(action)));
                }
                return;
            }
            match &event {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        self.dispatch(Action::Quit);
                        return;
                    }
                    if let Some(action) = self
                        .form
                        .as_mut()
                        .and_then(|form| form.key(key.code, key.modifiers))
                    {
                        self.dispatch(Action::Form(action));
                    }
                    return;
                }
                Event::Paste(text) => {
                    if let Some(form) = &mut self.form {
                        form.paste(text);
                    }
                    return;
                }
                Event::Mouse(mouse)
                    if matches!(
                        mouse.kind,
                        MouseEventKind::Down(MouseButton::Left)
                            | MouseEventKind::Drag(MouseButton::Left)
                    ) =>
                {
                    if let Some(form) = &mut self.form {
                        if form
                            .field_rects
                            .iter()
                            .any(|rect| rect.contains((mouse.column, mouse.row).into()))
                        {
                            form.click(
                                mouse.column,
                                mouse.row,
                                matches!(mouse.kind, MouseEventKind::Drag(_)),
                            );
                            return;
                        }
                    }
                    if matches!(mouse.kind, MouseEventKind::Down(_)) {
                        if let Some(hit) = self
                            .hits
                            .iter()
                            .rev()
                            .find(|hit| hit.rect.contains((mouse.column, mouse.row).into()))
                        {
                            self.dispatch(hit.action.clone());
                        }
                    }
                    return;
                }
                Event::Mouse(mouse)
                    if matches!(
                        mouse.kind,
                        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
                    ) =>
                {
                    if let Some(form) = &mut self.form {
                        if crate::scoped_pr_observed_tabs_v1::resources::immutable_images::inline_v1::ui_v1::anchor_open(form) {
                            form.dispatch(&crate::forms::FormAction::InlineAnchorScroll(if mouse.kind==MouseEventKind::ScrollDown {3}else{-3}));
                            return;
                        }
                        if !form.error.is_empty()
                            && form.error_rect.contains((mouse.column, mouse.row).into())
                        {
                            form.dispatch(&crate::forms::FormAction::ErrorScroll(
                                if mouse.kind == MouseEventKind::ScrollDown {
                                    3
                                } else {
                                    -3
                                },
                            ));
                            return;
                        }
                        if (form.preview
                            || matches!(
                                form.purpose,
                                crate::forms::FormPurpose::ScopedActionsV3(_)
                                    | crate::forms::FormPurpose::ScopedActionsWorkflowV6(_)
                                    | crate::forms::FormPurpose::ScopedActionsDispatchV7(_)
                            ))
                            && form.uses_prepared_preview()
                        {
                            form.preview_scroll = form.preview_scroll.saturating_add_signed(
                                if mouse.kind == MouseEventKind::ScrollDown {
                                    3
                                } else {
                                    -3
                                },
                            );
                            return;
                        }
                        if let Some(field) = form.fields.get_mut(form.focus) {
                            field
                                .editor
                                .scroll(if mouse.kind == MouseEventKind::ScrollDown {
                                    3
                                } else {
                                    -3
                                });
                        }
                    }
                    return;
                }
                _ => {}
            }
            if !matches!(event, Event::Resize(_, _)) {
                return;
            }
        }
        if self.overlay == Some(Overlay::Settings) {
            if let Event::Key(ref key) = event {
                if key.kind != KeyEventKind::Release {
                    self.settings_key(key.code, key.modifiers);
                }
                return;
            }
            if let Event::Paste(ref text) = event {
                self.settings_insert(&safe_text(text).replace(['\n', '\t'], " "));
                return;
            }
        }
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    self.dispatch(Action::Quit);
                    return;
                }
                if let Some(overlay) = self.overlay {
                    match key.code {
                        KeyCode::Down | KeyCode::Char('j') if overlay == Overlay::PrReactions => {
                            self.dispatch(Action::PrReactionScroll(1))
                        }
                        KeyCode::Up | KeyCode::Char('k') if overlay == Overlay::PrReactions => {
                            self.dispatch(Action::PrReactionScroll(-1))
                        }
                        KeyCode::PageDown if overlay == Overlay::PrReactions => {
                            self.dispatch(Action::PrReactionScroll(self.pr_reaction_rows as i32))
                        }
                        KeyCode::PageUp if overlay == Overlay::PrReactions => {
                            self.dispatch(Action::PrReactionScroll(-(self.pr_reaction_rows as i32)))
                        }
                        KeyCode::Tab if overlay == Overlay::PrReactions => {
                            self.focus = (self.focus + 1) % self.hits.len().max(1);
                        }
                        KeyCode::BackTab if overlay == Overlay::PrReactions => {
                            self.focus = (self.focus + self.hits.len().saturating_sub(1))
                                % self.hits.len().max(1);
                        }
                        KeyCode::Enter if overlay == Overlay::PrReactions => {
                            if let Some(hit) = self.hits.get(self.focus) {
                                self.dispatch(hit.action.clone());
                            }
                        }
                        KeyCode::Down
                            if matches!(overlay, Overlay::Notice | Overlay::SearchRankDetails) =>
                        {
                            self.dispatch(Action::NoticeScroll(1))
                        }
                        KeyCode::Up
                            if matches!(overlay, Overlay::Notice | Overlay::SearchRankDetails) =>
                        {
                            self.dispatch(Action::NoticeScroll(-1))
                        }
                        KeyCode::PageDown
                            if matches!(overlay, Overlay::Notice | Overlay::SearchRankDetails) =>
                        {
                            self.dispatch(Action::NoticeScroll(8))
                        }
                        KeyCode::PageUp
                            if matches!(overlay, Overlay::Notice | Overlay::SearchRankDetails) =>
                        {
                            self.dispatch(Action::NoticeScroll(-8))
                        }
                        KeyCode::PageDown
                            if matches!(overlay, Overlay::LocalActions | Overlay::PrActions) =>
                        {
                            self.dispatch(Action::MenuScroll(if overlay == Overlay::PrActions {
                                self.pr_menu_rows as i32
                            } else {
                                8
                            }))
                        }
                        KeyCode::PageUp
                            if matches!(overlay, Overlay::LocalActions | Overlay::PrActions) =>
                        {
                            self.dispatch(Action::MenuScroll(if overlay == Overlay::PrActions {
                                -(self.pr_menu_rows as i32)
                            } else {
                                -8
                            }))
                        }
                        KeyCode::Down | KeyCode::Char('j') if overlay == Overlay::LocalActions => {
                            self.local_action_index = (self.local_action_index + 1)
                                % crate::local_actions::LocalCommand::ALL.len()
                        }
                        KeyCode::Up | KeyCode::Char('k') if overlay == Overlay::LocalActions => {
                            self.local_action_index = (self.local_action_index
                                + crate::local_actions::LocalCommand::ALL.len()
                                - 1)
                                % crate::local_actions::LocalCommand::ALL.len()
                        }
                        KeyCode::Enter if overlay == Overlay::LocalActions => {
                            self.dispatch(Action::LocalCommand(
                                crate::local_actions::LocalCommand::ALL[self.local_action_index].0,
                            ))
                        }
                        KeyCode::Down | KeyCode::Char('j') if overlay == Overlay::PrActions => {
                            self.pr_action_index =
                                (self.pr_action_index + 1) % self.pr_controls().len();
                        }
                        KeyCode::Up | KeyCode::Char('k') if overlay == Overlay::PrActions => {
                            self.pr_action_index =
                                (self.pr_action_index + self.pr_controls().len() - 1)
                                    % self.pr_controls().len();
                        }
                        KeyCode::Home if overlay == Overlay::PrActions => self.pr_action_index = 0,
                        KeyCode::End if overlay == Overlay::PrActions => {
                            self.pr_action_index = self.pr_controls().len().saturating_sub(1)
                        }
                        KeyCode::Enter if overlay == Overlay::PrActions => {
                            if let Some((_, action)) = self.pr_controls().get(self.pr_action_index)
                            {
                                self.dispatch(action.clone());
                            }
                        }
                        KeyCode::Esc | KeyCode::Char('?') => self.dispatch(Action::Close),
                        KeyCode::Char('q') => self.dispatch(Action::Quit),
                        KeyCode::Down | KeyCode::Char('j') if overlay == Overlay::Palette => {
                            self.palette_index =
                                (self.palette_index + 1) % self.palette_controls().len();
                        }
                        KeyCode::Up | KeyCode::Char('k') if overlay == Overlay::Palette => {
                            self.palette_index =
                                (self.palette_index + self.palette_controls().len() - 1)
                                    % self.palette_controls().len();
                        }
                        KeyCode::Enter if overlay == Overlay::Palette => {
                            if let Some((_, action)) =
                                self.palette_controls().get(self.palette_index)
                            {
                                self.dispatch(action.clone());
                            }
                        }
                        KeyCode::Enter => self.dispatch(Action::Close),
                        _ => {}
                    }
                    return;
                }
                // The History query owns typed characters before any shortcut.
                if crate::repository_ui::history_filter::key(self, key) {
                    return;
                }
                // A Dashboard name being typed owns every key before any shortcut.
                if self.view == View::Dashboard {
                    if let Some(action) = self
                        .metrics
                        .as_ref()
                        .and_then(|metrics| crate::metrics_ui::name_entry_action(key, metrics))
                    {
                        self.dispatch(Action::Metrics(action));
                        return;
                    }
                }
                if let Some(shortcut) = pressed_shortcut(key.code, key.modifiers) {
                    if let Some(action) = self
                        .bindings
                        .iter()
                        .find(|(_, binding)| **binding == shortcut)
                        .and_then(|(name, _)| named_action(name))
                    {
                        self.dispatch(action);
                        return;
                    }
                }
                // The History commit detail's cursor keys come before the generic preview's.
                if crate::history::detail::key(self, key) {
                    return;
                }
                // List plus preview split: Tab between the panes, keys of the focused preview.
                if crate::list_preview_v92::key(self, key) {
                    return;
                }
                // TASK-120: `y` copies the focused item (SHA, URL, path, log line, comment).
                if key.code == KeyCode::Char('y')
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                    && matches!(self.view, View::Inbox | View::Actions | View::Repository)
                    // A view that binds `y` itself (the core.worktree `y Confirm unset`)
                    // keeps it: the key does what its painted control says.
                    && !crate::footer::main_controls(self).iter().any(|control| {
                        control.key == "y"
                            && control.action
                                != Action::Clipboard(crate::clipboard::ClipboardAction::CopyFocused)
                    })
                {
                    self.dispatch(Action::Clipboard(
                        crate::clipboard::ClipboardAction::CopyFocused,
                    ));
                    return;
                }
                if matches!(self.view, View::Inbox | View::Actions)
                    && self
                        .pr_checks_route
                        .as_ref()
                        .is_some_and(|route| route.scoped_v1())
                    && key.code == KeyCode::Char('C')
                {
                    self.dispatch(Action::PrCheckChoice(
                        crate::pr_checks_native::scoped::choices::PrCheckChoiceUiV1::Open,
                    ));
                    return;
                }
                if self.view == View::Actions
                    && self
                        .pr_checks_route
                        .as_ref()
                        .is_some_and(|route| route.scoped_v1())
                    && matches!(key.code, KeyCode::Char('b') | KeyCode::Esc)
                {
                    if key.code == KeyCode::Esc && self.loading[View::Actions.index()] {
                        self.dispatch(Action::Cancel);
                        return;
                    }
                    self.dispatch(Action::Actions(
                        crate::actions_workspace::ActionsAction::Back,
                    ));
                    return;
                }
                if self.view == View::Actions
                    && self.scoped_actions_v1.is_none()
                    && self.pr_checks_route.as_ref().is_some_and(|r| r.scoped_v1())
                {
                    let local = match key.code {
                        KeyCode::Char('d') => Some(crate::pr_checks_native::scoped::choices::continuations_v2::ui::PrCheckContinueUiV2::Details),
                        KeyCode::Char('n') => Some(crate::pr_checks_native::scoped::choices::continuations_v2::ui::PrCheckContinueUiV2::NextMatch),
                        KeyCode::Char('x') => Some(crate::pr_checks_native::scoped::choices::continuations_v2::ui::PrCheckContinueUiV2::ClearMatches),
                        KeyCode::Char('>') => Some(crate::pr_checks_native::scoped::choices::continuations_v2::ui::PrCheckContinueUiV2::FailedStep(true)),
                        KeyCode::Char('<') => Some(crate::pr_checks_native::scoped::choices::continuations_v2::ui::PrCheckContinueUiV2::FailedStep(false)),
                        _ => None,
                    };
                    if let Some(action) = local {
                        self.dispatch(Action::PrCheckContinueV2(action));
                        return;
                    }
                }
                if self.view == View::Actions && self.actions.is_some() {
                    let accepts_key = key.code != KeyCode::Enter || self.actions_content_focused;
                    if accepts_key {
                        let actions = self.actions.as_ref().unwrap();
                        if let Some(action) = crate::footer::resolve_key(
                            &crate::actions_ui::footer_keys(actions),
                            key,
                        ) {
                            self.dispatch(action);
                            return;
                        }
                        if let Some(action) = crate::actions_ui::keyboard_action(key, actions) {
                            self.dispatch(Action::Actions(action));
                            return;
                        }
                    }
                }
                if self.view == View::Dashboard
                    && self.metrics.is_some()
                    && (key.code != KeyCode::Enter || self.metrics_content_focused)
                    && (key.code != KeyCode::Esc || !self.loading[View::Dashboard.index()])
                {
                    let metrics = self.metrics.as_ref().unwrap();
                    if let Some(action) =
                        crate::footer::resolve_key(&crate::metrics_ui::footer_keys(metrics), key)
                    {
                        self.dispatch(action);
                        return;
                    }
                    if let Some(action) = crate::metrics_ui::keyboard_action(key, metrics) {
                        self.dispatch(Action::Metrics(action));
                        return;
                    }
                    if key.code == KeyCode::Enter && self.metrics_content_focused {
                        return;
                    }
                }
                if self.view == View::Dashboard
                    && self.metrics.is_none()
                    && self.metrics_content_focused
                    && key.code == KeyCode::Enter
                {
                    return;
                }
                if self.view == View::Repository {
                    // The stale core.worktree screen: arrows choose, Enter runs (TASK-118).
                    if crate::repository_worktrees_v118::key(self, key) {
                        return;
                    }
                    if self
                        .workspace
                        .as_ref()
                        .is_some_and(|w| w.actor_profile.is_some())
                    {
                        if let Some(action) = crate::repository_commit_actor_ui_v1::profile_key(
                            key.code,
                            key.modifiers,
                        ) {
                            self.dispatch(Action::LocalActor(action));
                            return;
                        }
                    }
                    // Keys painted in the key bar (and its mode tabs) resolve exactly like
                    // their click targets; disabled ones explain. The arms below cover keys
                    // whose control is not in the bar for the current state.
                    if let Some(action) =
                        crate::footer::resolve_key(&crate::repository_ui::footer_keys(self), key)
                    {
                        self.dispatch(action);
                        return;
                    }
                    if let Some(graph) = self.workspace.as_ref().and_then(|workspace| {
                        crate::repository_graph_ui_v1::key(workspace, key.code)
                    }) {
                        self.dispatch(Action::RepositoryRow(crate::workspace::RowAction::Graph(
                            graph,
                        )));
                        return;
                    }
                    if key.code == KeyCode::Char('f') {
                        self.dispatch(Action::OriginalDiffSidesV1(
                            crate::repository_diff_sides_v1::OriginalDiffActionV1::Open,
                        ));
                        return;
                    }
                    if key.code == KeyCode::Char('v') {
                        crate::repository_native_diff_native_v1::open(self);
                        return;
                    }
                    if matches!(key.code, KeyCode::Char('u') | KeyCode::Esc)
                        && !self.loading[View::Repository.index()]
                        && self
                            .workspace
                            .as_ref()
                            .is_some_and(|workspace| workspace.tree_file_return_v1().is_some())
                    {
                        self.dispatch(Action::RepositoryMode(
                            crate::workspace::RepositoryMode::Tree,
                        ));
                        return;
                    }
                    let file_action =
                        match key.code {
                            // `H` is an alias of `f`: History with a path chip (TASK-99.3).
                            KeyCode::Char('H') => Some(crate::workspace::FileAction::PathHistory),
                            KeyCode::Char('Y') => Some(crate::workspace::FileAction::Blame),
                            KeyCode::Char('u') => Some(crate::workspace::FileAction::Back),
                            KeyCode::Char('P') => Some(crate::workspace::FileAction::Previous),
                            KeyCode::Char('N') => Some(crate::workspace::FileAction::Next),
                            KeyCode::Char('G')
                                if self.workspace.as_ref().is_some_and(|workspace| {
                                    workspace.file_provenance.is_some()
                                }) =>
                            {
                                Some(crate::workspace::FileAction::Graph)
                            }
                            KeyCode::Char('D')
                                if self.workspace.as_ref().is_some_and(|workspace| {
                                    workspace.file_provenance.is_some()
                                }) =>
                            {
                                Some(crate::workspace::FileAction::Diff)
                            }
                            KeyCode::Char('F')
                                if self.workspace.as_ref().is_some_and(|workspace| {
                                    workspace.file_provenance.is_some()
                                }) =>
                            {
                                Some(crate::workspace::FileAction::OriginalFile)
                            }
                            _ => None,
                        };
                    if let Some(action) = file_action {
                        self.dispatch(Action::RepositoryRow(
                            crate::workspace::RowAction::FileProvenance(action),
                        ));
                        return;
                    }
                    if key.code == KeyCode::Char('C') {
                        self.dispatch(Action::Compare);
                        return;
                    }
                    if key.code == KeyCode::Char('I') {
                        self.dispatch(Action::RepositorySource);
                        return;
                    }
                    if key.code == KeyCode::Char('h') {
                        self.dispatch(Action::RepositoryHunk);
                        return;
                    }
                    if let Some((quick, _, _)) = crate::local_actions::QuickLocal::CONTROLS
                        .iter()
                        .find(|(_, _, shortcut)| key.code == KeyCode::Char(*shortcut))
                    {
                        self.dispatch(Action::LocalQuick(*quick));
                        return;
                    }
                    if self.repository_content_focused {
                        if key.modifiers.contains(KeyModifiers::SHIFT)
                            && matches!(key.code, KeyCode::Up | KeyCode::Down)
                        {
                            let previous = self.repository_list.selected;
                            let start = self.repository_anchor(previous);
                            self.move_repository_selection(key.code == KeyCode::Down);
                            if let (Some(start), Some(end)) =
                                (start, self.repository_anchor(self.repository_list.selected))
                            {
                                if self.repository_selection.is_none() {
                                    self.repository_selection = Some(DiffSelection {
                                        start: start.clone(),
                                        end: start,
                                    });
                                }
                                if !self.extend_repository_selection(end) {
                                    self.repository_list.selected = previous;
                                }
                            }
                            return;
                        }
                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                self.move_repository_selection(false);
                                return;
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                self.move_repository_selection(true);
                                return;
                            }
                            KeyCode::Enter => {
                                if let Some(action) = self
                                    .workspace
                                    .as_ref()
                                    .and_then(|workspace| {
                                        workspace
                                            .display_rows_v1()
                                            .get(self.repository_list.selected)
                                    })
                                    .and_then(|row| row.action.clone())
                                {
                                    if let crate::workspace::RowAction::Local(command) = action {
                                        self.dispatch(Action::LocalCommand(command));
                                    } else {
                                        self.dispatch(Action::RepositoryRow(action));
                                    }
                                }
                                return;
                            }
                            _ => {}
                        }
                    }
                }
                if self.view == View::Inbox && self.github.is_some() {
                    use crate::github_workspace::{GithubAction, PrTab};
                    let pending_observed = self.github.as_ref().is_some_and(|github| {
                        github.pr.is_none() && github.scoped_pr_navigation.is_some()
                    });
                    if pending_observed {
                        match key.code {
                            KeyCode::Char('b') | KeyCode::Esc => {
                                self.dispatch(Action::Github(GithubAction::Back));
                                return;
                            }
                            KeyCode::Char(
                                'c' | 'h' | 't' | 'o' | 'f' | 'u' | 'v' | 'k' | 'j' | 'p' | 'x'
                                | 'N' | '}' | 'L' | 'X' | 'O',
                            )
                            | KeyCode::Up
                            | KeyCode::Down
                            | KeyCode::Enter => return,
                            _ => {}
                        }
                    }
                    let pr_open = self
                        .github
                        .as_ref()
                        .is_some_and(|github| github.pr.is_some());
                    // Footer keys and tabs: the same actions their tokens carry. Enter keeps
                    // its focus-dependent meaning below.
                    if key.code != KeyCode::Enter {
                        if let Some(action) =
                            crate::footer::resolve_key(&crate::github_ui::footer_keys(self), key)
                        {
                            self.dispatch(action);
                            return;
                        }
                    }
                    // Tabs an observed section does not offer still answer their keys.
                    if let (KeyCode::Char(character), true) = (key.code, pr_open) {
                        if let Some((tab, _, _)) =
                            PrTab::ALL.iter().find(|(_, _, key)| *key == character)
                        {
                            self.dispatch(Action::Github(GithubAction::PrTab(*tab)));
                            return;
                        }
                    }
                    if self.github_content_focused && !pr_open {
                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                self.move_github_selection(false);
                                return;
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                self.move_github_selection(true);
                                return;
                            }
                            KeyCode::Enter => {
                                if let Some(key) = self
                                    .github
                                    .as_ref()
                                    .and_then(|github| github.selected.clone())
                                {
                                    self.dispatch(Action::Github(GithubAction::Open(key)));
                                }
                                return;
                            }
                            _ => {}
                        }
                    }
                    if self.github_content_focused && pr_open {
                        match key.code {
                            KeyCode::Up => {
                                self.move_github_pr_row(false);
                                return;
                            }
                            KeyCode::Down => {
                                self.move_github_pr_row(true);
                                return;
                            }
                            KeyCode::Enter => {
                                if self.github.as_ref().is_some_and(|github| {
                                    github.scoped_pr_resource.is_some()
                                        && github
                                            .pr
                                            .as_ref()
                                            .is_some_and(|page| page.tab == PrTab::Checks)
                                }) {
                                    self.dispatch(crate::scoped_pr_observed_tabs_v1::resources::native::check_action(self));
                                    return;
                                }
                                if self
                                    .github
                                    .as_ref()
                                    .and_then(|github| github.pr.as_ref())
                                    .is_some_and(|page| {
                                        page.check_at(self.inbox_list.selected).is_some()
                                    })
                                {
                                    self.dispatch(Action::PrChecks(
                                        crate::pr_checks_native::PrChecksUiAction::Open,
                                    ));
                                    return;
                                }
                                if self
                                    .github
                                    .as_ref()
                                    .and_then(|github| github.pr.as_ref())
                                    .is_some_and(|page| page.tab == PrTab::Timeline)
                                {
                                    self.dispatch(crate::pr_timeline_links_v13::action(self));
                                    return;
                                }
                                self.dispatch(Action::GithubRow(self.inbox_list.selected));
                                return;
                            }
                            _ => {}
                        }
                    }
                }
                match key.code {
                    KeyCode::F(1) => self.dispatch(Action::Help),
                    KeyCode::F(8)
                        if self.view == View::Inbox
                            && self.github.as_ref().is_some_and(|github| {
                                github.pr.is_none() && github.scoped_pr_navigation.is_none()
                            }) =>
                    {
                        self.dispatch(Action::NotificationsV63(
                            crate::notifications_ui_v63::NotificationActionV63::Open,
                        ))
                    }
                    KeyCode::F(10) if self.view == View::Inbox => {
                        self.dispatch(Action::OpenScopedReviewsV2)
                    }
                    KeyCode::F(8)
                        if self.view == View::Inbox
                            && self.github.as_ref().is_some_and(|github| {
                                github.scoped_pr_navigation.is_some()
                                    && github.pr.as_ref().is_some_and(|page| {
                                        page.tab == crate::github_workspace::PrTab::Summary
                                    })
                            }) =>
                    {
                        self.dispatch(Action::Github(
                            crate::github_workspace::GithubAction::ActionsHistoryV5,
                        ))
                    }
                    KeyCode::F(8) if self.view == View::Inbox && self.github.is_none() => {
                        self.status =
                            "Refresh the Inbox (r) first, then F8 opens notifications.".into()
                    }
                    KeyCode::F(9) if self.view == View::Inbox => self.dispatch(Action::Github(
                        crate::github_workspace::GithubAction::ReviewOperationHistoryV3,
                    )),
                    KeyCode::Char('/') => self.dispatch(Action::View(View::Search)),
                    KeyCode::Esc if self.loading[self.view.index()] => {
                        self.dispatch(Action::Cancel)
                    }
                    KeyCode::Tab => {
                        self.metrics_content_focused = false;
                        self.actions_content_focused = false;
                        self.repository_content_focused = false;
                        self.github_content_focused = false;
                        self.search_content_focused = false;
                        self.focus = (self.focus + 1) % self.hits.len().max(1);
                    }
                    KeyCode::BackTab => {
                        self.metrics_content_focused = false;
                        self.actions_content_focused = false;
                        self.repository_content_focused = false;
                        self.github_content_focused = false;
                        self.search_content_focused = false;
                        self.focus = (self.focus + self.hits.len().saturating_sub(1))
                            % self.hits.len().max(1)
                    }
                    KeyCode::Enter => {
                        if let Some(hit) = self.hits.get(self.focus) {
                            self.dispatch(hit.action.clone());
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => self.dispatch(Action::Scroll(1)),
                    KeyCode::Up | KeyCode::Char('k') => self.dispatch(Action::Scroll(-1)),
                    KeyCode::PageDown => self.dispatch(Action::Scroll(10)),
                    KeyCode::PageUp => self.dispatch(Action::Scroll(-10)),
                    _ => {}
                }
            }
            Event::Mouse(mouse) => {
                let wheel = match mouse.kind {
                    MouseEventKind::ScrollDown => Some(3),
                    MouseEventKind::ScrollUp => Some(-3),
                    _ => None,
                };
                if crate::list_preview_v92::mouse(
                    self,
                    mouse.column,
                    mouse.row,
                    mouse.kind == MouseEventKind::Down(MouseButton::Left),
                    wheel,
                ) {
                    return;
                }
                if crate::history::detail::mouse(self, mouse.column, mouse.row, wheel) {
                    return;
                }
                if crate::repository_panels_v93::mouse(self, mouse.column, mouse.row, wheel) {
                    return;
                }
                let hovering =
                    self.pointer_region
                        .contains(self.view, self.overlay, mouse.column, mouse.row);
                if self.overlay.is_none()
                    && self.view == View::Dashboard
                    && matches!(
                        mouse.kind,
                        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
                    )
                {
                    if self
                        .metrics_content
                        .contains((mouse.column, mouse.row).into())
                    {
                        if let Some(metrics) = &self.metrics {
                            let index = metrics.selected.saturating_add_signed(
                                if mouse.kind == MouseEventKind::ScrollDown {
                                    3
                                } else {
                                    -3
                                },
                            );
                            self.dispatch(Action::Metrics(
                                crate::metrics_workspace::MetricsWorkspaceAction::Select(index),
                            ));
                        }
                    }
                    return;
                }
                if self.overlay == Some(Overlay::PrReactions)
                    && matches!(
                        mouse.kind,
                        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
                    )
                {
                    if hovering {
                        self.dispatch(Action::PrReactionScroll(
                            if mouse.kind == MouseEventKind::ScrollDown {
                                3
                            } else {
                                -3
                            },
                        ));
                    }
                    return;
                }
                if matches!(
                    self.overlay,
                    Some(Overlay::Notice | Overlay::SearchRankDetails)
                ) && matches!(
                    mouse.kind,
                    MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
                ) {
                    if hovering {
                        self.dispatch(Action::NoticeScroll(
                            if mouse.kind == MouseEventKind::ScrollDown {
                                3
                            } else {
                                -3
                            },
                        ));
                    }
                    return;
                }
                if self.overlay.is_none()
                    && self.view == View::Actions
                    && matches!(
                        mouse.kind,
                        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
                    )
                {
                    if let Some(actions) = self.actions.as_ref().filter(|_| hovering) {
                        let index = crate::actions_ui::row_step(
                            actions,
                            if mouse.kind == MouseEventKind::ScrollDown {
                                3
                            } else {
                                -3
                            },
                        );
                        self.dispatch(Action::Actions(
                            crate::actions_workspace::ActionsAction::Select(index),
                        ));
                    }
                    return;
                }
                if matches!(
                    self.overlay,
                    Some(Overlay::LocalActions | Overlay::PrActions)
                ) && matches!(
                    mouse.kind,
                    MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
                ) {
                    if hovering {
                        self.dispatch(Action::MenuScroll(
                            if mouse.kind == MouseEventKind::ScrollDown {
                                3
                            } else {
                                -3
                            },
                        ));
                    }
                    return;
                }
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if let Some((index, hit)) =
                            self.hits.iter().enumerate().rev().find(|(_, hit)| {
                                hit.rect.contains((mouse.column, mouse.row).into())
                            })
                        {
                            let action = hit.action.clone();
                            let hit_x = hit.rect.x;
                            self.focus = index;
                            self.dispatch(action);
                            if self.overlay == Some(Overlay::Settings) && self.focus <= 1 {
                                self.settings_cursor =
                                    (mouse.column.saturating_sub(hit_x) as usize).min(
                                        self.settings_field()
                                            .map_or(0, |field| field.chars().count()),
                                    );
                            }
                        }
                    }
                    MouseEventKind::ScrollDown if self.overlay.is_none() && hovering => {
                        self.dispatch(Action::Scroll(3))
                    }
                    MouseEventKind::ScrollUp if self.overlay.is_none() && hovering => {
                        self.dispatch(Action::Scroll(-3))
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn shortcut(&self, name: &str) -> &str {
        self.bindings.get(name).map_or("", String::as_str)
    }
    /// The first-run Inbox hint, naming the configured Refresh, Commands and Help keys.
    fn inbox_hint(&self) -> String {
        format!(
            "Press {} (or click Refresh below) to load your PRs • {} all commands • {} help",
            self.shortcut("refresh"),
            self.shortcut("palette"),
            self.shortcut("help")
        )
    }
    /// Applies configured key bindings; a still-visible first-run Inbox hint names the new keys.
    pub fn set_bindings(&mut self, bindings: BTreeMap<String, String>) {
        let previous = self.inbox_hint();
        self.bindings = bindings;
        let current = self.inbox_hint();
        for line in &mut self.content[View::Inbox.index()] {
            if *line == previous {
                line.clone_from(&current);
            }
        }
    }
    fn repository_anchor(&self, index: usize) -> Option<crate::repository_view::DiffAnchor> {
        self.workspace
            .as_ref()
            .and_then(|workspace| workspace.display_rows_v1().get(index))
            .and_then(|row| {
                if let Some(crate::workspace::RowAction::DiffLine(anchor)) = &row.action {
                    Some(anchor.clone())
                } else {
                    None
                }
            })
    }
    pub fn repository_anchor_selected(&self, anchor: &crate::repository_view::DiffAnchor) -> bool {
        self.repository_selection
            .as_ref()
            .is_some_and(|selection| selection.contains(anchor))
    }
    fn extend_repository_selection(&mut self, anchor: crate::repository_view::DiffAnchor) -> bool {
        let Some(selection) = &mut self.repository_selection else {
            return false;
        };
        if !selection.same_file(&anchor) {
            self.status = "A range stays within one exact file and comparison".into();
            return false;
        }
        selection.end = anchor.clone();
        if let Some(index) = self.workspace.as_ref().and_then(|workspace| {
            workspace.display_rows_v1().iter().position(|row|matches!(&row.action,Some(crate::workspace::RowAction::DiffLine(candidate)) if candidate == &anchor))
        }) {
            self.repository_list.selected = index;
        }
        self.status = format!(
            "Selected {} canonical lines • h selects hunk • + Stage • - Unstage • m Local operations",
            self.local_context()
                .map_or(0, |context| context.selected_lines.len())
        );
        true
    }
    fn local_quick(&mut self, quick: crate::local_actions::QuickLocal) {
        if self.workspace.is_none() {
            return;
        }
        if quick != crate::local_actions::QuickLocal::Commit
            && !self.repository_content_focused
            && self.repository_selection.is_none()
        {
            // The row cursor is only painted while content has focus; never
            // act on an invisible row.
            self.status =
                "Select a changed file, diff line or hunk first (arrows or click) • + Stage • - Unstage"
                    .into();
            return;
        }
        if quick != crate::local_actions::QuickLocal::Commit
            && crate::repository_panels_v93::selection_hidden(self)
        {
            self.status =
                "The selected file is collapsed with the untracked files • u shows them".into();
            return;
        }
        // A cursor on a diff line with no explicit range stages its whole hunk.
        if quick != crate::local_actions::QuickLocal::Commit
            && self.repository_selection.is_none()
            && self
                .repository_anchor(self.repository_list.selected)
                .is_some()
        {
            self.select_repository_hunk();
        }
        let row = self
            .workspace
            .as_ref()
            .and_then(|workspace| {
                workspace
                    .display_rows_v1()
                    .get(self.repository_list.selected)
                    .cloned()
            })
            .and_then(|row| row.action);
        match quick.command(row.as_ref(), self.repository_selection.is_some()) {
            Ok(command) => {
                self.dispatch(Action::LocalCommand(command));
                self.local_command_preview = self.local_command.is_some()
                    && crate::local_actions::QuickLocal::previews_immediately(command);
            }
            Err(reason) => self.status = reason.into(),
        }
    }
    fn select_repository_hunk(&mut self) {
        let Some(anchor) = self.repository_anchor(self.repository_list.selected) else {
            self.status = "Select a diff line before selecting its hunk".into();
            return;
        };
        let anchors = self
            .workspace
            .as_ref()
            .unwrap()
            .display_rows_v1()
            .iter()
            .filter_map(|row| {
                if let Some(crate::workspace::RowAction::DiffLine(candidate)) = &row.action {
                    Some(candidate)
                } else {
                    None
                }
            })
            .filter(|candidate| {
                candidate.file_index == anchor.file_index
                    && candidate.hunk_index == anchor.hunk_index
            })
            .cloned()
            .collect::<Vec<_>>();
        if let (Some(first), Some(last)) = (anchors.first(), anchors.last()) {
            self.repository_selection = Some(DiffSelection {
                start: first.clone(),
                end: last.clone(),
            });
            self.status = format!(
                "Selected hunk • {} canonical lines • + Stage • - Unstage • m Local operations",
                anchors.len()
            );
        }
    }
    pub fn local_context(&self) -> Option<crate::local_actions::LocalContext> {
        let workspace = self.workspace.as_ref()?;
        let Some(selection) = &self.repository_selection else {
            return Some(crate::local_actions::LocalContext::from_row(
                workspace,
                self.repository_list.selected,
            ));
        };
        let mut context = crate::local_actions::LocalContext::from_workspace(
            workspace,
            None,
            Some(selection.start.clone()),
        );
        context.selected_lines = workspace
            .display_rows_v1()
            .iter()
            .filter_map(|row| {
                if let Some(crate::workspace::RowAction::DiffLine(anchor)) = &row.action {
                    Some(anchor)
                } else {
                    None
                }
            })
            .filter(|anchor| selection.contains(anchor))
            .filter(|anchor| {
                matches!(
                    workspace
                        .comparison_v1()
                        .and_then(|comparison| comparison.diff.files.get(anchor.file_index))
                        .and_then(|file| file.hunks.get(anchor.hunk_index))
                        .and_then(|hunk| hunk.lines.get(anchor.line_index))
                        .map(|line| line.kind),
                    Some(crate::diff::LineKind::Added | crate::diff::LineKind::Removed)
                )
            })
            .map(|anchor| (anchor.hunk_index, anchor.line_index))
            .collect();
        context.selected_lines.sort_unstable();
        context.selected_lines.dedup();
        Some(context)
    }
    pub fn theme(&self) -> crate::theme::Theme {
        crate::theme::Theme::new(
            crate::config::ThemeName::from_index(self.settings_theme),
            if self.no_color {
                crate::theme::ColorMode::NoColor
            } else {
                self.color_mode
            },
        )
        .with_view(self.view)
    }

    fn move_github_selection(&mut self, down: bool) {
        if let Some(github) = &mut self.github {
            let length = github.visible().len();
            let index = github.selected_index();
            let next = if down {
                (index + 1).min(length.saturating_sub(1))
            } else {
                index.saturating_sub(1)
            };
            github.select_index(next);
            self.inbox_list.selected = next;
            if next < self.inbox_list.top {
                self.inbox_list.top = next;
            } else if next >= self.inbox_list.top + self.github_visible_rows {
                self.inbox_list.top = next + 1 - self.github_visible_rows.max(1);
            }
        }
    }

    fn move_github_pr_row(&mut self, down: bool) {
        if let Some(next) = self
            .github
            .as_ref()
            .and_then(|github| github.pr_row_navigation_target(self.inbox_list.selected, down))
        {
            self.dispatch(Action::GithubRow(next));
        } else {
            self.dispatch(Action::Scroll(if down { 1 } else { -1 }));
        }
    }

    /// History (TASK-99): moving the `▌` bar onto a commit by keyboard chooses it exactly as a
    /// click on its row does, so the saved version (`Viewed:`, actor rows, `a`/`c`, Tree,
    /// Working and the Source identity) follows the bar. Read-only and in memory for a loaded
    /// commit; nothing changes when the bar stays on the saved commit, and a running read (a
    /// page of older commits) is not replaced by mere cursor movement.
    pub(crate) fn choose_history_bar_commit(&mut self) {
        let Some(workspace) = &self.workspace else {
            return;
        };
        if !workspace.graph_visible_v1()
            || workspace.graph_view_v1 != crate::repository_graph_v1::GraphViewV1::Timeline
            || self.loading[View::Repository.index()]
        {
            return;
        }
        let Some(crate::workspace::RowAction::Commit(id)) = workspace
            .display_rows_v1()
            .get(self.repository_list.selected)
            .and_then(|row| row.action.clone())
        else {
            return;
        };
        if id != workspace.revision {
            self.history_bar_choice_v99 = Some(id.clone());
            self.dispatch(Action::RepositoryRow(crate::workspace::RowAction::Commit(
                id,
            )));
        }
    }

    /// Leaving the History table for another mode starts from the commit under the `▌` bar,
    /// as a click on that row would, even when ↑/↓ moved the bar during a running read
    /// (TASK-99; keyboard and mouse share one meaning).
    fn sync_history_cursor_revision_v99(&mut self) {
        let selected = self.repository_list.selected;
        let Some(workspace) = self.workspace.as_mut() else {
            return;
        };
        if !workspace.graph_visible_v1()
            || workspace.graph_view_v1 != crate::repository_graph_v1::GraphViewV1::Timeline
        {
            return;
        }
        let Some(crate::workspace::RowAction::Commit(id)) = workspace
            .display_rows_v1()
            .get(selected)
            .and_then(|row| row.action.clone())
        else {
            return;
        };
        if id != workspace.revision
            && workspace
                .graph_state_v1
                .as_ref()
                .is_some_and(|state| state.selected_commit(&id).is_some())
        {
            workspace.revision = id;
        }
    }

    fn move_repository_selection(&mut self, down: bool) {
        let Some(workspace) = &self.workspace else {
            self.dispatch(Action::Scroll(if down { 1 } else { -1 }));
            return;
        };
        // A refreshed workspace can be shorter than the previous one.
        self.repository_list.selected = self
            .repository_list
            .selected
            .min(workspace.display_rows_v1().len().saturating_sub(1));
        // Rows the Repository panels collapse (untracked files) are skipped.
        let hidden = crate::repository_panels_v93::hidden_rows(self).unwrap_or_default();
        let workspace = self.workspace.as_ref().unwrap();
        let shown = |index: &usize| {
            workspace.display_rows_v1()[*index].action.is_some()
                && !hidden.get(*index).copied().unwrap_or(false)
        };
        let next = if down {
            (self.repository_list.selected + 1..workspace.display_rows_v1().len()).find(shown)
        } else {
            (0..self.repository_list.selected).rev().find(shown)
        };
        if let Some(next) = next {
            self.repository_list.selected = next;
            if next < self.repository_list.top {
                self.repository_list.top = next;
            } else if next >= self.repository_list.top + self.repository_visible_rows {
                self.repository_list.top = next + 1 - self.repository_visible_rows.max(1);
            }
            self.choose_history_bar_commit();
        } else {
            self.dispatch(Action::Scroll(if down { 1 } else { -1 }));
        }
    }

    fn settings_field(&self) -> Option<&String> {
        match self.focus {
            0 => Some(&self.settings_host),
            1 => Some(&self.settings_set),
            _ => None,
        }
    }

    fn settings_insert(&mut self, text: &str) {
        let cursor = self.settings_cursor;
        let field = match self.focus {
            0 => &mut self.settings_host,
            1 => &mut self.settings_set,
            _ => return,
        };
        let byte = field
            .char_indices()
            .nth(cursor)
            .map_or(field.len(), |(index, _)| index);
        field.insert_str(byte, text);
        self.settings_cursor += text.chars().count();
    }

    fn settings_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
            self.dispatch(Action::Quit);
            return;
        }
        match code {
            KeyCode::Esc => self.dispatch(Action::Close),
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = if code == KeyCode::Tab {
                    (self.focus + 1) % SETTINGS_CONTROLS
                } else {
                    (self.focus + SETTINGS_CONTROLS - 1) % SETTINGS_CONTROLS
                };
                self.settings_cursor = self
                    .settings_field()
                    .map_or(0, |value| value.chars().count());
            }
            KeyCode::Enter if self.focus == 2 => self.dispatch(Action::SettingsTheme),
            KeyCode::Enter if self.focus == 3 => self.dispatch(Action::SettingsDensity),
            KeyCode::Enter if self.focus == 4 => self.dispatch(Action::SettingsSave),
            KeyCode::Enter if self.focus == 5 => self.dispatch(Action::Close),
            KeyCode::Enter => {
                self.focus = (self.focus + 1) % SETTINGS_CONTROLS;
                self.settings_cursor = self
                    .settings_field()
                    .map_or(0, |value| value.chars().count());
            }
            KeyCode::Home => self.settings_cursor = 0,
            KeyCode::End => {
                self.settings_cursor = self
                    .settings_field()
                    .map_or(0, |value| value.chars().count())
            }
            KeyCode::Left => self.settings_cursor = self.settings_cursor.saturating_sub(1),
            KeyCode::Right => {
                self.settings_cursor = (self.settings_cursor + 1).min(
                    self.settings_field()
                        .map_or(0, |value| value.chars().count()),
                )
            }
            KeyCode::Char('a') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.settings_cursor = 0
            }
            KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                if self.focus == 0 {
                    self.settings_host.clear();
                } else if self.focus == 1 {
                    self.settings_set.clear();
                }
                self.settings_cursor = 0;
            }
            KeyCode::Backspace | KeyCode::Delete => {
                let position = if code == KeyCode::Backspace {
                    self.settings_cursor.saturating_sub(1)
                } else {
                    self.settings_cursor
                };
                if code == KeyCode::Backspace && self.settings_cursor == 0 {
                    return;
                }
                let field = match self.focus {
                    0 => &mut self.settings_host,
                    1 => &mut self.settings_set,
                    _ => return,
                };
                if let Some((byte, _)) = field.char_indices().nth(position) {
                    field.remove(byte);
                }
                if code == KeyCode::Backspace {
                    self.settings_cursor = position;
                }
            }
            KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
                self.settings_insert(&c.to_string())
            }
            _ => {}
        }
    }
}

/// Settings controls in focus (and hit) order: Host, Repository set, Theme, Density, Save,
/// Close.
const SETTINGS_CONTROLS: usize = 6;

fn named_action(name: &str) -> Option<Action> {
    Some(match name {
        "inbox" => Action::View(View::Inbox),
        "repository" => Action::View(View::Repository),
        "search" => Action::View(View::Search),
        "actions" => Action::View(View::Actions),
        "dashboard" => Action::View(View::Dashboard),
        "help" => Action::Help,
        "palette" => Action::Palette,
        "back" => Action::Navigation(crate::navigation_trail::NavigationAction::Back),
        "settings" => Action::Settings,
        "refresh" => Action::Refresh,
        "quit" => Action::Quit,
        _ => return None,
    })
}

pub(crate) fn pressed_shortcut(code: KeyCode, modifiers: KeyModifiers) -> Option<String> {
    let key = match code {
        KeyCode::Char(' ') => "space".into(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "enter".into(),
        KeyCode::Up => "up".into(),
        KeyCode::Down => "down".into(),
        KeyCode::Left => "left".into(),
        KeyCode::Right => "right".into(),
        KeyCode::Home => "home".into(),
        KeyCode::End => "end".into(),
        KeyCode::PageUp => "pageup".into(),
        KeyCode::PageDown => "pagedown".into(),
        _ => return None,
    };
    let mut prefixes = Vec::new();
    if modifiers.contains(KeyModifiers::ALT) {
        prefixes.push("alt");
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        prefixes.push("ctrl");
    }
    if prefixes.is_empty() {
        Some(key)
    } else {
        Some(format!("{}+{key}", prefixes.join("+")))
    }
}

/// Strip terminal controls from untrusted labels while keeping useful line breaks.
pub fn safe_text(text: &str) -> String {
    crate::providers::runner::sanitize_terminal(text)
}

#[cfg(test)]
#[path = "app/pointer_hover_tests.rs"]
mod pointer_hover_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn navigation_context_v10_each_view_keeps_its_list_place_across_switches() {
        let mut app = App::new();
        app.inbox_list = crate::ui_table::ListState {
            selected: 2,
            top: 1,
        };
        app.dispatch(Action::View(View::Repository));
        assert_eq!(app.inbox_list, Default::default());
        app.repository_list.selected = 4;
        app.dispatch(Action::View(View::Actions));
        app.content_list.selected = 1;
        app.dispatch(Action::View(View::Search));
        // Search, Actions and Dashboard share one list; each keeps its own place.
        assert_eq!(app.content_list, Default::default());
        app.dispatch(Action::View(View::Inbox));
        assert_eq!(
            app.inbox_list,
            crate::ui_table::ListState {
                selected: 2,
                top: 1
            }
        );
        app.dispatch(Action::View(View::Repository));
        assert_eq!(app.repository_list.selected, 4);
        app.dispatch(Action::View(View::Actions));
        assert_eq!(app.content_list.selected, 1);
        // Choosing the view already shown still starts its list from the top.
        app.dispatch(Action::View(View::Actions));
        assert_eq!(app.content_list, Default::default());
    }
    #[test]
    fn app_constructor_v142_preserves_empty_private_lifecycle_and_normal_help_navigation() {
        let mut app = App::new();
        assert_eq!(app.view, View::Inbox);
        assert_eq!(app.color_mode, crate::theme::ColorMode::TrueColor);
        assert!(app.closed_conversation_draft_v20.is_none());
        assert!(app.form.is_none() && app.overlay.is_none());
        assert!(app.pr_native_diff_work_v1.is_none());
        assert!(app.scoped_actions_work_v1.is_none());
        assert!(app.personal_pr_entry_work_v13.is_none());
        assert!(app.loading.iter().all(|loading| !loading));
        app.color_mode = crate::theme::ColorMode::NoColor;
        app.dispatch(Action::Help);
        assert_eq!(app.overlay, Some(Overlay::Help));
        assert!(app.context_help.is_some());
        app.dispatch(Action::Close);
        assert!(app.overlay.is_none());
        assert_eq!(app.color_mode, crate::theme::ColorMode::NoColor);
        assert!(app.closed_conversation_draft_v20.is_none());
        assert!(app.form.is_none());
    }

    #[test]
    fn metrics_unavailable_content_enter_cannot_activate_an_obsolete_toolbar_target() {
        let mut app = App {
            view: View::Dashboard,
            metrics_content_focused: true,
            hits: vec![HitRegion {
                rect: Rect::new(0, 1, 8, 1),
                action: Action::View(View::Inbox),
            }],
            focus: 0,
            ..Default::default()
        };
        app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        assert_eq!(app.view, View::Dashboard);
        assert!(!app.metrics_load_requested && !app.refresh_requested);
        assert_eq!(app.metrics_action, None);
        // Deliberate Tab focus remains a complete route to the visible views.
        app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Tab,
            KeyModifiers::NONE,
        )));
        app.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        assert_eq!(app.view, View::Inbox);
    }
    #[test]
    fn metrics_native_back_and_refresh_have_shared_keyboard_mouse_semantics_without_scope_expansion(
    ) {
        let make = || App {
            view: View::Inbox,
            github_origin: Some(View::Dashboard),
            metrics_selected_repository: Some("org/private".into()),
            metrics_days: 30,
            ..Default::default()
        };
        let mut keyboard = make();
        keyboard.dispatch(Action::Github(crate::github_workspace::GithubAction::Back));
        let mut mouse = make();
        mouse.hits.push(HitRegion {
            rect: Rect::new(5, 5, 10, 1),
            action: Action::Github(crate::github_workspace::GithubAction::Back),
        });
        mouse.handle_event(Event::Mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 6,
            row: 5,
            modifiers: KeyModifiers::NONE,
        }));
        for app in [&mut keyboard, &mut mouse] {
            assert_eq!(app.view, View::Dashboard);
            assert!(app.metrics_load_requested);
            assert_eq!(app.github_origin, None);
            assert_eq!(
                app.metrics_selected_repository.as_deref(),
                Some("org/private")
            );
            assert_eq!(app.metrics_days, 30);
            app.dispatch(Action::Refresh);
            assert!(app.refresh_requested);
            assert_eq!(
                app.metrics_selected_repository.as_deref(),
                Some("org/private")
            );
        }
    }
    use crossterm::event::{KeyEvent, MouseEvent};
    #[test]
    fn local_workspace_repository_navigation_requests_missing_state_without_duplicate_loading() {
        let mut keyboard = App::default();
        keyboard.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('2'),
            KeyModifiers::NONE,
        )));
        assert!(keyboard.repository_load_requested);
        assert_eq!(keyboard.view, View::Repository);

        let mut mouse = App::default();
        mouse.hits.push(HitRegion {
            rect: Rect::new(0, 0, 12, 1),
            action: Action::View(View::Repository),
        });
        mouse.handle_event(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }));
        assert!(mouse.repository_load_requested);
        assert_eq!(mouse.view, keyboard.view);
        assert_eq!(mouse.status, keyboard.status);

        mouse.repository_load_requested = false;
        mouse.loading[View::Repository.index()] = true;
        mouse.dispatch(Action::View(View::Repository));
        assert!(
            !mouse.repository_load_requested,
            "Do not cancel an existing discovery with a duplicate request"
        );
        mouse.loading[View::Repository.index()] = false;
        mouse.dispatch(Action::View(View::Repository));
        assert!(
            mouse.repository_load_requested,
            "An empty or failed discovery can be retried through navigation"
        );
    }

    #[test]
    fn local_workspace_unborn_compare_explains_missing_saved_versions_without_preparing_a_form() {
        let directory = tempfile::tempdir().unwrap();
        let initialized = std::process::Command::new("git")
            .args(["init", "--initial-branch=main"])
            .arg(directory.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        assert!(initialized.status.success());
        let workspace = crate::workspace::Workspace::load(
            directory.path(),
            crate::workspace::RepositoryMode::Overview,
            "HEAD",
            None,
        )
        .unwrap()
        .unwrap();
        assert!(workspace.status.head.is_none());
        let mut app = App {
            workspace: Some(workspace),
            ..App::default()
        };
        app.dispatch(Action::Compare);
        assert_eq!(app.overlay, Some(Overlay::Notice));
        assert!(app.notice.contains("no saved commits yet"));
        assert!(app.notice.contains("Working and Staged"));
        assert!(!app.compare_requested);
        assert!(app.form.is_none());
        assert!(!directory.path().join(".git/index").exists());
    }

    #[test]
    fn actor_identity_avatar_context_follows_active_verified_surface() {
        let identity = crate::providers::gh::HostIdentity {
            host: "github.invalid".into(),
            login: "alice".into(),
            account_id: Some(42),
            gh_version: "offline".into(),
        };
        let github = crate::github_workspace::GithubWorkspace::empty(
            identity.clone(),
            vec!["example/repo".into()],
            Default::default(),
        );
        let actions = crate::actions_workspace::ActionsWorkspace::new(
            identity.clone(),
            vec!["example/repo".into()],
        );
        let mut search = crate::search::workspace::SearchWorkspace::new(
            crate::storage::Scope::github(&identity.host, identity.account_id).unwrap(),
            vec![],
        );
        search.mode = crate::search::SearchMode::GitHub;
        search.remote_identity = Some(identity.clone());
        search.remote_repositories = vec!["example/repo".into()];
        let contexts = [github.context(), actions.context(), search.context()];
        let mut app = App {
            github: Some(github),
            actions: Some(actions),
            search: Some(search),
            ..Default::default()
        };
        for (view, context) in [View::Inbox, View::Actions, View::Search]
            .into_iter()
            .zip(contexts)
        {
            app.view = view;
            let (current, actual, actors) = app.avatar_source().unwrap();
            assert_eq!(current, identity);
            assert_eq!(actual, context);
            assert!(actors.is_empty());
        }
        app.github = None;
        app.actions = None;
        app.search = None;
        assert!(app.avatar_source().is_none());
    }

    #[test]
    fn actions_footer_key_click_and_watch_input_are_immediate_and_do_not_queue_workers() {
        use crate::actions_workspace::{ActionsAction, ActionsWorkspace};
        let identity = crate::providers::gh::HostIdentity {
            host: "github.invalid".into(),
            login: "octocat".into(),
            account_id: Some(42),
            gh_version: "offline".into(),
        };
        let mut workspace = ActionsWorkspace::new(identity, vec!["example/repo".into()]);
        workspace.prepare_rows();
        let mut app = App {
            view: View::Actions,
            actions: Some(workspace),
            ..Default::default()
        };
        // The retired control-page keys are inert; nothing pages or queues.
        for key in ['[', ']'] {
            app.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char(key),
                KeyModifiers::NONE,
            )));
        }
        assert!(app.actions_action.is_none() && app.overlay.is_none());
        // `w` resolves through the key bar to the painted Watch token's action; clicking
        // that token's target stops it again. Neither queues a worker.
        let watch = crate::actions_ui::footer_keys(app.actions.as_ref().unwrap())
            .into_iter()
            .find(|control| control.key == "w")
            .expect("Runs paints w Watch in the key bar");
        assert_eq!(watch.action, Action::Actions(ActionsAction::Watch));
        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('w'),
            KeyModifiers::NONE,
        )));
        assert!(app.actions.as_ref().unwrap().watching);
        app.hits.push(HitRegion {
            rect: Rect::new(10, 10, 20, 1),
            action: watch.action,
        });
        app.handle_event(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 12,
            row: 10,
            modifiers: KeyModifiers::NONE,
        }));
        assert!(!app.actions.as_ref().unwrap().watching);
        app.dispatch(Action::Actions(ActionsAction::Watch));
        assert!(app.actions.as_ref().unwrap().watching);
        assert!(
            app.actions_action.is_none(),
            "Pure navigation must not race worker snapshots"
        );
    }
    #[test]
    fn mouse_keyboard_unavailable_notice_is_read_only_scrollable_and_captures_input() {
        let mut app = App::default();
        app.dispatch(Action::Actions(
            crate::actions_workspace::ActionsAction::Unavailable(
                (0..40)
                    .map(|index| format!("Reason {index}: exact target is unavailable.\n"))
                    .collect(),
            ),
        ));
        assert_eq!(app.overlay, Some(Overlay::Notice));
        assert!(app.actions_action.is_none());
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|frame| crate::ui::draw(frame, &mut app))
            .unwrap();
        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::PageDown,
            KeyModifiers::NONE,
        )));
        assert_eq!(app.notice_scroll, 8);
        let hit = app
            .hits
            .iter()
            .find(|hit| hit.action == Action::NoticeScroll(8))
            .unwrap()
            .clone();
        app.handle_event(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: hit.rect.x,
            row: hit.rect.y,
            modifiers: KeyModifiers::NONE,
        }));
        assert_eq!(app.notice_scroll, 16);
        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('4'),
            KeyModifiers::NONE,
        )));
        assert_eq!(
            app.view,
            View::Inbox,
            "Notice must not activate an underlying view"
        );
        app.handle_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        assert_eq!(app.overlay, None);
    }

    #[test]
    fn bootstrap_shell_transitions_and_overlay_capture() {
        let mut app = App::default();
        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('2'),
            KeyModifiers::NONE,
        )));
        assert_eq!(app.view, View::Repository);
        app.dispatch(Action::Help);
        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('4'),
            KeyModifiers::NONE,
        )));
        assert_eq!(app.view, View::Repository);
        app.handle_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        assert_eq!(app.overlay, None);
    }

    #[test]
    fn bootstrap_mouse_and_keyboard_use_same_action() {
        let mut mouse = App::default();
        mouse.hits.push(HitRegion {
            rect: Rect::new(0, 0, 10, 1),
            action: Action::View(View::Actions),
        });
        mouse.handle_event(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 3,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }));
        let mut keyboard = App::default();
        keyboard.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('4'),
            KeyModifiers::NONE,
        )));
        assert_eq!(mouse.view, keyboard.view);
        assert_eq!(mouse.status, keyboard.status);
    }

    #[test]
    fn bootstrap_terminal_controls_are_sanitized() {
        assert_eq!(safe_text("hello\u{1b}[31m\u{7}world"), "helloworld");
    }

    #[test]
    fn mouse_keyboard_settings_support_unicode_caret_and_overlay_capture() {
        let mut app = App::default();
        app.dispatch(Action::Settings);
        app.settings_key(KeyCode::Char('u'), KeyModifiers::CONTROL);
        app.settings_insert("a日本b");
        app.settings_key(KeyCode::Left, KeyModifiers::NONE);
        app.settings_key(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(app.settings_host, "a日b");
        assert_eq!(app.settings_cursor, 2);
        app.settings_insert("x");
        assert_eq!(app.settings_host, "a日xb");
        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('4'),
            KeyModifiers::NONE,
        )));
        assert_eq!(
            app.view,
            View::Inbox,
            "typing into a field must not activate an underlying tab"
        );
        assert!(app.settings_host.contains('4'));
        app.settings_key(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(app.overlay, None);
    }
}
