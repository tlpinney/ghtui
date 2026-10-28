use anyhow::{bail, Result};
use crossterm::event;
use ghtui::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::operations_v3::ui_v3 as review_operation_ui_v3;
use ghtui::scoped_pr_observed_tabs_v1::resources::threads_v1::comments_v2::publication::ui as comment_ui_v2;
use ghtui::scoped_pr_observed_tabs_v1::resources::threads_v1::comments_v2::reactions_v3::form as reaction_ui_v3;
use ghtui::scoped_pr_observed_tabs_v1::resources::threads_v1::file_comments_v75::form as file_comment_ui_v75;
use ghtui::scoped_pr_observed_tabs_v1::resources::threads_v1::publication_v1::ui_v1 as thread_ui;
use ghtui::{app::App, ui};
use ghtui::{
    cli::{Command as CliCommand, Options},
    config::{Config, Paths, ThemeName},
    scoped_pr_observed_tabs_v1::resources::immutable_images::inline_v1::{
        native_v1 as inline_native, ui_v1 as inline_ui,
    },
    scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::{
        native_v2 as reviews_native, ui_v2 as reviews_ui,
    },
};
use ghtui::{
    providers::{
        gh::GithubProvider,
        runner::{CommandRunner, CommandSpec},
    },
    runtime::{Background, PreparedView},
    terminal::{restore_terminal, TerminalGuard},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::{self, IsTerminal},
    time::{Duration, Instant},
};

fn run() -> Result<()> {
    let options = Options::parse(std::env::args().skip(1)).map_err(anyhow::Error::msg)?;
    match options.command {
        CliCommand::Version => {
            println!("ghtui {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        CliCommand::Help => {
            println!("ghtui — a Git and GitHub terminal workspace\n\nUsage: ghtui [--config-dir PATH] [--host HOST] [--repo OWNER/NAME] [--no-color]\n       ghtui --help | --version\n\nRequires: git, gh (GitHub authentication is managed by gh)\nPlatforms: macOS, Linux\n\nKeys: 1-5 views, ? help, : commands, r refresh, q quit\nEvery visible control also accepts mouse clicks.");
            return Ok(());
        }
        CliCommand::Run => {}
    }
    let paths = Paths::discover(options.config_dir)?;
    let mut config = Config::load(&paths.config)?;
    if let Some(host) = options.host {
        config.host = host;
    }
    config.validate()?;
    let mut catalogue_local_mappings = std::sync::Arc::new(
        ghtui::repository_catalogue_workspace_v1::CatalogueLocalMappingsV1::from_config(&config),
    );
    if let Some(repository) = &options.repository {
        ghtui::config::validate_repository(repository)?;
    }
    // Repository views default to the launch checkout's GitHub remote. Unlike
    // --repo this never narrows the personal Inbox or steals local focus.
    let detected_repository = options
        .repository
        .is_none()
        .then(|| {
            std::env::current_dir()
                .ok()
                .and_then(|directory| ghtui::origin_repository::detect(&directory, &config.host))
        })
        .flatten();
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!("ghtui needs an interactive terminal; use --help for usage");
    }
    let previous_hook = std::panic::take_hook();
    let terminal_owner = std::thread::current().id();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().id() == terminal_owner {
            restore_terminal();
            previous_hook(info);
        }
    }));
    let mut guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new();
    app.color_mode = ghtui::theme::ColorMode::detect();
    app.original_tree_viewport_v2.width = terminal.size()?.width;
    app.no_color |= options.no_color || config.theme == ThemeName::NoColor;
    app.context = format!(
        "{} / {} / no revision selected",
        config.host,
        options
            .repository
            .as_deref()
            .or(config.active_set.as_deref())
            .unwrap_or("personal queues")
    );
    app.settings_host = config.host.clone();
    app.settings_set = config.active_set.clone().unwrap_or_default();
    app.personal_repository_scope_v105 =
        personal_repository_scope_v105(&config, options.repository.as_ref());
    app.settings_theme = config.theme.index();
    app.density = config.density;
    app.notifications_watch_seconds_v63 = config.refresh_seconds;
    app.set_bindings(config.resolved_bindings()?);
    let mut background = Background::new();
    let runner = CommandRunner::default();
    let mut draft_writer = ghtui::drafts::DraftWriter::new(paths.database.clone());
    let mut avatar_cache: Option<(String, ghtui::identity::AvatarCache)> = None;
    let mut review_avatar_lease_v7 = None;
    let mut catalogue_avatar_active = false;
    let mut next_refresh = Instant::now() + Duration::from_secs(config.refresh_seconds);
    let mut next_actions_refresh = Instant::now() + Duration::from_secs(5);
    let local_directory = std::env::current_dir()?;
    // PR files open File history only in their own repository's checkout (TASK-99.3).
    app.local_github_repository_v993 = detected_repository
        .clone()
        .or_else(|| ghtui::origin_repository::detect(&local_directory, &config.host));
    {
        // --repo scopes GitHub; it must never hide the checkout we are running in.
        // Keep explicit GitHub launch focus, but prepare local browsing independently.
        let activate_local = options.repository.is_none();
        let directory = local_directory.clone();
        background.submit(&mut app, ghtui::app::View::Repository, move |cancel| {
            if cancel.is_cancelled() { return Err("Local discovery cancelled".into()); }
            // A stale core.worktree shows its choices instead of an error (TASK-118).
            let (workspace, core_worktree_v118) = ghtui::repository_worktrees_v118::load(&directory, ghtui::workspace::RepositoryMode::Overview, "HEAD", None, cancel, false)?;
            let activate = activate_local && workspace.is_some();
            let context = workspace.as_ref().map(|workspace| format!("Local {} / {} / {}", workspace.directory.display(), workspace.status.branch.as_deref().unwrap_or("detached HEAD"), workspace.revision));
            let status = if core_worktree_v118.is_some() { "This checkout's core.worktree points at another folder • choose what to do in Repository" } else { "Local Git workspace ready • working files preserved" };
            Ok(PreparedView { lines: vec!["No local Git repository detected. Run ghtui inside a checkout to browse local work.".into()], context, status: status.into(), workspace, core_worktree_v118, github:None, form:None, activate_if_untouched: activate, ..PreparedView::default() })
        });
    }
    if config.load_inbox_on_start() && app.view == ghtui::app::View::Inbox {
        // Load the personal queues at once; Refresh stays available for later reloads.
        app.content[ghtui::app::View::Inbox.index()][2] =
            "Loading your authenticated GitHub inbox…".into();
        app.refresh_requested = true;
    }
    let mut render_timings = ghtui::render_timings_v66::RenderTimings::from_env();
    while !app.quit {
        // Purpose-closed State Refresh capture/refusal precedes retirement.
        // A refused new enqueue cannot cancel the old accepted source worker.
        ghtui::scoped_pr_collaboration_native_v17::prepare_state_source_sync_v80(
            &mut app,
            &paths.database,
        );
        // Retire the actual original actor job before ANY replacement Repo
        // request is queued. Do not use global cancel/app.view from a modal.
        if app.local_actor_cancel_requested {
            app.local_actor_cancel_requested = false;
            background.cancel(ghtui::app::View::Repository);
        }
        // TASK-105: an explicit Refresh after a confirmed scoped review write
        // re-admits ONLY the same host + numeric account + repository scope,
        // reading the current principal before any Personal queue query.
        if let ghtui::personal_readmission_v105::ReadmissionV105::Submit(request) =
            ghtui::personal_readmission_v105::take_refresh(
                &mut app,
                &personal_repository_scope_v105(&config, options.repository.as_ref()),
            )
        {
            let runner = runner.clone();
            let database = paths.database.clone();
            let exclusions = config.excluded_repositories.iter().cloned().collect();
            next_refresh = Instant::now()
                + Duration::from_secs(config.refresh_seconds + (std::process::id() as u64 % 3));
            background.submit(&mut app, ghtui::app::View::Inbox, move |cancel| {
                let provider = GithubProvider::new(request.host())
                    .map_err(|error| error.to_string())?
                    .with_runner(runner);
                let current = provider
                    .identity(&cancel)
                    .map_err(|error| error.to_string())?;
                if let Err(reason) = request.verify(&current) {
                    return Ok(PreparedView {
                        status: reason.clone(),
                        personal_readmission_v105: Some(request.refused(reason)),
                        ..PreparedView::default()
                    });
                }
                let workspace = ghtui::github_workspace::GithubWorkspace::legacy_startup_v80(
                    &provider,
                    request.repositories.clone(),
                    exclusions,
                    &database,
                    &cancel,
                )
                .map_err(|error| error.to_string())?;
                // The startup read observed the principal again: re-check it
                // before any private page can be installed.
                if let Err(reason) = request.verify(&workspace.identity) {
                    return Ok(PreparedView {
                        status: reason.clone(),
                        personal_readmission_v105: Some(request.refused(reason)),
                        ..PreparedView::default()
                    });
                }
                let pending = Some(ghtui::pr_ui::pending_message(&database, &workspace)?);
                Ok(PreparedView {
                    context: Some(workspace.context()),
                    status: workspace.freshness.label(),
                    pr_pending_message: pending,
                    github: Some(workspace),
                    personal_readmission_v105: Some(request.admitted()),
                    ..PreparedView::default()
                })
            });
        }
        ghtui::remote_scope_gate_v1::enforce_stopped_intent(&mut app, &mut background);
        if app.form.as_ref().is_some_and(|form| {
            form.view() != ghtui::app::View::Repository
                && !matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::PrInline(_)
                        | ghtui::forms::FormPurpose::PrThread(_)
                        | ghtui::forms::FormPurpose::PrThreadCommentV2(_)
                        | ghtui::forms::FormPurpose::PrThreadReactionV3(_)
                        | ghtui::forms::FormPurpose::PrFileCommentV75(_)
                        | ghtui::forms::FormPurpose::PrReviewV2(_)
                        | ghtui::forms::FormPurpose::PrReviewOperationV3(_)
                        | ghtui::forms::FormPurpose::ScopedActionsV3(_)
                        | ghtui::forms::FormPurpose::PrCollaborationV17(_)
                        | ghtui::forms::FormPurpose::ScopedActionsWorkflowV6(_)
                        | ghtui::forms::FormPurpose::ScopedActionsDispatchV7(_)
                        | ghtui::forms::FormPurpose::ScopedWorkflowEditV9(_)
                )
        }) {
            ghtui::remote_scope_gate_v1::refuse_legacy_request(
                &mut app,
                &mut background,
                ghtui::remote_scope_gate_v1::RemoteConsumerV1::RemoteForm,
            );
        }
        if let Some(action) = app.pr_action.clone() {
            if ghtui::scoped_pr_collaboration_native_v17::capture_action(
                &mut app,
                &action,
                &paths.database,
            ) {
                app.pr_action = None;
            }
        }
        if app.pr_action.is_some()
            || (app.pr_checks_request
                && !app
                    .pr_checks_origin
                    .as_ref()
                    .is_some_and(|origin| origin.scoped_v1()))
        {
            ghtui::remote_scope_gate_v1::refuse_legacy_request(
                &mut app,
                &mut background,
                ghtui::remote_scope_gate_v1::RemoteConsumerV1::MyPrs,
            );
        }
        if app.settings_save_requested {
            app.settings_save_requested = false;
            app.local_commit_actors.retire_private();
            app.local_actor_requested = false;
            app.local_actor_layout.clear();
            app.avatar_states.clear();
            background.cancel(ghtui::app::View::Repository);
            let mut updated = config.clone();
            updated.host = app.settings_host.clone();
            updated.active_set = if app.settings_set.is_empty() {
                None
            } else {
                Some(app.settings_set.clone())
            };
            updated.theme = ThemeName::from_index(app.settings_theme);
            updated.density = app.density;
            match updated.save(&paths.config) {
                Ok(()) => {
                    if updated.host != config.host || updated.active_set != config.active_set {
                        for view in ghtui::app::View::ALL {
                            background.cancel(view);
                            app.loading[view.index()] = false;
                        }
                        app.content[ghtui::app::View::Inbox.index()] = vec![
                            "Account host changed • Refresh to load this host's personal queues."
                                .into(),
                        ];
                        app.github = None;
                        app.pr_inspection = None;
                        app.pr_pending_message.clear();
                        app.pr_action = None;
                        app.form = None;
                        app.github_action = None;
                        app.actions = None;
                        app.metrics = None;
                        app.metrics_action = None;
                        app.metrics_selected_repository = None;
                        app.metrics_days = 7;
                        app.metrics_load_requested = app.view == ghtui::app::View::Dashboard;
                        app.metrics_content = Default::default();
                        app.github_origin = None;
                        app.actions_action = None;
                        app.actions_load_requested = app.view == ghtui::app::View::Actions;
                        background.cancel(ghtui::app::View::Search);
                        app.search = None;
                        app.search_action = None;
                        app.search_controls = Default::default();
                        app.search_load_requested = app.view == ghtui::app::View::Search;
                        app.avatar_states.clear();
                        app.notice.clear();
                        app.notice_scroll = 0;
                        if let Some((_, cache)) = avatar_cache.take() {
                            cache.shutdown();
                        }
                    }
                    config = updated;
                    app.personal_repository_scope_v105 =
                        personal_repository_scope_v105(&config, options.repository.as_ref());
                    catalogue_local_mappings = std::sync::Arc::new(
                        ghtui::repository_catalogue_workspace_v1::CatalogueLocalMappingsV1::from_config(&config),
                    );
                    next_refresh = Instant::now() + Duration::from_secs(config.refresh_seconds);
                    app.notifications_watch_seconds_v63 = config.refresh_seconds;
                    app.set_bindings(config.resolved_bindings()?);
                    app.no_color = options.no_color
                        || std::env::var_os("NO_COLOR").is_some()
                        || config.theme == ThemeName::NoColor;
                    app.context = format!(
                        "{} / {} / no revision selected",
                        config.host,
                        config.active_set.as_deref().unwrap_or("personal queues")
                    );
                    app.overlay = None;
                    app.status = "Settings saved locally".into();
                }
                Err(error) => app.status = format!("Settings error: {error:#}"),
            }
        }
        if app.notifications_cancel_v63 {
            app.notifications_cancel_v63 = false;
            background.cancel_notifications_v63(&mut app);
        }
        background.retire_stale_notifications_v63(&mut app);
        background.tick_notifications_v63(&mut app);
        ghtui::personal_pr_entry_v13::retire_invalid(&mut app);
        ghtui::notification_pr_v63::retire_invalid(&mut app);
        if app.personal_pr_entry_cancel_v13 {
            app.personal_pr_entry_cancel_v13 = false;
            background.cancel_personal_pr_entry_v13(&mut app);
        }
        background.retire_stale_personal_pr_entry_v13(&mut app);
        ghtui::scoped_pr_observed_tabs_v1::resources::threads_v1::ui_v1::retire_invalid(&mut app);
        if app.scoped_threads_cancel_v1 {
            app.scoped_threads_cancel_v1 = false;
            background.cancel_scoped_threads_v1(&mut app);
        }
        background.retire_stale_scoped_threads_v1(&mut app);
        if app.scoped_reviews_cancel_v2 {
            app.scoped_reviews_cancel_v2 = false;
            background.cancel_scoped_reviews_v2(&mut app);
        }
        background.retire_stale_scoped_reviews_v2(&mut app);
        reviews_ui::retire_invalid_form(&mut app);
        review_operation_ui_v3::retire_invalid_form(&mut app);
        reviews_native::retire_invalid(&mut app);
        if app.scoped_inline_cancel_v1 {
            app.scoped_inline_cancel_v1 = false;
            background.cancel_scoped_inline_v1(&mut app);
        }
        if app.pr_native_diff_cancel_v1 {
            app.pr_native_diff_cancel_v1 = false;
            background.cancel_pr_native_diff_v1(&mut app);
        }
        background.retire_stale_pr_native_diff_v1(&mut app);
        background.retire_stale_scoped_inline_v1(&mut app);
        if app.scoped_pr_collaboration_cancel_v17 {
            app.scoped_pr_collaboration_cancel_v17 = false;
            background.cancel_scoped_pr_collaboration_v17(&mut app);
        }
        background.retire_stale_scoped_pr_collaboration_v17(&mut app);
        ghtui::scoped_actions_native_v1::synchronize(&mut app);
        if app.scoped_actions_cancel_v1 {
            app.scoped_actions_cancel_v1 = false;
            background.cancel_scoped_actions_v1(&mut app);
        }
        background.retire_stale_scoped_actions_v1(&mut app);
        ghtui::scoped_actions_publication_ui_v3::retire_invalid_form(&mut app);
        background.poll(&mut app);
        app.observe_navigation();
        inline_ui::retire_invalid_form(&mut app);
        thread_ui::retire_invalid_form(&mut app);
        comment_ui_v2::retire_invalid_form(&mut app);
        reaction_ui_v3::retire_invalid_form(&mut app);
        file_comment_ui_v75::retire_invalid_form(&mut app);
        reviews_ui::retire_invalid_form(&mut app);
        reviews_native::retire_invalid(&mut app);
        if let Some(form) = &mut app.form {
            if form.dirty
                && !form.submitting
                && (!matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::PrInline(_)
                        | ghtui::forms::FormPurpose::PrReviewV2(_)
                        | ghtui::forms::FormPurpose::PrReviewOperationV3(_)
                        | ghtui::forms::FormPurpose::PrThreadReactionV3(_)
                        | ghtui::forms::FormPurpose::ScopedActionsV3(_)
                        | ghtui::forms::FormPurpose::PrFileCommentV75(_)
                        | ghtui::forms::FormPurpose::PrCollaborationV17(_)
                        | ghtui::forms::FormPurpose::ScopedActionsWorkflowV6(_)
                        | ghtui::forms::FormPurpose::ScopedActionsDispatchV7(_)
                        | ghtui::forms::FormPurpose::ScopedWorkflowEditV9(_)
                ) || !app.form_dismissed && app.overlay == Some(ghtui::app::Overlay::Form))
            {
                form.prepare_draft_autosave_v77();
                let encoded = if matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::ScopedWorkflowEditV9(_)
                ) {
                    form.workflow_edit_draft_v9()
                } else if matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::PrCollaborationV17(_)
                ) {
                    ghtui::scoped_pr_collaboration_ui_v17::encoded_draft(form)
                } else if matches!(form.purpose, ghtui::forms::FormPurpose::PrInline(_)) {
                    inline_ui::encoded_draft(form)
                } else if matches!(form.purpose, ghtui::forms::FormPurpose::PrFileCommentV75(_)) {
                    file_comment_ui_v75::encoded_draft(form)
                } else if matches!(form.purpose, ghtui::forms::FormPurpose::PrThread(_)) {
                    thread_ui::encoded_draft(form)
                } else if matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::PrThreadCommentV2(_)
                ) {
                    comment_ui_v2::encoded_draft(form)
                } else if matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::PrThreadReactionV3(_)
                ) {
                    reaction_ui_v3::encoded_draft(form)
                } else if matches!(form.purpose, ghtui::forms::FormPurpose::PrReviewV2(_)) {
                    reviews_ui::encoded_draft(form)
                } else if matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::PrReviewOperationV3(_)
                ) {
                    review_operation_ui_v3::encoded_draft(form)
                } else {
                    Ok(form.draft())
                };
                match encoded.and_then(|encoded| {
                    draft_writer
                        .save(form.scope(), form.resource(), encoded)
                        .map_err(|error| error.to_string())
                }) {
                    Ok(()) => form.settle_draft_autosave_v77(true),
                    Err(error) => {
                        form.settle_draft_autosave_v77(false);
                        form.error = format!("Draft error: {error}");
                    }
                }
            }
        }
        if let Some(work) = app.notifications_work_v63.take() {
            match GithubProvider::new(work.host()) {
                Ok(provider) => {
                    background.submit_notifications_v63(
                        &mut app,
                        work,
                        provider.with_runner(runner.clone()),
                        paths.database.clone(),
                    );
                }
                Err(_) => {
                    app.status = "Notification provider unavailable; nothing queued".into();
                }
            }
        }
        if let Some(work) = app.pr_native_diff_work_v1.take() {
            match GithubProvider::new(work.host()) {
                Ok(provider) => {
                    background.submit_pr_native_diff_v1(
                        &mut app,
                        work,
                        provider.with_runner(runner.clone()),
                        paths.database.clone(),
                    );
                }
                Err(_) => {
                    ghtui::scoped_pr_native_diff_native_v1::submission_failed(&mut app, &work);
                }
            }
        }
        inline_native::capture_form(&mut app);
        ghtui::scoped_pr_observed_tabs_v1::resources::threads_v1::native_v1::capture_personal_history_requested_v75(&mut app, &paths.database);
        ghtui::scoped_pr_observed_tabs_v1::resources::threads_v1::native_v1::capture_form_with_drafts(
            &mut app,
            &paths.database,
            &draft_writer.client(),
        );
        if let Some(owner) = app.scoped_threads_work_v1.take() {
            match GithubProvider::new(&owner.intent.scope().host) {
                Ok(provider) => {
                    background.submit_scoped_threads_with_drafts_v1(
                        &mut app,
                        owner,
                        provider.with_runner(runner.clone()),
                        paths.database.clone(),
                        draft_writer.client(),
                    );
                }
                Err(_) => {
                    if let Some(view) = &mut app.scoped_threads_v1 {
                        view.message =
                            "Thread provider unavailable • Back returns to original Files";
                    }
                }
            }
        }
        reviews_native::capture_requested_history_v15(&mut app, &paths.database);
        reviews_native::capture_form(&mut app);
        if let Some(work) = app.scoped_reviews_work_v2.take() {
            match GithubProvider::new(&work.scope().host) {
                Ok(provider) => {
                    background.submit_scoped_reviews_v2(
                        &mut app,
                        work,
                        provider.with_runner(runner.clone()),
                        paths.database.clone(),
                        draft_writer.client(),
                    );
                }
                Err(_) => {
                    app.status = "Original review provider unavailable • nothing queued".into()
                }
            }
        }
        if let Some(work) = app.scoped_inline_work_v1.take() {
            match GithubProvider::new(&work.intent().scope().host) {
                Ok(provider) => {
                    background.submit_scoped_inline_v1(
                        &mut app,
                        work,
                        provider.with_runner(runner.clone()),
                        paths.database.clone(),
                        draft_writer.client(),
                    );
                }
                Err(_) => {
                    app.status = "Original inline provider unavailable • nothing submitted".into();
                }
            }
        }
        ghtui::scoped_pr_collaboration_native_v17::prepare_requested(&mut app);
        if let Some(work) = app.scoped_pr_collaboration_work_v17.take() {
            match GithubProvider::new(work.host()) {
                Ok(provider) => {
                    if !background.submit_scoped_pr_collaboration_v17(
                        &mut app,
                        work.clone(),
                        provider.with_runner(runner.clone()),
                        paths.database.clone(),
                        draft_writer.client(),
                    ) {
                        ghtui::scoped_pr_collaboration_native_v17::submission_failed(
                            &mut app, &work,
                        );
                    }
                }
                Err(_) => {
                    ghtui::scoped_pr_collaboration_native_v17::submission_failed(&mut app, &work)
                }
            }
        }
        if app
            .form
            .as_ref()
            .is_some_and(|form| form.picker_requested.is_some())
        {
            let index = app.form.as_mut().unwrap().picker_requested.take().unwrap();
            let mut form = app.form.as_ref().unwrap().clone();
            app.form.as_mut().unwrap().submitting = true;
            let previous = app.github.clone();
            let runner = runner.clone();
            let view = form.view();
            background.submit(&mut app, view, move |cancel| {
                if let ghtui::forms::FormPurpose::PrMetadata(intent) = &form.purpose {
                    let provider = GithubProvider::new(&intent.identity.host)
                        .map_err(|error| error.to_string())?
                        .with_runner(runner);
                    return ghtui::pr_metadata_native::load_choices(
                        form,
                        previous.as_ref(),
                        &provider,
                        &cancel,
                    );
                }
                if let ghtui::forms::FormPurpose::Pr(intent) = &form.purpose {
                    let provider = GithubProvider::new(&intent.identity.host)
                        .map_err(|error| error.to_string())?
                        .with_runner(runner);
                    return ghtui::pr_metadata_base::prepare_picker(
                        form,
                        &previous.ok_or("Original PR page is no longer active")?,
                        &provider,
                        &cancel,
                    );
                }
                let field = form
                    .fields
                    .get(index)
                    .ok_or("Picker field is unavailable")?;
                form.picker = Some(match &form.purpose {
                    ghtui::forms::FormPurpose::Local(intent) => {
                        let inventory = ghtui::local_actions::LocalInventory::load(
                            &intent.context.directory,
                            cancel,
                        )?;
                        ghtui::local_picker::LocalPicker::new(
                            intent.command,
                            &field.key,
                            &inventory,
                        )
                    }
                    ghtui::forms::FormPurpose::Compare(comparison) => {
                        let inventory = ghtui::local_actions::LocalInventory::load(
                            &comparison.intent.directory,
                            cancel,
                        )?;
                        ghtui::comparison::picker(index, &inventory)?
                    }
                    _ => return Err("This form does not use a local Git inventory".into()),
                });
                form.submitting = false;
                Ok(PreparedView {
                    form: Some(form),
                    status: "Local inventory ready • browsing changes nothing".into(),
                    ..PreparedView::default()
                })
            });
        }
        if app.form.as_ref().is_some_and(|form| {
            form.preview_requested
                && !matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::PrInline(_)
                        | ghtui::forms::FormPurpose::PrThread(_)
                        | ghtui::forms::FormPurpose::PrThreadCommentV2(_)
                        | ghtui::forms::FormPurpose::PrThreadReactionV3(_)
                        | ghtui::forms::FormPurpose::PrFileCommentV75(_)
                        | ghtui::forms::FormPurpose::PrReviewV2(_)
                        | ghtui::forms::FormPurpose::PrReviewOperationV3(_)
                        | ghtui::forms::FormPurpose::ScopedActionsV3(_)
                        | ghtui::forms::FormPurpose::PrCollaborationV17(_)
                        | ghtui::forms::FormPurpose::ScopedActionsWorkflowV6(_)
                        | ghtui::forms::FormPurpose::ScopedActionsDispatchV7(_)
                        | ghtui::forms::FormPurpose::ScopedWorkflowEditV9(_)
                )
        }) {
            // Capture one original clock and accepted autosave cutoff BEFORE
            // queueing. The worker waits for acknowledgements only: it never
            // resends pending bodies or renews private-cache preparation.
            let corpus_fence = if let Some(form) = app
                .form
                .as_ref()
                .filter(|form| matches!(form.purpose, ghtui::forms::FormPurpose::Corpus(_)))
            {
                let deadline = Instant::now() + Duration::from_secs(30);
                match draft_writer.capture_corpus_preview_v4(
                    form.scope(),
                    form.resource(),
                    form.draft(),
                    &paths.database,
                    deadline,
                ) {
                    Ok(fence) => Some(fence),
                    Err(error) => {
                        let form = app.form.as_mut().expect("original corpus form");
                        form.preview_requested = false;
                        form.error = error;
                        continue;
                    }
                }
            } else {
                None
            };
            let mut form = app.form.as_ref().unwrap().clone();
            app.form.as_mut().unwrap().preview_requested = false;
            app.form.as_mut().unwrap().submitting = true;
            form.preview_requested = false;
            let values = form.values();
            let view = form.view();
            let runner = runner.clone();
            let database = paths.database.clone();
            let draft_client = draft_writer.client();
            let waiting_corpus = corpus_fence.is_some();
            let pr_create_host = config.host.clone();
            background.submit(&mut app, view, move |cancel| {
                if let ghtui::forms::FormPurpose::PrMetadata(intent)=&form.purpose {
                    if form.publication.is_some() {
                        let provider=GithubProvider::new(&intent.identity.host).map_err(|error|error.to_string())?.with_runner(runner);
                        return ghtui::pr_metadata_native::original(form,&provider,&database,&cancel);
                    }
                    return ghtui::pr_metadata_native::preview_form(form);
                }
                if form.metadata_original() {
                    let ghtui::forms::FormPurpose::Pr(intent)=&form.purpose else{unreachable!()};
                    let provider=GithubProvider::new(&intent.identity.host).map_err(|error|error.to_string())?.with_runner(runner);
                    return ghtui::pr_metadata_base::prepare_original(form,&provider,&database,&cancel);
                }
                if let ghtui::forms::FormPurpose::Actions(intent)=&form.purpose {
                    if form.actions_publication.is_some() {
                        let provider=GithubProvider::new(&intent.workspace.identity.host).map_err(|error|error.to_string())?.with_runner(runner);
                        return ghtui::actions_routes::preview_original(form,&provider,&database,&cancel);
                    }
                }
                let compare_intent =
                    if matches!(form.purpose, ghtui::forms::FormPurpose::Compare(_)) {
                        Some(form.compare_intent()?)
                    } else {
                        None
                    };
                match &mut form.purpose {
                    ghtui::forms::FormPurpose::PrCollaborationV17(_) => return Err("Original Conversation preview requires its SAME purpose-specific worker; no legacy fallback".into()),
                    ghtui::forms::FormPurpose::PrThread(_) => return Err("Original thread preview requires its purpose-specific worker; no legacy fallback".into()),
                    ghtui::forms::FormPurpose::PrThreadCommentV2(_) => return Err("Original selected-comment preview requires its purpose-specific worker • no legacy fallback".into()),
                    ghtui::forms::FormPurpose::PrFileCommentV75(_) => return Err("Original FILE preview requires its same Thread worker • no legacy fallback".into()),
                    ghtui::forms::FormPurpose::PrThreadReactionV3(_) => return Err("Original reaction preview requires its purpose-specific worker • no legacy fallback".into()),
                    ghtui::forms::FormPurpose::PrInline(_) => return Err("Original scoped inline previews require their purpose-specific worker; no legacy fallback".into()),
                    ghtui::forms::FormPurpose::PrReviewV2(_)=>return Err("Original scoped review previews require their purpose-specific worker; no legacy fallback".into()),
                    ghtui::forms::FormPurpose::PrReviewOperationV3(_)=>return Err("Original pending operation requires its same Review worker; no legacy fallback".into()),
                    ghtui::forms::FormPurpose::ScopedActionsV3(_) | ghtui::forms::FormPurpose::ScopedActionsWorkflowV6(_) | ghtui::forms::FormPurpose::ScopedActionsDispatchV7(_) | ghtui::forms::FormPurpose::ScopedWorkflowEditV9(_)=>return Err("Original Actions requires its SAME purpose-specific Actions worker; no legacy fallback".into()),
                    ghtui::forms::FormPurpose::PrMetadata(_) => unreachable!("metadata worker routed above"),
                    ghtui::forms::FormPurpose::Corpus(intent) => {
                        let fence = corpus_fence.as_ref().ok_or("Original repository preview draft cutoff unavailable")?;
                        draft_client.wait_corpus_preview_v4(fence, &cancel)?;
                        fence.check_current(&cancel)?;
                        let provider = GithubProvider::new(&intent.identity.host).map_err(|error|error.to_string())?.with_runner(runner)
                            .with_scoped_read_window(fence.deadline(), 1024 * 1024).map_err(|error|error.to_string())?;
                        let cache = ghtui::storage::Cache::open(&database).map_err(|error|error.to_string())?;
                        fence.check_current(&cancel)?;
                        match ghtui::corpus_ui::preview(intent,&values,&cache,&provider,&cancel) {
                            Ok(()) => {},
                            Err(error) if ghtui::pr_ui::account_failure(&error) => return Ok(PreparedView {clear_private:true,status:format!("Repository account verification failed • private selections hidden: {error}"),..Default::default()}),
                            Err(error) => return Err(error),
                        }
                    }
                    ghtui::forms::FormPurpose::Compare(comparison) => {
                        comparison.intent = compare_intent.unwrap();
                        comparison.plan = Some(ghtui::comparison::ComparisonPlan::prepare(
                            &comparison.intent,
                            cancel.clone(),
                        )?);
                    }
                    ghtui::forms::FormPurpose::Local(intent) if intent.command == ghtui::local_actions::LocalCommand::CreatePullRequest => {
                        let provider = GithubProvider::new(&pr_create_host)
                            .map_err(|error| error.to_string())?
                            .with_runner(runner);
                        intent.pr_create_v1 = Some(ghtui::pr_create_v1::PrCreatePlanV1::prepare(
                            &intent.context,
                            &values,
                            &provider,
                            &cancel,
                        )?);
                    }
                    ghtui::forms::FormPurpose::Local(intent) => {
                        intent.plan = Some(ghtui::local_actions::LocalPlan::prepare_selected(
                            intent.command,
                            &intent.context,
                            &values,
                            &intent.selections,
                            cancel.clone(),
                        )?);
                        // A ready preview retires an earlier "preview first" refusal.
                        form.error.clear();
                    }
                    ghtui::forms::FormPurpose::Actions(intent) => {
                        let provider = GithubProvider::new(intent.workspace.identity.host.clone())
                            .map_err(|error| error.to_string())?
                            .with_runner(runner);
                        let prepared = ghtui::actions_workspace::ActionsPlan::prepare(
                                &intent.workspace,
                                intent.command,
                                &values,
                                &provider,
                                &cancel,
                            );
                        match prepared {
                            Ok(plan) => intent.plan = Some(plan),
                            Err(error) if matches!(error.kind,ghtui::providers::gh::GithubErrorKind::IdentityChanged | ghtui::providers::gh::GithubErrorKind::Authentication) => return Ok(PreparedView {clear_private:true,status:format!("Actions identity unavailable • private views hidden: {error}"),..Default::default()}),
                            Err(error) => return Err(error.to_string()),
                        }
                    }
                    ghtui::forms::FormPurpose::PrAction(intent) => {
                        let provider = GithubProvider::new(&intent.context.identity.host)
                            .map_err(|error| error.to_string())?
                            .with_runner(runner);
                        let prepared = ghtui::pr_actions::PrPlan::prepare(
                            &provider,
                            &intent.context,
                            intent.command,
                            &values,
                            &cancel,
                        );
                        match prepared {
                            Ok(plan) => intent.plan = Some(plan),
                            Err(error) if ghtui::pr_ui::account_failure(&error) => return Ok(PreparedView {clear_private:true,status:format!("PR account verification failed • private pages hidden: {error}"),..Default::default()}),
                            Err(error) => return Err(error),
                        }
                    }
                    ghtui::forms::FormPurpose::Pr(intent) => {
                        if cancel.is_cancelled() {
                            return Err("Metadata preview cancelled; nothing published".into());
                        }
                        form.metadata_preview = Some(ghtui::pr_metadata_preview::PrMetadataPreview::prepare_selected(intent, &values, form.base_selection.as_ref())?);
                        form.error.clear();
                    }
                }
                form.preview = true;
                form.preview_scroll = 0;
                form.submitting = false;
                if let Some(fence) = &corpus_fence {
                    // Original Corpus ACK/source clock, not the independent
                    // legacy presentation bound in Background::submit.
                    form.prepare_preview_display_v1(&cancel,fence.deadline()).map_err(|error|error.to_string())?;
                    fence.check_current(&cancel)?;
                }
                Ok(PreparedView {
                    lines: vec![],
                    context: None,
                    status: if matches!(form.purpose, ghtui::forms::FormPurpose::Compare(_)) {
                        "Read-only comparison preview ready • no Git write or confirmation required"
                            .into()
                    } else {
                        "Read-only preview ready • nothing changed until confirmation".into()
                    },
                    workspace: None,
                    github: None,
                    form: Some(form),
                    activate_if_untouched: false,
                    ..PreparedView::default()
                })
            });
            if waiting_corpus {
                app.status =
                    "Waiting for accepted local draft saves • Preview has not queried GitHub"
                        .into();
            }
        }
        for ack in draft_writer.poll() {
            if let Some(form) = &mut app.form {
                if form.resource() == ack.resource && form.scope() == ack.scope {
                    if let Err(error) = ack.result {
                        if !form.workflow_edit_draft_error_v9() {
                            form.error = format!("Draft error: {error}");
                        }
                    } else if app.overlay == Some(ghtui::app::Overlay::Form)
                        && !app.loading[app.view.index()]
                        // A late local draft ack must not hide the visible preview status,
                        // nor a failed confirmation's outcome (e.g. "Pull request not created").
                        && !((form.preview || !form.error.is_empty())
                            && matches!(form.purpose, ghtui::forms::FormPurpose::Local(_)))
                    {
                        app.status = if form.actions_publication.is_some() {
                            "Original Actions reference saved locally • inspect its recorded state; nothing resent".into()
                        } else {
                            "Draft saved locally • nothing published".into()
                        };
                    }
                }
            }
        }
        if app.form.as_ref().is_some_and(|form| form.editor_requested) {
            if app.form.as_ref().is_some_and(|form| {
                matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::PrInline(_)
                        | ghtui::forms::FormPurpose::PrThread(_)
                        | ghtui::forms::FormPurpose::PrThreadCommentV2(_)
                        | ghtui::forms::FormPurpose::PrThreadReactionV3(_)
                        | ghtui::forms::FormPurpose::PrFileCommentV75(_)
                        | ghtui::forms::FormPurpose::PrCollaborationV17(_)
                )
            }) {
                let form = app.form.as_mut().expect("checked inline form");
                form.editor_requested = false;
                let is_conversation = matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::PrCollaborationV17(_)
                );
                let is_thread = matches!(form.purpose, ghtui::forms::FormPurpose::PrThread(_));
                let is_file =
                    matches!(form.purpose, ghtui::forms::FormPurpose::PrFileCommentV75(_));
                let is_comment = matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::PrThreadCommentV2(_)
                );
                let admitted = if is_conversation {
                    ghtui::scoped_pr_collaboration_ui_v17::validate_input(form, "")
                } else if matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::PrThreadReactionV3(_)
                ) {
                    Err("Reactions have no editable body or external editor".into())
                } else if is_file {
                    file_comment_ui_v75::validate_input(form, "")
                } else if is_comment {
                    comment_ui_v2::validate_input(form, "")
                } else if is_thread {
                    thread_ui::validate_input(form, "")
                } else {
                    inline_ui::validate_input(form, "")
                };
                match admitted {
                    Err(error) => form.error = error,
                    Ok(()) => {
                        let edited = external_editor_bounded(
                            &mut guard,
                            &form.fields[0].editor.text,
                            inline_ui::INLINE_BODY_BYTES_V1,
                        );
                        terminal.resize(terminal.size()?.into())?;
                        match edited {
                            Ok(text) => {
                                let replaced = if is_file {
                                    file_comment_ui_v75::replace_editor_result(form, &text)
                                } else if is_conversation {
                                    ghtui::scoped_pr_collaboration_ui_v17::replace_editor_result(
                                        form, &text,
                                    )
                                } else if is_comment {
                                    comment_ui_v2::replace_editor_result(form, &text)
                                } else if is_thread {
                                    thread_ui::replace_editor_result(form, &text)
                                } else {
                                    inline_ui::replace_editor_result(form, &text)
                                };
                                if let Err(error) = replaced {
                                    form.error = error;
                                }
                            }
                            Err(error) => {
                                form.error = format!("Editor error: {error:#} • draft retained")
                            }
                        }
                    }
                }
            } else if app.form.as_ref().is_some_and(|form| {
                matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::PrReviewV2(_)
                        | ghtui::forms::FormPurpose::PrReviewOperationV3(_)
                        | ghtui::forms::FormPurpose::ScopedActionsV3(_)
                        | ghtui::forms::FormPurpose::ScopedActionsWorkflowV6(_)
                        | ghtui::forms::FormPurpose::ScopedActionsDispatchV7(_)
                        | ghtui::forms::FormPurpose::ScopedWorkflowEditV9(_)
                )
            }) {
                let form = app.form.as_mut().expect("checked original review form");
                form.editor_requested = false;
                if let Some(field) = form.fields.get(form.active_field) {
                    let edited = external_editor_bounded(&mut guard, &field.editor.text, 65_536);
                    terminal.resize(terminal.size()?.into())?;
                    match edited {
                        Ok(text) => {
                            let replaced = if matches!(
                                form.purpose,
                                ghtui::forms::FormPurpose::PrReviewOperationV3(_)
                            ) {
                                review_operation_ui_v3::replace_editor_result(form, &text)
                            } else {
                                reviews_ui::replace_editor_result(form, &text)
                            };
                            if let Err(error) = replaced {
                                form.error = error;
                            }
                        }
                        Err(error) => {
                            form.error =
                                format!("Editor error: {error:#} • original review text retained")
                        }
                    }
                }
            } else {
                let form = app.form.as_mut().unwrap();
                form.editor_requested = false;
                if let Some(field) = form.fields.get_mut(form.active_field) {
                    let before = field.editor.text.clone();
                    let field_key = field.key.clone();
                    let edited = external_editor(&mut guard, &field.editor.text);
                    // Fullscreen re-entry needs a complete repaint, not an external
                    // cursor-position query (which requires a terminal DSR reply).
                    terminal.resize(terminal.size()?.into())?;
                    match edited {
                        Ok(text) => {
                            field.editor =
                                ghtui::editor::TextBuffer::new(text).map_err(anyhow::Error::msg)?;
                            form.dirty = true;
                            form.field_edited(&field_key, &before);
                            form.invalidate_plan();
                            form.preview = false;
                        }
                        Err(error) => {
                            form.error = format!("Editor error: {error:#} • draft retained")
                        }
                    }
                }
            }
        }
        if app.form.as_ref().is_some_and(|form| {
            form.submit_requested
                && !matches!(
                    form.purpose,
                    ghtui::forms::FormPurpose::PrInline(_)
                        | ghtui::forms::FormPurpose::PrThread(_)
                        | ghtui::forms::FormPurpose::PrThreadCommentV2(_)
                        | ghtui::forms::FormPurpose::PrThreadReactionV3(_)
                        | ghtui::forms::FormPurpose::PrFileCommentV75(_)
                        | ghtui::forms::FormPurpose::PrReviewV2(_)
                        | ghtui::forms::FormPurpose::PrReviewOperationV3(_)
                        | ghtui::forms::FormPurpose::ScopedActionsV3(_)
                        | ghtui::forms::FormPurpose::PrCollaborationV17(_)
                        | ghtui::forms::FormPurpose::ScopedActionsWorkflowV6(_)
                        | ghtui::forms::FormPurpose::ScopedActionsDispatchV7(_)
                        | ghtui::forms::FormPurpose::ScopedWorkflowEditV9(_)
                )
        }) {
            let mut form = app.form.as_mut().unwrap().clone();
            app.form.as_mut().unwrap().submit_requested = false;
            app.form.as_mut().unwrap().submitting = true;
            form.submit_requested = false;
            form.submitting = false;
            let previous = app.github.clone();
            let previous_search = app.search.clone();
            if matches!(
                form.purpose,
                ghtui::forms::FormPurpose::Pr(_)
                    | ghtui::forms::FormPurpose::PrAction(_)
                    | ghtui::forms::FormPurpose::PrMetadata(_)
            ) {
                background.cancel(ghtui::app::View::Search);
                app.loading[ghtui::app::View::Search.index()] = false;
                if let Some(search) = &mut app.search {
                    search.page = None;
                    search.preview = None;
                    search.remote_items.clear();
                    search.blend_evidence = None;
                }
                app.source_context = None;
                if matches!(form.purpose, ghtui::forms::FormPurpose::PrMetadata(_)) {
                    background.cancel(ghtui::app::View::Dashboard);
                    app.loading[ghtui::app::View::Dashboard.index()] = false;
                    app.metrics = None;
                    app.metrics_content = Default::default();
                    app.content[ghtui::app::View::Dashboard.index()]=vec!["PR metadata publication pending • old Dashboard values hidden; Refresh verifies its sources.".into()];
                }
            }
            let runner = runner.clone();
            let draft_client = draft_writer.client();
            let pending_drafts = draft_writer.take_pending();
            let view = form.view();
            let database = paths.database.clone();
            background.submit(&mut app, view, move |cancel| {
                use ghtui::forms::FormPurpose;
                // Structured metadata authenticates BEFORE any private draft or
                // original SQL. Do not copy the separately tracked legacy gap.
                if let FormPurpose::PrMetadata(intent) = &form.purpose {
                    let provider = GithubProvider::new(&intent.identity.host)
                        .map_err(|error| error.to_string())?
                        .with_runner(runner);
                    return ghtui::pr_metadata_native::submit(
                        form,
                        previous.ok_or("Exact PR page is no longer active")?,
                        &provider,
                        &database,
                        &draft_client,
                        pending_drafts,
                        &cancel,
                    );
                }
                draft_client.save_pending(pending_drafts)?;
                if form.metadata_original() {
                    let FormPurpose::Pr(intent) = &form.purpose else {
                        unreachable!()
                    };
                    let provider = GithubProvider::new(&intent.identity.host)
                        .map_err(|error| error.to_string())?
                        .with_runner(runner);
                    return ghtui::pr_metadata_base::prepare_original(
                        form, &provider, &database, &cancel,
                    );
                }
                if let FormPurpose::Corpus(intent) = &form.purpose {
                    let provider = GithubProvider::new(&intent.identity.host)
                        .map_err(|error| error.to_string())?
                        .with_runner(runner);
                    let search = previous_search.ok_or("Search workspace is no longer active")?;
                    return ghtui::corpus_routes::submit(
                        form,
                        search,
                        &provider,
                        &database,
                        &draft_client,
                        &cancel,
                    );
                }
                if let FormPurpose::PrAction(intent) = &form.purpose {
                    let provider = GithubProvider::new(&intent.context.identity.host)
                        .map_err(|error| error.to_string())?
                        .with_runner(runner);
                    return ghtui::pr_ui::submit_native(
                        form,
                        &provider,
                        &database,
                        &draft_client,
                        &cancel,
                    );
                }
                if matches!(form.purpose, FormPurpose::Compare(_)) {
                    let intent = form.compare_intent()?;
                    // Recheck pinned refs even if a previous read-only Preview exists.
                    let plan = ghtui::comparison::ComparisonPlan::prepare(&intent, cancel)?;
                    let workspace = plan.open();
                    form.submitted = true;
                    form.dirty = false;
                    let draft_result = draft_client.remove(form.scope(), form.resource());
                    return Ok(PreparedView {
                        context: Some(format!(
                            "Local {} / comparison / {}",
                            workspace.directory.display(),
                            workspace.revision
                        )),
                        status: format!(
                            "Comparison opened • read-only{}",
                            draft_result.err().map_or(String::new(), |error| format!(
                                " • draft cleanup failed: {error}"
                            ))
                        ),
                        workspace: Some(workspace),
                        form: Some(form),
                        navigate_to: Some(ghtui::app::View::Repository),
                        ..PreparedView::default()
                    });
                }
                if let FormPurpose::Actions(intent) = &form.purpose {
                    let provider = GithubProvider::new(intent.workspace.identity.host.clone())
                        .map_err(|error| error.to_string())?
                        .with_runner(runner);
                    return ghtui::actions_routes::submit(
                        form,
                        &provider,
                        &database,
                        &draft_client,
                        &cancel,
                    );
                }
                let pr_create = match &form.purpose {
                    FormPurpose::Local(intent) => intent.pr_create_v1.clone(),
                    _ => None,
                };
                if let Some(plan) = pr_create {
                    let provider = GithubProvider::new(&plan.host)
                        .map_err(|error| error.to_string())?
                        .with_runner(runner);
                    match plan.submit(&provider, &cancel) {
                        Ok(created) => {
                            let draft_result = draft_client.remove(form.scope(), form.resource());
                            form.submitted = true;
                            form.dirty = false;
                            return Ok(PreparedView {
                                status: format!(
                                    "Created pull request {}#{}{} • {}{}{}",
                                    created.repository,
                                    created.number,
                                    if created.draft { " (draft)" } else { "" },
                                    created.url,
                                    created
                                        .reviewers_note
                                        .map_or(String::new(), |note| format!(" • {note}")),
                                    draft_result.err().map_or(String::new(), |error| format!(
                                        " • draft cleanup failed: {error}"
                                    ))
                                ),
                                refresh_authored_queue: true,
                                form: Some(form),
                                ..PreparedView::default()
                            });
                        }
                        Err(error) => {
                            if let FormPurpose::Local(intent) = &mut form.purpose {
                                intent.pr_create_v1 = None;
                            }
                            form.preview = false;
                            form.error = error;
                            return Ok(PreparedView {
                                status: "Pull request not created • draft kept locally".into(),
                                form: Some(form),
                                ..PreparedView::default()
                            });
                        }
                    }
                }
                if let FormPurpose::Local(intent) = &mut form.purpose {
                    let plan = intent
                        .plan
                        .as_ref()
                        .ok_or("Choose Preview before confirmation")?;
                    let outcome = if matches!(intent.command, ghtui::local_actions::LocalCommand::StashApply | ghtui::local_actions::LocalCommand::StashPop) {
                        let Some(origin) = intent.original_stash_source_v1.clone() else {
                            intent.plan=None;form.preview=false;form.submit_requested=false;form.submitting=false;
                            form.error="Original stash source unavailable; old confirmation retired. Open a fresh original local intent.".into();
                            return Ok(PreparedView{form:Some(form),status:"Stash source unavailable • zero execution submitted".into(),..Default::default()});
                        };
                        match plan.execute_stash_with_inspection_v1(true, cancel, origin) {
                            Ok(outcome) => outcome,
                            Err(failure) => {
                                let status=match failure.failed_intent() {
                                    Some(intent) if intent.command_succeeded()=>"Stash succeeded • refresh failed • F3 Inspect result",
                                    Some(_)=>"Stash failed/refused • old confirmation retired • F3 Inspect result",
                                    None=>"Stash refused before witness capture • old confirmation retired • no result inspection",
                                };
                                intent.failed_stash_v1 = failure.failed_intent().cloned();
                                intent.plan = None;
                                form.preview = false;
                                form.submit_requested = false;
                                form.submitting = false;
                                form.error = failure.message;
                                return Ok(PreparedView { form: Some(form), status: status.into(), ..Default::default() });
                            }
                        }
                    } else { plan.execute(true, cancel)? };
                    let status = format!(
                        "Completed • {}{}{}",
                        intent.command.label(),
                        ghtui::local_actions::completion_target(intent.command, &intent.context),
                        if outcome.output.is_empty() {
                            String::new()
                        } else {
                            format!(" • {}", outcome.output)
                        }
                    );
                    let draft_result = draft_client.remove(form.scope(), form.resource());
                    form.submitted = true;
                    form.dirty = false;
                    return Ok(PreparedView {
                        lines: vec![],
                        context: Some(format!(
                            "Local {} / {} / {}",
                            outcome.workspace.directory.display(),
                            outcome
                                .workspace
                                .status
                                .branch
                                .as_deref()
                                .unwrap_or("detached HEAD"),
                            outcome.workspace.revision
                        )),
                        status: if let Err(error) = draft_result {
                            format!("{status} • draft cleanup failed: {error}")
                        } else {
                            status
                        },
                        workspace: Some(outcome.workspace),
                        github: None,
                        form: Some(form),
                        activate_if_untouched: false,
                        ..PreparedView::default()
                    });
                }
                let FormPurpose::Pr(intent) = &form.purpose else {
                    unreachable!()
                };
                form.validate_metadata_preview()?;
                let intent = intent.as_ref().clone();
                let provider = GithubProvider::new(&intent.identity.host)
                    .map_err(|error| error.to_string())?
                    .with_runner(runner);
                if let Err(prepared) =
                    ghtui::pr_metadata_base::revalidate(&form, &provider, &cancel)
                {
                    return Ok(*prepared);
                }
                let workspace = previous.ok_or("The PR page is no longer active")?;
                let restored = form
                    .restored_draft
                    .then(|| form.publication.clone())
                    .flatten();
                let mut journal = ghtui::pr_write_journal::PrWriteJournal::open(&database)
                    .map_err(|error| error.to_string())?;
                let result = ghtui::pr_legacy::execute_legacy(
                    &intent,
                    &workspace,
                    &form.values(),
                    &provider,
                    &mut journal,
                    true,
                    restored.as_ref(),
                    |marker| {
                        form.publication = Some(marker.publication_reference());
                        draft_client.save_and_wait(form.scope(), form.resource(), form.draft())
                    },
                    &cancel,
                );
                ghtui::pr_ui::present(
                    result,
                    form,
                    workspace,
                    intent.kind.label(),
                    &draft_client,
                    &database,
                )
            });
        }
        let review_avatar_owned = ghtui::scoped_pr_observed_tabs_v1::resources::immutable_images::reviews_v2::avatars_v7::poll_review(
            &mut app,&mut avatar_cache,&paths.avatars,config.avatars,&mut review_avatar_lease_v7,
        );
        let related_avatar_owned = !review_avatar_owned
            && ghtui::repository_related_pr_v1::avatars_v1::poll_related(
                &mut app,
                &mut avatar_cache,
                &paths.avatars,
                config.avatars,
            );
        let catalogue_avatar_owned = !review_avatar_owned
            && !related_avatar_owned
            && ghtui::repository_catalogue_avatars_v1::poll_catalogue(
                &mut app,
                &mut avatar_cache,
                &paths.avatars,
                config.avatars,
                &mut catalogue_avatar_active,
            );
        let remote_avatar_owned = !review_avatar_owned
            && !related_avatar_owned
            && !catalogue_avatar_owned
            && ghtui::remote_repository_native_v1::sync_avatars(
                &mut app,
                &mut avatar_cache,
                &paths.avatars,
                config.avatars,
            );
        let actor_color_mode = app.theme().mode;
        if app.graph_reread_dialog_v3.is_none()
            && app.original_tree_jump_v1.is_none()
            && app.original_diff_picker_v1.is_none()
            && app.native_diff_v1.is_none()
            && app.native_mutable_diff_v2.is_none()
            && !remote_avatar_owned
            && !catalogue_avatar_owned
            && !related_avatar_owned
            && !review_avatar_owned
            && !ghtui::repository_commit_actor_avatars_v1::poll_repository(
                &app.local_commit_actors,
                app.view == ghtui::app::View::Repository && app.remote_repository.is_none(),
                &mut app.avatar_states,
                &mut avatar_cache,
                &paths.avatars,
                config.avatars,
                actor_color_mode,
            )
        {
            ghtui::actor_avatar_states_v1::sync_active_surface(
                &mut app,
                &mut avatar_cache,
                &paths.avatars,
                config.avatars,
            );
        }
        if app.view == ghtui::app::View::Inbox
            && app.overlay.is_none()
            && app.remote_scope.is_none()
            && !app.loading[0]
            && !ghtui::scoped_pr_collaboration_native_v17::state_source_requires_explicit_refresh_v80(&app)
            && (app
                .github
                .as_ref()
                .is_some_and(|github| github.needs_refresh)
                || (Instant::now() >= next_refresh
                    && app
                        .github
                        .as_ref()
                        .is_some_and(|github| github.pr.is_none())))
        {
            app.refresh_requested = true;
        }
        if app.refresh_requested && app.view == ghtui::app::View::Repository {
            app.refresh_requested = false;
            app.repository_load_requested = true;
            app.repository_reconcile_requested =
                !ghtui::repository_graph_reread_native_v3::refresh_available(&app);
        }
        if app.compare_requested {
            app.compare_requested = false;
            if let Some(workspace) = &app.workspace {
                let intent = ghtui::comparison::CompareIntent::from_workspace(workspace);
                let database = paths.database.clone();
                background.submit(&mut app, ghtui::app::View::Repository, move |_cancel| {
                    let mut form = ghtui::forms::Form::compare(intent)?;
                    if let Some(draft) = ghtui::storage::Cache::open(&database)
                        .map_err(|error| error.to_string())?
                        .draft(&form.scope(), &form.resource())
                        .map_err(|error| error.to_string())?
                    {
                        form.restore_draft(&draft)?;
                    }
                    Ok(PreparedView {
                        form: Some(form),
                        status: "Choose A and B • snapshot or proposed changes • read-only".into(),
                        ..PreparedView::default()
                    })
                });
            }
        }
        if let Some(command) = app.local_command.take() {
            let quick_preview = std::mem::take(&mut app.local_command_preview);
            if let Some(context) = app.local_context() {
                let database = paths.database.clone();
                let original_stash_source_v1 = app
                    .workspace
                    .as_ref()
                    .and_then(|workspace| workspace.actor_origin.clone());
                background.submit(&mut app, ghtui::app::View::Repository, move |cancel| {
                    let mut form = ghtui::forms::Form::local(command, context)?;
                    if let Some(draft) = ghtui::storage::Cache::open(&database)
                        .map_err(|error| error.to_string())?
                        .draft(&form.scope(), &form.resource())
                        .map_err(|error| error.to_string())?
                    {
                        form.restore_draft(&draft)?;
                    }
                    if let ghtui::forms::FormPurpose::Local(intent) = &mut form.purpose {
                        intent.original_stash_source_v1 = original_stash_source_v1;
                    }
                    if quick_preview {
                        ghtui::local_actions::preview_form(&mut form, cancel)?;
                    }
                    Ok(PreparedView {
                        lines: vec![],
                        context: None,
                        status: if quick_preview {
                            "Preview ready • Ctrl-S confirm • Esc keep and close; nothing changed yet"
                                .into()
                        } else if matches!(
                            command,
                            ghtui::local_actions::LocalCommand::Commit
                                | ghtui::local_actions::LocalCommand::Amend
                        ) {
                            "Write the message • Ctrl-P previews the staged changes • then Ctrl-S commits"
                                .into()
                        } else {
                            "Local intent ready • Preview first; nothing changed".into()
                        },
                        workspace: None,
                        github: None,
                        form: Some(form),
                        activate_if_untouched: false,
                        ..PreparedView::default()
                    })
                });
            }
        }
        if let Some(restore) = app.navigation_restore.take() {
            app.navigation_restore_inflight = true;
            let visit = restore.original_local.clone();
            if !background.submit_original_local_v1(&mut app, visit, move |owner| {
                let workspace = ghtui::navigation_state::read_repository_with_operation_v1(
                    &restore,
                    owner.operation(),
                )?;
                Ok(PreparedView {
                    workspace: Some(workspace),
                    navigation_restore: Some(restore),
                    status: "Original source prepared • read-only".into(),
                    ..PreparedView::default()
                })
            }) {
                app.abandon_navigation_restore();
            }
        }
        if app.local_actor_requested {
            app.local_actor_requested = false;
            let runner = runner.clone();
            let database = paths.database.clone();
            background.submit_local_actor(&mut app, move |owner| {
                owner.guard()?; // Queued time/cancellation precedes any local setup or child.
                let provider =
                    GithubProvider::new(&owner.choice().original_source().source().identity().host)
                        .map_err(|error| error.to_string())?
                        .with_runner(runner);
                let cache =
                    ghtui::storage::Cache::open(&database).map_err(|error| error.to_string())?;
                let result = ghtui::repository_commit_actor_native_v1::verify_local_actor_v1(
                    &owner, &provider, &cache,
                );
                Ok(PreparedView {
                    local_actor_result: Some(result),
                    ..PreparedView::default()
                })
            });
        }
        if app.original_stash_recovery_cancel_requested_v1 {
            app.original_stash_recovery_cancel_requested_v1 = false;
            if background.cancel_current_stash_recovery_v1() {
                app.loading[ghtui::app::View::Repository.index()] = false;
            }
        }
        if let Some(intent) = app.original_stash_recovery_request_v1.take() {
            background.submit_stash_recovery_v1(&mut app, intent);
        }
        if app.related_pr_cancel_v1 {
            app.related_pr_cancel_v1 = false;
            if background.cancel_current_related_pr_v1() {
                app.loading[ghtui::app::View::Repository.index()] = false;
            }
        }
        if app.related_pr_requested_v1 {
            app.related_pr_requested_v1 = false;
            let runner = runner.clone();
            let database = paths.database.clone();
            background.submit_related_pr_v1(&mut app, move |owner| {
                owner.guard()?;
                let provider = GithubProvider::new(owner.host())
                    .map_err(|e| e.to_string())?
                    .with_runner(runner);
                ghtui::repository_related_pr_v1::read(&owner, &provider, &database).map(|result| {
                    PreparedView {
                        related_pr_v1: Some(result),
                        ..Default::default()
                    }
                })
            });
        }
        if app.original_diff_cancel_v1 {
            app.original_diff_cancel_v1 = false;
            if background.cancel_current_original_diff_v1() {
                app.loading[ghtui::app::View::Repository.index()] = false;
            }
        }
        if let Some(work) = app.original_diff_work_v1.take() {
            background.submit_original_diff_sides_v1(&mut app, work);
        }
        if app.original_tree_cancel_v1 {
            app.original_tree_cancel_v1 = false;
            if background.cancel_current_original_tree_v2() {
                app.loading[ghtui::app::View::Repository.index()] = false;
            }
        }
        if let Some(work) = app.original_tree_viewport_v2.cancel_metadata.take() {
            if background.cancel_original_tree_metadata_v2(&work) {
                app.loading[ghtui::app::View::Repository.index()] = false;
            }
        }
        if let Some(work) = app.original_tree_work_v1.take() {
            background.submit_original_tree_v1(&mut app, work);
        }
        if let Some(request) = ghtui::repository_graph_reread_native_v3::take_request(&mut app) {
            background.submit_original_local_v1(&mut app, None, move |owner| {
                ghtui::repository_graph_reread_native_v3::work(&request, &owner)
            });
        }
        if let Some(mismatch) = app.core_worktree_unset_v118.take() {
            // The previewed and confirmed unset of a stale core.worktree (TASK-118); the
            // write refuses when the setting changed since the preview.
            background.submit(&mut app, ghtui::app::View::Repository, move |cancel| {
                let (workspace, core_worktree_v118) =
                    ghtui::repository_worktrees_v118::unset_and_load(&mismatch, cancel)?;
                let context = workspace.as_ref().map(|workspace| {
                    format!(
                        "Local {} / {} / {}",
                        workspace.directory.display(),
                        workspace
                            .status
                            .branch
                            .as_deref()
                            .unwrap_or("detached HEAD"),
                        workspace.revision
                    )
                });
                Ok(PreparedView {
                    lines: vec!["No local repository selected.".into()],
                    context,
                    status: "core.worktree unset • Repository now reads this checkout".into(),
                    workspace,
                    core_worktree_v118,
                    ..PreparedView::default()
                })
            });
        }
        if app.repository_load_requested || app.repository_action.is_some() {
            app.repository_load_requested = false;
            // Opening another worktree (TASK-41) is a fresh read-only load of that directory.
            let opened_worktree = app.repository_directory_request_v41.take();
            let opened = opened_worktree.is_some();
            let previous = if opened_worktree.is_some() {
                None
            } else {
                app.workspace.clone()
            };
            let mode = app.repository_mode;
            let action = app.repository_action.take();
            let reconcile = app.repository_reconcile_requested;
            app.repository_reconcile_requested = false;
            let directory = opened_worktree.unwrap_or_else(|| {
                previous.as_ref().map_or_else(
                    || local_directory.clone(),
                    |workspace| workspace.directory.clone(),
                )
            });
            if !reconcile
                && previous
                    .as_ref()
                    .is_some_and(|workspace| workspace.actor_origin.is_some())
            {
                let snapshot_refresh = action.is_none()
                    && ghtui::repository_graph_reread_native_v3::refresh_available(&app)
                    && previous
                        .as_ref()
                        .is_some_and(|workspace| mode == workspace.mode);
                background.submit_original_local_v1(&mut app, None, move |owner| {
                    let mut workspace = previous.ok_or("Original local source is unavailable")?;
                    let status = if matches!(
                        action.as_ref(),
                        Some(ghtui::workspace::RowAction::Graph(
                            ghtui::repository_graph_v1::GraphActionV1::Window(
                                ghtui::repository_graph_v1::GraphWindowActionV2::First
                            )
                        ))
                    ) {
                        "First loaded graph projection ready • no Git reads"
                    } else if snapshot_refresh {
                        "Original Graph snapshot refreshed • loaded pages and exact position retained"
                    } else {
                        "Repository loaded • read-only exploration"
                    };
                    if let Some(action) = action {
                        workspace.open_original_with_operation_v1(action, owner.operation())?;
                    } else {
                        workspace = workspace.load_original_with_operation_v1(
                            mode,
                            &workspace.revision,
                            owner.operation(),
                        )?;
                    }
                    let context = Some(format!(
                        "Local {} / {} / {}",
                        workspace.directory.display(),
                        workspace
                            .status
                            .branch
                            .as_deref()
                            .unwrap_or("detached HEAD"),
                        workspace.revision
                    ));
                    Ok(PreparedView {
                        context,
                        status: status.into(),
                        workspace: Some(workspace),
                        ..PreparedView::default()
                    })
                });
            } else {
                background.submit(&mut app, ghtui::app::View::Repository, move |cancel| {
                    if cancel.is_cancelled() {
                        return Err("Repository loading cancelled".into());
                    }
                    if reconcile && previous.is_some() {
                        ghtui::local_actions::reconcile(&directory, cancel.clone())?;
                    }
                    let (workspace, core_worktree_v118) =
                        if let (Some(mut workspace), Some(action)) = (previous.clone(), action) {
                            workspace.open_with_cancellation(action, cancel)?;
                            (Some(workspace), None)
                        } else {
                            let revision = previous
                                .as_ref()
                                .map_or("HEAD", |workspace| workspace.revision.as_str())
                                .to_owned();
                            // A stale core.worktree shows its choices (TASK-118).
                            ghtui::repository_worktrees_v118::load(
                                &directory,
                                mode,
                                &revision,
                                previous.and_then(|workspace| workspace.tree),
                                cancel,
                                opened,
                            )?
                        };
                    let status = if core_worktree_v118.is_some() {
                        "This checkout's core.worktree points at another folder • nothing changed"
                    } else {
                        "Repository loaded • read-only exploration"
                    };
                    let context = workspace.as_ref().map(|workspace| {
                        format!(
                            "Local {} / {} / {}",
                            workspace.directory.display(),
                            workspace
                                .status
                                .branch
                                .as_deref()
                                .unwrap_or("detached HEAD"),
                            workspace.revision
                        )
                    });
                    Ok(PreparedView {
                        lines: vec!["No local repository selected.".into()],
                        context,
                        status: status.into(),
                        workspace,
                        core_worktree_v118,
                        github: None,
                        form: None,
                        activate_if_untouched: false,
                        ..PreparedView::default()
                    })
                });
            }
        }
        if app.cancel_requested {
            app.cancel_requested = false;
            if app.navigation_restore_inflight {
                background.cancel(ghtui::app::View::Repository);
                app.loading[ghtui::app::View::Repository.index()] = false;
                app.abandon_navigation_restore();
            }
            if let Some(form) = app.form.as_ref().filter(|form| form.submitting) {
                background.request_cancel(form.view());
                app.status = "Cancellation requested • waiting for cleanup; inspect the eventual result before retrying".into();
            } else {
                background.cancel(app.view);
                app.loading[app.view.index()] = false;
                app.status = "Cancelled • r Refresh to retry".into();
                if app.view == ghtui::app::View::Dashboard {
                    app.metrics = None;
                    app.metrics_action = None;
                    app.metrics_load_requested = false;
                    app.metrics_content = Default::default();
                    app.content[ghtui::app::View::Dashboard.index()] = vec!["Dashboard read cancelled • previous values hidden".into(),"Refresh retains the original selected repository and window intent; no mutation was sent.".into()];
                    app.context = "Dashboard • previous values hidden • read cancelled".into();
                }
            }
            if let Some(github) = &mut app.github {
                github.needs_refresh = false;
            }
        }
        if app.scoped_pr_tab_cancel_requested {
            app.scoped_pr_tab_cancel_requested = false;
            background.cancel(ghtui::app::View::Inbox);
            app.loading[ghtui::app::View::Inbox.index()] = false;
        }
        if app.pr_checks_cancel_actions {
            app.pr_checks_cancel_actions = false;
            background.cancel(ghtui::app::View::Actions);
            app.loading[ghtui::app::View::Actions.index()] = false;
            background.cancel(ghtui::app::View::Inbox); // Owned chosen read cannot reopen after Back.
            app.loading[ghtui::app::View::Inbox.index()] = false;
        }
        if app.catalogue_cancel_requested {
            app.catalogue_cancel_requested = false;
            background.cancel_catalogue();
        }
        if ghtui::remote_repository_native_v1::take_cancel(&mut app) {
            background.cancel_remote_repository_v1();
        }
        if let Some(work) = ghtui::remote_repository_native_v1::take_request(&mut app) {
            let owner = work.owner.clone();
            let host = owner.host().to_owned();
            let runner = runner.clone();
            let database = paths.database.clone();
            background.submit_remote_repository_v1(&mut app, owner, move |cancel| {
                let provider = GithubProvider::new(host)
                    .map_err(|_| "Remote browsing host unavailable".to_owned())?
                    .with_runner(runner);
                ghtui::remote_repository_native_v1::execute(work, &provider, &database, &cancel)
            });
        }
        if let Some(work) = ghtui::repository_catalogue_native::take_request(&mut app) {
            for view in ghtui::repository_catalogue_native::retire_for_work(&mut app, &work) {
                background.cancel(view);
            }
            let recovery = work.recovery_owner().cloned();
            let owner = work.owner().clone();
            let host = owner.host().to_owned();
            let runner = runner.clone();
            let database = paths.database.clone();
            let local_mappings = catalogue_local_mappings.clone();
            let task = move |cancel| {
                let provider = GithubProvider::new(host)
                    .map_err(|_| "Discovery host unavailable".to_owned())?
                    .with_runner(runner);
                ghtui::repository_catalogue_native::execute_with_local_mappings(
                    work,
                    &provider,
                    &database,
                    &cancel,
                    &local_mappings,
                )
            };
            if let Some(recovery) = recovery {
                background.submit_catalogue_recovery(&mut app, owner, recovery, task);
            } else {
                background.submit_catalogue(&mut app, owner, task);
            }
        }
        if app.pr_checks_request
            && app
                .pr_checks_origin
                .as_ref()
                .is_some_and(|origin| origin.scoped_v1())
        {
            let captured = ghtui::pr_checks_native::scoped::take_open_v1(&mut app);
            match captured {
                Ok(input) => {
                    let origin = input.origin().clone();
                    let host = origin.scoped_owner_v1().unwrap().0.scope().host.clone();
                    let runner = runner.clone();
                    let database = paths.database.clone();
                    let deadline = Instant::now() + Duration::from_secs(30);
                    background.submit_scoped_pr_check_v1(
                        &mut app,
                        ghtui::app::View::Inbox,
                        origin,
                        move |cancel| {
                            let remaining = deadline.saturating_duration_since(Instant::now());
                            if remaining.is_zero() || cancel.is_cancelled() {
                                return Err("Original scoped check request expired".into());
                            }
                            let provider = GithubProvider::new(host)
                                .map_err(|e| e.to_string())?
                                .with_runner(runner)
                                .with_scoped_read_window(deadline, 1024 * 1024)
                                .map_err(|e| e.to_string())?;
                            let mut limits =
                                ghtui::pr_checks_native::scoped::ScopedPrCheckReadLimitsV1::default(
                                );
                            limits.access.transport.max_duration = remaining;
                            limits.bridge.max_duration = remaining;
                            ghtui::pr_checks_native::scoped::load_v1(
                                input, &provider, &database, &limits, &cancel,
                            )
                        },
                    );
                }
                Err(_) => {
                    ghtui::pr_checks_native::retire(&mut app, true);
                    ghtui::repository_catalogue_native::privacy(&mut app);
                }
            }
        }
        if app.pr_checks_request {
            app.pr_checks_request = false;
            app.refresh_requested = false;
            if let Some(origin) = app.pr_checks_origin.clone() {
                let runner = runner.clone();
                let database = paths.database.clone();
                let host = config.host.clone();
                background.submit(&mut app, ghtui::app::View::Inbox, move |cancel| {
                    let provider = GithubProvider::new(host)
                        .map_err(|e| e.to_string())?
                        .with_runner(runner);
                    ghtui::pr_checks_native::load(origin, &provider, &database, &cancel)
                });
            }
        }
        if let Some(action) = app.pr_action.take() {
            app.refresh_requested = false;
            let context = app.pr_context();
            let inspection = app.pr_inspection.clone();
            let database = paths.database.clone();
            let runner = runner.clone();
            background.submit(&mut app, ghtui::app::View::Inbox, move |cancel| {
                let context = context?;
                let provider = GithubProvider::new(&context.identity.host)
                    .map_err(|error| error.to_string())?
                    .with_runner(runner);
                ghtui::pr_ui::load(action, context, inspection, &provider, &database, &cancel)
            });
        }
        // STOP before any queued Open/Tab worker, authentication or legacy
        // fallback. This predicate never matches pure cached Back.
        if matches!(
            app.github_action.as_ref(),
            Some(
                ghtui::github_workspace::GithubAction::Open(_)
                    | ghtui::github_workspace::GithubAction::PrTab(_)
                    | ghtui::github_workspace::GithubAction::Resource(_)
            )
        ) {
            if let Some(reason) =
                ghtui::remote_scope_gate_v1::stop_before_fallback(app.remote_scope.as_deref())
            {
                app.github_action = None;
                app.refresh_requested = false;
                ghtui::remote_scope_gate_v1::retire_remote(&mut app, &mut background, reason);
                app.status = reason.label().into();
            }
        }
        if matches!(
            app.github_action.as_ref(),
            Some(ghtui::github_workspace::GithubAction::PrTab(
                ghtui::github_workspace::PrTab::Metadata
            ))
        ) {
            app.github_action.take();
            app.refresh_requested = false;
            ghtui::scoped_pr_collaboration_native_v17::capture_metadata_section_v51(
                &mut app,
                &paths.database,
            );
        }
        // Selected initial PR Open only. Source/scope owners remain unchanged;
        // cached inbox navigation is captured privately, not shown while reading.
        if app.remote_scope.is_some()
            && matches!(
                app.github_action.as_ref(),
                Some(ghtui::github_workspace::GithubAction::Open(_))
            )
        {
            let Some(ghtui::github_workspace::GithubAction::Open(selected)) =
                app.github_action.take()
            else {
                unreachable!()
            };
            app.refresh_requested = false;
            if let (Some(active), Some(original), Some(previous)) = (
                app.remote_scope.clone(),
                app.remote_scope_source.clone(),
                app.github.take(),
            ) {
                let runner = runner.clone();
                let database = paths.database.clone();
                let host = original.scope().host.clone();
                for view in [
                    ghtui::app::View::Inbox,
                    ghtui::app::View::Actions,
                    ghtui::app::View::Dashboard,
                ] {
                    background.cancel(view);
                }
                if !ghtui::remote_scope_gate_v1::independent_local_search(app.search.as_ref()) {
                    background.cancel(ghtui::app::View::Search);
                }
                ghtui::repository_catalogue_native::retire_remote_views(&mut app);
                background.submit_scoped_remote(
                    &mut app,
                    ghtui::app::View::Inbox,
                    active.clone(),
                    original.clone(),
                    move |cancel| {
                        let provider =
                            match GithubProvider::new(host) {
                                Ok(provider) => provider.with_runner(runner),
                                Err(_) => return Ok(PreparedView {
                                    clear_private: true,
                                    status:
                                        "Scoped PR host unavailable • previous private pages hidden"
                                            .into(),
                                    ..PreparedView::default()
                                }),
                            };
                        match ghtui::scoped_pr_read_v1::open(
                            original, active, previous, &selected, &provider, &database, &cancel,
                        ) {
                            Ok(result) => {
                                let context = result.workspace.context();
                                Ok(PreparedView {
                                    remote_scope: Some(result.original_active),
                                    remote_scope_source: Some(result.original_source),
                                    verified_identity: Some(result.closed_identity),
                                    github: Some(result.workspace),
                                    context: Some(context),
                                    status: "Selected PR • current read • cached inbox".into(),
                                    ..PreparedView::default()
                                })
                            }
                            Err(reason) => Ok(PreparedView {
                                clear_private: true,
                                status: reason.to_string(),
                                ..PreparedView::default()
                            }),
                        }
                    },
                );
            } else {
                ghtui::remote_scope_gate_v1::retire_remote(
                    &mut app,
                    &mut background,
                    ghtui::remote_scope_gate_v1::ScopeEntryStopV1::ScopedRouteRequired,
                );
                app.status = "Original selected PR/source unavailable • no query sent".into();
            }
        }
        // Actual selected Files/Checks reads and bounded loaded controls reuse
        // the SAME page-only owner/generation publisher. No legacy fallback.
        ghtui::personal_pr_entry_v13::capture_requested_files(&mut app, &paths.database);
        if let Some(work) = ghtui::personal_pr_entry_v13::take_work(&mut app) {
            match GithubProvider::new(work.host()).and_then(|provider| {
                provider
                    .with_runner(runner.clone())
                    .with_scoped_read_window(work.original_deadline(), 1024 * 1024)
            }) {
                Ok(provider) => {
                    background.submit_personal_pr_entry_v13(&mut app, work, provider);
                }
                Err(_) => {
                    app.status =
                        "Original personal provider unavailable • cached page retained".into();
                }
            }
        }
        if app.remote_scope.is_some()
            && app
                .github_action
                .as_ref()
                .is_some_and(ghtui::scoped_pr_observed_tabs_v1::resources::native::resource_action)
        {
            use ghtui::scoped_pr_observed_tabs_v1::resources::{self, native};
            let action = app.github_action.take().unwrap();
            app.refresh_requested = false;
            let selected = app.inbox_list.selected;
            let deadline = Instant::now() + Duration::from_secs(30);
            let owners = app
                .remote_scope
                .clone()
                .zip(app.remote_scope_source.clone())
                .zip(
                    app.github
                        .as_ref()
                        .and_then(|github| github.scoped_pr_navigation.clone()),
                );
            if let Some(((active, original), navigation)) = owners {
                match app.github.as_mut().ok_or(ghtui::scoped_pr_observed_tabs_v1::ObservedPrTabFailureV1::SelectedPageUnavailable)
                    .and_then(|github|native::take_work(github,action,selected)) {
                    Ok(work) => {
                        let work_active = active.clone();
                        let work_original = original.clone();
                        let runner = runner.clone();
                        let database = paths.database.clone();
                        background.submit_scoped_pr_tab(&mut app,active,original,navigation,move |cancel| {
                            if Instant::now() >= deadline || cancel.is_cancelled() {
                                return Err("Original selected resource preparation expired/cancelled; Back remains available".into());
                            }
                            let (completion,status) = match work {
                                native::PrResourceWorkV1::Images(input) => {
                                    let provider=match GithubProvider::new(&work_original.scope().host).and_then(|provider|provider.with_runner(runner).with_scoped_read_window(deadline,1024*1024)) {
                                        Ok(provider)=>provider,
                                        Err(_)=>return Ok(PreparedView{clear_private:true,status:"Original immutable image host/budget unavailable".into(),..Default::default()}),
                                    };
                                    let mut access=ghtui::providers::gh::RepositoryCatalogueAccessLimits::default();
                                    access.transport.max_duration=deadline.saturating_duration_since(Instant::now());
                                    match resources::immutable_images::read_images(work_original,work_active,*input,&provider,&database,&Default::default(),&access,&cancel) {
                                        Ok(result)=>{
                                            let status=if result.display_limited{"Immutable image display bound • existing file row/focus retained"}else{"Immutable old/new images • complete trees + digest-verified blobs + exact patch • read-only ranges"};
                                            (native::PrResourceCompletionV1::Images(Box::new(result)),status.to_owned())
                                        },
                                        Err(reason)=>return Ok(PreparedView{clear_private:true,status:reason.to_string(),..Default::default()}),
                                    }
                                }
                                native::PrResourceWorkV1::Loaded(input) => {
                                    let result = native::prepare_loaded(work_original,work_active,*input,&cancel)
                                        .map_err(|reason|reason.to_string())?;
                                    (native::PrResourceCompletionV1::Loaded(Box::new(result)),"Loaded patch navigation • no new API/SQL reads".to_owned())
                                }
                                native::PrResourceWorkV1::Read(input) => {
                                    let provider = match GithubProvider::new(&work_original.scope().host)
                                        .and_then(|provider|provider.with_runner(runner).with_scoped_read_window(deadline,1024*1024)) {
                                        Ok(provider) => provider,
                                        Err(_) => return Ok(PreparedView {clear_private:true,status:"Selected resource host/budget unavailable • original scope stays tagged".into(),..Default::default()}),
                                    };
                                    let mut access = ghtui::providers::gh::RepositoryCatalogueAccessLimits::default();
                                    access.transport.max_duration = deadline.saturating_duration_since(Instant::now());
                                    match resources::read_resource(work_original,work_active,*input,&provider,&database,
                                        &ghtui::providers::gh::PrResourceReadLimitsV1::default(),&access,&cancel) {
                                        Ok(result) => {
                                            let status = if result.display_limited {"Selected resource display bound • previous section retained"}
                                                else {"Selected PR Files/Checks • current exact head • observed bounded sources"};
                                            (native::PrResourceCompletionV1::Read(Box::new(result)),status.to_owned())
                                        }
                                        Err(reason) => return Ok(PreparedView {clear_private:true,status:reason.to_string(),..Default::default()}),
                                    }
                                }
                            };
                            if Instant::now() >= deadline || cancel.is_cancelled() {
                                return Err("Original selected resource preparation expired/cancelled; no private result published".into());
                            }
                            Ok(PreparedView {scoped_pr_resource:Some(completion),status,..Default::default()})
                        });
                    }
                    Err(reason) => app.status = reason.to_string(),
                }
            } else {
                app.status = "Original selected PR/source unavailable • no resource query or replacement sent".into();
            }
        }
        // Page-only observed preparation, never a large workspace clone,
        // load_tab, API/SQL request or current permission inference.
        if app.remote_scope.is_some()
            && matches!(app.github_action.as_ref(), Some(ghtui::github_workspace::GithubAction::PrTab(tab))
                if ghtui::scoped_pr_observed_tabs_v1::ObservedPrTabV1::from_tab(*tab).is_some())
        {
            let Some(ghtui::github_workspace::GithubAction::PrTab(tab)) = app.github_action.take()
            else {
                unreachable!()
            };
            app.refresh_requested = false;
            let observed =
                ghtui::scoped_pr_observed_tabs_v1::ObservedPrTabV1::from_tab(tab).unwrap();
            let owners = app
                .remote_scope
                .clone()
                .zip(app.remote_scope_source.clone())
                .zip(
                    app.github
                        .as_ref()
                        .and_then(|github| github.scoped_pr_navigation.clone()),
                );
            if let Some(((active, original), navigation)) = owners {
                match app.github.as_mut().ok_or(ghtui::scoped_pr_observed_tabs_v1::ObservedPrTabFailureV1::SelectedPageUnavailable)
                    .and_then(|github| ghtui::scoped_pr_observed_tabs_v1::take_input(github, observed)) {
                    Ok(input) => {
                        let work_active = active.clone();
                        let work_original = original.clone();
                        background.submit_scoped_pr_tab(&mut app, active, original, navigation, move |cancel| {
                            let result = ghtui::scoped_pr_observed_tabs_v1::prepare(work_original, work_active, input, &cancel)
                                .map_err(|error| error.to_string())?;
                            let status = if result.display_limited {
                                "Observed display cap • previous section retained".into()
                            } else { format!("Observed {:?} • no new reads", observed) };
                            Ok(PreparedView { scoped_pr_tab: Some(result), status, ..PreparedView::default() })
                        });
                    }
                    Err(reason) => app.status = reason.to_string(),
                }
            } else {
                app.status =
                    "Original observed PR page unavailable • no query or replacement sent".into();
            }
        }
        // Manual tagged refresh retains both immutable original owners. It
        // never routes empty/unresolved/fenced intent through a broad query.
        if app.refresh_requested
            && app.view == ghtui::app::View::Inbox
            && app.remote_scope.is_some()
        {
            app.refresh_requested = false;
            if let Some(reason) =
                ghtui::remote_scope_gate_v1::stop_before_fallback(app.remote_scope.as_deref())
            {
                ghtui::remote_scope_gate_v1::retire_remote(&mut app, &mut background, reason);
                app.status = reason.label().into();
            } else if let (Some(active), Some(original)) =
                (app.remote_scope.clone(), app.remote_scope_source.clone())
            {
                let runner = runner.clone();
                let database = paths.database.clone();
                let host = original.scope().host.clone();
                let navigation = app
                    .github
                    .as_ref()
                    .filter(|workspace| {
                        workspace
                            .scope()
                            .is_ok_and(|scope| scope == *original.scope())
                    })
                    .map(|workspace| (workspace.filter, workspace.selected.clone()));
                // Hide old private projections while rechecking current IDs.
                // Keep local Git/search and original stored intent unchanged.
                for view in [
                    ghtui::app::View::Inbox,
                    ghtui::app::View::Actions,
                    ghtui::app::View::Dashboard,
                ] {
                    background.cancel(view);
                }
                if !ghtui::remote_scope_gate_v1::independent_local_search(app.search.as_ref()) {
                    background.cancel(ghtui::app::View::Search);
                }
                ghtui::repository_catalogue_native::retire_remote_views(&mut app);
                background.submit_scoped_remote(
                    &mut app,
                    ghtui::app::View::Inbox,
                    active.clone(),
                    original.clone(),
                    move |cancel| {
                        let provider = match GithubProvider::new(host) {
                            Ok(provider) => provider.with_runner(runner),
                            Err(_) => return Ok(PreparedView {
                                clear_private: true,
                                status: "Scoped host setup unavailable • previous private pages hidden".into(),
                                ..PreparedView::default()
                            }),
                        };
                        match ghtui::scoped_inbox_route_v1::refresh(
                            original.clone(), active, &provider, &database, &cancel,
                        ) {
                            Ok(mut result) => {
                                if let Some((filter, selected)) = navigation {
                                    // Navigation intent only: select exclusively
                                    // among newly observed rows, never old bodies.
                                    result.workspace.selected = selected;
                                    result.workspace.set_filter(filter);
                                }
                                let context = result.workspace.context();
                                Ok(PreparedView {
                                    remote_scope: Some(result.current_scope),
                                    remote_scope_source: Some(original),
                                    verified_identity: Some(result.closed_identity),
                                    github: Some(result.workspace),
                                    context: Some(context),
                                    status: "My PRs refreshed • exact original IDs • current read is non-atomic".into(),
                                    // No pending_message/Cache/SQL after proof.
                                    ..PreparedView::default()
                                })
                            }
                            Err(reason) => Ok(PreparedView {
                                clear_private: true,
                                status: reason.to_string(),
                                ..PreparedView::default()
                            }),
                        }
                    },
                );
            } else {
                ghtui::remote_scope_gate_v1::retire_remote(
                    &mut app,
                    &mut background,
                    ghtui::remote_scope_gate_v1::ScopeEntryStopV1::ScopedRouteRequired,
                );
                app.status =
                    "Original repository source unavailable • scope stays tagged • no query sent"
                        .into();
            }
        }
        // Explicit LOCAL erasure is not a legacy remote read. Retain genuine
        // original owners/numeric Inbox; never build fallback repository names.
        if matches!(
            app.github_action,
            Some(ghtui::github_workspace::GithubAction::ClearCache)
        ) && app.remote_scope_source.is_some()
        {
            let _ = background.submit_catalogue_clear_v5(&mut app, &paths.database);
            // Consume only this input. Queue-full/stale refusal has not
            // cancelled or changed any old slot/loading/status/source.
            app.github_action = None;
        }
        if (app.github_action.is_some()
            || (app.refresh_requested && app.view == ghtui::app::View::Inbox))
            && !ghtui::remote_scope_gate_v1::refuse_legacy_request(
                &mut app,
                &mut background,
                ghtui::remote_scope_gate_v1::RemoteConsumerV1::MyPrs,
            )
        {
            let action = app.github_action.take();
            app.refresh_requested = false;
            next_refresh = Instant::now()
                + Duration::from_secs(config.refresh_seconds + (std::process::id() as u64 % 3));
            let previous = app.github.clone();
            let host = config.host.clone();
            let scoped_repositories = options
                .repository
                .clone()
                .map(|repo| vec![repo])
                .or_else(|| {
                    config
                        .active_set
                        .as_ref()
                        .and_then(|set| config.repository_sets.get(set).cloned())
                })
                .unwrap_or_default();
            let exclusions = config.excluded_repositories.iter().cloned().collect();
            let database = paths.database.clone();
            let runner = runner.clone();
            background.submit(&mut app, ghtui::app::View::Inbox, move |cancel| {
                use ghtui::github_workspace::{GithubAction, GithubWorkspace};
                let provider = GithubProvider::new(host)
                    .map_err(|error| error.to_string())?
                    .with_runner(runner);
                let workspace = if let Some(previous) = previous {
                    match action {
                        Some(GithubAction::Open(key)) => previous.open(&key, &provider, &cancel),
                        Some(GithubAction::PrTab(tab)) => {
                            let mut workspace = previous
                                .load_tab(tab, &provider, &cancel)
                                .map_err(|error| error.to_string())?;
                            if let Some(progress) = workspace
                                .pr
                                .as_mut()
                                .filter(|page| page.tab == ghtui::github_workspace::PrTab::Progress)
                                .and_then(|page| page.review_progress.as_mut())
                            {
                                progress
                                    .restore_position(&database)
                                    .map_err(|error| error.to_string())?;
                            }
                            Ok(workspace)
                        }
                        Some(GithubAction::ClearCache) => previous.clear_cache(&database),
                        Some(GithubAction::FileCommentV75|GithubAction::Resource(_)|GithubAction::ThreadHistoryV1|GithubAction::ReviewOperationHistoryV3|GithubAction::ReviewThreadsV4|GithubAction::ActionsHistoryV5) => {
                            return Err("Loaded scoped resource controls require the original scoped page; no legacy read sent".into());
                        }
                        Some(GithubAction::Compose(kind)) => {
                            let pending = ghtui::pr_ui::pending_message(&database, &previous)?;
                            if !pending.is_empty() {
                                return Err(pending);
                            }
                            let target = previous
                                .pr
                                .as_ref()
                                .ok_or("No PR is open")?
                                .details
                                .summary
                                .clone();
                            let intent = ghtui::forms::PrIntent {
                                kind,
                                target,
                                identity: previous.identity.clone(),
                            };
                            let mut form = ghtui::forms::Form::pr(intent)?;
                            if let Some(body) = ghtui::storage::Cache::open(&database)
                                .map_err(|error| error.to_string())?
                                .draft(&form.scope(), &form.resource())
                                .map_err(|error| error.to_string())?
                            {
                                form.restore_draft(&body)?;
                            }
                            return Ok(PreparedView {
                                lines: vec![],
                                context: Some(previous.context()),
                                status: "Local draft ready • nothing published".into(),
                                workspace: None,
                                github: Some(previous),
                                form: Some(form),
                                activate_if_untouched: false,
                                ..PreparedView::default()
                            });
                        }
                        _ if previous.pr.is_some() => {
                            previous.reconcile_page(&provider, &database, &cancel)
                        }
                        Some(GithubAction::Filter(
                            ghtui::github_workspace::InboxFilter::Merged,
                        )) => previous.load_recently_merged_v1(&provider, &cancel),
                        Some(GithubAction::Browse(key)) => {
                            provider
                                .open_pr_in_browser_v1(&key.repository, key.number, &cancel)
                                .map_err(|error| {
                                    format!("Could not open {} in the browser: {error}", key.label())
                                })?;
                            return Ok(PreparedView {
                                lines: vec![],
                                context: Some(previous.context()),
                                status: format!(
                                    "Opened {} in the browser • nothing changed on GitHub",
                                    key.label()
                                ),
                                workspace: None,
                                github: Some(previous),
                                form: None,
                                activate_if_untouched: false,
                                ..PreparedView::default()
                            });
                        }
                        _ => previous.refresh(&provider, &database, &cancel).and_then(
                            |mut refreshed| {
                                // TASK-12 AC2: review decision and check rollup for every row.
                                refreshed.load_inbox_rollup_v1(&provider, &cancel);
                                // Refreshing on the Merged tab re-reads it too.
                                if refreshed.filter == ghtui::github_workspace::InboxFilter::Merged {
                                    return refreshed.load_recently_merged_v1(&provider, &cancel);
                                }
                                Ok(refreshed)
                            },
                        ),
                    }
                } else {
                    GithubWorkspace::legacy_startup_v80(
                        &provider, scoped_repositories, exclusions, &database, &cancel,
                    )
                }
                .map_err(|error| error.to_string())?;
                let pending = if workspace.identity.account_id.is_some() {
                    Some(ghtui::pr_ui::pending_message(&database, &workspace)?)
                } else {
                    None
                };
                Ok(PreparedView {
                    context: Some(workspace.context()),
                    status: workspace.freshness.label(),
                    clear_private: workspace.identity.account_id.is_none(),
                    pr_pending_message: pending,
                    lines: vec![],
                    workspace: None,
                    github: Some(workspace),
                    form: None,
                    activate_if_untouched: false,
                    ..PreparedView::default()
                })
            });
        }
        // Scoped finite watch is distinct from legacy workspace.watching.
        if app.refresh_requested {
            ghtui::scoped_actions_native_v1::forward_refresh(&mut app);
        }
        ghtui::scoped_actions_native_v1::read_ui_v2::tick(&mut app);
        ghtui::scoped_actions_native_v1::publication_v3::capture_form_work(&mut app);
        if let Some(work) = ghtui::scoped_actions_native_v1::take_work(&mut app) {
            match GithubProvider::new(work.host())
                .map(|provider| provider.with_runner(runner.clone()))
            {
                Ok(provider) => {
                    background.submit_scoped_actions_publication_v3(
                        &mut app,
                        work,
                        provider,
                        paths.database.clone(),
                        draft_writer.client(),
                    );
                }
                Err(_) => ghtui::scoped_actions_native_v1::submission_failed(&mut app, &work),
            }
        }
        ghtui::pr_checks_native::scoped::choices::continuations_v2::tick(&mut app);
        if app.pr_check_continuation_cancel_v2 {
            app.pr_check_continuation_cancel_v2 = false;
            background.cancel(ghtui::app::View::Actions);
            app.loading[ghtui::app::View::Actions.index()] = false;
        }
        if let Some(work) =
            ghtui::pr_checks_native::scoped::choices::continuations_v2::corrective_v3::take_work(
                &mut app,
            )
        {
            let owner = work.owner().clone();
            let deadline = owner.deadline();
            let host = owner
                .origin()
                .scoped_owner_v1()
                .unwrap()
                .0
                .scope()
                .host
                .clone();
            let runner = runner.clone();
            let database = paths.database.clone();
            if !background.submit_pr_check_continuation_v2(&mut app, owner, move |cancel| {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() || cancel.is_cancelled() {
                    return Err("Original scoped continuation expired".into());
                }
                let provider = GithubProvider::new(host)
                    .map_err(|e| e.to_string())?
                    .with_runner(runner)
                    .with_scoped_read_window(deadline, 1024 * 1024)
                    .map_err(|e| e.to_string())?;
                let mut limits =
                    ghtui::pr_checks_native::scoped::ScopedPrCheckReadLimitsV1::default();
                limits.access.transport.max_duration = remaining;
                limits.bridge.max_duration = remaining;
                limits.logs.max_duration = remaining;
                ghtui::pr_checks_native::scoped::choices::continuations_v2::execute(
                    work, &provider, &database, &limits, &cancel,
                )
            }) {
                ghtui::pr_checks_native::scoped::choices::continuations_v2::stop(&mut app);
            }
        }
        ghtui::pr_checks_native::scoped::choices::queue_other_logs(&mut app);
        if let Some(work) = ghtui::pr_checks_native::scoped::choices::take_work(&mut app) {
            let origin = work.origin().clone();
            let view = work.view();
            let host = origin.scoped_owner_v1().unwrap().0.scope().host.clone();
            let runner = runner.clone();
            let database = paths.database.clone();
            let deadline = Instant::now() + Duration::from_secs(30);
            background.submit_scoped_pr_check_v1(&mut app, view, origin, move |cancel| {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() || cancel.is_cancelled() {
                    return Err("Original candidate/member read expired".into());
                }
                let provider = GithubProvider::new(host)
                    .map_err(|e| e.to_string())?
                    .with_runner(runner)
                    .with_scoped_read_window(deadline, 1024 * 1024)
                    .map_err(|e| e.to_string())?;
                let mut limits =
                    ghtui::pr_checks_native::scoped::ScopedPrCheckReadLimitsV1::default();
                limits.access.transport.max_duration = remaining;
                limits.bridge.max_duration = remaining;
                limits.logs.max_duration = remaining;
                ghtui::pr_checks_native::scoped::choices::execute(
                    work, &provider, &database, &limits, &cancel,
                )
            });
        }
        if app
            .pr_checks_route
            .as_ref()
            .is_some_and(|route| route.scoped_v1())
            && app.actions_action.is_some()
        {
            if let Some((route, workspace, action)) =
                ghtui::pr_checks_native::scoped::take_actions_v1(&mut app)
            {
                let origin = route.scoped_origin_v1().clone();
                let host = origin.scoped_owner_v1().unwrap().0.scope().host.clone();
                let runner = runner.clone();
                let database = paths.database.clone();
                let deadline = Instant::now() + Duration::from_secs(30);
                background.submit_scoped_pr_check_v1(
                    &mut app,
                    ghtui::app::View::Actions,
                    origin,
                    move |cancel| {
                        let remaining = deadline.saturating_duration_since(Instant::now());
                        if remaining.is_zero() || cancel.is_cancelled() {
                            return Err("Original scoped check/log request expired".into());
                        }
                        let provider = GithubProvider::new(host)
                            .map_err(|e| e.to_string())?
                            .with_runner(runner)
                            .with_scoped_read_window(deadline, 1024 * 1024)
                            .map_err(|e| e.to_string())?;
                        let mut limits =
                            ghtui::pr_checks_native::scoped::ScopedPrCheckReadLimitsV1::default();
                        limits.access.transport.max_duration = remaining;
                        limits.logs.max_duration = remaining;
                        ghtui::pr_checks_native::scoped::apply_v1(
                            route, workspace, action, &provider, &database, &limits, &cancel,
                        )
                    },
                );
            }
        }
        if app.view == ghtui::app::View::Actions
            && app.remote_repository.is_none()
            && app.overlay.is_none()
            && !app.loading[3]
            && app
                .pr_checks_route
                .as_ref()
                .is_none_or(|route| !route.scoped_v1())
            && app.actions.as_ref().is_some_and(|actions| {
                actions.watching
                        && actions.watch_applies()
                        // TASK-117: the automatic live watch waits while a form is open.
                        && (!actions.live.armed || app.form.is_none())
            })
            && app.scoped_actions_v1.is_none()
            && Instant::now() >= next_actions_refresh
            && !ghtui::remote_scope_gate_v1::refuse_legacy_request(
                &mut app,
                &mut background,
                ghtui::remote_scope_gate_v1::RemoteConsumerV1::Watch,
            )
        {
            app.actions_action = Some(ghtui::actions_workspace::ActionsAction::Refresh);
        }
        if (app.actions_load_requested
            || app.actions_action.is_some()
            || (app.refresh_requested && app.view == ghtui::app::View::Actions))
            && !ghtui::remote_scope_gate_v1::refuse_legacy_request(
                &mut app,
                &mut background,
                ghtui::remote_scope_gate_v1::RemoteConsumerV1::Actions,
            )
        {
            let requested_refresh = app.refresh_requested && app.view == ghtui::app::View::Actions;
            app.actions_load_requested = false;
            app.refresh_requested = false;
            let action = app.actions_action.take().or_else(|| {
                requested_refresh.then_some(ghtui::actions_workspace::ActionsAction::Refresh)
            });
            let inspection = app.actions_inspection.take();
            // TASK-117: a live run backs off from 5 s to 30 s while nothing changes.
            next_actions_refresh = Instant::now()
                + Duration::from_secs(
                    app.actions
                        .as_ref()
                        .map_or(5, |actions| actions.poll_interval_secs()),
                );
            let previous = app.actions.clone();
            let pr_check_route = app.pr_checks_route.clone();
            let mut repositories = options
                .repository
                .clone()
                .map(|repo| vec![repo])
                .or_else(|| {
                    config
                        .active_set
                        .as_ref()
                        .and_then(|set| config.repository_sets.get(set).cloned())
                })
                .or_else(|| {
                    detected_repository
                        .clone()
                        .map(|repository| vec![repository])
                })
                .unwrap_or_else(|| {
                    if let Some(github) = &app.github {
                        github
                            .inbox
                            .pull_requests
                            .iter()
                            .map(|pr| pr.repository.clone())
                            .collect()
                    } else {
                        config.local_repositories.keys().cloned().collect()
                    }
                });
            repositories.retain(|repo| !config.excluded_repositories.contains(repo));
            repositories.sort();
            repositories.dedup();
            if previous.is_none() && repositories.is_empty() {
                app.content[ghtui::app::View::Actions.index()] = vec!["Choose a repository scope before loading Actions.".into(),"Run ghtui inside a GitHub checkout, use --repo OWNER/NAME, choose a saved repository set, or load your PR inbox first.".into(),"No GitHub request was made; an unknown scope is not an empty successful catalogue.".into()];
                app.status = "Actions scope needed • no repository selected".into();
            } else {
                let host = config.host.clone();
                let runner = runner.clone();
                let database = paths.database.clone();
                background.submit(&mut app,ghtui::app::View::Actions,move|cancel| {
                    use ghtui::actions_workspace::{ActionsAction,ActionsWorkspace};
                    let provider = GithubProvider::new(host).map_err(|error|error.to_string())?.with_runner(runner);
                    if let Some(route)=pr_check_route {
                        if matches!(action,Some(ActionsAction::OpenJob(_) | ActionsAction::Logs(_))) {
                            let workspace=previous.ok_or("Original PR check attempt is unavailable")?;
                            return ghtui::pr_checks_native::apply(route,workspace,action.unwrap(),&provider,&database,&cancel);
                        }
                    }
                    let verifying = previous.is_none() || matches!(action,Some(ActionsAction::Refresh));
                    let verified = if verifying {
                        match provider.identity(&cancel) {
                            Ok(identity) if identity.account_id.is_some()=>Some(identity),
                            result=>return Ok(PreparedView {clear_private:true,context:Some("GitHub identity unavailable • private pages hidden".into()),status:format!("Identity unavailable • private data hidden: {}",result.err().map_or_else(||"numeric account ID missing".into(),|error|error.to_string())),..PreparedView::default()}),
                        }
                    }else{None};
                    let mut workspace = if let Some(previous) = previous.filter(|previous|verified.as_ref().is_none_or(|identity|identity.same_account(&previous.identity))) {previous} else {
                        let identity = verified.ok_or("No verified Actions identity")?;
                        ghtui::storage::Scope::github(&identity.host,identity.account_id).map_err(|error|error.to_string())?;
                        let mut workspace = ActionsWorkspace::new(identity,repositories);
                        // A commit's runs (TASK-52) load their own filtered page below.
                        if !matches!(action,Some(ActionsAction::CommitRuns(_))) {
                            workspace.load_catalog(&provider,&cancel).map_err(|error|error.to_string())?;
                        }
                        workspace
                    };
                    if let Some(ActionsAction::Compose(command)) = action {
                        return ghtui::actions_routes::open(workspace,command,&provider,&database,&cancel);
                    }
                    use ghtui::actions_recovery::{self,RecoveryAcknowledgement};
                    match action.as_ref() {
                        Some(ActionsAction::PublicationList)=>return actions_recovery::list(workspace,&provider,&database,&cancel),
                        Some(ActionsAction::PublicationInspect(id))=>return actions_recovery::inspect(workspace,*id,&provider,&database,&cancel),
                        Some(ActionsAction::PublicationAcknowledge(id))=>return actions_recovery::acknowledge(workspace,RecoveryAcknowledgement::ObservedState(*id),inspection,&provider,&database,&cancel),
                        Some(ActionsAction::PublicationAcknowledgeAcceptance(id))=>return actions_recovery::acknowledge(workspace,RecoveryAcknowledgement::InspectedAcceptance(*id),inspection,&provider,&database,&cancel),
                        Some(ActionsAction::Acknowledge)=>return Err("Open Publications and inspect the exact ORIGINAL attempt; a provider-only acknowledgement cannot release durable protection".into()),
                        _=>{},
                    }
                    if let Some(action) = action {
                        if let Err(error) = workspace.apply(&provider,&action,&cancel) {
                            if matches!(error.kind,ghtui::providers::gh::GithubErrorKind::IdentityChanged | ghtui::providers::gh::GithubErrorKind::Authentication) {return Ok(PreparedView {clear_private:true,status:format!("Actions identity changed or unavailable • private views hidden: {error}"),..Default::default()});}
                            return Err(error.to_string());
                        }
                    }
                    let status = workspace.warnings.last().cloned().unwrap_or_else(||"Actions loaded".into());
                    let journal=ghtui::actions_write_journal::ActionsWriteJournal::open(&database).map_err(|error|error.to_string())?;
                    actions_recovery::decorate(&mut workspace,&provider,&journal)?;
                    Ok(PreparedView {context:Some(workspace.context()),status,actions:Some(workspace),clear_actions_inspection:true,..PreparedView::default()})
                });
            }
        }
        if (app.metrics_load_requested
            || app.metrics_action.is_some()
            || (app.refresh_requested && app.view == ghtui::app::View::Dashboard))
            && !ghtui::remote_scope_gate_v1::refuse_legacy_request(
                &mut app,
                &mut background,
                ghtui::remote_scope_gate_v1::RemoteConsumerV1::Dashboard,
            )
        {
            let refresh = app.refresh_requested && app.view == ghtui::app::View::Dashboard;
            app.metrics_load_requested = false;
            app.refresh_requested = false;
            let action = app
                .metrics_action
                .take()
                .or_else(|| refresh.then_some(ghtui::metrics::MetricsAction::Refresh));
            let mut declared = options
                .repository
                .clone()
                .map(|repository| vec![repository])
                .or_else(|| {
                    config
                        .active_set
                        .as_ref()
                        .and_then(|set| config.repository_sets.get(set).cloned())
                })
                .or_else(|| {
                    detected_repository
                        .clone()
                        .map(|repository| vec![repository])
                })
                .unwrap_or_else(|| config.local_repositories.keys().cloned().collect());
            declared.retain(|repository| {
                !config
                    .excluded_repositories
                    .iter()
                    .any(|excluded| excluded.eq_ignore_ascii_case(repository))
            });
            declared.sort();
            declared.dedup();
            if declared.is_empty() {
                app.metrics = None;
                app.content[ghtui::app::View::Dashboard.index()] = vec!["Dashboard scope needed • no loaded records; missing data is not zero.".into(),"Choose --repo OWNER/NAME, a saved repository set, or explicit configured local repository mappings.".into(),"No GitHub request was made. No automatic organization discovery, hydration or clone.".into()];
                app.context = "Dashboard • explicit repository scope needed".into();
                app.status = "Dashboard scope needed • no query sent".into();
            } else {
                let request = ghtui::metrics_routes::MetricsRequest {
                    declared,
                    exclusions: config.excluded_repositories.iter().cloned().collect(),
                    previous: app.metrics.clone(),
                    action: action.clone(),
                    database: paths.database.clone(),
                    selected_repository: app.metrics_selected_repository.clone(),
                    window_days: app.metrics_days,
                };
                if let Some(metrics) = &mut app.metrics {
                    metrics.pending =
                        Some(action.unwrap_or(ghtui::metrics::MetricsAction::Refresh));
                }
                let host = config.host.clone();
                let runner = runner.clone();
                if !background.submit(&mut app, ghtui::app::View::Dashboard, move |cancel| {
                    let provider = GithubProvider::new(host)
                        .map_err(|error| error.to_string())?
                        .with_runner(runner);
                    ghtui::metrics_routes::prepare(request, &provider, &cancel)
                }) {
                    app.metrics = None;
                    app.metrics_content = Default::default();
                    app.content[ghtui::app::View::Dashboard.index()] = vec![
                        "Dashboard queue unavailable • previous values hidden; Refresh to retry."
                            .into(),
                    ];
                }
            }
        }
        if app.source_action.is_some() {
            ghtui::remote_scope_gate_v1::refuse_legacy_request(
                &mut app,
                &mut background,
                ghtui::remote_scope_gate_v1::RemoteConsumerV1::SourceLoad,
            );
        }
        if let Some(action) = app.source_action.take() {
            let request = app.source_controls.request();
            let previous = app.search.clone();
            let runner = runner.clone();
            let database = paths.database.clone();
            let host = config.host.clone();
            app.search_load_requested = false;
            app.search_action = None;
            app.refresh_requested = false;
            if let Some(search) = &mut app.search {
                search.page = None;
                search.preview = None;
                search.remote_items.clear();
                search.blend_evidence = None;
            }
            app.source_context = None;
            background.submit(&mut app, ghtui::app::View::Search, move |cancel| {
                let request = request?;
                let search = previous.ok_or("Search corpus is not ready; wait for discovery")?;
                let provider = GithubProvider::new(host).map_err(|error|error.to_string())?.with_runner(runner);
                let result = match action {
                    ghtui::search_sources::SourceAction::Load => ghtui::search_sources::load(search,request,&provider,&database,&cancel),
                    ghtui::search_sources::SourceAction::Remove => ghtui::search_sources::remove(search,request,&provider,&database,&cancel),
                    _ => return Err("Unexpected worker source action".into()),
                };
                match result {
                    Ok(prepared) => Ok(prepared),
                    Err(error) if ghtui::pr_ui::account_failure(&error) => Ok(PreparedView {clear_private:true,status:format!("Source account unavailable • private search hidden: {error}"),..Default::default()}),
                    Err(error) => Ok(PreparedView {source_message:Some(format!("Source not loaded: {error}")),status:"Source load failed • no remote mutation; inspect source coverage before retrying".into(),..Default::default()}),
                }
            });
        }
        if app.corpus_requested
            && !ghtui::remote_scope_gate_v1::refuse_legacy_request(
                &mut app,
                &mut background,
                ghtui::remote_scope_gate_v1::RemoteConsumerV1::Corpus,
            )
        {
            app.corpus_requested = false;
            app.search_load_requested = false;
            app.search_action = None;
            app.refresh_requested = false;
            let previous = app.search.clone();
            let config = config.clone();
            let directory = local_directory.clone();
            let database = paths.database.clone();
            let base = if paths.root.is_absolute() {
                paths.root.join("corpus")
            } else {
                directory.join(&paths.root).join("corpus")
            };
            let mut repositories = options
                .repository
                .clone()
                .map(|repository| vec![repository])
                .or_else(|| {
                    config
                        .active_set
                        .as_ref()
                        .and_then(|set| config.repository_sets.get(set).cloned())
                })
                .or_else(|| {
                    detected_repository
                        .clone()
                        .map(|repository| vec![repository])
                })
                .unwrap_or_default();
            repositories.extend(config.local_repositories.keys().cloned());
            repositories.retain(|repository| !config.excluded_repositories.contains(repository));
            repositories.sort();
            repositories.dedup();
            let runner = runner.clone();
            background.submit(&mut app,ghtui::app::View::Search,move|cancel| {
                let search = match previous { Some(search) => search, None => ghtui::search::workspace::SearchWorkspace::from_config(ghtui::storage::Scope::new("local","workspace").map_err(|error|error.to_string())?,&config,&directory,&cancel).map_err(|error|error.to_string())? };
                let provider = GithubProvider::new(&config.host).map_err(|error|error.to_string())?.with_runner(runner);
                match ghtui::corpus_routes::open(&provider,base,repositories,search,&database,&cancel) {
                    Ok(prepared) => Ok(prepared),
                    Err(error) if ghtui::pr_ui::account_failure(&error) => Ok(PreparedView {clear_private:true,status:format!("Repository account verification failed • private selections hidden: {error}"),..Default::default()}),
                    Err(error) => Err(error),
                }
            });
        }
        // TASK-121: the debounced live fuzzy search for the typed query.
        ghtui::search_ui::tick_live(&mut app);
        if (app.search_load_requested
            || app.search_action.is_some()
            || (app.refresh_requested && app.view == ghtui::app::View::Search))
            && !ghtui::remote_scope_gate_v1::refuse_legacy_request(
                &mut app,
                &mut background,
                ghtui::remote_scope_gate_v1::RemoteConsumerV1::Search,
            )
        {
            let initializing = app.search_load_requested;
            app.search_load_requested = false;
            let action = app.search_action.take();
            app.refresh_requested = false;
            let previous = app.search.clone();
            let config = config.clone();
            let directory = local_directory.clone();
            let database = paths.database.clone();
            let index_root = paths.index.clone();
            let input = app.search_controls.query.text.clone();
            let name = app.search_controls.name.text.clone();
            let explicit_repository = options.repository.clone();
            let detected_repository = detected_repository.clone();
            let remote_scope = app.remote_scope.clone();
            let runner = runner.clone();
            background.submit(&mut app, ghtui::app::View::Search, move |cancel| {
                use ghtui::{
                    search::workspace::{SearchAction, SearchWorkspace},
                    storage::{Cache, Scope},
                };
                let mut cache = Cache::open(&database).map_err(|error| error.to_string())?;
                let mut search = if let Some(previous) = previous {
                    previous
                } else {
                    SearchWorkspace::from_config(
                        Scope::new("local", "workspace").map_err(|error| error.to_string())?,
                        &config,
                        &directory,
                        &cancel,
                    )
                    .map_err(|error| error.to_string())?
                };
                if initializing { ghtui::search_sources::restore(&cache, &mut search)?; }
                if let Some(SearchAction::Mode(mode))=&action {search.set_mode(*mode);}
                // Declared repository names are intent only, not account authority.
                // A local/current checkout never acquires an inferred GitHub mapping.
                if remote_scope.is_none() && search.remote_repositories.is_empty() {
                    search.remote_repositories=explicit_repository.clone().map(|repository|vec![repository])
                        .or_else(||config.active_set.as_ref().and_then(|set|config.repository_sets.get(set).cloned()))
                        .or_else(||detected_repository.clone().map(|repository|vec![repository]))
                        .unwrap_or_else(||search.selections.iter().filter(|selection|config.local_repositories.contains_key(&selection.repository)).map(|selection|selection.repository.clone()).collect());
                    search.remote_repositories.retain(|repository|!config.excluded_repositories.contains(repository));
                    search.remote_repositories.sort();search.remote_repositories.dedup();
                }
                let remote_provider = if search.mode == ghtui::search::SearchMode::GitHub || !search.metadata_sources.discussions.is_empty() || search.scope.host != "local"
                    || matches!(action,Some(SearchAction::BlendPreferences))
                    || search.mode == ghtui::search::SearchMode::Ranked && search.blend_policy.enabled {
                    let provider=GithubProvider::new(config.host.clone()).map_err(|error|error.to_string())?.with_runner(runner.clone());
                    let identity=match provider.identity(&cancel) {
                        Ok(identity)=>identity,
                        Err(error)=>return Ok(PreparedView {clear_private:true,status:format!("Identity unavailable • private search hidden: {error}"),..PreparedView::default()}),
                    };
                    let scope=match Scope::github(&identity.host,identity.account_id) {
                        Ok(scope)=>scope,
                        Err(error)=>return Ok(PreparedView {clear_private:true,status:format!("Unverified account • private search hidden: {error}"),..PreparedView::default()}),
                    };
                    if search.remote_identity.as_ref().is_some_and(|previous|!previous.same_account(&identity)) {
                        let mode=search.mode;
                        let mut fresh=SearchWorkspace::from_config(scope,&config,&directory,&cancel).map_err(|error|error.to_string())?;
                        fresh.set_mode(mode);
                        fresh.remote_identity=Some(identity);
                        ghtui::search_sources::restore(&cache,&mut fresh)?;
                        fresh.load_saved(&cache).map_err(|error|error.to_string())?;
                        return Ok(PreparedView {context:Some(fresh.context()),status:"Account changed • previous private search hidden; enter a new query".into(),search:Some(fresh),restore_search_input:true,..PreparedView::default()});
                    }
                    let changed_scope = search.scope != scope;
                    search.scope=scope;
                    search.remote_identity=Some(identity);
                    if changed_scope { ghtui::search_sources::restore(&cache, &mut search)?; }
                    if changed_scope && !matches!(action,Some(SearchAction::BlendPreferences)) {
                        search.blend_policy=ghtui::search_blend::load_policy(&cache,&search.scope)?;
                    }
                    if remote_scope.is_none() && search.remote_repositories.is_empty() {
                        search.remote_repositories=explicit_repository.clone().map(|repository|vec![repository])
                            .or_else(||config.active_set.as_ref().and_then(|set|config.repository_sets.get(set).cloned()))
                        .or_else(||detected_repository.clone().map(|repository|vec![repository]))
                            .unwrap_or_else(||search.selections.iter().filter(|selection|config.local_repositories.contains_key(&selection.repository)).map(|selection|selection.repository.clone()).collect());
                        search.remote_repositories.retain(|repository|!config.excluded_repositories.contains(repository));
                        search.remote_repositories.sort();search.remote_repositories.dedup();
                    }
                    Some(provider)
                } else {None};
                search
                    .load_saved(&cache)
                    .map_err(|error| error.to_string())?;
                let restore_search_input = matches!(action, Some(SearchAction::Saved(_)));
                let status = match action {
                    Some(SearchAction::BlendPreferences) => {
                        ghtui::search_blend::save_policy(&cache,&search.scope,&search.blend_policy)?;
                        ghtui::search_blend::saved_status(&search.blend_policy)
                    }
                    Some(SearchAction::Mode(ghtui::search::SearchMode::GitHub)) => "GitHub scope verified • enter a query or choose a saved search".into(),
                    Some(SearchAction::Mode(_)) => "Search mode ready • enter a query for this explicit scope".into(),
                    Some(SearchAction::Save) => {
                        // Save exactly the visible query, not a previous successful search.
                        search.input = input;
                        // Editing a saved query by saving it again keeps its pin.
                        let pinned = search.saved.iter().any(|saved| saved.name == name && saved.pinned);
                        search
                            .save(&cache, &name, pinned)
                            .map_err(|error| error.to_string())?;
                        format!("Saved query: {name} • {} scope retained",if search.mode == ghtui::search::SearchMode::GitHub {"verified account/repository"} else {"local"})
                    }
                    Some(SearchAction::PinSaved(target)) => {
                        let pinned = search.toggle_saved_pin(&cache, &target).map_err(|error| error.to_string())?;
                        if pinned { format!("Pinned saved query: {target} • listed first") } else { format!("Unpinned saved query: {target}") }
                    }
                    Some(SearchAction::RenameSaved(target)) => {
                        search.rename_saved(&cache, &target, &name).map_err(|error| error.to_string())?;
                        format!("Renamed saved query: {target} → {}", name.trim())
                    }
                    Some(SearchAction::DeleteSaved(target)) => {
                        search.delete_saved(&cache, &target).map_err(|error| error.to_string())?;
                        format!("Deleted saved query: {target} • S saves the current query again")
                    }
                    Some(SearchAction::Saved(name)) => {
                        search
                            .restore_saved(&name)
                            .map_err(|error| error.to_string())?;
                        let input = search.input.clone();
                        if search.mode != ghtui::search::SearchMode::GitHub && search.selections.is_empty() && search.metadata_sources.is_empty() {
                            return Ok(PreparedView {
                                context: Some(search.context()),
                                status: format!("Restored query: {name} • unavailable saved scope; no broader search"),
                                search: Some(search),
                                restore_search_input: true,
                                ..PreparedView::default()
                            });
                        }
                        if let Err(error)=execute_search(&mut search,&input,&mut cache,&index_root,&database,remote_provider.as_ref(),&cancel) {
                            if let Some(provider)=&remote_provider {
                                search.page=None;search.preview=None;search.remote_items.clear();search.blend_evidence=None;
                                if let Err(identity_error)=verify_search_identity(&search,provider,&cancel) {return Ok(PreparedView {clear_private:true,status:identity_error,..PreparedView::default()});}
                                return Ok(PreparedView {context:Some(search.context()),status:ghtui::search_sources::visibility::hidden_read_status(&error),search:Some(search),restore_search_input:true,..PreparedView::default()});
                            }
                            return Err(error);
                        }
                        if search
                            .page
                            .as_ref()
                            .is_some_and(|page| !page.hits.is_empty())
                        {
                            search.preview(0).map_err(|error| error.to_string())?;
                        }
                        format!("Restored query: {name} • same selected scope")
                    }
                    Some(SearchAction::Open(index)) => {
                        let mut access_targets=search.metadata_sources.discussions.clone();
                        if let Some(evidence)=&search.blend_evidence {access_targets.extend_from_slice(evidence.access_targets());}
                        if !access_targets.is_empty() {
                            let provider=remote_provider.as_ref().ok_or("Verify current account/repository access before opening a private cached source")?;
                            let identity=search.remote_identity.as_ref().ok_or("Private cached source account unavailable")?;
                            if let Err(error)=ghtui::search_sources::visibility::access(provider,identity,&search.scope,&access_targets,&cancel) {
                                search.page=None;search.preview=None;search.remote_items.clear();search.blend_evidence=None;
                                if let Err(identity_error)=verify_search_identity(&search,provider,&cancel){return Ok(PreparedView{clear_private:true,status:identity_error,..Default::default()});}
                                return Ok(PreparedView{context:Some(search.context()),search:Some(search),status:error,..Default::default()});
                            }
                        }
                        if let Some(evidence)=&search.blend_evidence {
                            if let Err(error)=evidence.validate(&cache) {
                                search.page=None;search.preview=None;search.remote_items.clear();search.blend_evidence=None;
                                if let Some(provider)=&remote_provider {
                                    if let Err(identity_error)=verify_search_identity(&search,provider,&cancel) {return Ok(PreparedView {clear_private:true,status:identity_error,..Default::default()});}
                                }
                                return Ok(PreparedView {context:Some(search.context()),status:format!("Search visibility changed • prior results hidden • run the same scoped search again: {error}"),search:Some(search),..Default::default()});
                            }
                        }
                        let destination = search
                            .destination(index)
                            .map_err(|error| error.to_string())?;
                        if matches!(destination,ghtui::search::model::Destination::LocalCommit {..})
                            || search.result_document(index).map_err(|error|error.to_string())?.metadata.discussion.is_some() {
                            let proofs=ghtui::search_sources::visibility::before(&database,&search)?;
                            let source_context=ghtui::search_sources::context::prepare(&search,index,&cache,&cancel)?;
                            if let Some(provider)=&remote_provider {
                                let identity=search.remote_identity.as_ref().ok_or("Private source account unavailable")?;
                                if let Err(error)=ghtui::search_sources::visibility::access(provider,identity,&search.scope,&access_targets,&cancel) {
                                    search.page=None;search.preview=None;search.remote_items.clear();search.blend_evidence=None;
                                    if let Err(identity_error)=verify_search_identity(&search,provider,&cancel){return Ok(PreparedView{clear_private:true,status:identity_error,..Default::default()});}
                                    return Ok(PreparedView{context:Some(search.context()),search:Some(search),status:error,..Default::default()});
                                }
                                if let Err(error)=verify_search_identity(&search,provider,&cancel) {return Ok(PreparedView {clear_private:true,status:error,..Default::default()});}
                            }
                            ghtui::search_sources::visibility::after(&database,&proofs)?;
                            return Ok(PreparedView {context:Some(search.context()),status:"Opened exact cached source • canonical discussion or original commit • no Git/GitHub changes".into(),
                                search:Some(search),source_context:Some(source_context),..Default::default()});
                        }
                        if let ghtui::search::model::Destination::RemoteCodeCandidate {..}=&destination {
                            let provider=remote_provider.as_ref().ok_or("Remote search identity must be verified before opening code")?;
                            let content=provider.resolve_remote_content(search.remote_item(index).map_err(|error|error.to_string())?,search.corpus_limits.max_file_bytes.min(2*1024*1024),&cancel).map_err(|error|error.to_string())?;
                            search.open_remote_file(index,&content.file,&content.bytes).map_err(|error|error.to_string())?;
                            if let Err(error)=verify_search_identity(&search,provider,&cancel) {return Ok(PreparedView {clear_private:true,status:error,..PreparedView::default()});}
                            return Ok(PreparedView {context:Some(format!("{} / {} / {} / exact saved file",content.file.host,content.file.repository,content.file.commit_sha)),status:"Opened verified GitHub saved file • exact blob/commit • no local Git changes".into(),search:Some(search),..PreparedView::default()});
                        }
                        if let ghtui::search::model::Destination::PullRequest {host,repository,number,..}=&destination {
                            let provider=remote_provider.as_ref().ok_or("Verify the remote search account before opening its PR")?;
                            let identity=search.remote_identity.clone().ok_or("Remote search identity is unavailable")?;
                            let github=ghtui::github_workspace::GithubWorkspace::empty(identity,vec![repository.clone()],Default::default()).open_search_result(search.remote_item(index).map_err(|error|error.to_string())?,provider,&cancel).map_err(|error|error.to_string())?;
                            if let Err(error)=verify_search_identity(&search,provider,&cancel) {return Ok(PreparedView {clear_private:true,status:error,..PreparedView::default()});}
                            return Ok(PreparedView {context:Some(format!("{host} / {repository}#{number} / exact search PR")),status:"Opened exact GitHub PR search destination".into(),github:Some(github),search:Some(search),navigate_to:Some(ghtui::app::View::Inbox),..PreparedView::default()});
                        }
                        if let Some(status)=search.open_remote_overview(index).map_err(|error|error.to_string())? {
                            return Ok(PreparedView {context:Some(search.context()),status,search:Some(search),..PreparedView::default()});
                        }
                        let ghtui::search::model::Destination::LocalFile {
                            directory,
                            path,
                            revision,
                            line,
                            column,
                        } = destination
                        else {
                            return Err("This result requires its native GitHub viewer".into());
                        };
                        let workspace = ghtui::workspace::Workspace::open_search_file(
                            &directory,
                            &path,
                            revision.as_deref(),
                            line,
                            cancel,
                        )?;
                        return Ok(PreparedView {
                            context: Some(format!(
                                "{}:{}:{} / {}",
                                path.display(),
                                line,
                                column,
                                revision.as_deref().unwrap_or("working files • mutable")
                            )),
                            status: "Opened exact search destination • no checkout or Git changes"
                                .into(),
                            workspace: Some(workspace),
                            search: Some(search),
                            navigate_to: Some(ghtui::app::View::Repository),
                            ..PreparedView::default()
                        });
                    }
                    _ if initializing && input.trim().is_empty() => {
                        "Selected local search corpus ready • no clone or fetch".into()
                    }
                    _ => {
                        if let Err(error)=execute_search(&mut search,&input,&mut cache,&index_root,&database,remote_provider.as_ref(),&cancel) {
                            if let Some(provider)=&remote_provider {
                                search.page=None;search.preview=None;search.remote_items.clear();search.blend_evidence=None;
                                if let Err(identity_error)=verify_search_identity(&search,provider,&cancel) {return Ok(PreparedView {clear_private:true,status:identity_error,..PreparedView::default()});}
                                return Ok(PreparedView {context:Some(search.context()),status:ghtui::search_sources::visibility::hidden_read_status(&error),search:Some(search),..PreparedView::default()});
                            }
                            return Err(error);
                        }
                        if search
                            .page
                            .as_ref()
                            .is_some_and(|page| !page.hits.is_empty())
                        {
                            search.preview(0).map_err(|error| error.to_string())?;
                        }
                        "Search complete • exact source and exclusions shown".into()
                    }
                };
                if let Some(provider)=&remote_provider {
                    if let Err(error)=verify_search_identity(&search,provider,&cancel) {return Ok(PreparedView {clear_private:true,status:error,..PreparedView::default()});}
                }
                Ok(PreparedView {
                    context: Some(search.context()),
                    status,
                    search: Some(search),
                    restore_search_input,
                    ..PreparedView::default()
                })
            });
        }
        if app.refresh_requested {
            app.refresh_requested = false;
            let view = app.view;
            let lines = app.content[view.index()].clone();
            background.submit(&mut app, view, move |cancel| {
                let result = CommandRunner::default()
                    .run(CommandSpec::new("gh").args(["--version"]), &cancel)
                    .map_err(|error| error.to_string())?;
                let version = result
                    .stdout_text()
                    .lines()
                    .next()
                    .unwrap_or("gh ready")
                    .to_owned();
                Ok(PreparedView {
                    lines,
                    context: None,
                    status: format!("{version} • GitHub provider ready"),
                    workspace: None,
                    github: None,
                    form: None,
                    activate_if_untouched: false,
                    ..PreparedView::default()
                })
            });
        }
        ghtui::suggestion_apply_v1::tick(&mut app, &paths.database);
        ghtui::pr_branch_edit_v27::tick(&mut app, &paths.database);
        if let Some(work) = ghtui::pr_branch_edit_v27::take_editor_request(&mut app) {
            let edited = edit_file_in_place(&mut guard, &work);
            // Fullscreen re-entry needs a complete repaint.
            terminal.resize(terminal.size()?.into())?;
            ghtui::pr_branch_edit_v27::editor_finished(
                &mut app,
                edited.map_err(|error| format!("{error:#}")),
            );
        }
        let render_started = render_timings.start();
        ghtui::clipboard::tick(&mut app);
        let drawn = terminal
            .draw(|frame| {
                ui::draw(frame, &mut app);
                // TASK-120: keep the frame as drawn for drag selection, then paint the selection.
                let selection = &mut app.copy_v120.selection;
                selection.remember_frame(frame.buffer_mut());
                selection.paint(frame.buffer_mut());
            })?
            .area;
        render_timings.record(render_started, drawn.width, drawn.height);
        ghtui::clipboard::flush_terminal(&mut app)?;
        if event::poll(Duration::from_millis(40))? {
            let input = event::read()?;
            if !ghtui::clipboard::pre_event(&mut app, &input) {
                app.handle_event(input);
            }
        }
    }
    render_timings.finish();
    if let Some((_, cache)) = avatar_cache {
        cache.shutdown();
    }
    if let Some(form) = &app.form {
        if form.dirty && !form.submitting {
            draft_writer
                .save(form.scope(), form.resource(), form.draft())
                .map_err(anyhow::Error::msg)?;
        }
    }
    drop(background);
    drop(draft_writer);
    Ok(())
}

fn execute_search(
    search: &mut ghtui::search::workspace::SearchWorkspace,
    input: &str,
    cache: &mut ghtui::storage::Cache,
    index_root: &std::path::Path,
    database: &std::path::Path,
    remote: Option<&GithubProvider>,
    cancel: &ghtui::providers::runner::CancellationToken,
) -> Result<(), String> {
    let blend = search.mode == ghtui::search::SearchMode::Ranked && search.blend_policy.enabled;
    if !blend && !search.metadata_sources.discussions.is_empty() {
        ghtui::search_sources::visibility::access(
            remote.ok_or("Verify current private source access before searching")?,
            search
                .remote_identity
                .as_ref()
                .ok_or("Private source account unavailable")?,
            &search.scope,
            &search.metadata_sources.discussions,
            cancel,
        )?;
    }
    let proofs = ghtui::search_sources::visibility::before(database, search)?;
    let result = match search.mode {
        ghtui::search::SearchMode::Ranked if search.blend_policy.enabled => {
            let policy = search.blend_policy.clone();
            let evidence = ghtui::search_blend::worker::execute(
                search,
                input,
                &policy,
                cache,
                index_root,
                database,
                remote.ok_or("Verify the selected account before explicit Ranked blending")?,
                cancel,
            )?;
            search.blend_evidence = Some(evidence);
            Ok(())
        }
        ghtui::search::SearchMode::Ranked => search
            .search_ranked(input, cache, index_root, cancel)
            .map_err(|error| error.to_string()),
        ghtui::search::SearchMode::GitHub => {
            let identity = search
                .remote_identity
                .clone()
                .ok_or("Remote identity is unavailable")?;
            let repositories = search.remote_repositories.clone();
            search
                .search_github(
                    input,
                    remote.ok_or("Remote GitHub provider is unavailable")?,
                    &identity,
                    &repositories,
                    cancel,
                )
                .map_err(|error| error.to_string())
        }
        _ => search
            .search_cached(input, cache, cancel)
            .map_err(|error| error.to_string()),
    };
    if result.is_ok() {
        if !blend && !search.metadata_sources.discussions.is_empty() {
            ghtui::search_sources::visibility::access(
                remote.ok_or("Private source provider unavailable")?,
                search
                    .remote_identity
                    .as_ref()
                    .ok_or("Private source account unavailable")?,
                &search.scope,
                &search.metadata_sources.discussions,
                cancel,
            )?;
        }
        ghtui::search_sources::visibility::after(database, &proofs)?;
    }
    result
}
fn verify_search_identity(
    search: &ghtui::search::workspace::SearchWorkspace,
    provider: &GithubProvider,
    cancel: &ghtui::providers::runner::CancellationToken,
) -> Result<(), String> {
    let current = provider.identity(cancel).map_err(|error| {
        format!("Identity verification failed • private search hidden: {error}")
    })?;
    if search
        .remote_identity
        .as_ref()
        .is_none_or(|expected| !expected.same_account(&current))
        || current.account_id.is_none()
    {
        return Err("Account changed during remote search • private results hidden; no prior scope is unlocked".into());
    }
    Ok(())
}

/// TASK-27: opens an existing work file of a PR branch edit in `$VISUAL` or
/// `$EDITOR`. The file stays where it is whatever the editor does.
fn edit_file_in_place(guard: &mut TerminalGuard, path: &std::path::Path) -> Result<()> {
    let editor = std::env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_else(|| "vi".into());
    let args = shell_words::split(&editor).map_err(anyhow::Error::msg)?;
    let program = args
        .first()
        .ok_or_else(|| anyhow::anyhow!("EDITOR is empty"))?;
    guard.suspend()?;
    let status = std::process::Command::new(program)
        .args(&args[1..])
        .arg(path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();
    guard.resume()?;
    let status = status?;
    if !status.success() {
        bail!("{program} exited with {status}");
    }
    Ok(())
}

fn external_editor(guard: &mut TerminalGuard, text: &str) -> Result<String> {
    external_editor_bounded(guard, text, ghtui::editor::MAX_TEXT_BYTES)
}
fn external_editor_bounded(guard: &mut TerminalGuard, text: &str, limit: usize) -> Result<String> {
    use std::io::{Read, Write};
    if text.len() > limit {
        bail!("Editor input exceeds {limit} UTF8 bytes; draft retained")
    }
    let editor = std::env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_else(|| "vi".into());
    let args = shell_words::split(&editor).map_err(anyhow::Error::msg)?;
    let program = args
        .first()
        .ok_or_else(|| anyhow::anyhow!("EDITOR is empty"))?;
    let mut draft = tempfile::Builder::new()
        .prefix("ghtui-draft-")
        .suffix(".md")
        .tempfile()?;
    draft.write_all(text.as_bytes())?;
    draft.flush()?;
    guard.suspend()?;
    let status = std::process::Command::new(program)
        .args(&args[1..])
        .arg(draft.path())
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();
    let resumed = guard.resume();
    resumed?;
    if !status?.success() {
        bail!("Editor exited without success");
    }
    let mut bytes = Vec::new();
    std::fs::File::open(draft.path())?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        if limit == ghtui::editor::MAX_TEXT_BYTES {
            bail!("Editor output exceeds 256 KiB");
        }
        bail!("Editor output exceeds {limit} UTF8 bytes; original inline draft retained");
    }
    String::from_utf8(bytes).map_err(Into::into)
}

/// TASK-105: the host and repository scope a Personal startup read uses.
fn personal_repository_scope_v105(
    config: &Config,
    repository: Option<&String>,
) -> ghtui::personal_readmission_v105::PersonalRepositoryScopeV105 {
    ghtui::personal_readmission_v105::PersonalRepositoryScopeV105 {
        host: config.host.clone(),
        repositories: repository
            .cloned()
            .map(|repository| vec![repository])
            .or_else(|| {
                config
                    .active_set
                    .as_ref()
                    .and_then(|set| config.repository_sets.get(set).cloned())
            })
            .unwrap_or_default(),
    }
}
fn main() {
    ghtui::providers::runner::initialize_workflow_environment_v9();
    if let Err(error) = run() {
        eprintln!("ghtui: {error:#}");
        std::process::exit(1);
    }
}
