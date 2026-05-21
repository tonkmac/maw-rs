use maw_cli::{run_cli, CliOutput};

const PEER_KEY: &str = "feedfacefeedfacefeedfacefeedfacefeedfacefeedfacefeedfacefeedface";
const FROM: &str = "mawjs:m5";
const NOW: &str = "1700000000";

fn run(args: &[&str]) -> CliOutput {
    run_cli(&args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>())
}

fn ok(args: &[&str], expected: &str) {
    let output = run(args);
    assert_eq!(output.code, 0, "stderr for {args:?}: {}", output.stderr);
    assert!(
        output.stdout.contains(expected),
        "stdout for {args:?} did not contain {expected:?}: {}",
        output.stdout
    );
    assert!(
        output.stderr.is_empty(),
        "stderr for {args:?}: {}",
        output.stderr
    );
}

fn usage(args: &[&str], expected: &str) {
    let output = run(args);
    assert_eq!(output.code, 2, "stdout for {args:?}: {}", output.stdout);
    assert!(
        output.stderr.contains(expected),
        "stderr for {args:?} did not contain {expected:?}: {}",
        output.stderr
    );
    assert!(
        output.stdout.is_empty(),
        "stdout for {args:?}: {}",
        output.stdout
    );
}

#[test]
fn auth_sign_parsers_cover_optional_value_branches() {
    ok(
        &[
            "auth",
            "sign-v1",
            "--plan-json",
            "--token",
            "tok",
            "--method",
            "POST",
            "--path",
            "/api",
            "--now",
            NOW,
            "--body-hash",
            "abc",
        ],
        "\"method\":\"POST\"",
    );
    ok(
        &[
            "auth",
            "sign-headers",
            "--plan-json",
            "--token",
            "tok",
            "--method",
            "PUT",
            "--path",
            "/headers",
            "--now",
            NOW,
            "--body",
            "payload",
        ],
        "\"bodyHash\"",
    );
    usage(
        &["auth", "sign-v1", "--method"],
        "auth: missing --method value",
    );
    usage(
        &["auth", "sign-v1", "--odd"],
        "auth sign-v1: unknown argument --odd",
    );
    usage(
        &["auth", "sign-headers", "--now", "bad", "--token", "tok"],
        "auth sign-headers: --now must be an integer",
    );
    usage(
        &["auth", "sign-headers", "--odd"],
        "auth sign-headers: unknown argument --odd",
    );
}

#[test]
fn auth_verify_v1_parser_covers_all_value_and_required_branches() {
    ok(
        &[
            "auth",
            "verify-v1",
            "--plan-json",
            "--token",
            "tok",
            "--method",
            "PATCH",
            "--path",
            "/v1",
            "--signature",
            "bad",
            "--signed-at",
            NOW,
            "--now",
            NOW,
            "--body-hash",
            "abc",
        ],
        "\"kind\":\"verify-v1\"",
    );
    usage(
        &[
            "auth",
            "verify-v1",
            "--token",
            "tok",
            "--signature",
            "sig",
            "--now",
            NOW,
        ],
        "auth verify-v1: --signed-at is required",
    );
    usage(
        &[
            "auth",
            "verify-v1",
            "--token",
            "tok",
            "--signed-at",
            NOW,
            "--now",
            NOW,
        ],
        "auth verify-v1: --signature is required",
    );
    usage(
        &["auth", "verify-v1", "--odd"],
        "auth verify-v1: unknown argument --odd",
    );
    usage(
        &[
            "auth",
            "verify-v1",
            "--token",
            "tok",
            "--signature",
            "sig",
            "--signed-at",
            "bad",
        ],
        "auth verify-v1: --signed-at must be an integer",
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn auth_from_verify_parsers_cover_legacy_and_v3_value_branches() {
    ok(
        &[
            "auth",
            "verify-legacy-from",
            "--plan-json",
            "--cached-pubkey",
            PEER_KEY,
            "--from",
            FROM,
            "--signed-at",
            NOW,
            "--signature",
            "sig",
            "--method",
            "POST",
            "--path",
            "/legacy",
            "--now",
            NOW,
            "--body",
            "body",
        ],
        "\"kind\":\"verify-legacy-from\"",
    );
    ok(
        &[
            "auth",
            "verify-v3-from",
            "--plan-json",
            "--cached-pubkey",
            PEER_KEY,
            "--from",
            FROM,
            "--timestamp",
            NOW,
            "--signature-v3",
            "sig",
            "--method",
            "POST",
            "--path",
            "/v3",
            "--now",
            NOW,
            "--body",
            "body",
        ],
        "\"kind\":\"verify-v3-from\"",
    );
    usage(
        &[
            "auth",
            "verify-legacy-from",
            "--signature",
            "sig",
            "--signed-at",
            NOW,
            "--now",
            NOW,
        ],
        "auth verify-legacy-from: --from is required",
    );
    usage(
        &[
            "auth",
            "verify-legacy-from",
            "--from",
            FROM,
            "--signed-at",
            NOW,
            "--now",
            NOW,
        ],
        "auth verify-legacy-from: --signature is required",
    );
    usage(
        &[
            "auth",
            "verify-legacy-from",
            "--from",
            FROM,
            "--signature",
            "sig",
            "--now",
            NOW,
        ],
        "auth verify-legacy-from: --signed-at is required",
    );
    usage(
        &[
            "auth",
            "verify-legacy-from",
            "--from",
            FROM,
            "--signature",
            "sig",
            "--signed-at",
            NOW,
            "--now",
            "bad",
        ],
        "auth verify-legacy-from: --now must be an integer",
    );
    usage(
        &["auth", "verify-legacy-from", "--odd"],
        "auth verify-legacy-from: unknown argument --odd",
    );
    usage(
        &[
            "auth",
            "verify-v3-from",
            "--signature-v3",
            "sig",
            "--timestamp",
            NOW,
            "--now",
            NOW,
        ],
        "auth verify-v3-from: --from is required",
    );
    usage(
        &[
            "auth",
            "verify-v3-from",
            "--from",
            FROM,
            "--signature-v3",
            "sig",
            "--now",
            NOW,
        ],
        "auth verify-v3-from: --timestamp is required",
    );
    usage(
        &[
            "auth",
            "verify-v3-from",
            "--from",
            FROM,
            "--signature-v3",
            "sig",
            "--timestamp",
            NOW,
            "--now",
            "bad",
        ],
        "auth verify-v3-from: --now must be an integer",
    );
    usage(
        &["auth", "verify-v3-from", "--odd"],
        "auth verify-v3-from: unknown argument --odd",
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn auth_payload_and_hmac_parsers_cover_remaining_required_branches() {
    ok(
        &[
            "auth",
            "from-sign-payload",
            "--plan-json",
            "--legacy",
            "--from",
            FROM,
            "--signed-at",
            NOW,
            "--method",
            "POST",
            "--path",
            "/payload",
            "--body-hash",
            "abc",
        ],
        "\"version\":\"legacy\"",
    );
    ok(
        &[
            "auth",
            "from-sign-payload",
            "--plan-json",
            "--from",
            FROM,
            "--timestamp",
            NOW,
            "--method",
            "POST",
            "--path",
            "/payload",
            "--body-hash",
            "abc",
        ],
        "\"version\":\"v3\"",
    );
    ok(
        &[
            "auth",
            "hmac-verify",
            "--plan-json",
            "--secret",
            "s",
            "--payload",
            "p",
            "--signature",
            "sig",
        ],
        "\"kind\":\"hmac-verify\"",
    );
    usage(
        &["auth", "from-sign-payload", "--timestamp", NOW],
        "auth from-sign-payload: --from is required",
    );
    usage(
        &[
            "auth",
            "from-sign-payload",
            "--timestamp",
            "bad",
            "--from",
            FROM,
        ],
        "auth from-sign-payload: --timestamp must be an integer",
    );
    usage(
        &["auth", "from-sign-payload", "--odd"],
        "auth from-sign-payload: unknown argument --odd",
    );
    usage(
        &[
            "auth",
            "hmac-verify",
            "--payload",
            "p",
            "--signature",
            "sig",
        ],
        "auth hmac-verify: --secret is required",
    );
    usage(
        &["auth", "hmac-verify", "--secret", "s", "--signature", "sig"],
        "auth hmac-verify: --payload is required",
    );
    usage(
        &["auth", "hmac-verify", "--secret", "s", "--payload", "p"],
        "auth hmac-verify: --signature is required",
    );
    usage(
        &["auth", "hmac-verify", "--odd"],
        "auth hmac-verify: unknown argument --odd",
    );
}
