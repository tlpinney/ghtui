//! TASK-92: the list plus preview split. At [`crate::ui_chrome::SPLIT_MIN_WIDTH`] columns or
//! more the Inbox queue and the Actions Runs list show the selected item in a preview pane
//! beside the list; narrower screens keep drilling down with Enter. A preview only presents
//! data the list already holds (the cached PR summary, the loaded run and its loaded jobs): it
//! never starts a read, so moving the cursor costs no `gh` command.
use crate::{
    actions_workspace::{status_role, ActionsView, ActionsWorkspace},
    app::{Action, App, HitRegion, View},
    github_workspace::{GithubAction, GithubWorkspace},
    identity::{ActorIdentity, InitialsChip},
    providers::{gh::PullRequest, runner::sanitize_terminal},
    theme::{ColorMode, Role, SemanticToken, Theme},
    ui_chrome::{key_token_line, label_value, pane_block, pane_styles},
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

/// Which pane of a split owns the content focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pane {
    #[default]
    List,
    Preview,
}

/// Width of the label column in preview detail rows.
const LABEL: usize = 8;
/// The key that opens the previewed item, shown in the preview's top border.
const OPEN_KEY: &str = "Enter";

/// Whether the current view is a list that has a preview (whatever the width).
pub fn offered(app: &App) -> bool {
    match app.view {
        View::Inbox => app.github.as_ref().is_some_and(|github| {
            github.pr.is_none()
                && github.scoped_pr_navigation.is_none()
                && !crate::scoped_pr_collaboration_native_v17::retired_presentation(app)
        }),
        View::Actions => {
            app.scoped_actions_v1.is_none()
                && app
                    .actions
                    .as_ref()
                    .is_some_and(|actions| actions.view == ActionsView::Runs)
        }
        View::Repository => {
            crate::repository_panels_v93::active(app) || crate::history::detail::offered(app)
        }
        _ => false,
    }
}

/// The terminal width from which the current view's list and preview sit side by side.
pub fn min_width(app: &App) -> u16 {
    match app.view {
        // History's commit detail splits at the shared width (TASK-99.4).
        View::Repository if crate::history::detail::offered(app) => {
            crate::ui_chrome::SPLIT_MIN_WIDTH
        }
        View::Repository => crate::repository_panels_v93::SPLIT_MIN_WIDTH,
        _ => crate::ui_chrome::SPLIT_MIN_WIDTH,
    }
}

/// The current view's list and preview areas: History's commit detail is a narrower column.
pub fn split(app: &App, area: Rect) -> crate::ui_chrome::ListPreview {
    if app.view == View::Repository && crate::history::detail::offered(app) {
        crate::ui_chrome::split_list_detail(area)
    } else {
        crate::ui_chrome::split_list_preview(area)
    }
}

/// The current view's content focus flag.
fn content_focused(app: &App) -> bool {
    match app.view {
        View::Inbox => app.github_content_focused,
        View::Actions => app.actions_content_focused,
        View::Repository => app.repository_content_focused,
        _ => false,
    }
}

/// Whether the list (not the preview) shows the focused selection bar.
pub fn list_focused(app: &App) -> bool {
    app.preview_area_v92.is_none() || app.content_pane == Pane::List
}

/// Whether the preview pane owns the focus.
pub fn preview_focused(app: &App) -> bool {
    app.preview_area_v92.is_some()
        && app.content_pane == Pane::Preview
        && app.overlay.is_none()
        && content_focused(app)
}

/// Gives the pane focus (a click or Tab); the list keeps its selection.
pub fn focus(app: &mut App, pane: Pane) {
    if pane == Pane::Preview && app.preview_area_v92.is_none() {
        return;
    }
    if app.content_pane != pane {
        app.preview_scroll_v92 = 0;
    }
    app.content_pane = pane;
    match app.view {
        View::Inbox => app.github_content_focused = true,
        View::Actions => app.actions_content_focused = true,
        View::Repository => app.repository_content_focused = true,
        _ => {}
    }
    app.status = match pane {
        Pane::List => format!("{} list focused", app.view.label()),
        Pane::Preview if crate::history::detail::offered(app) => {
            "Commit detail focused • ↑↓ select • Enter opens • Tab leaves".into()
        }
        Pane::Preview => "Preview focused • ↑↓ scroll • Enter opens • Tab leaves".into(),
    };
}

/// The action that opens the previewed item: the same one Enter on its list row dispatches.
pub fn open_action(app: &App) -> Option<Action> {
    match app.view {
        View::Inbox => app
            .github
            .as_ref()
            .and_then(selected_pr)
            .map(|pr| Action::Github(GithubAction::Open(pr.into()))),
        View::Actions => app.actions.as_ref().and_then(|actions| {
            actions
                .rows
                .get(actions.selected)
                .and_then(|row| row.action.clone())
                .map(Action::Actions)
        }),
        View::Repository if crate::history::detail::offered(app) => Some(
            Action::HistoryDetailV994(crate::history::detail::DetailUiActionV994::Activate),
        ),
        View::Repository => crate::repository_panels_v93::open_action(app),
        _ => None,
    }
}

/// Split keys before the view's own: Tab moves List → Preview (then on to the controls),
/// Shift-Tab returns to the list; the focused preview scrolls with ↑↓ and opens with Enter.
/// Returns whether the key was used.
pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if app.overlay.is_some() || app.preview_area_v92.is_none() || !offered(app) {
        return false;
    }
    let focused = content_focused(app);
    match (key.code, app.content_pane) {
        (KeyCode::Tab, Pane::List) if focused => {
            focus(app, Pane::Preview);
            true
        }
        // Leaving the preview continues the ordinary Tab traversal of the controls.
        (KeyCode::Tab, Pane::Preview) => {
            app.content_pane = Pane::List;
            false
        }
        (KeyCode::BackTab, Pane::Preview) if focused => {
            focus(app, Pane::List);
            true
        }
        (KeyCode::Up | KeyCode::Char('k'), Pane::Preview) if focused => {
            app.preview_scroll_v92 = app.preview_scroll_v92.saturating_sub(1);
            true
        }
        (KeyCode::Down | KeyCode::Char('j'), Pane::Preview) if focused => {
            app.preview_scroll_v92 = app.preview_scroll_v92.saturating_add(1);
            true
        }
        (KeyCode::PageUp, Pane::Preview) if focused => {
            app.preview_scroll_v92 = app.preview_scroll_v92.saturating_sub(10);
            true
        }
        (KeyCode::PageDown, Pane::Preview) if focused => {
            app.preview_scroll_v92 = app.preview_scroll_v92.saturating_add(10);
            true
        }
        (KeyCode::Enter, Pane::Preview) if focused => {
            if let Some(action) = open_action(app) {
                app.dispatch(action);
            }
            true
        }
        _ => false,
    }
}

/// Mouse on the preview pane: a click focuses it, the wheel scrolls it. A click anywhere else
/// returns the focus to the list pane. Returns whether the event was used.
pub fn mouse(app: &mut App, column: u16, row: u16, click: bool, wheel: Option<isize>) -> bool {
    if app.overlay.is_some() || !offered(app) {
        return false;
    }
    let Some(area) = app.preview_area_v92 else {
        return false;
    };
    let inside = area.contains((column, row).into());
    if !inside {
        if click {
            app.content_pane = Pane::List;
        }
        return false;
    }
    // The border's Open token is an ordinary hit; let the click reach it.
    let on_hit = app
        .hits
        .iter()
        .any(|hit| hit.rect.contains((column, row).into()));
    if let Some(delta) = wheel {
        app.preview_scroll_v92 = app.preview_scroll_v92.saturating_add_signed(delta);
        return true;
    }
    if click && !on_hit {
        focus(app, Pane::Preview);
        return true;
    }
    false
}

/// What a preview shows: its title, an optional status badge for the right border, and lines.
pub struct Preview {
    /// Identifies the previewed item; a different item starts at the top.
    pub item: String,
    pub title: String,
    pub badge: Option<(String, SemanticToken)>,
    pub lines: Vec<Line<'static>>,
}

/// The pull request the Inbox bar is on (the same index the queue paints).
fn selected_pr(github: &GithubWorkspace) -> Option<&PullRequest> {
    github.visible().get(github.selected_index()).copied()
}

/// An initials chip and login, as the tables paint an actor.
fn actor_spans(theme: Theme, identity: &ActorIdentity) -> Vec<Span<'static>> {
    let style = if theme.mode == ColorMode::NoColor {
        theme.style(SemanticToken::Actor)
    } else {
        theme.identity_chip(InitialsChip::slot(identity))
    };
    vec![
        Span::styled(InitialsChip::text(identity, theme.mode), style),
        Span::raw(" "),
        Span::styled(
            sanitize_terminal(&crate::ui_table::actor_text(identity)),
            theme.role(Role::Text),
        ),
    ]
}

fn label(theme: Theme, text: &str) -> Span<'static> {
    let pad = LABEL.saturating_sub(crate::editor::display_width(text));
    Span::styled(
        format!("{text}{} ", " ".repeat(pad)),
        theme.role(Role::TextMuted),
    )
}

fn heading(theme: Theme, text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_uppercase(),
        theme
            .role(Role::Accent)
            .add_modifier(ratatui::style::Modifier::BOLD),
    ))
}

/// The cached Inbox summary of the selected pull request. Only fields the queue read already
/// returned are shown; reviews, checks and the diff load when the PR opens.
pub fn inbox(github: &GithubWorkspace, theme: Theme, width: usize) -> Preview {
    let Some(pr) = selected_pr(github) else {
        return Preview {
            item: String::new(),
            title: "Preview".into(),
            badge: None,
            lines: vec![Line::from(Span::styled(
                "No pull request selected",
                theme.role(Role::TextMuted),
            ))],
        };
    };
    let state = if pr.draft {
        format!("{} · draft", pr.state)
    } else {
        pr.state.clone()
    };
    let identity = ActorIdentity::github(&github.identity.host, &pr.author);
    let mut author = vec![label(theme, "Author")];
    author.extend(actor_spans(theme, &identity));
    // An unknown opening time stays unknown rather than reading as `opened — ago`.
    if let Some(age) = crate::ui_table::age_at(&pr.created_at, crate::storage::now()) {
        author.push(Span::styled(
            format!(" · opened {age} ago"),
            theme.role(Role::TextMuted),
        ));
    }
    let queues = pr
        .queues
        .iter()
        .map(|queue| match queue {
            crate::providers::gh::Queue::Authored => "Created",
            crate::providers::gh::Queue::Assigned => "Assigned",
            crate::providers::gh::Queue::ReviewRequested => "Review requested",
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut lines = vec![
        Line::from(Span::styled(
            sanitize_terminal(&pr.title),
            theme
                .role(Role::Text)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                sanitize_terminal(&format!("{}#{}", pr.repository, pr.number)),
                theme.role(Role::Text),
            ),
            Span::raw("  "),
            Span::styled(
                sanitize_terminal(&state.to_uppercase()),
                theme.style(if pr.state.eq_ignore_ascii_case("open") {
                    SemanticToken::Success
                } else {
                    SemanticToken::Muted
                }),
            ),
        ]),
        Line::default(),
        Line::from(author),
        label_value(theme, "Opened", &pr.created_at, LABEL),
        label_value(theme, "Updated", &pr.updated_at, LABEL),
        label_value(
            theme,
            "Queues",
            if queues.is_empty() { "—" } else { &queues },
            LABEL,
        ),
        label_value(theme, "Link", &pr.url, LABEL),
        Line::default(),
        heading(theme, "Reviewers"),
    ];
    lines.extend(reviewer_lines(github, pr, theme));
    lines.push(Line::default());
    lines.push(heading(theme, "Description"));
    // The description reads as GitHub renders it (TASK-114): no raw HTML or Markdown syntax.
    let body = crate::markdown_v114::lines(theme, &pr.body, width.max(1));
    if body.is_empty() {
        lines.push(Line::from(Span::styled(
            "No description",
            theme.role(Role::TextMuted),
        )));
    } else {
        lines.extend(body);
    }
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "Threads, checks and files load when the PR opens",
        theme.role(Role::TextDim),
    )));
    Preview {
        item: format!("pr\0{}\0{}", pr.repository, pr.number),
        title: "Preview".into(),
        badge: None,
        lines,
    }
}

/// One line per reviewer or commenter from the Inbox read, with where each stands. The PR's
/// author is left out (the Author row names them); a PR the read skipped says so.
fn reviewer_lines(github: &GithubWorkspace, pr: &PullRequest, theme: Theme) -> Vec<Line<'static>> {
    use crate::providers::gh::ReviewerRoleV1;
    let key = crate::inbox_status_v1::row_key(&pr.host, &pr.repository, pr.number);
    let Some(reviewers) = github
        .identity
        .account_id
        .and(github.inbox_reviewers_v1.get(&key))
    else {
        return vec![Line::from(Span::styled(
            "Not loaded yet • press r to refresh",
            theme.role(Role::TextMuted),
        ))];
    };
    let lines: Vec<Line<'static>> = reviewers
        .iter()
        .filter(|reviewer| {
            reviewer.role != ReviewerRoleV1::Commented
                || !reviewer.actor.login.eq_ignore_ascii_case(&pr.author.login)
        })
        .map(|reviewer| {
            let identity = ActorIdentity::github(&github.identity.host, &reviewer.actor);
            let token = match reviewer.role {
                ReviewerRoleV1::Approved => SemanticToken::Success,
                ReviewerRoleV1::ChangesRequested => SemanticToken::Failure,
                ReviewerRoleV1::Requested => SemanticToken::Pending,
                _ => SemanticToken::Muted,
            };
            let mut spans = actor_spans(theme, &identity);
            spans.push(Span::styled(" · ", theme.role(Role::TextMuted)));
            spans.push(Span::styled(reviewer.role.label(), theme.style(token)));
            Line::from(spans)
        })
        .collect();
    if lines.is_empty() {
        return vec![Line::from(Span::styled(
            "No reviewers or comments yet",
            theme.role(Role::TextMuted),
        ))];
    }
    lines
}

/// The selected run from the loaded Runs list, with its jobs when this run's attempt is
/// already loaded (after opening it); nothing is read for the preview.
pub fn run(actions: &ActionsWorkspace, theme: Theme) -> Preview {
    let Some(item) = actions.runs.get(actions.selected) else {
        return Preview {
            item: String::new(),
            title: "Preview".into(),
            badge: None,
            lines: vec![Line::from(Span::styled(
                "No run selected",
                theme.role(Role::TextMuted),
            ))],
        };
    };
    let run = &item.run;
    let role = status_role(&run.status, &run.conclusion);
    let outcome = if run.conclusion.is_empty() {
        run.status.clone()
    } else {
        run.conclusion.clone()
    };
    let identity = ActorIdentity::github(&actions.identity.host, &run.actor);
    let mut second = vec![Span::styled(
        crate::ui_table::short_sha(&run.head_sha).to_string(),
        theme.role(Role::Commit),
    )];
    second.push(Span::styled(" · ", theme.role(Role::TextMuted)));
    second.extend(actor_spans(theme, &identity));
    let took = (run.status == "completed")
        .then(|| crate::ui_table::duration(&run.created_at, &run.updated_at))
        .flatten();
    second.push(Span::styled(
        format!(
            "{} · attempt {}",
            took.map(|took| format!(" · {took}")).unwrap_or_default(),
            run.attempt
        ),
        theme.role(Role::TextMuted),
    ));
    let mut lines = vec![
        Line::from(Span::styled(
            sanitize_terminal(&format!(
                "{} · {} · {}",
                run.name, run.event, run.head_branch
            )),
            theme
                .role(Role::Text)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )),
        Line::from(second),
        Line::default(),
        label_value(theme, "Repo", &item.repository, LABEL),
        label_value(theme, "Commit", &run.head_sha, LABEL),
        label_value(theme, "Created", &run.created_at, LABEL),
        label_value(theme, "Updated", &run.updated_at, LABEL),
        Line::default(),
        heading(theme, "Jobs"),
    ];
    let loaded = actions.run.as_ref().filter(|page| {
        page.repository == item.repository
            && page.attempt.run.id == run.id
            && page.attempt.run.attempt == run.attempt
    });
    match loaded {
        Some(page) if !page.attempt.jobs.is_empty() => {
            let name_width = page
                .attempt
                .jobs
                .iter()
                .map(|job| crate::editor::display_width(&sanitize_terminal(&job.name)))
                .max()
                .unwrap_or(0)
                .min(28);
            for job in &page.attempt.jobs {
                let role = status_role(&job.status, &job.conclusion);
                let state = if job.conclusion.is_empty() {
                    &job.status
                } else {
                    &job.conclusion
                };
                let took = crate::ui_table::duration(&job.started_at, &job.completed_at)
                    .unwrap_or_else(|| crate::ui_table::NONE.into());
                let name = crate::ui_chrome::fit(&sanitize_terminal(&job.name), name_width);
                let pad = name_width.saturating_sub(crate::editor::display_width(&name));
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", role.symbol()), theme.style(role)),
                    Span::styled(
                        format!("{name}{}  ", " ".repeat(pad)),
                        theme.role(Role::Text),
                    ),
                    Span::styled(
                        format!("{:<10}", sanitize_terminal(&state.to_uppercase())),
                        theme.style(role),
                    ),
                    Span::styled(format!(" {took:>6}"), theme.role(Role::TextMuted)),
                ]));
            }
        }
        Some(_) => lines.push(Line::from(Span::styled(
            "No jobs reported for this attempt",
            theme.role(Role::TextMuted),
        ))),
        None => lines.push(Line::from(Span::styled(
            "Jobs load when the run opens (Enter)",
            theme.role(Role::TextMuted),
        ))),
    }
    Preview {
        item: format!("run\0{}\0{}\0{}", item.repository, run.id, run.attempt),
        // The run cell's own form (`123 #2` for a later attempt), so the run reads the same here.
        title: if run.attempt > 1 {
            format!("Run {} #{}", run.id, run.attempt)
        } else {
            format!("Run {}", run.id)
        },
        badge: Some((
            format!("{} {}", role.symbol(), outcome.to_uppercase()),
            role,
        )),
        lines,
    }
}

/// Paints the preview pane into `area` and registers its Open token (the same action Enter
/// gives). The pane carries the focus marker while it owns the focus.
pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    // History draws its commit detail with item links and its own changed-files read.
    if app.view == View::Repository && crate::history::detail::offered(app) {
        crate::history::detail::draw_pane(frame, app, area);
        return;
    }
    let theme = app.theme();
    let preview = match app.view {
        View::Inbox => app.github.as_ref().map(|github| {
            // The body column inside the borders and the one-cell padding on each side.
            inbox(github, theme, usize::from(area.width.saturating_sub(4)))
        }),
        View::Actions => app.actions.as_ref().map(|actions| run(actions, theme)),
        View::Repository => Some(crate::repository_panels_v93::preview(app, theme)),
        _ => None,
    };
    let Some(preview) = preview else {
        return;
    };
    let focused = preview_focused(app);
    let (border, _) = pane_styles(theme, focused);
    let mut block = pane_block(theme, &[&preview.title], focused);
    let open = open_action(app);
    // ` ✗ FAILURE ─ Enter Open ─╮`: the badge, then the clickable Open token.
    let mut spans = vec![];
    if let Some((badge, role)) = &preview.badge {
        spans.push(Span::styled(
            format!(" {} ", sanitize_terminal(badge)),
            theme.style(*role),
        ));
        spans.push(Span::styled("─", border));
    }
    // The token's hit is pushed next, so it is focused when the focus index points there.
    let token_focused = open.is_some() && app.focus == app.hits.len() && !content_focused(app);
    let token = key_token_line(theme, OPEN_KEY, "Open", token_focused);
    let token_width = token.width() as u16;
    let badge_width = Line::from(spans.clone()).width() as u16;
    let title_width =
        crate::editor::display_width(&crate::ui_chrome::crumb_title(&[&preview.title], focused))
            as u16;
    let mut open_rect = None;
    if open.is_some() && area.width > title_width + badge_width + token_width + 7 {
        // Right-aligned titles end one border cell before the corner.
        let x = area.right() - 2 - token_width - 1;
        open_rect = Some(Rect::new(x, area.y, token_width + 2, 1));
        spans.push(Span::styled(" ", theme.role(Role::TextMuted)));
        spans.extend(token.spans);
        spans.push(Span::styled(" ", theme.role(Role::TextMuted)));
        spans.push(Span::styled("─", border));
    }
    if !spans.is_empty() && area.width > title_width + badge_width + 4 {
        block = block.title(Line::from(spans).right_aligned());
    }
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let body = Rect::new(
        inner.x.saturating_add(1),
        inner.y,
        inner.width.saturating_sub(2),
        inner.height,
    );
    if app.preview_item_v92 != preview.item {
        app.preview_item_v92 = preview.item.clone();
        app.preview_scroll_v92 = 0;
    }
    // The scroll stops once the last row is in view (rows counted as wrapped at the width;
    // word wrapping can only add rows, so the end always stays reachable).
    let width = usize::from(body.width.max(1));
    let rows = preview
        .lines
        .iter()
        .map(|line| line.width().div_ceil(width).max(1))
        .sum::<usize>();
    app.preview_scroll_v92 = app
        .preview_scroll_v92
        .min(rows.saturating_sub(usize::from(body.height)));
    let paragraph = Paragraph::new(preview.lines).wrap(Wrap { trim: false });
    frame.render_widget(
        paragraph
            .style(theme.base())
            .scroll((app.preview_scroll_v92.min(u16::MAX as usize) as u16, 0)),
        body,
    );
    // Open targets, all with the action Enter gives: the `Enter Open` token, the pane title
    // naming the item and the item's headline row while it is in view.
    let Some(action) = open else {
        return;
    };
    let content = content_focused(app);
    let mut targets = vec![];
    if let Some(rect) = open_rect {
        targets.push(rect);
    }
    let title_rect = Rect::new(
        area.x + 2,
        area.y,
        title_width.min(area.width.saturating_sub(4)),
        1,
    );
    targets.push(title_rect);
    if app.preview_scroll_v92 == 0 && !body.is_empty() {
        targets.push(Rect::new(body.x, body.y, body.width, 1));
    }
    for rect in targets {
        // The title and headline show keyboard focus like any control (the token draws its own).
        if Some(rect) != open_rect && !content && app.focus == app.hits.len() {
            frame.buffer_mut().set_style(
                rect,
                ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::UNDERLINED),
            );
        }
        app.hits.push(HitRegion {
            rect,
            action: action.clone(),
        });
    }
}

#[cfg(test)]
#[path = "list_preview_v92_tests.rs"]
mod tests;
