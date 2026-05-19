use maw_matcher::{
    normalize_target, resolve_by_name, resolve_session_target, resolve_worktree_target, Named,
    ResolveOptions, ResolveResult,
};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Item {
    name: String,
}

impl Named for Item {
    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveFixture {
    name: String,
    mode: Mode,
    input: ResolveInput,
    expected: ExpectedResolve,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Mode {
    #[serde(rename = "byName")]
    ByName,
    #[serde(rename = "session")]
    Session,
    #[serde(rename = "worktree")]
    Worktree,
}

#[derive(Debug, Deserialize)]
struct ResolveInput {
    target: String,
    items: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct ExpectedResolve {
    kind: String,
    #[serde(rename = "match")]
    #[serde(default)]
    match_name: Option<String>,
    #[serde(default)]
    candidates: Option<Vec<String>>,
    #[serde(default)]
    hints: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct NormalizeFixture {
    name: String,
    input: String,
    expected: String,
}

fn portable_shape(result: ResolveResult<Item>) -> ExpectedResolve {
    match result {
        ResolveResult::Exact { matched } => ExpectedResolve {
            kind: "exact".to_owned(),
            match_name: Some(matched.name),
            candidates: None,
            hints: None,
        },
        ResolveResult::Fuzzy { matched } => ExpectedResolve {
            kind: "fuzzy".to_owned(),
            match_name: Some(matched.name),
            candidates: None,
            hints: None,
        },
        ResolveResult::Ambiguous { candidates } => ExpectedResolve {
            kind: "ambiguous".to_owned(),
            match_name: None,
            candidates: Some(candidates.into_iter().map(|item| item.name).collect()),
            hints: None,
        },
        ResolveResult::None { hints } => ExpectedResolve {
            kind: "none".to_owned(),
            match_name: None,
            candidates: None,
            hints: hints.map(|items| items.into_iter().map(|item| item.name).collect()),
        },
    }
}

fn resolve_fixture(fixture: &ResolveFixture) -> ResolveResult<Item> {
    let items: Vec<Item> = fixture
        .input
        .items
        .iter()
        .cloned()
        .map(|name| Item { name })
        .collect();
    match fixture.mode {
        Mode::ByName => resolve_by_name(&fixture.input.target, &items, ResolveOptions::default()),
        Mode::Session => resolve_session_target(&fixture.input.target, &items),
        Mode::Worktree => resolve_worktree_target(&fixture.input.target, &items),
    }
}

#[test]
fn matcher_resolve_target_fixtures_match_maw_js_portable_spec() {
    let fixtures: Vec<ResolveFixture> = serde_json::from_str(include_str!(
        "fixtures/matcher-resolve-target.fixtures.json"
    ))
    .expect("valid matcher fixture json");

    assert_eq!(fixtures.len(), 16, "maw-js alpha.739 fixture count changed");
    for fixture in fixtures {
        assert_eq!(
            portable_shape(resolve_fixture(&fixture)),
            fixture.expected,
            "fixture failed: {}",
            fixture.name
        );
    }
}

#[test]
fn normalize_target_fixtures_match_maw_js_portable_spec() {
    let fixtures: Vec<NormalizeFixture> =
        serde_json::from_str(include_str!("fixtures/normalize-target.fixtures.json"))
            .expect("valid normalize fixture json");

    assert_eq!(
        fixtures.len(),
        12,
        "maw-js alpha.739 normalize fixture count changed"
    );
    for fixture in fixtures {
        assert_eq!(
            normalize_target(&fixture.input),
            fixture.expected,
            "fixture failed: {}",
            fixture.name
        );
    }
}
