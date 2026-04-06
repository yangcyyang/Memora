use anyhow::{Context, Result};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

/// Swift script that uses MacOS native Vision.framework for offline OCR.
const SWIFT_OCR_SCRIPT: &str = r#"
import Cocoa
import Vision

guard CommandLine.arguments.count > 1 else {
    exit(1)
}

let imagePath = CommandLine.arguments[1]
guard let image = NSImage(contentsOfFile: imagePath),
      let cgImage = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
    exit(1)
}

let requestHandler = VNImageRequestHandler(cgImage: cgImage, options: [:])
let request = VNRecognizeTextRequest { (request, error) in
    guard let observations = request.results as? [VNRecognizedTextObservation] else { return }
    var resultText = ""
    for observation in observations {
        guard let topCandidate = observation.topCandidates(1).first else { continue }
        resultText += topCandidate.string + "\n"
    }
    print(resultText)
}

// Favor Chinese and English recognition
request.recognitionLanguages = ["zh-Hans", "zh-Hant", "en-US"]
request.usesLanguageCorrection = true

do {
    try requestHandler.perform([request])
} catch {
    exit(1)
}
"#;

#[tauri::command]
pub async fn capture_and_ocr() -> Result<String, String> {
    tokio::task::spawn_blocking(|| {
        do_capture_and_ocr().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn do_capture_and_ocr() -> Result<String> {
    // 1. Ask MacOS to do an interactive bounding-box screenshot
    let temp_dir = std::env::temp_dir();
    let img_path = temp_dir.join("memora_ocr_capture.png");

    let status = Command::new("screencapture")
        .args(["-i", "-x", img_path.to_string_lossy().as_ref()])
        .status()
        .context("Failed to invoke screencapture")?;

    if !status.success() {
        return Ok("".to_string()); // User likely cancelled the screenshot via Escape
    }

    if !img_path.exists() {
        return Ok("".to_string());
    }

    // 2. Write the Swift script to disk temporarily
    let script_path = temp_dir.join("memora_vision_ocr.swift");
    std::fs::write(&script_path, SWIFT_OCR_SCRIPT)
        .context("Failed to write transient swift script")?;

    // 3. Execute the Swift script to perform Vision OCR
    let output = Command::new("/usr/bin/swift")
        .arg(&script_path)
        .arg(&img_path)
        .output()
        .context("Failed to execute swift OCR script")?;

    // Clean up
    let _ = std::fs::remove_file(&img_path);
    let _ = std::fs::remove_file(&script_path);

    if !output.status.success() {
        anyhow::bail!("OCR Script failed with exit code: {:?}", output.status.code());
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(text)
}
