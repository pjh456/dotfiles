use notify_rust::Notification;

pub fn send(summary: &str, body: &str, icon: &str) {
    let result = Notification::new()
        .appname("dotctl")
        .icon(icon)
        .summary(summary)
        .body(body)
        .show();
    if let Err(err) = result {
        eprintln!("dotctl: could not show notification: {err}");
    }
}
