//! TASK-92 list plus preview split: side by side from 120 columns, drill-down below; the Inbox
//! preview shows only the cached PR summary (no read is queued while the cursor moves); the
//! Actions Runs preview shows the selected run and its jobs once loaded; Tab, Shift-Tab and a
//! click move the pane focus, whose `▶` marker and accent bar follow it.
use super::*;
use crate::{
    actions_workspace::{RunPage, ScopedRun},
    github_workspace::{GithubAction, GithubWorkspace, PrKey},
    providers::gh::{ActionsJob, Actor, HostIdentity, PullRequest, Queue, RunAttempt, WorkflowRun},
    ui_chrome::{split_list_preview, FOCUS_MARKER, SPLIT_MIN_WIDTH},
};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};
use std::collections::BTreeSet;

fn identity() -> HostIdentity {
    HostIdentity {
        host: "github.invalid".into(),
        login: "thowd22".into(),
        account_id: Some(42),
        gh_version: "offline".into(),
    }
}

fn actor(login: &str) -> Actor {
    Actor {
        login: login.into(),
        display_name: None,
        avatar_url: None,
        actor_type: "User".into(),
    }
}

fn pull_request(number: u64, title: &str, author: &str) -> PullRequest {
    PullRequest {
        host: "github.invalid".into(),
        id: format!("PR_{number}"),
        repository: "example/actual".into(),
        number,
        title: title.into(),
        body: format!("Body of pull request {number}.\nSecond line {number}."),
        url: format!("https://github.invalid/example/actual/pull/{number}"),
        author: actor(author),
        state: "open".into(),
        draft: number == 3,
        created_at: format!("2026-01-0{number}T10:00:00Z"),
        updated_at: format!("2026-01-0{number}T12:30:00Z"),
        queues: BTreeSet::from([Queue::Authored]),
    }
}

fn inbox() -> App {
    let mut github = GithubWorkspace::empty(identity(), vec![], BTreeSet::new());
    github.inbox.pull_requests = vec![
        pull_request(1, "Fix dark playback", "thowd22"),
        pull_request(2, "Theme tokens for panes", "sam-ops"),
        pull_request(3, "Pin runner images", "kjones"),
    ];
    github.selected = github.inbox.pull_requests.first().map(Into::into);
    App {
        view: View::Inbox,
        github: Some(github),
        github_content_focused: true,
        ..Default::default()
    }
}

fn run(id: u64, name: &str, conclusion: &str) -> ScopedRun {
    ScopedRun {
        repository: "example/repo".into(),
        run: WorkflowRun {
            id,
            workflow_id: 7,
            name: name.into(),
            title: "Release".into(),
            head_sha: format!("b7cdcd3{id:033}"),
            head_branch: "main".into(),
            event: "workflow_dispatch".into(),
            status: "completed".into(),
            conclusion: conclusion.into(),
            attempt: 1,
            actor: actor("thowd22"),
            triggering_actor: actor("thowd22"),
            created_at: "2026-10-13T12:00:00Z".into(),
            updated_at: "2026-10-13T12:08:37Z".into(),
            url: String::new(),
        },
    }
}

fn job(id: u64, name: &str, conclusion: &str) -> ActionsJob {
    ActionsJob {
        id,
        run_id: 124,
        attempt: 1,
        head_sha: String::new(),
        name: name.into(),
        status: "completed".into(),
        conclusion: conclusion.into(),
        started_at: "2026-10-13T12:00:00Z".into(),
        completed_at: "2026-10-13T12:08:29Z".into(),
        runner_id: 0,
        runner_name: String::new(),
        runner_group_name: String::new(),
        labels: vec![],
        check_run_url: String::new(),
        steps: vec![],
    }
}

fn actions() -> App {
    let mut workspace =
        crate::actions_workspace::ActionsWorkspace::new(identity(), vec!["example/repo".into()]);
    workspace.runs = vec![run(123, "CI", "success"), run(124, "Release", "failure")];
    // Run 124 was opened before: its attempt and jobs are loaded; run 123's are not.
    workspace.run = Some(RunPage {
        repository: "example/repo".into(),
        attempt: RunAttempt {
            run: workspace.runs[1].run.clone(),
            jobs: vec![
                job(1, "build aarch64-musl", "cancelled"),
                job(2, "build x86_64-musl", "failure"),
            ],
            definition_note: String::new(),
        },
        selected_job: None,
        logs: None,
        logs_skipped_bytes: 0,
        annotations: vec![],
    });
    workspace.prepare_rows();
    App {
        view: View::Actions,
        actions: Some(workspace),
        actions_content_focused: true,
        ..Default::default()
    }
}

fn paint(app: &mut App, width: u16, height: u16) -> (Buffer, Vec<String>) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| crate::ui::draw(frame, app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let lines = (0..height)
        .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
        .collect();
    (buffer, lines)
}

fn key(app: &mut App, code: KeyCode) {
    let modifiers = if code == KeyCode::BackTab {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    app.handle_event(Event::Key(KeyEvent::new(code, modifiers)));
}

fn click(app: &mut App, column: u16, row: u16) {
    app.handle_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }));
}

/// The text of the screen's columns `[from, to)`.
fn columns(lines: &[String], from: u16, to: u16) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            line.chars()
                .skip(usize::from(from))
                .take(usize::from(to - from))
                .collect()
        })
        .collect()
}

fn holds(lines: &[String], needle: &str) -> bool {
    lines.iter().any(|line| line.contains(needle))
}

/// The top border row of the pane that starts at column `x`.
fn title(lines: &[String], area: Rect) -> String {
    columns(lines, area.x, area.right())[usize::from(area.y)].clone()
}

#[test]
fn design_split_v92_split_list_preview_is_side_by_side_from_120_columns_only() {
    let narrow = split_list_preview(Rect::new(0, 1, SPLIT_MIN_WIDTH - 1, 30));
    assert_eq!(narrow.list, Rect::new(0, 1, 119, 30));
    assert_eq!(narrow.preview, None);
    for width in [SPLIT_MIN_WIDTH, 140, 200] {
        let split = split_list_preview(Rect::new(0, 1, width, 30));
        let preview = split.preview.unwrap();
        assert_eq!(split.list.x, 0);
        assert_eq!(split.list.right(), preview.x, "the panes touch");
        assert_eq!(preview.right(), width, "together they fill the area");
        assert_eq!((split.list.y, preview.y, preview.height), (1, 1, 30));
        assert!(
            split.list.width > preview.width,
            "the list keeps the larger share"
        );
    }
    assert_eq!(split_list_preview(Rect::new(0, 0, 140, 30)).list.width, 84);
}

#[test]
fn design_split_v92_inbox_preview_shows_the_cached_summary_beside_the_list_without_reads() {
    let mut app = inbox();
    let (_, lines) = paint(&mut app, 140, 42);
    let preview = app
        .preview_area_v92
        .expect("a preview beside the list at 140");
    assert_eq!(preview.x, 84);
    let right = columns(&lines, preview.x, preview.right());
    let left = columns(&lines, 0, preview.x);
    assert!(
        title(&lines, preview).contains("Preview"),
        "{}",
        lines.join("\n")
    );
    assert!(
        title(&lines, preview).contains("Enter Open"),
        "{}",
        lines.join("\n")
    );
    // The selected (first) row's cached summary: title, reference, author chip, both full UTC
    // timestamps, link and body; nothing that would need another read.
    for needle in [
        "Fix dark playback",
        "example/actual#1",
        "OPEN",
        "TH thowd22",
        "2026-01-01T10:00:00Z",
        "2026-01-01T12:30:00Z",
        "https://github.invalid/example/actual/pull/1",
        "DESCRIPTION",
        "Body of pull request 1.",
        "Second line 1.",
    ] {
        assert!(
            holds(&right, needle),
            "{needle} missing:\n{}",
            right.join("\n")
        );
    }
    // The list keeps its table on the left with every row and no preview text.
    for needle in ["example/actual#1", "example/actual#2", "example/actual#3"] {
        assert!(holds(&left, needle), "{}", left.join("\n"));
    }
    assert!(!holds(&left, "Body of pull request"));
    // Moving the cursor re-presents the next cached PR; no gh read or refresh is queued.
    key(&mut app, KeyCode::Down);
    let (_, lines) = paint(&mut app, 140, 42);
    let right = columns(&lines, preview.x, preview.right());
    assert!(
        holds(&right, "Theme tokens for panes"),
        "{}",
        right.join("\n")
    );
    assert!(holds(&right, "SO sam-ops"), "{}", right.join("\n"));
    assert!(!holds(&right, "Fix dark playback"));
    key(&mut app, KeyCode::Down);
    let (_, lines) = paint(&mut app, 140, 42);
    let right = columns(&lines, preview.x, preview.right());
    assert!(holds(&right, "OPEN · DRAFT"), "{}", right.join("\n"));
    assert!(app.github_action.is_none(), "{:?}", app.github_action);
    assert!(!app.refresh_requested);
    assert!(app.github.as_ref().unwrap().pr.is_none());
}

#[test]
fn design_split_v92_inbox_drills_down_below_120_columns() {
    for (width, height) in [(80u16, 24u16), (119, 30)] {
        let mut app = inbox();
        let (_, lines) = paint(&mut app, width, height);
        assert_eq!(app.preview_area_v92, None, "{width}");
        assert!(
            !holds(&lines, "Body of pull request"),
            "{}",
            lines.join("\n")
        );
        assert!(!holds(&lines, "─ Preview"), "{}", lines.join("\n"));
        assert!(holds(&lines, "example/actual#3"));
        // Enter drills down: it opens the selected PR page (the read runs in the runtime).
        key(&mut app, KeyCode::Enter);
        assert_eq!(
            app.github_action,
            Some(GithubAction::Open(PrKey::from(
                &app.github.as_ref().unwrap().inbox.pull_requests[0]
            )))
        );
    }
}

#[test]
fn design_split_v92_tab_shift_tab_and_clicks_move_the_pane_focus_marker() {
    let mut app = inbox();
    let (buffer, lines) = paint(&mut app, 140, 42);
    let preview = app.preview_area_v92.unwrap();
    let list_title = |lines: &[String]| title(lines, Rect::new(0, preview.y, preview.x, 1));
    let marker = format!("{FOCUS_MARKER} Preview");
    assert!(
        list_title(&lines).contains(FOCUS_MARKER),
        "{}",
        list_title(&lines)
    );
    assert!(!title(&lines, preview).contains(&marker));
    let theme = app.theme();
    let bar_row = (0..42u16)
        .find(|y| buffer[(1, *y)].symbol() == crate::ui_table::SELECTION_BAR)
        .expect("the list bar");
    assert_eq!(
        buffer[(1, bar_row)].fg,
        theme.role(Role::Accent).fg.unwrap()
    );

    // Tab: the preview takes the focus; the list keeps its selection with a dim bar.
    key(&mut app, KeyCode::Tab);
    assert_eq!(app.content_pane, Pane::Preview);
    let (buffer, lines) = paint(&mut app, 140, 42);
    assert!(
        title(&lines, preview).contains(&marker),
        "{}",
        title(&lines, preview)
    );
    assert!(
        !list_title(&lines).contains(FOCUS_MARKER),
        "{}",
        list_title(&lines)
    );
    assert_eq!(
        buffer[(1, bar_row)].symbol(),
        crate::ui_table::SELECTION_BAR
    );
    assert_eq!(
        buffer[(1, bar_row)].fg,
        theme.role(Role::TextDim).fg.unwrap()
    );
    // ↓ scrolls the preview, not the list; Enter opens the previewed PR.
    key(&mut app, KeyCode::Down);
    assert_eq!(app.preview_scroll_v92, 1);
    assert_eq!(app.github.as_ref().unwrap().selected_index(), 0);
    // Shift-Tab returns to the list.
    key(&mut app, KeyCode::BackTab);
    assert_eq!(app.content_pane, Pane::List);
    assert!(app.github_content_focused);

    // A click on the preview body focuses it; a click on a list row opens that row's PR as
    // before and returns the focus to the list pane.
    paint(&mut app, 140, 42);
    click(&mut app, preview.x + 10, preview.y + 6);
    assert_eq!(app.content_pane, Pane::Preview);
    assert!(app.github_action.is_none(), "a preview click reads nothing");
    let (_, lines) = paint(&mut app, 140, 42);
    assert!(title(&lines, preview).contains(&marker));
    let row = lines
        .iter()
        .position(|line| line.contains("example/actual#2"))
        .unwrap() as u16;
    click(&mut app, 5, row);
    assert_eq!(app.content_pane, Pane::List);
    assert_eq!(
        app.github_action,
        Some(GithubAction::Open(PrKey::from(
            &app.github.as_ref().unwrap().inbox.pull_requests[1]
        )))
    );

    // The border's `Enter Open` token is clickable and Enter in the focused preview opens the
    // same selected PR.
    let mut app = inbox();
    paint(&mut app, 140, 42);
    let open = app
        .hits
        .iter()
        .find(|hit| hit.rect.y == preview.y && hit.rect.x > preview.x)
        .cloned()
        .expect("the Open token hit");
    let expected = Some(GithubAction::Open(PrKey::from(
        &app.github.as_ref().unwrap().inbox.pull_requests[0],
    )));
    click(&mut app, open.rect.x + 1, open.rect.y);
    assert_eq!(app.github_action, expected);
    let mut app = inbox();
    paint(&mut app, 140, 42);
    key(&mut app, KeyCode::Tab);
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.github_action, expected);
}

#[test]
fn design_split_v92_actions_runs_preview_shows_the_run_and_its_loaded_jobs() {
    let mut app = actions();
    let (_, lines) = paint(&mut app, 140, 42);
    let preview = app.preview_area_v92.expect("Runs preview at 140");
    let right = columns(&lines, preview.x, preview.right());
    // Run 123's jobs were never read: the preview says so instead of reading them.
    assert!(
        title(&lines, preview).contains("Run 123"),
        "{}",
        lines.join("\n")
    );
    assert!(title(&lines, preview).contains("✓ SUCCESS"));
    for needle in [
        "CI · workflow_dispatch · main",
        "b7cdcd3 · TH thowd22 · 8m",
        &format!("b7cdcd3{:033}", 123),
        "2026-10-13T12:00:00Z",
        "JOBS",
        "Jobs load when the run opens (Enter)",
    ] {
        assert!(holds(&right, needle), "{needle}:\n{}", right.join("\n"));
    }
    key(&mut app, KeyCode::Down);
    let (_, lines) = paint(&mut app, 140, 42);
    let right = columns(&lines, preview.x, preview.right());
    assert!(title(&lines, preview).contains("Run 124"));
    assert!(title(&lines, preview).contains("✗ FAILURE"));
    let cancelled = right
        .iter()
        .find(|line| line.contains("build aarch64-musl"))
        .unwrap_or_else(|| panic!("{}", right.join("\n")));
    let failed = right
        .iter()
        .find(|line| line.contains("build x86_64-musl"))
        .unwrap();
    assert!(
        cancelled.contains("⊘") && cancelled.contains("CANCELLED"),
        "{cancelled}"
    );
    assert!(
        failed.contains("✗") && failed.contains("FAILURE"),
        "{failed}"
    );
    // Status and duration columns line up.
    assert_eq!(cancelled.find("CANCELLED"), failed.find("FAILURE"));
    assert!(cancelled.contains("8m") && failed.contains("8m"));
    assert!(app.actions_action.is_none(), "{:?}", app.actions_action);
    // Enter in the focused preview opens the run, as Enter on its row does.
    key(&mut app, KeyCode::Tab);
    assert_eq!(app.content_pane, Pane::Preview);
    key(&mut app, KeyCode::Enter);
    assert!(
        matches!(
            app.actions_action,
            Some(crate::actions_workspace::ActionsAction::OpenRun { id: 124, .. })
        ),
        "{:?}",
        app.actions_action
    );
    // Below 120 columns the Runs list keeps the whole pane.
    let mut app = actions();
    let (_, lines) = paint(&mut app, 80, 24);
    assert_eq!(app.preview_area_v92, None);
    assert!(!holds(&lines, "Jobs load when"), "{}", lines.join("\n"));
}

/// TASK-96: the same Inbox at both densities.
fn inbox_at(density: crate::config::Density) -> App {
    App { density, ..inbox() }
}

/// The Inbox table's column-title row and the cells from `PR` to `TITLE`.
fn title_offset(lines: &[String]) -> usize {
    let header = lines
        .iter()
        .find(|line| line.contains(" PR ") && line.contains("TITLE"))
        .expect("the Inbox column titles");
    let chars = header.chars().collect::<Vec<_>>();
    let find = |needle: &str| {
        (0..chars.len())
            .find(|start| {
                chars[*start..]
                    .iter()
                    .collect::<String>()
                    .starts_with(needle)
            })
            .unwrap()
    };
    find("TITLE") - find("PR ")
}

#[test]
fn design_density_v96_layout_helpers_follow_the_frame_density() {
    use crate::config::Density;
    use crate::ui_table::{column_gap, layout, Column, COLUMN_GAP, COMPACT_COLUMN_GAP};
    let columns = [
        Column::new("A", 4),
        Column::new("B", 4).flex(1),
        Column::new("C", 3).right(),
    ];
    crate::ui_chrome::set_density(Density::Comfortable);
    assert_eq!(column_gap(), COLUMN_GAP);
    assert!(crate::ui_chrome::splits());
    let comfortable = layout(&columns, &[4, 9, 3], 40);
    assert_eq!(comfortable[1].x, 4 + COLUMN_GAP);
    assert!(split_list_preview(Rect::new(0, 0, 140, 30))
        .preview
        .is_some());

    crate::ui_chrome::set_density(Density::Compact);
    assert_eq!(column_gap(), COMPACT_COLUMN_GAP);
    assert!(!crate::ui_chrome::splits());
    let compact = layout(&columns, &[4, 9, 3], 40);
    assert_eq!(compact[1].x, 4 + COMPACT_COLUMN_GAP);
    assert_eq!(
        compact[2].x,
        compact[1].x + compact[1].width + COMPACT_COLUMN_GAP
    );
    // Compact fits all three columns where Comfortable's wider gaps drop the last one.
    assert_eq!(layout(&columns, &[4, 4, 3], 13).len(), 3);
    crate::ui_chrome::set_density(Density::Comfortable);
    assert_eq!(layout(&columns, &[4, 4, 3], 13).len(), 2);
    // Compact never splits, at any width.
    crate::ui_chrome::set_density(Density::Compact);
    for width in [SPLIT_MIN_WIDTH, 140, 200] {
        let split = split_list_preview(Rect::new(0, 1, width, 30));
        assert_eq!(split.preview, None);
        assert_eq!(split.list, Rect::new(0, 1, width, 30));
    }
    crate::ui_chrome::set_density(Density::Comfortable);
}

#[test]
fn design_density_v96_compact_inbox_keeps_the_whole_list_and_one_summary_row() {
    use crate::config::Density;
    // Comfortable at 140: list plus preview, the `PULL REQUESTS` section header, gap of two.
    let mut comfortable = inbox_at(Density::Comfortable);
    let (_, lines) = paint(&mut comfortable, 140, 42);
    assert!(comfortable.preview_area_v92.is_some());
    assert!(holds(&lines, "PULL REQUESTS"), "{}", lines.join("\n"));
    assert_eq!(title_offset(&lines), "example/actual#1".len() + 2);

    // Compact at 140: no preview, the summary row carries freshness, gap of one.
    let mut compact = inbox_at(Density::Compact);
    let (_, lines) = paint(&mut compact, 140, 42);
    assert_eq!(compact.preview_area_v92, None, "{}", lines.join("\n"));
    assert!(!holds(&lines, "─ Preview"), "{}", lines.join("\n"));
    assert!(!holds(&lines, "PULL REQUESTS"), "{}", lines.join("\n"));
    let freshness = compact.github.as_ref().unwrap().freshness.label();
    let summary = lines
        .iter()
        .find(|line| line.contains("Queue:"))
        .expect("the queue summary row");
    assert!(summary.contains(&freshness), "{summary}");
    // Freshness is right-aligned against the pane border.
    assert!(
        summary.trim_end_matches(['│', ' ']).ends_with(&freshness),
        "{summary}"
    );
    assert_eq!(title_offset(&lines), "example/actual#1".len() + 1);
    for needle in ["example/actual#1", "example/actual#2", "example/actual#3"] {
        assert!(holds(&lines, needle), "{}", lines.join("\n"));
    }
    // The same row actions: a click on a row and Enter open the same PR.
    let row = lines
        .iter()
        .position(|line| line.contains("example/actual#2"))
        .unwrap() as u16;
    click(&mut compact, 10, row);
    let selected = compact.github.as_ref().unwrap().inbox.pull_requests[1].clone();
    assert_eq!(
        compact.github_action,
        Some(GithubAction::Open(PrKey::from(&selected)))
    );
    compact.github_action = None;
    key(&mut compact, KeyCode::Enter);
    assert!(compact.github_action.is_some());

    // At 80x24 both modes drill down; Compact gives the list one more line and one line per
    // PR, where Comfortable's items take two (14 lines: 7 items, against 15).
    let mut comfortable = inbox_at(Density::Comfortable);
    paint(&mut comfortable, 80, 24);
    let mut compact = inbox_at(Density::Compact);
    let (_, lines) = paint(&mut compact, 80, 24);
    assert_eq!(comfortable.github_visible_rows, 7);
    assert_eq!(
        compact.github_visible_rows,
        2 * comfortable.github_visible_rows + 1,
        "{}",
        lines.join("\n")
    );
    crate::ui_chrome::set_density(Density::Comfortable);
}

/// TASK-96: an Inbox of `count` pull requests (`example/actual#1`…), every third a draft.
fn long_inbox(density: crate::config::Density, count: u64) -> App {
    let mut app = inbox_at(density);
    let github = app.github.as_mut().unwrap();
    github.inbox.pull_requests = (1..=count)
        .map(|number| {
            let mut pr = pull_request(
                number % 9 + 1,
                &format!("Change number {number}"),
                "sam-ops",
            );
            pr.number = number;
            pr.id = format!("PR_{number}");
            pr.url = format!("https://github.invalid/example/actual/pull/{number}");
            pr.draft = number % 3 == 0;
            pr
        })
        .collect();
    github.selected = github.inbox.pull_requests.first().map(Into::into);
    app
}

/// The line index holding PR `number`'s table row: `example/actual#N ` padded in its cell and
/// its title on one line (the split preview names the PR on a line of its own).
fn pr_line(lines: &[String], number: u64) -> usize {
    let reference = format!("example/actual#{number} ");
    let title = format!("Change number {number}");
    lines
        .iter()
        .position(|line| line.contains(&reference) && line.contains(&title))
        .unwrap_or_else(|| panic!("no row for #{number}:\n{}", lines.join("\n")))
}

#[test]
fn design_density_v96_comfortable_inbox_items_are_two_lines_and_compact_one() {
    use crate::config::Density;
    for (width, height) in [(80u16, 24u16), (140, 42)] {
        let mut app = long_inbox(Density::Comfortable, 30);
        let (buffer, lines) = paint(&mut app, width, height);
        let screen = lines.join("\n");
        let header = lines
            .iter()
            .position(|line| line.contains(" PR ") && line.contains("TITLE"))
            .expect("the Inbox column titles");
        let visible = app.github_visible_rows;
        assert!(visible >= 3, "{screen}");
        for number in 1..=visible as u64 {
            let row = pr_line(&lines, number);
            assert_eq!(row, header + 1 + 2 * (number as usize - 1), "{screen}");
            // The muted second line: state (draft kept), last update; never another PR.
            // Only the list pane's part of the line (the split preview shares the row).
            let detail = lines[row + 1].split("││").next().unwrap();
            let state = if number % 3 == 0 {
                "Open · draft · updated "
            } else {
                "Open · updated "
            };
            assert!(detail.contains(state), "{detail}\n{screen}");
            assert!(!detail.contains("example/actual#"), "{detail}");
            // The detail line starts under TITLE.
            assert_eq!(
                detail.find("Open"),
                lines[header].find("TITLE"),
                "{detail}\n{}",
                lines[header]
            );
            // Queues are named on the detail line only when the QUEUE column left.
            assert_eq!(
                detail.contains("Created"),
                !lines[header].contains("QUEUE"),
                "{detail}\n{}",
                lines[header]
            );
        }
        assert!(
            !holds(&lines, &format!("example/actual#{} ", visible + 1)),
            "{screen}"
        );
        // The `▌` bar marks the selected item once; its detail line shares the Selection
        // background so the two lines read as one item.
        let first = pr_line(&lines, 1);
        // The list pane starts at the left edge in both layouts; its bar sits inside the border.
        let x0 = 1;
        assert_eq!(
            buffer[(x0, first as u16)].symbol(),
            crate::ui_table::SELECTION_BAR
        );
        assert_ne!(
            buffer[(x0, first as u16 + 1)].symbol(),
            crate::ui_table::SELECTION_BAR
        );
        let selection = crate::ui_table::selection_style(app.theme())
            .bg
            .unwrap_or_default();
        assert_eq!(buffer[(x0 + 4, first as u16 + 1)].bg, selection);
        assert_ne!(buffer[(x0 + 4, first as u16 + 2)].bg, selection);

        // A click on either line of an item opens that same PR.
        for line in [pr_line(&lines, 2), pr_line(&lines, 2) + 1] {
            app.github_action = None;
            click(&mut app, 12, line as u16);
            let pr = app.github.as_ref().unwrap().inbox.pull_requests[1].clone();
            assert_eq!(
                app.github_action,
                Some(GithubAction::Open(PrKey::from(&pr)))
            );
        }
        // The keyboard moves item by item and scrolls by whole items past the last one.
        app.github_action = None;
        let selected = app.github.as_ref().unwrap().selected_index();
        for _ in selected..visible {
            key(&mut app, KeyCode::Down);
        }
        assert_eq!(app.github.as_ref().unwrap().selected_index(), visible);
        assert_eq!(app.inbox_list.top, 1);
        let (buffer, lines) = paint(&mut app, width, height);
        let row = pr_line(&lines, visible as u64 + 1);
        assert_eq!(row, header + 1 + 2 * (visible - 1), "{}", lines.join("\n"));
        assert_eq!(
            buffer[(x0, row as u16)].symbol(),
            crate::ui_table::SELECTION_BAR
        );
        assert!(!holds(&lines, "example/actual#1 "), "{}", lines.join("\n"));
        assert!(
            app.github_action.is_none(),
            "moving the cursor reads nothing"
        );
        // The wheel scrolls whole items too.
        app.dispatch(Action::Scroll(3));
        let (_, lines) = paint(&mut app, width, height);
        assert_eq!(pr_line(&lines, 5), header + 1, "{}", lines.join("\n"));

        // Compact: one line per PR, no detail line, the same number of PRs on one line each.
        let mut compact = long_inbox(Density::Compact, 30);
        let (_, lines) = paint(&mut compact, width, height);
        let header = lines
            .iter()
            .position(|line| line.contains(" PR ") && line.contains("TITLE"))
            .unwrap();
        assert!(
            compact.github_visible_rows > visible,
            "{}",
            lines.join("\n")
        );
        for number in 1..=3 {
            assert_eq!(pr_line(&lines, number), header + number as usize);
        }
        assert!(!holds(&lines, "Open · updated"), "{}", lines.join("\n"));
    }
    crate::ui_chrome::set_density(Density::Comfortable);
}

#[test]
fn design_density_v96_settings_density_is_one_focus_stop_by_key_and_click() {
    use crate::config::Density;
    let mut app = inbox();
    app.dispatch(Action::Settings);
    let (_, lines) = paint(&mut app, 80, 24);
    assert!(
        holds(&lines, "[Density: Comfortable]"),
        "{}",
        lines.join("\n")
    );
    // Host, Repository set, Theme, Density, Save, Close: hit order is focus order.
    let order = app
        .hits
        .iter()
        .filter_map(|hit| match hit.action {
            Action::SettingsField(index) => Some(format!("field{index}")),
            Action::SettingsTheme => Some("theme".into()),
            Action::SettingsDensity => Some("density".into()),
            Action::SettingsSave => Some("save".into()),
            Action::Close => Some("close".into()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        order,
        ["field0", "field1", "theme", "density", "save", "close"]
    );
    // Keyboard: three Tabs reach Density; Enter toggles it live without saving.
    for _ in 0..3 {
        key(&mut app, KeyCode::Tab);
    }
    assert_eq!(app.focus, 3);
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.density, Density::Compact);
    assert_eq!(app.settings_theme, 0, "Density is not the theme control");
    assert!(!app.settings_save_requested);
    let (_, lines) = paint(&mut app, 80, 24);
    assert!(holds(&lines, "[Density: Compact]"), "{}", lines.join("\n"));
    // The next stop is Save; Shift-Tab returns to Density.
    key(&mut app, KeyCode::Tab);
    assert_eq!(app.focus, 4);
    key(&mut app, KeyCode::BackTab);
    assert_eq!(app.focus, 3);
    // Mouse: a click on the control toggles it back.
    let row = lines
        .iter()
        .position(|line| line.contains("[Density: Compact]"))
        .unwrap() as u16;
    let column = lines[usize::from(row)]
        .chars()
        .position(|c| c == '[')
        .unwrap() as u16;
    click(&mut app, column + 2, row);
    assert_eq!(app.density, Density::Comfortable);
    let (_, lines) = paint(&mut app, 80, 24);
    assert!(holds(&lines, "[Density: Comfortable]"));
    // Save is the fifth stop and Close the sixth.
    key(&mut app, KeyCode::Tab);
    key(&mut app, KeyCode::Enter);
    assert!(app.settings_save_requested);
    key(&mut app, KeyCode::Tab);
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.overlay, None);
}

#[test]
fn inbox_preview_lists_reviewers_and_commenters_from_the_inbox_read() {
    use crate::providers::gh::{InboxReviewerV1, ReviewerRoleV1};
    let mut app = inbox();
    // Before the Inbox read reports on this PR, the section says so instead of "none".
    let (_, lines) = paint(&mut app, 140, 42);
    let preview = app.preview_area_v92.expect("preview at 140");
    let right = columns(&lines, preview.x, preview.right());
    assert!(holds(&right, "REVIEWERS"), "{}", right.join("\n"));
    assert!(holds(&right, "Not loaded yet"), "{}", right.join("\n"));

    let reviewer = |login: &str, kind: &str, role| InboxReviewerV1 {
        actor: Actor {
            actor_type: kind.into(),
            ..actor(login)
        },
        role,
    };
    let github = app.github.as_mut().unwrap();
    let key = crate::inbox_status_v1::row_key("github.invalid", "example/actual", 1);
    github.inbox_reviewers_v1.insert(
        key.clone(),
        vec![
            reviewer("alice", "User", ReviewerRoleV1::Approved),
            reviewer("bob", "User", ReviewerRoleV1::Requested),
            reviewer("coderabbitai", "Bot", ReviewerRoleV1::Commented),
            // The PR author's own comments are not a review.
            reviewer("thowd22", "User", ReviewerRoleV1::Commented),
        ],
    );
    let (_, lines) = paint(&mut app, 140, 42);
    let right = columns(&lines, preview.x, preview.right());
    for needle in [
        "AL alice · approved",
        "BO bob · requested",
        "CO coderabbitai [bot] · commented",
    ] {
        assert!(
            holds(&right, needle),
            "{needle} missing:\n{}",
            right.join("\n")
        );
    }
    assert!(
        !holds(&right, "thowd22 · commented"),
        "{}",
        right.join("\n")
    );
    assert!(!holds(&right, "Not loaded yet"));

    // A PR the read reported with nobody on it.
    app.github
        .as_mut()
        .unwrap()
        .inbox_reviewers_v1
        .insert(key, Vec::new());
    let (_, lines) = paint(&mut app, 140, 42);
    let right = columns(&lines, preview.x, preview.right());
    assert!(
        holds(&right, "No reviewers or comments yet"),
        "{}",
        right.join("\n")
    );
    // Rendering the section never queues a read.
    assert!(app.github_action.is_none());
    assert!(!app.refresh_requested);
}

/// The cell (column, row) where `needle` starts on screen.
fn find_cell(lines: &[String], needle: &str) -> Option<(u16, u16)> {
    lines.iter().enumerate().find_map(|(y, line)| {
        line.find(needle)
            .map(|byte| (line[..byte].chars().count() as u16, y as u16))
    })
}

#[test]
fn inbox_merged_tab_reads_on_demand_by_key_and_click_and_lists_merges() {
    use crate::github_workspace::InboxFilter;
    for mouse in [false, true] {
        let mut app = inbox();
        let (_, lines) = paint(&mut app, 160, 42);
        assert!(holds(&lines, "M Merged"), "{}", lines.join("\n"));
        if mouse {
            let (x, y) = find_cell(&lines, "M Merged").unwrap();
            click(&mut app, x, y);
        } else {
            app.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('M'),
                KeyModifiers::SHIFT,
            )));
        }
        let github = app.github.as_ref().unwrap();
        assert_eq!(github.filter, InboxFilter::Merged, "mouse={mouse}");
        // The first choice queues the tab's one read; the open queues are not re-read.
        assert_eq!(
            app.github_action,
            Some(GithubAction::Filter(InboxFilter::Merged)),
            "mouse={mouse}"
        );
        assert!(!app.refresh_requested);
        let (_, lines) = paint(&mut app, 160, 42);
        assert!(
            holds(&lines, "Loading merged pull requests…"),
            "{}",
            lines.join("\n")
        );
        assert!(holds(&lines, "Queue: Merged • last 7 days • loading…"));

        // The read lands: one merged PR, with its merge time in STATUS.
        app.github_action = None;
        let github = app.github.as_mut().unwrap();
        let mut merged = pull_request(9, "Ship the merged tab", "sam-ops");
        merged.state = "merged".into();
        merged.queues.clear();
        let merged_at = crate::metrics_workspace::utc_label(crate::storage::now() - 7200);
        github.merged_at_v1.insert(
            PrKey::from(&merged),
            format!("{}T{}Z", &merged_at[..10], &merged_at[11..19]),
        );
        github.merged_v1 = Some(vec![merged]);
        github.set_filter(InboxFilter::Merged);
        let (_, lines) = paint(&mut app, 160, 42);
        for needle in [
            "Queue: Merged • last 7 days • 1 involving you",
            "Ship the merged tab",
            "Merged 2h ago",
        ] {
            assert!(holds(&lines, needle), "{needle}:\n{}", lines.join("\n"));
        }
        // The open queues keep their own counts and rows.
        assert!(!holds(&lines, "Fix dark playback"));

        // Choosing the tab again uses the loaded list; a/click All goes back.
        app.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('M'),
            KeyModifiers::SHIFT,
        )));
        assert_eq!(app.github_action, None);
        if mouse {
            let (x, y) = find_cell(&lines, "a All").unwrap();
            click(&mut app, x, y);
        } else {
            key(&mut app, KeyCode::Char('a'));
        }
        assert_eq!(app.github.as_ref().unwrap().filter, InboxFilter::All);
        let (_, lines) = paint(&mut app, 160, 42);
        assert!(holds(&lines, "Fix dark playback"), "{}", lines.join("\n"));
        assert!(!holds(&lines, "Ship the merged tab"));
    }
}
