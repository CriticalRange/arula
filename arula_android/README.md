# Arula Terminal for Android

Android version of Arula AI terminal assistant, built with Java UI and Rust core integration.

## Architecture

### Components

1. **Java UI Layer** (`app/src/main/java/com/arula/terminal/`)
   - `MainActivity.java` - Main chat interface
   - `MessageAdapter.java` - RecyclerView adapter for messages
   - `ArulaNative.java` - JNI bridge to Rust core
   - `SettingsActivity.java` - Configuration management

2. **JNI Bridge** (`app/src/main/cpp/`)
   - `arula_jni.cpp` - C++ JNI implementation
   - Handles communication between Java and Rust

3. **Rust JNI Bridge** (`arula_jni/src/`)
   - `platform/android/` - Android-specific implementations
   - `mod.rs` - JNI exports and platform abstractions
   - `terminal.rs` - Termux:API integration
   - References `arula_core` from the parent workspace

## Features

- Full AI conversation interface
- Tool execution support (bash, file operations, etc.)
- Termux:API integration for terminal commands
- Configuration management
- Conversation history
- Real-time streaming responses

## Build Requirements

- Android Studio 2022.3.1 or later
- Android NDK 25.2.9519653 or later
- Rust 1.70+ with Android targets
- Termux:API (optional, for enhanced features)

## Build Instructions

1. Install Rust and Android targets:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
   ```

2. Install Android NDK via Android Studio:
   - Open Android Studio > Settings > SDK Manager > SDK Tools
   - Install NDK (Side by side)

3. Build the Rust JNI library:
   ```bash
   cd arula_android
   ./build_native.sh
   ```
   This will compile the native library for all Android architectures and place them in `app/src/main/jniLibs/`.

4. Open the `arula_android` folder in Android Studio and build the project

## Integration Points

### Termux:API Features

- Terminal command execution
- File system access
- System notifications
- Battery/Network status
- Sensor data (accelerometer, GPS, etc.)

### Android-Specific Adaptations

- **File System**: Uses Android scoped storage and content URIs
- **Terminal**: Integrates with Termux for command execution
- **Configuration**: Stores in SharedPreferences
- **Background**: Uses Android services for AI processing
- **Notifications**: Native Android notifications

## Next Steps

1. Implement remaining Android platform modules
2. Add Termux:API integration
3. Create comprehensive settings UI
4. Add conversation export/import
5. Implement background AI service
6. Add Voice input support

## Dependencies

### Android
- AndroidX AppCompat, Material Design
- RecyclerView for message list
- Termux View library for terminal emulation

### Rust
- jni for Java interop
- tokio for async runtime
- serde for JSON handling
- termux-api for Termux integration

## License

Same as main Arula project