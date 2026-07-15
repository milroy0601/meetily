// Chat client for llama-helper in interactive chat mode.
// Spawns a one-shot process with `--mode chat` for each inference request.

use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Request sent to llama-helper in chat mode.
/// The `type` field must be `"chat"` to match the variant in llama-helper's Request enum.
#[derive(Debug, Serialize)]
struct ChatRequest {
    #[serde(rename = "type")]
    request_type: &'static str,
    context: String,
    question: String,
    max_tokens: Option<i32>,
    context_size: Option<u32>,
    temperature: Option<f32>,
}

impl ChatRequest {
    fn new(context: String, question: String) -> Self {
        Self {
            request_type: "chat",
            context,
            question,
            max_tokens: Some(512),
            context_size: Some(4096),
            temperature: Some(0.7),
        }
    }
}

/// Response received from llama-helper in chat mode
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ChatResponse {
    #[serde(rename = "chat_response")]
    ChatResponse {
        text: String,
        error: Option<String>,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
    },
    #[serde(rename = "goodbye")]
    Goodbye,
}

/// Cached path to the llama-helper binary, resolved once at startup.
fn get_helper_binary_path() -> Result<PathBuf> {
    use once_cell::sync::OnceCell;
    static HELPER_PATH: OnceCell<PathBuf> = OnceCell::new();

    HELPER_PATH
        .get_or_try_init(|| {
            // Check env var first
            if let Ok(env_path) = std::env::var("MEETILY_LLAMA_HELPER") {
                if !env_path.is_empty() {
                    let path = PathBuf::from(env_path);
                    if path.exists() {
                        log::info!("Using llama-helper from env: {}", path.display());
                        return Ok(path);
                    }
                }
            }

            // Check relative to current executable
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    // Try exact match first
                    let exact_name = if cfg!(windows) { "llama-helper.exe" } else { "llama-helper" };
                    let exact_path = exe_dir.join(exact_name);
                    if exact_path.exists() {
                        log::info!("Found exact llama-helper next to exe: {}", exact_path.display());
                        return Ok(exact_path);
                    }

                    // Fall back to fuzzy match in exe dir
                    if let Ok(entries) = std::fs::read_dir(exe_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                if name.starts_with("llama-helper") && !name.ends_with(".d") {
                                    log::info!("Found llama-helper next to exe via fuzzy match: {}", path.display());
                                    return Ok(path);
                                }
                            }
                        }
                    }
                }
            }

            // Fallback: workspace target/ directories
            // CARGO_MANIFEST_DIR for the Tauri crate is frontend/src-tauri/.
            // The workspace root is three levels up. Check both frontend/target/
            // (for standalone cargo builds) and the workspace root target/.
            if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
                let manifest = PathBuf::from(&manifest_dir);
                let frontend_root = manifest
                    .parent()                     // src-tauri/
                    .and_then(|p| p.parent())     // frontend/
                    .map(|p| p.to_path_buf());
                let workspace_root = manifest
                    .parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.parent())     // MeetilyPro/
                    .map(|p| p.to_path_buf());

                let mut candidates = Vec::new();

                // Check frontend/target/ (standalone Tauri build)
                if let Some(ref root) = frontend_root {
                    candidates.push(root.join("target/release/llama-helper"));
                    candidates.push(root.join("target/debug/llama-helper"));
                }

                // Check workspace root target/ (workspace-level cargo build)
                if let Some(ref root) = workspace_root {
                    candidates.push(root.join("target/release/llama-helper"));
                    candidates.push(root.join("target/debug/llama-helper"));
                }

                for candidate in &candidates {
                    if candidate.exists() {
                        log::info!("Found llama-helper in target: {}", candidate.display());
                        return Ok(candidate.clone());
                    }
                }
            }

            Err(anyhow!("Could not find llama-helper binary"))
        })
        .cloned()
}

/// Strip `<think>...</think>` blocks from model output (reasoning models like Qwen 3.5 emit these).
fn strip_think_tags(text: &str) -> String {
    // Remove <think>...</think> blocks including their content
    let re = regex::Regex::new(r"(?s)<think>.*?</think>").unwrap();
    re.replace_all(text, "").trim().to_string()
}

/// Public API: Ask a question with context using llama-helper in chat mode.
/// Spawns a one-shot process per request. The model stays loaded in memory
/// only for the duration of the single request.
pub async fn ask_llama(
    app_data_dir: &PathBuf,
    model_name: &str,
    context: &str,
    question: &str,
) -> Result<String> {
    use super::models;

    // Resolve model path
    let model_path = models::get_model_path(app_data_dir, model_name)?;

    if !model_path.exists() {
        return Err(anyhow!(
            "Model file not found: {}. Please download the model '{}' first.",
            model_path.display(),
            model_name
        ));
    }

    let helper_path = get_helper_binary_path()?;
    let model_str = model_path.to_string_lossy().to_string();

    log::info!(
        "Spawning llama-helper chat: model={}, question_len={}, context_len={}",
        model_name,
        question.len(),
        context.len()
    );

    let mut child = tokio::process::Command::new(&helper_path)
        .arg("--mode")
        .arg("chat")
        .arg("--model-path")
        .arg(&model_str)
        .arg("--context-size")
        .arg("4096")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .context("Failed to spawn llama-helper for chat request")?;

    let mut stdin = child
        .stdin
        .take()
        .context("Failed to capture stdin")?;
    let stdout = child
        .stdout
        .take()
        .context("Failed to capture stdout")?;

    let request = ChatRequest::new(context.to_string(), question.to_string());

    let request_json = serde_json::to_string(&request)?;

    // Send request
    stdin.write_all(request_json.as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;
    drop(stdin); // Close stdin to signal EOF

    // Read response
    let mut reader = BufReader::new(stdout);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    let response: ChatResponse = serde_json::from_str(response_line.trim())
        .context("Failed to parse chat response")?;

    // Wait for process to finish
    let _ = child.wait().await;

    match response {
        ChatResponse::ChatResponse { text, error } => {
            if let Some(err) = error {
                if !err.is_empty() {
                    return Err(anyhow!("Chat error: {}", err));
                }
            }
            Ok(strip_think_tags(&text))
        }
        ChatResponse::Error { message } => Err(anyhow!("Chat error: {}", message)),
        ChatResponse::Goodbye => Err(anyhow!("Chat process exited unexpectedly")),
    }
}
