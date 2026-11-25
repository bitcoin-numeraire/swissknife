#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{webview::NewWindowResponse, AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use url::{Host, Url};

fn launcher_url(url: &Url) -> bool {
    matches!(
        (url.scheme(), url.host_str()),
        ("tauri", Some("localhost")) | ("http" | "https", Some("tauri.localhost"))
    )
}

fn origin(input: &str) -> Result<Url, String> {
    let url = Url::parse(input.trim()).map_err(|_| "Enter a complete instance URL.")?;
    let host = url.host().ok_or("Enter an instance hostname.")?;
    let loopback = match host {
        Host::Domain(name) => name == "localhost",
        Host::Ipv4(ip) => ip.is_loopback(),
        Host::Ipv6(ip) => ip.is_loopback(),
    };
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        return Err("Use HTTPS or a local loopback HTTP address.".into());
    }
    if matches!(
        url.host_str().map(|name| name.trim_end_matches('.')),
        Some("ipc.localhost" | "tauri.localhost")
    ) {
        return Err("This address is reserved for the desktop application.".into());
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Use an origin without credentials, a path, a query or a fragment.".into());
    }
    Ok(url)
}

fn can_navigate(url: &Url, instance: &Url, login: Option<&Url>) -> bool {
    url.username().is_empty()
        && url.password().is_none()
        && (url.origin() == instance.origin() || login.is_some_and(|allowed| url.origin() == allowed.origin()))
}

fn open_wallet(app: &AppHandle, instance: &str, login: Option<&str>) -> Result<(), String> {
    let instance = origin(instance)?;
    let login = login.filter(|value| !value.trim().is_empty()).map(origin).transpose()?;
    if let Some(wallet) = app.get_webview_window("wallet") {
        return wallet.set_focus().map_err(|e| e.to_string());
    }
    let title = format!("SwissKnife — {}", instance.origin().ascii_serialization());
    WebviewWindowBuilder::new(app, "wallet", WebviewUrl::External(instance.clone()))
        .title(title)
        .inner_size(1220., 840.)
        .min_inner_size(400., 480.)
        .on_navigation(move |url| can_navigate(url, &instance, login.as_ref()))
        .on_new_window(|url, _| {
            if url.scheme() == "https" && url.username().is_empty() && url.password().is_none() {
                let _ = open::that_detached(url.as_str());
            }
            NewWindowResponse::Deny
        })
        .build()
        .map_err(|e| e.to_string())?;
    if let Some(launcher) = app.get_webview_window("launcher") {
        launcher.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn connect(window: tauri::WebviewWindow, instance: String, login: Option<String>) -> Result<(), String> {
    if window.label() != "launcher" || !launcher_url(&window.url().map_err(|e| e.to_string())?) {
        return Err("Open the connection from the desktop launcher.".into());
    }
    open_wallet(window.app_handle(), &instance, login.as_deref())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![connect])
        .setup(|app| {
            WebviewWindowBuilder::new(app, "launcher", WebviewUrl::App("index.html".into()))
                .title("SwissKnife")
                .inner_size(480., 560.)
                .min_inner_size(380., 500.)
                .on_navigation(launcher_url)
                .on_new_window(|_, _| NewWindowResponse::Deny)
                .build()?;
            let args: Vec<_> = std::env::args().collect();
            if let Some(index) = args.iter().position(|arg| arg == "--instance") {
                let instance = args.get(index + 1).ok_or("--instance requires a dashboard origin")?;
                let login = args
                    .iter()
                    .position(|arg| arg == "--login-origin")
                    .and_then(|i| args.get(i + 1));
                open_wallet(app.handle(), instance, login.map(String::as_str))?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "wallet" && matches!(event, tauri::WindowEvent::Destroyed) {
                if let Some(launcher) = window.app_handle().get_webview_window("launcher") {
                    let _ = launcher.show();
                    let _ = launcher.set_focus();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("could not run SwissKnife Desktop");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_secure_and_loopback_origins() {
        for value in [
            "https://app.numeraire.tech",
            "http://localhost:8080",
            "http://127.0.0.1:8080",
            "http://[::1]:8080",
        ] {
            assert!(origin(value).is_ok(), "{value}");
        }
        for value in [
            "file:///etc",
            "https://user@example.com",
            "https://foo.test/?token=a",
            "https://foo.test/x",
            "http://example.org",
            "https://tauri.localhost.",
            "https://ipc.localhost",
            "https://foo.test/#fragment",
        ] {
            assert!(origin(value).is_err(), "{value}");
        }
    }

    #[test]
    fn wallet_navigation_uses_complete_origins() {
        let instance = origin("https://wallet.example").unwrap();
        let login = origin("https://login.example").unwrap();
        for url in ["https://wallet.example/me", "https://login.example/authorize"] {
            assert!(can_navigate(&Url::parse(url).unwrap(), &instance, Some(&login)));
        }
        for url in [
            "https://wallet.example.net",
            "https://wallet.example:8443",
            "http://wallet.example",
            "https://wallet.example@elsewhere.example",
            "https://user@wallet.example",
        ] {
            assert!(!can_navigate(&Url::parse(url).unwrap(), &instance, Some(&login)));
        }
        assert!(!can_navigate(&login, &instance, None));
        assert!(!launcher_url(&instance));
    }
}
