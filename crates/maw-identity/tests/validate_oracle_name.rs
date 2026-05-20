use maw_identity::assert_valid_oracle_name;

#[test]
fn rejects_foo_view_with_message_suggesting_foo() {
    let err = assert_valid_oracle_name("foo-view").expect_err("foo-view is reserved");
    let message = err.to_string();
    assert!(message.contains("-view"));
    assert!(message.contains("'foo'"));
    assert_eq!(err.name(), "foo-view");
    assert_eq!(err.suggestion(), "foo");
}

#[test]
fn rejects_mawjs_view() {
    assert!(assert_valid_oracle_name("mawjs-view").is_err());
}

#[test]
fn accepts_bare_foo() {
    assert_valid_oracle_name("foo").expect("bare oracle names are valid");
}

#[test]
fn accepts_mawjs_neo() {
    assert_valid_oracle_name("mawjs-neo").expect("hyphenated oracle names are valid");
}

#[test]
fn accepts_view_foo_prefix() {
    assert_valid_oracle_name("view-foo").expect("view prefix is allowed");
}

#[test]
fn rejects_multi_word_oracle_view_suffix() {
    let err = assert_valid_oracle_name("multi-word-oracle-view")
        .expect_err("reserved suffix applies regardless of name length");
    assert!(err.to_string().contains("-view"));
    assert_eq!(err.suggestion(), "multi-word-oracle");
}
