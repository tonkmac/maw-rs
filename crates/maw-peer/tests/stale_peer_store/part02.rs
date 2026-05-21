struct TestDir {
    path: PathBuf,
}

const DAY_MS: u64 = 24 * 60 * 60 * 1000;
const NOW_MS: u64 = 1_779_062_400_000; // 2026-05-18T00:00:00.000Z

fn iso_days_ago(days: u64) -> String {
    let ms = NOW_MS - days * DAY_MS;
    match days {
        1 => "2026-05-17T00:00:00.000Z",
        3 => "2026-05-15T00:00:00.000Z",
        4 => "2026-05-14T00:00:00.000Z",
        7 => "2026-05-11T00:00:00.000Z",
        8 => "2026-05-10T00:00:00.000Z",
        9 => "2026-05-09T00:00:00.000Z",
        10 => "2026-05-08T00:00:00.000Z",
        20 => "2026-04-28T00:00:00.000Z",
        30 => "2026-04-18T00:00:00.000Z",
        40 => "2026-04-08T00:00:00.000Z",
        _ => panic!("unexpected fixture day offset {days}; ms={ms}"),
    }
    .to_owned()
}

fn store_from<const N: usize>(peers: [(&str, &str, String, Option<String>); N]) -> PeerStoreFile {
    let peers = peers
        .into_iter()
        .map(|(alias, url, added_at, last_seen)| {
            (
                alias.to_owned(),
                PeerRecord {
                    url: url.to_owned(),
                    node: None,
                    added_at,
                    last_seen,
                    last_error: None,
                    nickname: None,
                    pubkey: None,
                    pubkey_first_seen: None,
                    identity: None,
                    one_way: None,
                    last_symmetric_check: None,
                },
            )
        })
        .collect();
    PeerStoreFile { version: 1, peers }
}

impl TestDir {
    fn new(prefix: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
