#![allow(dead_code)]
#![allow(private_interfaces)]

use jni::JNIEnv;
use jni::objects::{JClass, JString};

pub mod platform;

// Export for JNI
pub use platform::android::*;


