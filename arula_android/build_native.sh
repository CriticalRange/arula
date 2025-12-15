#!/bin/bash
# Build script for arula_android native library
# Requires: Android NDK, Rust targets for Android

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JNI_LIBS_DIR="$SCRIPT_DIR/app/src/main/jniLibs"

# Check for Android NDK
if [ -z "$ANDROID_NDK_HOME" ]; then
    # Try common locations
    if [ -d "$HOME/Android/Sdk/ndk" ]; then
        NDK_VERSION=$(ls -1 "$HOME/Android/Sdk/ndk" | head -n1)
        export ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/$NDK_VERSION"
    else
        echo "Error: ANDROID_NDK_HOME not set and NDK not found"
        echo "Install NDK via: Android Studio > SDK Manager > SDK Tools > NDK"
        exit 1
    fi
fi

echo "Using Android NDK: $ANDROID_NDK_HOME"

# Install Rust targets if not already installed
install_targets() {
    echo "Installing Rust Android targets..."
    rustup target add aarch64-linux-android
    rustup target add armv7-linux-androideabi
    rustup target add i686-linux-android
    rustup target add x86_64-linux-android
}

# Check if targets are installed
if ! rustup target list --installed | grep -q android; then
    install_targets
fi

# Build for each architecture
cd "$SCRIPT_DIR/arula_jni"

build_for_target() {
    local rust_target=$1
    local android_abi=$2
    
    echo "Building for $rust_target ($android_abi)..."
    
    # Set linker
    local TOOLCHAIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64"
    local API_LEVEL=24
    
    case $rust_target in
        aarch64-linux-android)
            export CC="$TOOLCHAIN/bin/aarch64-linux-android${API_LEVEL}-clang"
            export AR="$TOOLCHAIN/bin/llvm-ar"
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$CC
            ;;
        armv7-linux-androideabi)
            export CC="$TOOLCHAIN/bin/armv7a-linux-androideabi${API_LEVEL}-clang"
            export AR="$TOOLCHAIN/bin/llvm-ar"
            export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER=$CC
            ;;
        i686-linux-android)
            export CC="$TOOLCHAIN/bin/i686-linux-android${API_LEVEL}-clang"
            export AR="$TOOLCHAIN/bin/llvm-ar"
            export CARGO_TARGET_I686_LINUX_ANDROID_LINKER=$CC
            ;;
        x86_64-linux-android)
            export CC="$TOOLCHAIN/bin/x86_64-linux-android${API_LEVEL}-clang"
            export AR="$TOOLCHAIN/bin/llvm-ar"
            export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER=$CC
            ;;
    esac
    
    cargo build --release --target $rust_target
    
    # Copy to jniLibs
    mkdir -p "$JNI_LIBS_DIR/$android_abi"
    cp "target/$rust_target/release/libarula_android.so" "$JNI_LIBS_DIR/$android_abi/"
    
    echo "✓ Built $android_abi"
}

# Build all targets
build_for_target "aarch64-linux-android" "arm64-v8a"
build_for_target "armv7-linux-androideabi" "armeabi-v7a"
build_for_target "i686-linux-android" "x86"
build_for_target "x86_64-linux-android" "x86_64"

echo ""
echo "✓ All builds complete!"
echo "Native libraries installed to: $JNI_LIBS_DIR"
ls -la "$JNI_LIBS_DIR"/*/
