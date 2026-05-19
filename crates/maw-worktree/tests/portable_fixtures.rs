use maw_worktree::{resolve_worktree_window, Session, Window, WorktreeWindowResolution};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Fixture {
    name: String,
    input: FixtureInput,
    expected: ExpectedResolution,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureInput {
    main_repo_name: String,
    wt_name: String,
    sessions: Vec<FixtureSession>,
}

#[derive(Debug, Deserialize)]
struct FixtureSession {
    name: String,
    windows: Vec<FixtureWindow>,
}

#[derive(Debug, Deserialize)]
struct FixtureWindow {
    index: u32,
    name: String,
    active: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum ExpectedResolution {
    Bound {
        window: String,
    },
    Ambiguous {
        query: String,
        candidates: Vec<String>,
    },
    None,
}

impl From<FixtureSession> for Session {
    fn from(session: FixtureSession) -> Self {
        Self {
            name: session.name,
            windows: session.windows.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<FixtureWindow> for Window {
    fn from(window: FixtureWindow) -> Self {
        Self {
            index: window.index,
            name: window.name,
            active: window.active,
        }
    }
}

fn expected_resolution(expected: ExpectedResolution) -> WorktreeWindowResolution {
    match expected {
        ExpectedResolution::Bound { window } => WorktreeWindowResolution::Bound { window },
        ExpectedResolution::Ambiguous { query, candidates } => {
            WorktreeWindowResolution::Ambiguous { query, candidates }
        }
        ExpectedResolution::None => WorktreeWindowResolution::None,
    }
}

#[test]
fn worktree_window_fixtures_match_maw_js_portable_spec() {
    let fixtures: Vec<Fixture> =
        serde_json::from_str(include_str!("fixtures/worktree-window-match.fixtures.json"))
            .expect("valid worktree window fixture json");

    for fixture in fixtures {
        let sessions: Vec<Session> = fixture.input.sessions.into_iter().map(Into::into).collect();
        let actual = resolve_worktree_window(
            &fixture.input.main_repo_name,
            &fixture.input.wt_name,
            &sessions,
        );
        assert_eq!(
            actual,
            expected_resolution(fixture.expected),
            "{}",
            fixture.name
        );
    }
}
