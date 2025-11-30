use std::path::Path;

fn main() {
    // Test the logger initialization
    match crate::utils::logger::init_global_logger() {
        Ok(_) => println!("✅ Logger initialized successfully"),
        Err(e) => {
            println!("❌ Failed to initialize logger: {}", e);
            return;
        }
    }

    // Test logging
    crate::utils::logger::info("Test INFO message");
    crate::utils::logger::debug("Test DEBUG message");
    crate::utils::logger::warn("Test WARN message");
    crate::utils::logger::error("Test ERROR message");

    // Check if the logs directory and file were created
    let logs_dir = Path::new(".arula/logs");
    let log_file = logs_dir.join("latest.log");

    if logs_dir.exists() {
        println!("✅ Logs directory created successfully");
    } else {
        println!("❌ Logs directory not created");
    }

    if log_file.exists() {
        println!("✅ latest.log file created successfully");
    } else {
        println!("❌ latest.log file not created");
    }

    println!("📝 Check .arula/logs/latest.log for logged messages");
}
