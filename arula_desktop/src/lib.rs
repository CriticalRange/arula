//! Arula Desktop - A modern AI assistant GUI built with Iced.

pub mod animation;
pub mod canvas;
pub mod config;
pub mod constants;
pub mod dispatcher;
pub mod session;
pub mod styles;
pub mod theme;

pub use animation::{
    LiquidMenuState, LivingBackgroundState, SettingsMenuState, SettingsPage, TiltCardState,
    TransitionDirection,
};
pub use config::{collect_provider_options, ConfigForm};
pub use constants::*;
pub use dispatcher::Dispatcher;
// Re-export UiEvent from core for convenience
pub use arula_core::UiEvent;
// Re-export project_context from core
pub use arula_core::detect_project;
pub use arula_core::generate_auto_manifest;
pub use arula_core::is_ai_enhanced;
pub use arula_core::manifest_exists;
pub use arula_core::DetectedProject;
pub use arula_core::ProjectType;
pub use arula_core::MANIFEST_MARKER_AI;
pub use arula_core::MANIFEST_MARKER_AUTO;
pub use session::{MessageEntry, Session};
pub use styles::*;
pub use theme::{app_theme, app_theme_with_mode, palette, palette_from_mode, PaletteColors, ThemeMode};
