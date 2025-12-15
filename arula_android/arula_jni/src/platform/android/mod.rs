//! Android-specific platform implementations

use anyhow::Result;
use jni::{JNIEnv, objects::{JClass, JString, JObject}, sys::jobject};
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod terminal;
pub mod filesystem;
pub mod command;
pub mod config;
pub mod notification;

pub use terminal::AndroidTerminal;
pub use filesystem::AndroidFileSystem;
pub use command::AndroidCommandExecutor;
pub use config::AndroidConfig;
pub use notification::AndroidNotification;

/// Android platform context
#[derive(Clone)]
pub struct AndroidContext {
    // Note: JVM is obtained from the JNI call, not stored
    pub context: Arc<Mutex<Option<jobject>>>,
    pub callback: Arc<Mutex<Option<jobject>>>,
}

impl AndroidContext {
    pub fn new() -> Self {
        Self {
            context: Arc::new(Mutex::new(None)),
            callback: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn set_context(&self, ctx: jobject) {
        *self.context.lock().await = Some(ctx);
    }

    pub async fn set_callback(&self, cb: jobject) {
        *self.callback.lock().await = Some(cb);
    }
}

impl Default for AndroidContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Android platform backend implementing all platform-specific traits
pub struct AndroidPlatform {
    ctx: AndroidContext,
    terminal: AndroidTerminal,
    filesystem: AndroidFileSystem,
    command: AndroidCommandExecutor,
    config: AndroidConfig,
    notification: AndroidNotification,
}

impl AndroidPlatform {
    pub fn new(ctx: AndroidContext) -> Self {
        Self {
            ctx: ctx.clone(),
            terminal: AndroidTerminal::new(ctx.clone()),
            filesystem: AndroidFileSystem::new(ctx.clone()),
            command: AndroidCommandExecutor::new(ctx.clone()),
            config: AndroidConfig::new(ctx.clone()),
            notification: AndroidNotification::new(ctx),
        }
    }

    pub fn terminal(&self) -> &AndroidTerminal {
        &self.terminal
    }

    pub fn filesystem(&self) -> &AndroidFileSystem {
        &self.filesystem
    }

    pub fn command(&self) -> &AndroidCommandExecutor {
        &self.command
    }

    pub fn config(&self) -> &AndroidConfig {
        &self.config
    }

    pub fn notification(&self) -> &AndroidNotification {
        &self.notification
    }
}

/// JNI exports for Android integration
#[no_mangle]
pub extern "C" fn Java_com_arula_terminal_ArulaNative_initialize<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    config_json: JString<'local>,
) -> bool {
    let config_str: String = match env.get_string(&config_json) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("Failed to get config string: {:?}", e);
            return false;
        }
    };

    // Initialize android logger
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("ArulaCore"),
    );

    log::info!("Arula Android Core initialized with config: {}", config_str);
    true
}

#[no_mangle]
pub extern "C" fn Java_com_arula_terminal_ArulaNative_sendMessage<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    message: JString<'local>,
) {
    // Send message to AI
    match env.get_string(&message) {
        Ok(msg) => {
            let msg_str: String = msg.into();
            log::info!("Sending message: {}", msg_str);
        }
        Err(e) => {
            log::error!("Failed to get message string: {:?}", e);
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_arula_terminal_ArulaNative_setConfig<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    _config_json: JString<'local>,
) {
    // Update configuration
}

#[no_mangle]
pub extern "C" fn Java_com_arula_terminal_ArulaNative_getConfig<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> JString<'local> {
    // Return current configuration
    let config = "{}";
    match env.new_string(config) {
        Ok(s) => s,
        Err(_) => JString::default(),
    }
}

#[no_mangle]
pub extern "C" fn Java_com_arula_terminal_ArulaNative_cleanup<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
) {
    // Cleanup resources
    log::info!("Android Arula cleanup");
}

#[no_mangle]
pub extern "C" fn Java_com_arula_terminal_ArulaNative_setCallback<'local>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
    _callback: JObject<'local>,
) {
    // Store callback for later use
    log::info!("Setting Android callback");
}

/// Callback functions from Rust to Java
pub mod callbacks {
    pub fn on_message(message: &str) {
        // Call Java callback
        log::info!("Message: {}", message);
    }

    pub fn on_stream_chunk(chunk: &str) {
        // Call Java callback for streaming
        log::debug!("Stream: {}", chunk);
    }

    pub fn on_tool_start(tool_name: &str, tool_id: &str) {
        // Notify Java of tool execution
        log::info!("Tool started: {} ({})", tool_name, tool_id);
    }

    pub fn on_tool_complete(tool_id: &str, result: &str) {
        // Notify Java of tool completion
        log::info!("Tool completed: {} - {}", tool_id, result);
    }

    pub fn on_error(error: &str) {
        // Notify Java of error
        log::error!("Error: {}", error);
    }
}