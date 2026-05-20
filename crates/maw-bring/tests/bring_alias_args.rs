// Ported from maw-js src/cli/top-aliases.ts parseBringArgs tests in
// test/cli/dispatch-match.test.ts and test/isolated/top-aliases-runtime-coverage.test.ts.

use maw_bring::{parse_bring_args, BringAliasOptions, BringArgsError, ParsedBringArgs};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn bring_alias_defaults_to_wake_split_mode() {
    assert_eq!(
        parse_bring_args(&args(&["neo"])),
        Ok(ParsedBringArgs {
            oracle: "neo".to_owned(),
            opts: BringAliasOptions::default(),
        })
    );
}

#[test]
fn bring_alias_preserves_split_mode_for_explicit_split_engine_and_tab() {
    assert_eq!(
        parse_bring_args(&args(&["neo", "--split", "-e", "codex"])),
        Ok(ParsedBringArgs {
            oracle: "neo".to_owned(),
            opts: BringAliasOptions {
                engine: Some("codex".to_owned()),
                ..BringAliasOptions::default()
            },
        })
    );

    assert_eq!(
        parse_bring_args(&args(&["neo", "--tab"])),
        Ok(ParsedBringArgs {
            oracle: "neo".to_owned(),
            opts: BringAliasOptions::default(),
        })
    );
}

#[test]
fn bring_alias_parses_pick_and_to_target_like_maw_js() {
    assert_eq!(
        parse_bring_args(&args(&[
            "mawjs-features",
            "--pick",
            "--engine",
            "claude",
            "--to",
            "50-mawjs:maw-js-1816",
        ])),
        Ok(ParsedBringArgs {
            oracle: "mawjs-features".to_owned(),
            opts: BringAliasOptions {
                split: true,
                engine: Some("claude".to_owned()),
                pick: true,
                session: Some("50-mawjs".to_owned()),
                split_target: Some("50-mawjs:maw-js-1816".to_owned()),
            },
        })
    );

    assert_eq!(
        parse_bring_args(&args(&["neo", "--to", "50-mawjs"])),
        Ok(ParsedBringArgs {
            oracle: "neo".to_owned(),
            opts: BringAliasOptions {
                session: Some("50-mawjs".to_owned()),
                ..BringAliasOptions::default()
            },
        })
    );
}

#[test]
fn bring_alias_rejects_missing_oracle_with_usage() {
    let error = parse_bring_args(&args(&["--tab"])).expect_err("missing oracle should fail");

    assert_eq!(
        error,
        BringArgsError {
            message: "bring: missing oracle name".to_owned(),
            usage: maw_bring::bring_usage_lines(),
        }
    );
    assert!(error.usage.join("\n").contains("usage: maw bring"));
}
