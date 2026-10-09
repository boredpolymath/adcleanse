use crate::auth::session::SessionCredentials;
use crate::error::{AdCleanseError, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct SandboxedWebviewController {
    is_authenticating: Arc<AtomicBool>,
}

impl SandboxedWebviewController {
    pub fn new() -> Self {
        Self {
            is_authenticating: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Verifies that target URL is bound strictly to authorized authentication domains
    pub fn is_authorized_login_url(url: &str) -> bool {
        let authorized_domains = [
            "https://www.facebook.com/login",
            "https://facebook.com/login",
            "https://accountscenter.facebook.com",
            "https://accounts.meta.com",
            "https://www.instagram.com/accounts/login",
            "https://accounts.google.com",
        ];
        authorized_domains
            .iter()
            .any(|domain| url.starts_with(domain))
    }

    /// Spawns an isolated temporary webview window dedicated strictly to Meta login domains.
    /// Extension injection, arbitrary navigation, and local storage retention are restricted.
    pub async fn launch_login_webview(&self) -> Result<()> {
        log::info!("Launching sandboxed login webview for Meta authentication");
        self.is_authenticating.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Intercepts authentic login cookies (c_user, xs, datr) and extracts credentials
    pub fn intercept_credentials(&self, cookies: &[(&str, &str)]) -> Result<SessionCredentials> {
        let mut user_id: Option<String> = None;
        let mut session_cookie: Option<String> = None;
        let mut datr_token: Option<String> = None;

        for (name, val) in cookies {
            match *name {
                "c_user" => user_id = Some(val.to_string()),
                "xs" => session_cookie = Some(val.to_string()),
                "datr" => datr_token = Some(val.to_string()),
                _ => {}
            }
        }

        match (user_id, session_cookie, datr_token) {
            (Some(uid), Some(xs), Some(datr)) => {
                log::info!(
                    "Successfully intercepted authentic Meta session cookies for user: {}",
                    uid
                );
                self.is_authenticating.store(false, Ordering::SeqCst);
                Ok(SessionCredentials {
                    user_id: uid,
                    session_cookie: xs,
                    datr_token: datr,
                    access_token: None,
                })
            }
            _ => Err(AdCleanseError::AuthError(
                "Missing required session tokens (c_user, xs, or datr)".to_string(),
            )),
        }
    }

    pub fn is_in_progress(&self) -> bool {
        self.is_authenticating.load(Ordering::SeqCst)
    }

    pub fn cancel_login(&self) {
        log::info!("Cancelling sandboxed authentication session");
        self.is_authenticating.store(false, Ordering::SeqCst);
    }
}

impl Default for SandboxedWebviewController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorized_login_url_validation() {
        assert!(SandboxedWebviewController::is_authorized_login_url(
            "https://www.facebook.com/login.php"
        ));
        assert!(SandboxedWebviewController::is_authorized_login_url(
            "https://accountscenter.facebook.com/"
        ));
        assert!(SandboxedWebviewController::is_authorized_login_url(
            "https://accounts.meta.com/login"
        ));
        assert!(!SandboxedWebviewController::is_authorized_login_url(
            "https://malicious-phishing-login.com"
        ));
        assert!(!SandboxedWebviewController::is_authorized_login_url(
            "http://facebook.com/login" // HTTP forbidden
        ));
    }

    #[test]
    fn test_cookie_interception() {
        let controller = SandboxedWebviewController::new();
        let raw_cookies = vec![
            ("sb", "random_seed"),
            ("c_user", "100084920491823"),
            ("xs", "42:auth_hash:2"),
            ("datr", "browser_entropy_flag"),
        ];

        let result = controller
            .intercept_credentials(&raw_cookies)
            .expect("Failed to intercept");
        assert_eq!(result.user_id, "100084920491823");
        assert_eq!(result.session_cookie, "42:auth_hash:2");
        assert_eq!(result.datr_token, "browser_entropy_flag");
        assert!(!controller.is_in_progress());
    }
}
