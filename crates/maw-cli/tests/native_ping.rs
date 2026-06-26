use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_maw-rs"))
}

fn temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("maw-rs-{name}-{stamp}"));
    fs::create_dir_all(&path).expect("temp dir");
    path
}

fn write_script(path: &Path, body: &str) {
    fs::write(path, body).expect("write script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("chmod");
    }
}

#[test]
fn ping_native_peer_matches_committed_golden_without_real_maw_or_network() {
    let root = temp_dir("ping-native");
    let fake_bin = root.join("bin");
    let maw_home = root.join("maw-home");
    let config_dir = maw_home.join("config");
    fs::create_dir_all(&fake_bin).expect("fake bin");
    fs::create_dir_all(&config_dir).expect("config dir");

    write_script(
        &fake_bin.join("maw"),
        "#!/bin/sh\necho 'DELEGATED-MAW'\nexit 77\n",
    );
    write_script(
        &fake_bin.join("bun"),
        "#!/bin/sh\necho 'DELEGATED-BUN'\nexit 78\n",
    );
    write_script(
        &fake_bin.join("curl"),
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$FAKE_CURL_ARGS\"\nprintf '{\"enabled\":true,\"tokenPreview\":\"tokn****\"}__MAW_HTTP_STATUS__:200'\n",
    );
    fs::write(
        config_dir.join("maw.config.json"),
        r#"{"node":"local","oracle":"tester","namedPeers":[{"name":"fakepeer","url":"http://fake-peer.invalid:3456"}]}"#,
    )
    .expect("config");
    let curl_args = root.join("curl.args");
    let missing_peers = root.join("missing-peers.json");

    let output = Command::new(bin())
        .args(["ping", "fakepeer"])
        .env_clear()
        .env("MAW_HOME", &maw_home)
        .env("PEERS_FILE", &missing_peers)
        .env("MAW_JS_REF_DIR", "/nonexistent")
        .env("MAW_RS_PING_NOW_MS", "1000")
        .env("PATH", &fake_bin)
        .env("FAKE_CURL_ARGS", &curl_args)
        .output()
        .expect("run maw-rs");

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout");
    assert_eq!(
        stdout,
        include_str!("fixtures/native-ping/ping-peer-success.stdout")
    );
    assert!(!stdout.contains("DELEGATED-MAW"));
    assert!(!stdout.contains("DELEGATED-BUN"));
    assert_eq!(String::from_utf8(output.stderr).expect("stderr"), "");

    let curl = fs::read_to_string(curl_args).expect("curl args");
    assert!(curl.contains("http://fake-peer.invalid:3456/api/auth/status"));
    assert!(curl.contains("--\nhttp://fake-peer.invalid:3456/api/auth/status"));
    assert!(!curl.contains("sh\n-c"));
}
