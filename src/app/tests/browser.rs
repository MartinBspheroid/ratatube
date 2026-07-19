use crate::app::browser::is_allowed_browser_url;

#[test]
fn browser_dispatch_rejects_unsafe_schemes_and_hosts() {
    assert!(is_allowed_browser_url(
        "https://www.youtube.com/watch?v=safe"
    ));
    assert!(is_allowed_browser_url("https://youtu.be/safe"));
    assert!(!is_allowed_browser_url("file:///etc/passwd"));
    assert!(!is_allowed_browser_url(
        "https://youtube.com@example.com/attack"
    ));
    assert!(!is_allowed_browser_url("https://example.com/watch?v=no"));
}
