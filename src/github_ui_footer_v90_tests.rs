//! TASK-90 key bar for the Inbox and PR detail: tabs on one strip, the rest as footer keys
//! whose key and click carry the same action, compact at 80 columns, no toolbar rows.
use super::*;
use crate::app::View;
use crate::footer::{layout, resolve_key};
use crate::github_workspace::{GithubWorkspace, PrPage};
use crate::providers::gh::{Coverage, HostIdentity, PrDetails, PrSummary, PullRequest, Queue};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use std::collections::BTreeSet;

pub(super) fn pull_request(number: u64) -> PullRequest {
    PullRequest {
        host: "github.invalid".into(),
        id: format!("PR_{number}"),
        repository: "example/actual".into(),
        number,
        title: format!("Footer row {number}"),
        body: String::new(),
        url: String::new(),
        author: Default::default(),
        state: "open".into(),
        draft: false,
        created_at: String::new(),
        updated_at: String::new(),
        queues: BTreeSet::from([Queue::Authored]),
    }
}

pub(super) fn inbox(count: u64) -> App {
    let mut github = GithubWorkspace::empty(
        HostIdentity {
            host: "github.invalid".into(),
            login: "current-user".into(),
            account_id: Some(42),
            gh_version: "2.99.0".into(),
        },
        vec![],
        BTreeSet::new(),
    );
    github.inbox.pull_requests = (1..=count).map(pull_request).collect();
    github.selected = github.inbox.pull_requests.first().map(Into::into);
    App {
        view: View::Inbox,
        github: Some(github),
        github_content_focused: true,
        ..Default::default()
    }
}

pub(super) fn detail(tab: PrTab) -> App {
    let mut app = inbox(1);
    let pr = pull_request(7);
    app.github.as_mut().unwrap().pr = Some(PrPage {
        details: PrDetails {
            summary: PrSummary {
                pr,
                head_sha: "a".repeat(40),
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
            reviews: vec![],
            timeline: vec![],
            coverage: Coverage {
                complete: true,
                ..Default::default()
            },
        },
        tab,
        diff: None,
        checks: None,
        required_checks: None,
        rows: vec![],
        context: None,
        metadata_display_v51: None,
        review_progress: None,
    });
    app
}

fn press(code: KeyCode) -> KeyEvent {
    let modifiers = match code {
        KeyCode::Char(c) if c.is_ascii_uppercase() => KeyModifiers::SHIFT,
        _ => KeyModifiers::NONE,
    };
    KeyEvent::new(code, modifiers)
}

/// Draws the view plus the main key bar in one frame, returning the painted rows.
fn paint(app: &mut App, width: u16, height: u16) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    app.hits.clear();
    let controls = crate::footer::main_controls(app);
    terminal
        .draw(|frame| {
            draw(frame, app, Rect::new(0, 0, width, height - 1));
            crate::footer::draw(frame, app, Rect::new(0, height - 1, width, 1), &controls);
        })
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
        .collect()
}

fn tokens(app: &App, width: u16) -> Vec<String> {
    let controls = crate::footer::main_controls(app);
    layout(&controls, width)
        .into_iter()
        .map(|placed| controls[placed.index].token(placed.compact))
        .collect()
}

#[test]
fn design_footer_v90_inbox_queues_are_tabs_and_keys_match_their_clicks() {
    let mut app = inbox(3);
    let rows = paint(&mut app, 140, 42);
    for queue in [
        "a All",
        "c Created by me",
        "m Assigned to me",
        "v Needs my review",
    ] {
        assert!(
            rows[0].contains(queue),
            "{queue} not on the tab strip: {}",
            rows[0]
        );
    }
    for retired in ["[x Clear cache]", "[F8 Notifications]", "[a All]"] {
        assert!(
            rows.iter().all(|row| !row.contains(retired)),
            "{retired} toolbar remains"
        );
    }
    let footer = &rows[41];
    for token in [
        "Enter Open",
        "r Refresh",
        "F8 Notifications",
        "x Clear cache",
    ] {
        assert!(footer.contains(token), "{token} missing from {footer}");
    }
    assert!(footer.trim_end().ends_with("? Help  : Commands  q Quit"));
    let keys = footer_keys(&app);
    for (code, expected) in [
        (
            KeyCode::Char('c'),
            Action::Github(GithubAction::Filter(InboxFilter::Authored)),
        ),
        (
            KeyCode::Char('v'),
            Action::Github(GithubAction::Filter(InboxFilter::ReviewRequested)),
        ),
        (KeyCode::Char('x'), Action::Github(GithubAction::ClearCache)),
        (
            KeyCode::F(8),
            Action::NotificationsV63(crate::notifications_ui_v63::NotificationActionV63::Open),
        ),
    ] {
        assert_eq!(resolve_key(&keys, press(code)), Some(expected.clone()));
        assert!(
            app.hits.iter().any(|hit| hit.action == expected),
            "{expected:?} has no click target"
        );
    }
    // Every row under the column titles is content: one two-line item per PR at Comfortable
    // density (TASK-96), no bottom button.
    assert_eq!(app.github_visible_rows, (41 - 1 - 3 - 1) / 2);
}

#[test]
fn design_footer_v90_inbox_open_is_disabled_with_a_reason_when_the_queue_is_empty() {
    let app = inbox(0);
    let keys = footer_keys(&app);
    let open = keys.iter().find(|key| key.key == "Enter").unwrap();
    assert_eq!(
        open.disabled.as_deref(),
        Some("No pull request in this queue.")
    );
    assert!(!tokens(&app, 140).iter().any(|token| token == "Enter Open"));
}

#[test]
fn design_footer_v90_pr_detail_tabs_fit_80_columns_compact_and_140_full() {
    let mut app = detail(PrTab::Summary);
    let wide = paint(&mut app, 140, 42);
    for tab in [
        "o Summary",
        "c Conversation",
        "h Commits",
        "u Metadata",
        "v Review progress",
    ] {
        assert!(wide[0].contains(tab), "{tab} missing: {}", wide[0]);
    }
    let narrow = paint(&mut app, 80, 24);
    for tab in [
        "o Summary",
        "c Conv",
        "h Commits",
        "k Checks",
        "t Time",
        "u Metadata",
        "v Prog",
    ] {
        assert!(
            narrow[0].contains(tab),
            "{tab} missing at 80: {}",
            narrow[0]
        );
    }
    let tabs = app
        .hits
        .iter()
        .filter(|hit| matches!(hit.action, Action::Github(GithubAction::PrTab(_))))
        .count();
    assert_eq!(
        tabs,
        PrTab::ALL.len(),
        "every tab is a click target at 80 columns"
    );
    // The heading follows the tab strip directly: no toolbar rows above the content.
    assert!(narrow[1].contains("example/actual#7"), "{}", narrow[1]);
}

#[test]
fn design_footer_v90_pr_detail_footer_by_priority_with_global_tail() {
    let app = detail(PrTab::Summary);
    let wide = tokens(&app, 140);
    for token in [
        "b Back",
        "p PR actions",
        "J Thread history",
        "F9 Review history",
        "r Refresh",
        "x Clear cache",
        "? Help",
        ": Commands",
        "q Quit",
    ] {
        assert!(
            wide.iter().any(|placed| placed == token),
            "{token} missing: {wide:?}"
        );
    }
    let narrow = tokens(&app, 80);
    for token in [
        "b Back",
        "p Actions",
        "J History",
        "F9 Review",
        "? Help",
        ": Cmds",
        "q Quit",
    ] {
        assert!(
            narrow.iter().any(|placed| placed == token),
            "{token} missing: {narrow:?}"
        );
    }
    let keys = footer_keys(&app);
    for (code, expected) in [
        (KeyCode::Char('b'), GithubAction::Back),
        (KeyCode::Char('p'), GithubAction::Actions),
        (KeyCode::Char('J'), GithubAction::ThreadHistoryV1),
        (KeyCode::F(9), GithubAction::ReviewOperationHistoryV3),
        (KeyCode::Char('u'), GithubAction::PrTab(PrTab::Metadata)),
        (KeyCode::Char('x'), GithubAction::ClearCache),
    ] {
        assert_eq!(
            resolve_key(&keys, press(code)),
            Some(Action::Github(expected))
        );
    }
    // Summary-only history keys leave the bar on another tab.
    let conversation = tokens(&detail(PrTab::Conversation), 140);
    assert!(!conversation
        .iter()
        .any(|token| token.starts_with("J ") || token.starts_with("F9 ")));
}

#[test]
fn design_footer_v90_pr_checks_open_check_token_carries_the_check_action() {
    let mut app = detail(PrTab::Checks);
    let expected = crate::scoped_pr_observed_tabs_v1::resources::native::check_action(&app);
    let keys = footer_keys(&app);
    let check = keys.iter().find(|key| key.label == "Open check").unwrap();
    match &expected {
        Action::Explain(reason) => {
            assert_eq!(check.disabled.as_deref(), Some(reason.as_str()));
            assert!(!tokens(&app, 140)
                .iter()
                .any(|token| token == "Enter Open check"));
        }
        action => {
            assert_eq!(&check.action, action);
            paint(&mut app, 140, 42);
            assert!(app.hits.iter().any(|hit| &hit.action == action));
        }
    }
}

#[test]
fn design_footer_v90_inbox_keys_route_through_the_app_like_clicks() {
    let mut app = detail(PrTab::Summary);
    paint(&mut app, 120, 40);
    app.handle_event(crossterm::event::Event::Key(press(KeyCode::Char('c'))));
    // The tab key queues the same PrTab action its tab click carries.
    assert_eq!(
        app.github_action,
        Some(GithubAction::PrTab(PrTab::Conversation))
    );
    app.handle_event(crossterm::event::Event::Key(press(KeyCode::Char('b'))));
    assert!(
        app.github.as_ref().unwrap().pr.is_none(),
        "b returns to the Inbox"
    );
}

#[test]
fn design_help_v97_inbox_and_every_pr_tab_list_footer_keys_in_help_and_commands() {
    let mut checked = 0;
    checked += crate::navigation_ui::assert_help_and_palette_cover_footer_v97(&inbox(3), "inbox");
    checked +=
        crate::navigation_ui::assert_help_and_palette_cover_footer_v97(&inbox(0), "empty inbox");
    for (tab, label, _) in PrTab::ALL {
        checked += crate::navigation_ui::assert_help_and_palette_cover_footer_v97(
            &detail(tab),
            &format!("PR {label}"),
        );
    }
    assert!(checked > 40, "{checked}");
}

#[test]
fn design_footer_v90_loading_pr_keeps_refresh_beside_cancel_at_80_columns() {
    let mut app = detail(PrTab::Summary);
    app.loading[View::Inbox.index()] = true;
    let shown = tokens(&app, 80);
    // A held read adds `Esc Cancel`; the replacement Refresh stays clickable beside it.
    for token in ["r Refresh", "Esc Cancel", "b Back"] {
        assert!(
            shown.iter().any(|t| t == token),
            "{token} missing: {shown:?}"
        );
    }
    app.loading[View::Inbox.index()] = false;
    assert!(!tokens(&app, 80).iter().any(|t| t == "Esc Cancel"));
}

/// The cell (column, row) where `needle` starts on screen.
fn cell_of(rows: &[String], needle: &str) -> Option<(u16, u16)> {
    rows.iter().enumerate().find_map(|(y, row)| {
        row.find(needle)
            .map(|byte| (row[..byte].chars().count() as u16, y as u16))
    })
}

#[test]
fn web_w_opens_the_selected_or_open_pr_in_the_browser_by_key_and_click() {
    use crate::github_workspace::PrKey;
    use crossterm::event::{Event, MouseButton, MouseEvent, MouseEventKind};
    let expected = |number| {
        GithubAction::Browse(PrKey {
            host: "github.invalid".into(),
            repository: "example/actual".into(),
            number,
        })
    };
    for mouse in [false, true] {
        // Inbox: the selected row (the second one).
        let mut app = inbox(3);
        app.handle_event(Event::Key(press(KeyCode::Down)));
        let selected = app.github.as_ref().unwrap().selected.clone().unwrap();
        let rows = paint(&mut app, 160, 42);
        assert!(
            rows.last().unwrap().contains("w Web"),
            "{}",
            rows.join("\n")
        );
        if mouse {
            let (x, y) = cell_of(&rows, "w Web").unwrap();
            app.handle_event(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: x,
                row: y,
                modifiers: KeyModifiers::NONE,
            }));
        } else {
            app.handle_event(Event::Key(press(KeyCode::Char('w'))));
        }
        assert_eq!(
            app.github_action,
            Some(expected(selected.number)),
            "mouse={mouse}"
        );
        // Opening the browser keeps the place in the list and reads nothing else.
        assert_eq!(app.github.as_ref().unwrap().selected, Some(selected));
        assert!(app.github.as_ref().unwrap().pr.is_none());
        assert!(!app.refresh_requested);
        assert!(
            app.status.starts_with("Opening example/actual#"),
            "{}",
            app.status
        );

        // An open PR: `w` carries that PR; `o` stays the Summary tab.
        let mut app = detail(PrTab::Conversation);
        let open = PrKey::from(
            &app.github
                .as_ref()
                .unwrap()
                .pr
                .as_ref()
                .unwrap()
                .details
                .summary
                .pr,
        );
        let keys = footer_keys(&app);
        assert_eq!(
            resolve_key(&keys, press(KeyCode::Char('w'))),
            Some(Action::Github(GithubAction::Browse(open.clone())))
        );
        assert_eq!(
            resolve_key(&keys, press(KeyCode::Char('o'))),
            Some(Action::Github(GithubAction::PrTab(PrTab::Summary)))
        );
        let rows = paint(&mut app, 160, 42);
        if mouse {
            let (x, y) = cell_of(&rows, "w Web").expect("w Web on the PR bar at 160");
            app.handle_event(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: x,
                row: y,
                modifiers: KeyModifiers::NONE,
            }));
        } else {
            app.handle_event(Event::Key(press(KeyCode::Char('w'))));
        }
        assert_eq!(app.github_action, Some(GithubAction::Browse(open)));
        assert!(
            app.github.as_ref().unwrap().pr.is_some(),
            "the PR stays open"
        );
    }
    // An empty queue keeps the key but explains why it does nothing.
    let app = inbox(0);
    let web = footer_keys(&app)
        .into_iter()
        .find(|control| control.key == "w")
        .unwrap();
    assert!(web.disabled.is_some());
}

#[test]
fn web_w_sends_exactly_gh_pr_view_web_for_the_host_repository_and_number() {
    assert_eq!(
        crate::providers::gh::open_pr_in_browser_args_v1("github.invalid", "example/actual", 7),
        [
            "pr",
            "view",
            "7",
            "--repo",
            "github.invalid/example/actual",
            "--web"
        ]
    );
}

#[test]
fn left_arrow_goes_back_from_a_pr_like_b_and_does_nothing_in_the_list() {
    for tab in [
        PrTab::Summary,
        PrTab::Conversation,
        PrTab::Files,
        PrTab::Checks,
    ] {
        let mut app = detail(tab);
        paint(&mut app, 120, 40);
        app.handle_event(crossterm::event::Event::Key(press(KeyCode::Left)));
        assert!(
            app.github.as_ref().unwrap().pr.is_none(),
            "← returns to the Inbox from {tab:?}"
        );
        assert_eq!(app.github_action, None, "Back reads nothing");
    }
    // In the list there is nothing to go back to: the selection and view stay.
    let mut app = inbox(3);
    app.handle_event(crossterm::event::Event::Key(press(KeyCode::Down)));
    let selected = app.github.as_ref().unwrap().selected.clone();
    app.handle_event(crossterm::event::Event::Key(press(KeyCode::Left)));
    assert_eq!(app.github.as_ref().unwrap().selected, selected);
    assert_eq!(app.view, View::Inbox);
    assert_eq!(app.github_action, None);
}
