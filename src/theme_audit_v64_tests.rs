//! TASK-64 AC3 source audit: drawn views take colours from theme roles, Git/CI states from the
//! one shared `SemanticToken::ci_state` mapping, and actors from the shared identity label.
//!
//! The scan reads every non-test source file under `src/` with string literals, character
//! literals and comments masked out, and removes every `#[cfg(test)]` item (a whole inline
//! `mod tests { … }`, or just the `mod tests;` declaration). An earlier audit stopped at the
//! first `#[cfg(test)]`, which hid the rest of every file that declares its test module near
//! the top.
use super::SemanticToken;
use regex::Regex;
use std::path::{Path, PathBuf};

/// Files that may name raw colours, with the exact number of lines that do and why. The count
/// is exact so a new raw colour fails here and a removed one asks for the entry to shrink.
const COLOR_ALLOWLIST: [(&str, usize, &str); 2] = [
    (
        "identity.rs",
        2,
        "avatar pictures: half-block canvases paint the decoded image's own pixel colours",
    ),
    (
        "providers/owned_pty_viewer_v4.rs",
        2,
        "embedded terminal: passes the child program's own indexed/RGB cell colours through \
         (read_only_pty_pane_v4 downsamples them to the colour mode)",
    ),
];

/// Files that format a login as `@{…}` in text, with the exact count and why that is not an
/// actor row. Actor rows draw through `ActorIdentity` (label, chip, avatar fallback); these are
/// the signed-in operator, an exact account echo beside an identity row, or a team mention.
const ACTOR_ALLOWLIST: [(&str, usize, &str); 27] = [
    ("identity.rs", 2, "the shared ActorIdentity::label itself"),
    ("providers/gh.rs", 1, "provider Actor::label: the plain username ActorIdentity wraps"),
    ("pr_conversation_cards_v115.rs", 1, "quote reply body text sent to GitHub (`> @author wrote:`), not a drawn actor"),
    ("pr_actor_profile_v1.rs", 2, "shared profile: Username and Signed-in account lines"),
    ("providers/gh/repository_catalogue/local_commit_actor_v1.rs", 1, "kind-aware host report label"),
    ("forms.rs", 5, "principal context and PR effect sentence of review/lifecycle/merge forms (owned by the merge/lifecycle forms task)"),
    ("actions_workspace.rs", 1, "view title: host / signed-in account / repository"),
    ("app.rs", 1, "Dashboard title: host / signed-in account"),
    ("github_workspace.rs", 2, "Inbox title's signed-in account; review team @org/slug mention"),
    ("metrics_routes.rs", 1, "Dashboard title: host / signed-in account"),
    ("notifications_v63.rs", 1, "signed-in observer context"),
    ("pr_metadata_member_picker.rs", 1, "picker header: signed-in account"),
    ("pr_metadata_native.rs", 1, "metadata preview: signed-in actor with numeric account"),
    ("pr_metadata_picker.rs", 3, "picker header account; exact account echo under the identity row"),
    ("remote_repository_v1.rs", 1, "remote repository title: signed-in account"),
    ("repository_catalogue_workspace_v1.rs", 1, "observed signed-in account"),
    ("scoped_actions_dispatch_ui_v7.rs", 1, "signed-in principal context of the dispatch form"),
    ("scoped_actions_publication_ui_v3.rs", 1, "signed-in principal of the original publication"),
    ("scoped_actions_publication_v3.rs", 1, "rerun requester (the signed-in account)"),
    ("scoped_actions_workflow_ui_v6.rs", 1, "signed-in principal context of the workflow form"),
    ("scoped_pr_collaboration_ui_v17.rs", 1, "signed-in principal context of the collaboration form"),
    ("scoped_pr_inline_plan_v1.rs", 1, "inline publication: signed-in account"),
    ("scoped_pr_inline_ui_v1.rs", 1, "signed-in principal context of the inline comment form"),
    ("scoped_pr_thread_historical_v5.rs", 1, "opening (signed-in) account"),
    ("search/workspace.rs", 1, "Search scope caption: the verified operator account"),
    ("search_sources/ui.rs", 1, "Search sources: the operator account"),
    ("suggestions.rs", 1, "co-author trailer limitation names the suggestion author"),
];

/// One source file as two same-length views: `code` masks string, char and comment contents;
/// `text` masks only comments. Both drop `#[cfg(test)]` items. Newlines are always kept.
struct Source {
    relative: String,
    code: String,
    text: String,
}

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Masks comments in both views and literal contents in `code` only.
fn mask(raw: &str) -> (Vec<u8>, Vec<u8>) {
    let bytes = raw.as_bytes();
    let mut code = bytes.to_vec();
    let mut text = bytes.to_vec();
    let blank = |view: &mut Vec<u8>, from: usize, to: usize| {
        for byte in &mut view[from..to] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    };
    let mut i = 0;
    while i < bytes.len() {
        let rest = &bytes[i..];
        if rest.starts_with(b"//") {
            let end = rest
                .iter()
                .position(|&b| b == b'\n')
                .map_or(bytes.len(), |n| i + n);
            blank(&mut code, i, end);
            blank(&mut text, i, end);
            i = end;
        } else if rest.starts_with(b"/*") {
            let (mut depth, mut j) = (1, i + 2);
            while j < bytes.len() && depth > 0 {
                if bytes[j..].starts_with(b"/*") {
                    depth += 1;
                    j += 2;
                } else if bytes[j..].starts_with(b"*/") {
                    depth -= 1;
                    j += 2;
                } else {
                    j += 1;
                }
            }
            blank(&mut code, i, j);
            blank(&mut text, i, j);
            i = j;
        } else if (bytes[i] == b'r' || rest.starts_with(b"br"))
            && (i == 0 || !is_ident(bytes[i - 1]))
            && {
                let start = if bytes[i] == b'b' { i + 2 } else { i + 1 };
                let hashes = bytes[start..].iter().take_while(|&&b| b == b'#').count();
                bytes.get(start + hashes) == Some(&b'"')
            }
        {
            let start = if bytes[i] == b'b' { i + 2 } else { i + 1 };
            let hashes = bytes[start..].iter().take_while(|&&b| b == b'#').count();
            let open = start + hashes + 1;
            let mut close = vec![b'"'];
            close.extend(std::iter::repeat_n(b'#', hashes));
            let end = bytes[open..]
                .windows(close.len())
                .position(|w| w == close.as_slice())
                .map_or(bytes.len(), |n| open + n);
            blank(&mut code, open, end);
            i = (end + close.len()).min(bytes.len());
        } else if bytes[i] == b'"' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b'"' {
                j += if bytes[j] == b'\\' { 2 } else { 1 };
            }
            let end = j.min(bytes.len());
            blank(&mut code, i + 1, end);
            i = end + 1;
        } else if bytes[i] == b'\'' {
            // A char literal ('x', '\n', '\u{..}', '{'), else a lifetime or label.
            let end = if bytes.get(i + 1) == Some(&b'\\') {
                // Skip the escaped character itself, so '\'' closes on its last quote.
                bytes
                    .get(i + 3..)
                    .and_then(|rest| rest.iter().position(|&b| b == b'\''))
                    .map(|n| i + 3 + n)
            } else {
                let width = raw[i + 1..].chars().next().map_or(1, char::len_utf8);
                (bytes.get(i + 1 + width) == Some(&b'\'')).then_some(i + 1 + width)
            };
            match end {
                Some(end) => {
                    blank(&mut code, i + 1, end);
                    i = end + 1;
                }
                None => i += 1,
            }
        } else {
            i += 1;
        }
    }
    (code, text)
}

/// Byte ranges of `#[cfg(test)]` items in the masked code view.
fn test_items(code: &[u8]) -> Vec<(usize, usize)> {
    const ATTR: &[u8] = b"#[cfg(test)]";
    let mut ranges = vec![];
    let mut at = 0;
    while let Some(found) = code[at..].windows(ATTR.len()).position(|w| w == ATTR) {
        let start = at + found;
        let mut i = start + ATTR.len();
        // Further attributes such as `#[path = "…"]` belong to the same item.
        loop {
            while i < code.len() && code[i].is_ascii_whitespace() {
                i += 1;
            }
            if code[i..].starts_with(b"#[") {
                let mut depth = 0;
                while i < code.len() {
                    match code[i] {
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
            } else {
                break;
            }
        }
        // The item ends at its first top-level `;` (or `,` for a field or arm), before an
        // enclosing `}`, or after its first balanced `{ … }`.
        let mut end = code.len();
        let mut nesting = 0usize;
        while i < code.len() {
            match code[i] {
                b'(' | b'[' => {
                    nesting += 1;
                    i += 1;
                }
                b')' | b']' => {
                    nesting = nesting.saturating_sub(1);
                    i += 1;
                }
                b';' | b',' if nesting == 0 => {
                    end = i + 1;
                    break;
                }
                b'}' => {
                    end = i;
                    break;
                }
                b'{' => {
                    let mut depth = 0;
                    while i < code.len() {
                        match code[i] {
                            b'{' => depth += 1,
                            b'}' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                    end = (i + 1).min(code.len());
                    break;
                }
                _ => i += 1,
            }
        }
        ranges.push((start, end));
        at = end.max(start + ATTR.len());
    }
    ranges
}

fn strip(raw: &str) -> (String, String) {
    let (mut code, mut text) = mask(raw);
    for (from, to) in test_items(&code) {
        for view in [&mut code, &mut text] {
            for byte in &mut view[from..to] {
                if *byte != b'\n' {
                    *byte = b' ';
                }
            }
        }
    }
    // Masking only ever replaces whole bytes of literals or comments with spaces; a cut
    // multi-byte character becomes spaces, so the views stay valid text.
    let lossy = |view: Vec<u8>| String::from_utf8_lossy(&view).into_owned();
    (lossy(code), lossy(text))
}

/// Test-only files: `tests.rs`, `*_tests*.rs` and `*test_support*.rs`, all declared behind
/// `#[cfg(test)]` modules.
fn test_only(path: &Path) -> bool {
    let stem = path.file_stem().unwrap().to_string_lossy();
    stem == "tests" || stem.contains("_tests") || stem.contains("test_support")
}

fn sources() -> Vec<Source> {
    fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, files);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = vec![];
    walk(&root, &mut files);
    files.sort();
    files
        .into_iter()
        .filter(|path| !test_only(path))
        .map(|path| {
            let raw = std::fs::read_to_string(&path).unwrap();
            let (code, text) = strip(&raw);
            Source {
                relative: path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                code,
                text,
            }
        })
        .collect()
}

const NAMED: [&str; 16] = [
    "Black",
    "Red",
    "Green",
    "Yellow",
    "Blue",
    "Magenta",
    "Cyan",
    "Gray",
    "DarkGray",
    "LightRed",
    "LightGreen",
    "LightYellow",
    "LightBlue",
    "LightMagenta",
    "LightCyan",
    "White",
];

/// Whether one masked code line names a raw colour: an RGB/indexed/named `Color` variant, a
/// parsed or numeric colour, a variant glob import, or a `Stylize` shorthand like `.red()`.
fn raw_color_line(line: &str) -> bool {
    let named = NAMED.iter().any(|name| {
        line.match_indices(&format!("Color::{name}"))
            .any(|(at, hit)| !line[at + hit.len()..].bytes().next().is_some_and(is_ident))
    });
    let shorthand = NAMED.iter().any(|name| {
        let snake = match *name {
            "DarkGray" => "dark_gray".to_owned(),
            name if name.starts_with("Light") => format!("light_{}", name[5..].to_lowercase()),
            name => name.to_lowercase(),
        };
        line.contains(&format!(".{snake}()")) || line.contains(&format!(".on_{snake}()"))
    });
    named
        || shorthand
        || [
            "Color::Rgb(",
            "Color::Indexed(",
            "Color::from_",
            "Color::*",
            "Color::{",
            "<Color>",
        ]
        .iter()
        .any(|pattern| line.contains(pattern))
}

fn allowance<'a>(
    list: &'a [(&'a str, usize, &'a str)],
    file: &str,
) -> Option<&'a (&'a str, usize, &'a str)> {
    list.iter().find(|(name, _, _)| *name == file)
}

/// Compares per-file hit lines with an exact-count allowlist and returns every mismatch.
fn check_allowlist(
    hits: &[(String, Vec<usize>)],
    list: &[(&str, usize, &str)],
    what: &str,
) -> Vec<String> {
    let mut problems = vec![];
    for (file, lines) in hits {
        match allowance(list, file) {
            Some((_, count, _)) if *count == lines.len() => {}
            Some((_, count, why)) => problems.push(format!(
                "{what}: src/{file} has {} lines {lines:?}, allowlist says {count} ({why})",
                lines.len()
            )),
            None => problems.push(format!("{what}: src/{file} lines {lines:?}")),
        }
    }
    for (file, count, _) in list {
        if !hits.iter().any(|(hit, _)| hit == file) {
            problems.push(format!(
                "{what}: allowlisted src/{file} ({count}) no longer has any; remove its entry"
            ));
        }
    }
    problems
}

#[test]
fn audit_v64_stripper_keeps_code_after_a_test_module_declaration() {
    // The earlier audit's hole: everything after `#[cfg(test)] mod tests;` was skipped.
    let raw = "fn a() {}\n#[cfg(test)]\n#[path = \"x_tests.rs\"]\nmod tests;\n\
               fn view() -> Style { Style::new().fg(Color::Red) }\n\
               #[cfg(test)]\nmod inline { fn t() { let _ = Color::Blue; let _ = \"}\"; } }\n\
               fn after() { let _ = '{'; let _ = \"Color::Green\"; } // Color::Cyan\n";
    let (code, text) = strip(raw);
    let hits: Vec<usize> = code
        .lines()
        .enumerate()
        .filter(|(_, line)| raw_color_line(line))
        .map(|(n, _)| n + 1)
        .collect();
    assert_eq!(hits, vec![5], "{code}");
    assert!(code.contains("fn after()"), "{code}");
    assert!(!code.contains("mod tests;") && !code.contains("mod inline"));
    assert!(text.contains("\"Color::Green\"") && !text.contains("Color::Cyan"));
    assert_eq!(code.lines().count(), raw.lines().count());
    for line in [
        "x.fg(Color::Rgb(1, 2, 3))",
        "use ratatui::style::Color::*;",
        "Style::new().red()",
        "span.on_dark_gray()",
        "\"#ff0000\".parse::<Color>()",
        "Color::from_u32(0xff0000)",
        "Color::LightBlue",
    ] {
        assert!(raw_color_line(line), "{line}");
    }
    for line in [
        "Color::Reset",
        "theme.style(SemanticToken::Failure)",
        "Color::Redish",
    ] {
        assert!(!raw_color_line(line), "{line}");
    }
}

#[test]
fn audit_v64_views_name_no_raw_colors_outside_the_theme() {
    let sources = sources();
    assert!(sources.len() > 300, "scanned {} files", sources.len());
    let hits: Vec<(String, Vec<usize>)> = sources
        .iter()
        .filter(|source| source.relative != "theme.rs")
        .filter_map(|source| {
            let lines: Vec<usize> = source
                .code
                .lines()
                .enumerate()
                .filter(|(_, line)| raw_color_line(line))
                .map(|(n, _)| n + 1)
                .collect();
            (!lines.is_empty()).then(|| (source.relative.clone(), lines))
        })
        .collect();
    let problems = check_allowlist(&hits, &COLOR_ALLOWLIST, "raw colour");
    assert!(
        problems.is_empty(),
        "use theme roles (src/theme.rs) instead of raw colours:\n{}",
        problems.join("\n")
    );
}

#[test]
fn audit_v64_actor_text_goes_through_the_shared_identity() {
    // `@{login}`, `@{}` or `@{who}` inside a string literal (not `@{-1}` or `@{upstream}`
    // revision syntax, which never starts a placeholder with a letter-only name and a brace
    // pair directly after a quote or space; email-like `{user}@{host}` is excluded too).
    let placeholder = Regex::new(r#"(^|["\s(\[])@\{[A-Za-z0-9_.]*(:[^}]*)?\}"#).unwrap();
    let hits: Vec<(String, Vec<usize>)> = sources()
        .iter()
        .filter_map(|source| {
            let lines: Vec<usize> = source
                .text
                .lines()
                .enumerate()
                .filter(|(_, line)| placeholder.is_match(line))
                .map(|(n, _)| n + 1)
                .collect();
            (!lines.is_empty()).then(|| (source.relative.clone(), lines))
        })
        .collect();
    let problems = check_allowlist(&hits, &ACTOR_ALLOWLIST, "raw @login");
    assert!(
        problems.is_empty(),
        "draw actors with crate::identity::ActorIdentity (label/chip/avatar fallback):\n{}",
        problems.join("\n")
    );
}

#[test]
fn audit_v64_ci_states_map_only_through_the_shared_token() {
    // A match arm turning a GitHub check/status/run state string into a token belongs in
    // `SemanticToken::ci_state`; copies drifted (PR Checks showed an errored status as waiting).
    let arm = Regex::new(
        r#""(success|failure|error|cancelled|skipped|neutral|stale|timed_out|startup_failure|action_required|in_progress|queued|completed)"(\s*\|\s*"[a-z_ ]+")*\s*=>\s*\{?\s*(crate::theme::)?SemanticToken::"#,
    )
    .unwrap();
    let offenders: Vec<String> = sources()
        .iter()
        .filter(|source| source.relative != "theme.rs")
        .flat_map(|source| {
            arm.find_iter(&source.text)
                .map(|hit| {
                    let line = source.text[..hit.start()].matches('\n').count() + 1;
                    format!("src/{}:{line}", source.relative)
                })
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "map CI states with SemanticToken::ci_state: {offenders:?}"
    );
    for (status, conclusion, token) in [
        ("completed", "ERROR", SemanticToken::Failure),
        ("COMPLETED", "SKIPPED", SemanticToken::Muted),
        ("IN_PROGRESS", "", SemanticToken::Running),
        ("completed", "mystery", SemanticToken::Muted),
        ("", "", SemanticToken::Muted),
    ] {
        assert_eq!(SemanticToken::ci_state(status, conclusion), token);
    }
}
