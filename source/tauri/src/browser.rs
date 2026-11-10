// Web addresses for plugin websites, and the check that a command comes from an app page.

use tauri::{AppHandle, Manager, Url, Webview};

pub const HOME: &str = "https://www.google.com";

pub fn local_url(app: &AppHandle, url: &Url) -> bool {
    (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
        || (matches!(url.scheme(), "http" | "https") && url.host_str() == Some("tauri.localhost"))
        || (cfg!(dev)
            && app
                .config()
                .build
                .dev_url
                .as_ref()
                .is_some_and(|dev| dev.origin() == url.origin()))
}

pub fn trusted(webview: &Webview) -> Result<(), String> {
    let url = webview.url().map_err(|e| e.to_string())?;
    if crate::session::is_main(webview.label()) && local_url(webview.app_handle(), &url) {
        Ok(())
    } else {
        Err("Only the app windows may use this command.".into())
    }
}

pub fn address(text: &str) -> Result<Url, String> {
    let text = text.trim();
    let url = if text.contains("://") {
        Url::parse(text)
    } else {
        Url::parse(&format!("https://{text}"))
    }
    .map_err(|_| "Enter a valid HTTP or HTTPS address.".to_string())?;
    if !allows(&url) || url.host_str().is_none() {
        return Err("Only HTTP and HTTPS addresses without credentials are allowed.".into());
    }
    Ok(url)
}

fn allows(url: &Url) -> bool {
    matches!(url.scheme(), "https" | "http") && url.username().is_empty() && url.password().is_none()
}

// Navigations in plugin websites; during an exam they are logged for the teacher.
pub fn navigation(app: &AppHandle, url: &Url) -> bool {
    if url.as_str() == "about:blank" {
        return true;
    }
    let allowed = allows(url);
    crate::exam::record(
        app,
        format!("{}: {url}", if allowed { "Navigation allowed" } else { "Navigation blocked" }),
    );
    allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_addresses_without_credentials() {
        for url in ["https://any.example/path", "http://any.example"] {
            assert!(allows(&Url::parse(url).unwrap()), "{url}");
        }
        for url in ["https://user@any.example", "https://user:password@any.example", "file:///tmp/test", "javascript:alert(1)"] {
            assert!(!allows(&Url::parse(url).unwrap()), "{url}");
        }
    }
}
