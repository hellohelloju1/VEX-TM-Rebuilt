use tournament_manager_rs::web_control::*;
#[test]
fn reads_fragmented_json_body() {
    struct Small(std::io::Cursor<Vec<u8>>);
    impl std::io::Read for Small {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            let n = b.len().min(3);
            std::io::Read::read(&mut self.0, &mut b[..n])
        }
    }
    let mut input=Small(std::io::Cursor::new(b"POST /TM/api HTTP/1.1\r\nHost: localhost:8790\r\nContent-Type: application/json\r\nContent-Length: 2\r\nOrigin: http://localhost:8790\r\n\r\n{}".to_vec()));
    let r = read(&mut input).unwrap();
    assert_eq!(r.body, b"{}");
    assert!(valid_post(&r));
}
#[test]
fn rejects_cross_origin_and_form_posts() {
    for extra in [
        "Content-Type: application/json\r\nOrigin: http://evil.example",
        "Content-Type: application/x-www-form-urlencoded",
    ] {
        let raw = format!(
            "POST /TM/api HTTP/1.1\r\nHost: localhost:8790\r\n{extra}\r\nContent-Length: 0\r\n\r\n"
        );
        assert!(!valid_post(&read(&mut raw.as_bytes()).unwrap()));
    }
}
#[test]
fn restricts_browser_commands() {
    for a in [
        "snapshot",
        "send_to_fc",
        "timer",
        "score",
        "submit",
    ] {
        assert!(allowed_action(a));
    }
    for a in [
        "hardware",
        "open",
        "save",
        "commit",
        "server",
        "display_settings",
        "unlock",
    ] {
        assert!(!allowed_action(a));
    }
}
#[test]
fn rejects_oversize_and_ambiguous_requests() {
    for raw in [
        "POST /TM/api HTTP/1.1\r\nContent-Length: 1000001\r\n\r\n",
        "POST /TM/api HTTP/1.1\r\nContent-Length: 2\r\nContent-Length: 3\r\n\r\n{}",
        "POST /TM/api HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n",
    ] {
        assert!(read(&mut raw.as_bytes()).is_err());
    }
}
#[test]
fn operator_auth_always_requires_a_nonempty_password() {
    let hash = tournament_manager_rs::auth::hash("test-web-password").unwrap();
    assert!(!authorized(None, &hash));
    assert!(!authorized(Some("wrong"), &hash));
    assert!(authorized(Some("test-web-password"), &hash));
    assert!(!authorized(Some(""), ""));
}
