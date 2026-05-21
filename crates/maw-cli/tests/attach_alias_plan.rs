use maw_cli::{run_cli, CliOutput};

fn run(args: &[&str]) -> CliOutput {
    run_cli(&args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>())
}

#[test]
fn top_level_a_alias_matches_attach_plan_json() {
    let alias = run(&["a", "50-mawjs", "--alive", "50-mawjs", "--plan-json"]);
    let attach = run(&["attach", "50-mawjs", "--alive", "50-mawjs", "--plan-json"]);
    assert_eq!(alias.code, 0, "{}", alias.stderr);
    assert_eq!(alias, attach);
    assert!(alias.stdout.contains("\"command\":\"attach\""));
    assert!(alias.stdout.contains("\"alias\":\"a\""));
    assert!(alias
        .stdout
        .contains("\"tmuxArgs\":[\"attach\",\"-t\",\"50-mawjs\"]"));
}

#[test]
fn attach_text_and_readonly_recovery_match_maw_js_surface() {
    let text = run(&["a", "50-mawjs:1.0", "--alive", "50-mawjs", "--print"]);
    assert_eq!(text.code, 0, "{}", text.stderr);
    assert!(text.stdout.contains("Run: tmux attach -t 50-mawjs"));
    assert!(text.stdout.contains("resolved: 50-mawjs:1.0 → 50-mawjs"));

    let readonly = run(&[
        "attach",
        "50-mawjs",
        "--alive",
        "50-mawjs",
        "--readonly",
        "--plan-json",
    ]);
    assert_eq!(readonly.code, 0, "{}", readonly.stderr);
    assert!(readonly
        .stdout
        .contains("\"tmuxArgs\":[\"attach\",\"-r\",\"-t\",\"50-mawjs\"]"));

    let missing = run(&["a", "ghost", "--alive", "50-mawjs", "--plan-json"]);
    assert_eq!(missing.code, 1);
    assert!(missing.stdout.contains("\"action\":\"recover\""));
}

#[test]
fn attach_usage_and_default_menu_show_only_live_ported_commands() {
    let usage = run(&[]);
    assert_eq!(usage.code, 0);
    assert!(usage.stdout.contains("ported commands:"));
    assert!(usage.stdout.contains("a|attach <target>"));
    assert!(usage.stdout.contains("ls [--compact|-c]"));
    assert!(!usage.stdout.contains("pair-api"));
    assert!(!usage.stdout.contains("plugin-manifest"));

    let help = run(&["a", "--help"]);
    assert_eq!(help.code, 0);
    assert!(help.stdout.contains("maw-rs a <target>"));

    let error = run(&["a", "--bad"]);
    assert_eq!(error.code, 2);
    assert!(error.stderr.contains("attach: unknown argument --bad"));
}
