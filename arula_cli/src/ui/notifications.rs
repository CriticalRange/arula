//! Desktop notifications support for terminal unfocused state
//! Based on codex-rs notifications implementation

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Desktop notification backend trait
pub trait NotificationBackend: Send + Sync {
    /// Send a notification with the given message
    fn notify(&mut self, message: &str) -> std::io::Result<()>;

    /// Get the backend kind
    fn kind(&self) -> NotificationBackendKind;
}

/// The type of notification backend being used
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationBackendKind {
    /// No notification support
    None,
    /// Linux desktop notifications via dbus
    LinuxDbus,
    /// macOS native notifications
    MacOS,
    /// Windows toast notifications
    WindowsToast,
}

/// Desktop notification manager
pub struct NotificationManager {
    backend: Option<Box<dyn NotificationBackend>>,
    terminal_focused: Arc<AtomicBool>,
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new(terminal_focused: Arc<AtomicBool>) -> Self {
        let backend = detect_backend();
        Self {
            backend,
            terminal_focused,
        }
    }

    /// Send a notification if the terminal is unfocused
    /// Returns true if a notification was sent
    pub fn notify_if_unfocused(&mut self, message: impl AsRef<str>) -> bool {
        if self.terminal_focused.load(Ordering::Relaxed) {
            return false;
        }

        let Some(backend) = self.backend.as_mut() else {
            return false;
        };

        let message = message.as_ref();
        match backend.notify(message) {
            Ok(()) => true,
            Err(e) => {
                eprintln!("Failed to send notification: {}", e);
                false
            }
        }
    }

    /// Set terminal focus state
    pub fn set_focused(&self, focused: bool) {
        self.terminal_focused.store(focused, Ordering::Relaxed);
    }

    /// Check if terminal is focused
    pub fn is_focused(&self) -> bool {
        self.terminal_focused.load(Ordering::Relaxed)
    }

    /// Get the backend kind
    pub fn backend_kind(&self) -> NotificationBackendKind {
        self.backend
            .as_ref()
            .map(|b| b.kind())
            .unwrap_or(NotificationBackendKind::None)
    }
}

/// Detect the appropriate notification backend for this platform
pub fn detect_backend() -> Option<Box<dyn NotificationBackend>> {
    #[cfg(target_os = "linux")]
    {
        LinuxDbusNotifier::new().map(|b| Box::new(b) as Box<dyn NotificationBackend>)
    }

    #[cfg(target_os = "macos")]
    {
        MacOsNotifier::new().map(|b| Box::new(b) as Box<dyn NotificationBackend>)
    }

    #[cfg(target_os = "windows")]
    {
        WindowsNotifier::new().map(|b| Box::new(b) as Box<dyn NotificationBackend>)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

/// Linux notification backend using dbus
#[cfg(target_os = "linux")]
pub struct LinuxDbusNotifier;

#[cfg(target_os = "linux")]
impl LinuxDbusNotifier {
    pub fn new() -> Option<Self> {
        // Check if dbus is available
        let has_dbus = std::process::Command::new("which")
            .arg("notify-send")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if has_dbus {
            Some(Self)
        } else {
            None
        }
    }
}

#[cfg(target_os = "linux")]
impl NotificationBackend for LinuxDbusNotifier {
    fn notify(&mut self, message: &str) -> std::io::Result<()> {
        std::process::Command::new("notify-send")
            .arg("ARULA")
            .arg(message)
            .arg("--urgency=low")
            .arg("--app-id=com.arula.cli")
            .status()
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to send notification: {}", e),
                )
            })?;
        Ok(())
    }

    fn kind(&self) -> NotificationBackendKind {
        NotificationBackendKind::LinuxDbus
    }
}

/// macOS notification backend
#[cfg(target_os = "macos")]
pub struct MacOsNotifier;

#[cfg(target_os = "macos")]
impl MacOsNotifier {
    pub fn new() -> Option<Self> {
        // macOS always has terminal-notifier or osascript available
        Some(Self)
    }
}

#[cfg(target_os = "macos")]
impl NotificationBackend for MacOsNotifier {
    fn notify(&mut self, message: &str) -> std::io::Result<()> {
        // Try terminal-notifier first, fall back to osascript
        let result = std::process::Command::new("terminal-notifier")
            .arg("-title")
            .arg("ARULA")
            .arg("-message")
            .arg(message)
            .arg("-sound")
            .arg("default")
            .status();

        if result.is_err() {
            // Fall back to osascript
            let script = format!(
                "display notification \"{}\" with title \"ARULA\"",
                message.replace('"', "\\'")
            );
            std::process::Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .status()
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to send notification: {}", e),
                    )
                })?;
        }

        Ok(())
    }

    fn kind(&self) -> NotificationBackendKind {
        NotificationBackendKind::MacOS
    }
}

/// Windows notification backend
#[cfg(target_os = "windows")]
pub struct WindowsNotifier;

#[cfg(target_os = "windows")]
impl WindowsNotifier {
    pub fn new() -> Option<Self> {
        // Windows toast notifications via PowerShell
        Some(Self)
    }
}

#[cfg(target_os = "windows")]
impl NotificationBackend for WindowsNotifier {
    fn notify(&mut self, message: &str) -> std::io::Result<()> {
        let escaped_message = message.replace('"', "\"\"").replace('\'', "\\'");
        let ps_script = format!(
            r#"
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime]::CreateToastNotifier("ARULA").Show(
    [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime]::new().LoadXml(
        "<toast><visual><binding template='ToastText01'><text id='1'>{}</text></binding></visual></toast>"
    )
)
"#,
            escaped_message
        );

        std::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(&ps_script)
            .status()
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to send notification: {}", e),
                )
            })?;
        Ok(())
    }

    fn kind(&self) -> NotificationBackendKind {
        NotificationBackendKind::WindowsToast
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_manager_creation() {
        let focus = Arc::new(AtomicBool::new(true));
        let manager = NotificationManager::new(focus);
        assert!(manager.is_focused());
    }

    #[test]
    fn test_notification_manager_unfocused() {
        let focus = Arc::new(AtomicBool::new(false));
        let mut manager = NotificationManager::new(focus);
        assert!(!manager.is_focused());
        // Should not crash even without backend
        let sent = manager.notify_if_unfocused("Test message");
        // May be false if no backend available
    }
}
