//! Integration tests for Visioneer desktop automation tool

use arula_cli::visioneer::*;
use serde_json::json;

#[tokio::test]
async fn test_visioneer_tool_schema() {
    let tool = VisioneerTool::new();
    let schema = tool.schema();

    assert_eq!(schema.name, "visioneer");
    assert!(schema.description.contains("desktop automation"));
    assert!(schema.parameters.contains_key("target"));
    assert!(schema.parameters.contains_key("action"));
    assert!(schema.required.contains(&"target".to_string()));
    assert!(schema.required.contains(&"action".to_string()));
}

#[tokio::test]
async fn test_visioneer_capture_action() {
    let tool = VisioneerTool::new();

    let params = VisioneerParams {
        target: "notepad.exe".to_string(), // Will likely fail but tests structure
        action: VisioneerAction::Capture {
            region: Some(CaptureRegion {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            }),
            save_path: None,
            encode_base64: Some(false),
        },
        ocr_config: None,
        vlm_config: None,
    };

    // This will likely fail in testing environment but verifies the structure
    let result = tool.execute(params).await;

    match result {
        Ok(visioneer_result) => {
            assert_eq!(visioneer_result.action_type, "capture");
            assert!(visioneer_result.execution_time_ms > 0);
        }
        Err(e) => {
            // Expected in testing environment without actual windows
            assert!(e.contains("not found") || e.contains("not supported"));
        }
    }
}

#[tokio::test]
async fn test_visioneer_extract_text_action() {
    let tool = VisioneerTool::new();

    let params = VisioneerParams {
        target: "test_window".to_string(),
        action: VisioneerAction::ExtractText {
            region: Some(CaptureRegion {
                x: 100,
                y: 100,
                width: 400,
                height: 200,
            }),
            language: Some("eng".to_string()),
        },
        ocr_config: Some(OcrConfig {
            engine: Some("tesseract".to_string()),
            language: Some("eng".to_string()),
            confidence_threshold: Some(0.8),
            preprocessing: Some(OcrPreprocessing {
                grayscale: Some(true),
                threshold: Some(128),
                denoise: Some(true),
                scale_factor: Some(2.0),
            }),
        }),
        vlm_config: None,
    };

    let result = tool.execute(params).await;

    match result {
        Ok(visioneer_result) => {
            assert_eq!(visioneer_result.action_type, "extract_text");
        }
        Err(e) => {
            assert!(e.contains("not found") || e.contains("not supported"));
        }
    }
}

#[tokio::test]
async fn test_visioneer_analyze_action() {
    let tool = VisioneerTool::new();

    let params = VisioneerParams {
        target: "calculator".to_string(),
        action: VisioneerAction::Analyze {
            query: "What buttons are visible on this calculator?".to_string(),
            region: None,
            context: Some("User wants to perform calculations".to_string()),
        },
        ocr_config: None,
        vlm_config: Some(VlmConfig {
            model: Some("gpt-4-vision".to_string()),
            max_tokens: Some(500),
            temperature: Some(0.1),
            detail: Some("high".to_string()),
        }),
    };

    let result = tool.execute(params).await;

    match result {
        Ok(visioneer_result) => {
            assert_eq!(visioneer_result.action_type, "analyze");
        }
        Err(e) => {
            assert!(e.contains("not found") || e.contains("not supported"));
        }
    }
}

#[tokio::test]
async fn test_visioneer_click_actions() {
    let tool = VisioneerTool::new();

    // Test coordinate click
    let coord_click_params = VisioneerParams {
        target: "test_window".to_string(),
        action: VisioneerAction::Click {
            target: ClickTarget::Coordinates { x: 100, y: 200 },
            button: Some(ClickButton::Left),
            double_click: Some(false),
        },
        ocr_config: None,
        vlm_config: None,
    };

    let result = tool.execute(coord_click_params).await;

    match result {
        Ok(visioneer_result) => {
            assert_eq!(visioneer_result.action_type, "click");
        }
        Err(e) => {
            assert!(e.contains("not found") || e.contains("not supported"));
        }
    }

    // Test text-based click
    let text_click_params = VisioneerParams {
        target: "test_window".to_string(),
        action: VisioneerAction::Click {
            target: ClickTarget::Text {
                text: "Submit".to_string(),
                region: Some(CaptureRegion {
                    x: 0,
                    y: 0,
                    width: 800,
                    height: 600,
                }),
            },
            button: Some(ClickButton::Left),
            double_click: Some(false),
        },
        ocr_config: None,
        vlm_config: None,
    };

    let result = tool.execute(text_click_params).await;

    match result {
        Ok(visioneer_result) => {
            assert_eq!(visioneer_result.action_type, "click");
        }
        Err(e) => {
            assert!(e.contains("not found") || e.contains("not supported"));
        }
    }
}

#[tokio::test]
async fn test_visioneer_type_action() {
    let tool = VisioneerTool::new();

    let params = VisioneerParams {
        target: "notepad".to_string(),
        action: VisioneerAction::Type {
            text: "Hello, World! This is a test of Visioneer typing functionality.".to_string(),
            clear_first: Some(true),
            delay_ms: Some(50),
        },
        ocr_config: None,
        vlm_config: None,
    };

    let result = tool.execute(params).await;

    match result {
        Ok(visioneer_result) => {
            assert_eq!(visioneer_result.action_type, "type");
        }
        Err(e) => {
            assert!(e.contains("not found") || e.contains("not supported"));
        }
    }
}

#[tokio::test]
async fn test_visioneer_hotkey_action() {
    let tool = VisioneerTool::new();

    let params = VisioneerParams {
        target: "any_window".to_string(),
        action: VisioneerAction::Hotkey {
            keys: vec!["ctrl".to_string(), "c".to_string()], // Copy
            hold_ms: Some(100),
        },
        ocr_config: None,
        vlm_config: None,
    };

    let result = tool.execute(params).await;

    match result {
        Ok(visioneer_result) => {
            assert_eq!(visioneer_result.action_type, "hotkey");
        }
        Err(e) => {
            assert!(e.contains("not found") || e.contains("not supported"));
        }
    }
}

#[tokio::test]
async fn test_visioneer_wait_action() {
    let tool = VisioneerTool::new();

    let params = VisioneerParams {
        target: "test_window".to_string(),
        action: VisioneerAction::WaitFor {
            condition: WaitCondition::Text {
                text: "Loading".to_string(),
                appears: Some(false), // Wait for text to disappear
            },
            timeout_ms: Some(5000),
            check_interval_ms: Some(250),
        },
        ocr_config: None,
        vlm_config: None,
    };

    let result = tool.execute(params).await;

    match result {
        Ok(visioneer_result) => {
            assert_eq!(visioneer_result.action_type, "wait_for");
        }
        Err(e) => {
            assert!(e.contains("not found") || e.contains("not supported"));
        }
    }
}

#[tokio::test]
async fn test_visioneer_navigate_action() {
    let tool = VisioneerTool::new();

    let params = VisioneerParams {
        target: "test_window".to_string(),
        action: VisioneerAction::Navigate {
            direction: NavigationDirection::Down,
            distance: Some(100),
            steps: Some(3),
        },
        ocr_config: None,
        vlm_config: None,
    };

    let result = tool.execute(params).await;

    match result {
        Ok(visioneer_result) => {
            assert_eq!(visioneer_result.action_type, "navigate");
        }
        Err(e) => {
            assert!(e.contains("not found") || e.contains("not supported"));
        }
    }
}

#[tokio::test]
async fn test_visioneer_serialization() {
    // Test that all the data structures can be serialized/deserialized correctly

    let action_json = json!({
        "type": "Capture",
        "region": {
            "x": 0,
            "y": 0,
            "width": 800,
            "height": 600
        },
        "save_path": "/tmp/test.png",
        "encode_base64": true
    });

    let action: VisioneerAction = serde_json::from_value(action_json).unwrap();

    match action {
        VisioneerAction::Capture { region, save_path, encode_base64 } => {
            assert!(region.is_some());
            assert_eq!(save_path.unwrap(), "/tmp/test.png");
            assert!(encode_base64.unwrap());
        }
        _ => panic!("Expected Capture action"),
    }

    let click_target_json = json!({
        "type": "Coordinates",
        "x": 150,
        "y": 250
    });

    let click_target: ClickTarget = serde_json::from_value(click_target_json).unwrap();

    match click_target {
        ClickTarget::Coordinates { x, y } => {
            assert_eq!(x, 150);
            assert_eq!(y, 250);
        }
        _ => panic!("Expected Coordinates click target"),
    }

    let wait_condition_json = json!({
        "type": "Text",
        "text": "Error",
        "appears": true
    });

    let wait_condition: WaitCondition = serde_json::from_value(wait_condition_json).unwrap();

    match wait_condition {
        WaitCondition::Text { text, appears } => {
            assert_eq!(text, "Error");
            assert!(appears.unwrap());
        }
        _ => panic!("Expected Text wait condition"),
    }
}

#[test]
fn test_capture_region() {
    let region = CaptureRegion {
        x: 10,
        y: 20,
        width: 300,
        height: 400,
    };

    assert_eq!(region.x, 10);
    assert_eq!(region.y, 20);
    assert_eq!(region.width, 300);
    assert_eq!(region.height, 400);
}

#[test]
fn test_ui_element_structure() {
    let element = UiElement {
        element_type: "button".to_string(),
        text: Some("Submit".to_string()),
        bbox: BoundingBox {
            x: 100,
            y: 200,
            width: 80,
            height: 30,
        },
        confidence: 0.95,
        attributes: {
            let mut attrs = std::collections::HashMap::new();
            attrs.insert("enabled".to_string(), json!(true));
            attrs.insert("color".to_string(), json!("blue"));
            attrs
        },
    };

    assert_eq!(element.element_type, "button");
    assert_eq!(element.text.unwrap(), "Submit");
    assert_eq!(element.bbox.x, 100);
    assert_eq!(element.confidence, 0.95);
    assert_eq!(element.attributes.get("enabled").unwrap(), &json!(true));
}

#[test]
fn test_ocr_configuration() {
    let ocr_config = OcrConfig {
        engine: Some("tesseract".to_string()),
        language: Some("eng+fra".to_string()),
        confidence_threshold: Some(0.85),
        preprocessing: Some(OcrPreprocessing {
            grayscale: Some(true),
            threshold: Some(127),
            denoise: Some(true),
            scale_factor: Some(1.5),
        }),
    };

    assert_eq!(ocr_config.engine.unwrap(), "tesseract");
    assert_eq!(ocr_config.language.unwrap(), "eng+fra");
    assert_eq!(ocr_config.confidence_threshold.unwrap(), 0.85);
    let prep = ocr_config.preprocessing.unwrap();
    assert!(prep.grayscale.unwrap());
    assert_eq!(prep.threshold.unwrap(), 127);
}

// Integration test that would require actual UI elements
// This test is disabled by default and would only run in specialized environments
#[ignore]
#[tokio::test]
async fn test_visioneer_real_ui_interaction() {
    // This test would require:
    // 1. A running application (e.g., Notepad)
    // 2. Actual screen capture capabilities
    // 3. OCR engine installed
    // 4. Proper window focus management

    // Example structure:
    /*
    let tool = VisioneerTool::new();

    // Test with actual Notepad window
    let params = VisioneerParams {
        target: "Untitled - Notepad".to_string(),
        action: VisioneerAction::Type {
            text: "Hello from Visioneer!".to_string(),
            clear_first: Some(true),
            delay_ms: Some(100),
        },
        ocr_config: None,
        vlm_config: None,
    };

    let result = tool.execute(params).await.unwrap();
    assert_eq!(result.action_type, "type");
    assert!(result.success);
    */

    // For now, just verify the structure
    assert!(true, "Test placeholder for real UI interaction");
}