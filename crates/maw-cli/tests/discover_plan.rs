// Ported from maw-js test/isolated/discover-plugin-peer-sources.test.ts
// into a side-by-side, fake-backed maw-rs discover plan CLI surface.

use maw_cli::run_cli;
use serde_json::json;

fn json_output(output: &maw_cli::CliOutput) -> serde_json::Value {
    assert_eq!(output.code, 0, "{}", output.stderr);
    serde_json::from_str(&output.stdout).unwrap_or_else(|error| {
        panic!("invalid json: {error}\n{}", output.stdout);
    })
}

#[test]
fn discover_plan_rejects_invalid_peer_source_before_fetch_or_live_probe() {
    let output = json_output(&run_cli(&[
        "discover".to_owned(),
        "--peers".to_owned(),
        "bogus".to_owned(),
        "--plan-json".to_owned(),
    ]));

    assert_eq!(
        output,
        json!({
            "command": "discover",
            "ok": false,
            "error": "invalid_peer_source",
            "output": "usage: maw discover [--peers config|scout|both] [--json] [--tree] [--awake]",
            "fetchCalls": 0,
            "liveProbeCalls": 0
        })
    );
}

#[test]
fn discover_plan_renders_inline_scout_text_without_live_probe() {
    let output = run_cli(&[
        "discover".to_owned(),
        "--peers=scout".to_owned(),
        "--discovered".to_owned(),
        "scout-node|scout-host|mawjs|http://scout:3456".to_owned(),
    ]);

    assert_eq!(output.code, 0, "{}", output.stderr);
    assert!(output.stdout.contains("scout-node"), "{}", output.stdout);
    assert!(
        output.stdout.contains("http://scout:3456"),
        "{}",
        output.stdout
    );
    assert!(
        !output.stdout.contains("tmux"),
        "scout text should not probe/render live state: {}",
        output.stdout
    );
}

#[test]
fn discover_plan_renders_config_json_with_live_peer_metadata() {
    let output = json_output(&run_cli(&[
        "discover".to_owned(),
        "--peers".to_owned(),
        "config".to_owned(),
        "--peer".to_owned(),
        "http://config:3456".to_owned(),
        "--named-peer".to_owned(),
        "named=http://named:3456".to_owned(),
        "--pane".to_owned(),
        "%1|claude|101-mawjs:agent.0|named|-|/repo/mawjs-oracle|-".to_owned(),
        "--json".to_owned(),
        "--plan-json".to_owned(),
    ]));

    assert_eq!(output["command"], "discover");
    assert_eq!(output["ok"], true);
    assert_eq!(output["mode"], "config");
    assert_eq!(output["total"], 2);
    assert_eq!(output["liveTotal"], 1);
    assert_eq!(output["fetchCalls"], 0);
    assert_eq!(output["liveProbeCalls"], 1);
    assert_eq!(output["live"]["panes"][0]["target"], "101-mawjs:agent.0");
    assert_eq!(output["live"]["sessions"][0]["name"], "101-mawjs");
    assert_eq!(
        output["peers"]
            .as_array()
            .expect("peers")
            .iter()
            .map(|peer| peer["url"].as_str().expect("peer url"))
            .collect::<Vec<_>>(),
        ["http://config:3456", "http://named:3456"]
    );
    let named = output["peers"]
        .as_array()
        .expect("peers")
        .iter()
        .find(|peer| peer["name"] == "named")
        .expect("named peer");
    assert_eq!(named["awake"], true);
    assert_eq!(named["liveTargets"], json!(["101-mawjs:agent.0"]));
    assert_eq!(
        output["plugins"],
        json!({"source": "plugin-registry", "total": 0, "records": []})
    );
    assert_eq!(
        output["fleet"],
        json!({"source": "fleet-config", "total": 0, "records": []})
    );
    assert_eq!(
        output["oracles"],
        json!({"source": "oracle-manifest", "total": 0, "records": []})
    );
    assert_eq!(
        output["ghq"],
        json!({"source": "ghq", "total": 0, "repos": []})
    );
}

#[test]
fn discover_plan_awake_json_filters_peers_but_preserves_live_panes() {
    let output = json_output(&run_cli(&[
        "discover".to_owned(),
        "--peer".to_owned(),
        "http://config:3456".to_owned(),
        "--named-peer".to_owned(),
        "named=http://named:3456".to_owned(),
        "--pane".to_owned(),
        "%1|claude|101-mawjs:agent.0|named|-|/repo/mawjs-oracle|-".to_owned(),
        "--awake".to_owned(),
        "--json".to_owned(),
        "--plan-json".to_owned(),
    ]));

    assert_eq!(output["ok"], true);
    assert_eq!(output["awake"], true);
    assert_eq!(output["total"], 1);
    assert_eq!(output["liveTotal"], 1);
    assert_eq!(output["live"]["panes"][0]["target"], "101-mawjs:agent.0");
    assert_eq!(
        output["peers"]
            .as_array()
            .expect("peers")
            .iter()
            .map(|peer| peer["name"].as_str().unwrap_or("-"))
            .collect::<Vec<_>>(),
        ["named"]
    );
    assert_eq!(output["plugins"]["records"], json!([]));
    assert_eq!(output["ghq"]["repos"], json!([]));
}
