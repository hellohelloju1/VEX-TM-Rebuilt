use tournament_manager_rs::web_sessions::*;
#[test]
fn sessions_are_random_revocable_and_bound_to_password() {
    let mut s = Sessions::default();
    let a = s.create("hash-a");
    let b = s.create("hash-a");
    assert_ne!(a, b);
    assert_eq!(a.len(), 64);
    assert!(s.valid(&a, "hash-a"));
    assert!(!s.valid(&a, "hash-b"));
    assert!(!s.valid(&a, ""));
    s.remove(&a);
    assert!(!s.valid(&a, "hash-a"));
}
#[test]
fn cookie_parser_selects_only_the_session_cookie() {
    assert_eq!(
        cookie("other=hello; tm_session=abc; theme=dark"),
        Some("abc")
    );
    assert_eq!(cookie("other=abc"), None);
}
