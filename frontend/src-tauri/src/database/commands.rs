use log::{error, info};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

use super::manager::DatabaseManager;
use crate::state::AppState;

#[derive(Serialize)]
pub struct DatabaseCheckResult {
    pub exists: bool,
    pub size: u64,
}

/// Check if this is the first launch (no database exists yet)
#[tauri::command]
pub async fn check_first_launch(app: AppHandle) -> Result<bool, String> {
    DatabaseManager::is_first_launch(&app)
        .await
        .map_err(|e| format!("Failed to check first launch: {}", e))
}

/// Open a dialog to select a folder or file for legacy database import
#[tauri::command]
pub async fn select_legacy_database_path(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    info!("Opening dialog to select legacy database location");

    let file_path = app
        .dialog()
        .file()
        .add_filter("Database Files", &["db"])
        .blocking_pick_file();

    if let Some(path) = file_path {
        let path_str = path.to_string();
        info!("User selected path: {}", path_str);
        Ok(Some(path_str))
    } else {
        info!("User cancelled file selection");
        Ok(None)
    }
}

/// Detect legacy database from a selected path (root repo, backend folder, or db file)
#[tauri::command]
pub async fn detect_legacy_database(selected_path: String) -> Result<Option<String>, String> {
    let path = PathBuf::from(&selected_path);

    info!("Detecting legacy database from path: {}", selected_path);

    // Case 1: User selected the .db file directly
    if path.is_file() {
        if let Some(extension) = path.extension() {
            if extension == "db" {
                info!("Direct .db file selected: {}", selected_path);
                return Ok(Some(selected_path));
            }
        }
    }

    // Case 2: User selected directory containing meeting_minutes.db
    if path.is_dir() {
        let direct_db = path.join("meeting_minutes.db");
        if direct_db.exists() && direct_db.is_file() {
            let db_path = direct_db.to_string_lossy().to_string();
            info!("Found database in selected directory: {}", db_path);
            return Ok(Some(db_path));
        }

        // Case 3: User selected root repo (check backend subdirectory)
        let backend_db = path.join("backend").join("meeting_minutes.db");
        if backend_db.exists() && backend_db.is_file() {
            let db_path = backend_db.to_string_lossy().to_string();
            info!("Found database in backend subdirectory: {}", db_path);
            return Ok(Some(db_path));
        }
    }

    info!("No legacy database found at path: {}", selected_path);
    Ok(None)
}

/// Check for legacy database in the default app data directory
#[tauri::command]
pub async fn check_default_legacy_database(app: AppHandle) -> Result<Option<String>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let legacy_db = app_data_dir.join("meeting_minutes.db");
    info!("Checking for default legacy database at: {:?}", legacy_db);

    if legacy_db.exists() && legacy_db.is_file() {
        let path_str = legacy_db.to_string_lossy().to_string();
        info!("Found default legacy database: {}", path_str);
        Ok(Some(path_str))
    } else {
        info!("No default legacy database found");
        Ok(None)
    }
}

/// Check if the Homebrew database exists and return its size
/// This is specifically for detecting old Python backend installations
#[tauri::command]
pub async fn check_homebrew_database(path: String) -> Result<Option<DatabaseCheckResult>, String> {
    let db_path = PathBuf::from(&path);
    
    info!("Checking for Homebrew database at: {}", path);
    
    // Check if file exists and is a regular file
    if db_path.exists() && db_path.is_file() {
        // Get file metadata to check size
        match std::fs::metadata(&db_path) {
            Ok(metadata) => {
                let size = metadata.len();
                info!("Found Homebrew database: {} ({} bytes)", path, size);
                
                // Only consider it valid if it has content (not empty)
                if size > 0 {
                    Ok(Some(DatabaseCheckResult {
                        exists: true,
                        size,
                    }))
                } else {
                    info!("Database file exists but is empty");
                    Ok(None)
                }
            }
            Err(e) => {
                error!("Failed to read database metadata: {}", e);
                Ok(None)
            }
        }
    } else {
        info!("No database found at Homebrew location");
        Ok(None)
    }
}

/// Import legacy database and initialize the database manager
#[tauri::command]
pub async fn import_and_initialize_database(
    app: AppHandle,
    legacy_db_path: String,
) -> Result<(), String> {
    info!(
        "Starting import of legacy database from: {}",
        legacy_db_path
    );

    // Import and get initialized manager
    let db_manager = DatabaseManager::import_legacy_database(&app, &legacy_db_path)
        .await
        .map_err(|e| {
            error!("Failed to import legacy database: {}", e);
            format!("Failed to import database: {}", e)
        })?;

    // Update app state with the new manager
    app.manage(AppState { db_manager });

    info!("Legacy database imported and initialized successfully");

    // Emit event to notify frontend that database is ready
    app.emit("database-initialized", ())
        .map_err(|e| format!("Failed to emit database-initialized event: {}", e))?;

    Ok(())
}

/// Initialize a fresh database (for users who don't want to import)
#[tauri::command]
pub async fn initialize_fresh_database(app: AppHandle) -> Result<(), String> {
    info!("Initializing fresh database");

    let db_manager = DatabaseManager::new_from_app_handle(&app)
        .await
        .map_err(|e| {
            error!("Failed to initialize fresh database: {}", e);
            format!("Failed to initialize database: {}", e)
        })?;

    // Update app state with the new manager
    app.manage(AppState { db_manager: db_manager.clone() });

    // Set default model configuration for fresh installs
    let pool = db_manager.pool();
    
    let default_summary_model = crate::summary::summary_engine::commands::get_recommended_summary_model_for_current_system()
        .unwrap_or("qwen3.5:2b");

    // Default Summary Model: Built-in AI (Qwen recommendation for this system)
    if let Err(e) = crate::database::repositories::setting::SettingsRepository::save_model_config(
        pool,
        "builtin-ai",
        default_summary_model,
        "large-v3", // Default whisper model (unused for builtin but required)
        None,
    ).await {
        error!("Failed to set default summary model config: {}", e);
    }

    // Default Transcription Model: Parakeet
    if let Err(e) = crate::database::repositories::setting::SettingsRepository::save_transcript_config(
        pool,
        "parakeet",
        crate::config::DEFAULT_PARAKEET_MODEL,
    ).await {
        error!("Failed to set default transcription model config: {}", e);
    }

    info!("Fresh database initialized successfully with default models");

    // Emit event to notify frontend that database is ready
    app.emit("database-initialized", ())
        .map_err(|e| format!("Failed to emit database-initialized event: {}", e))?;

    Ok(())
}

/// Get the database directory path
#[tauri::command]
pub async fn get_database_directory(app: AppHandle) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    Ok(app_data_dir.to_string_lossy().to_string())
}

/// Open the database folder in the system file explorer
#[tauri::command]
pub async fn open_database_folder(app: AppHandle) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    // Ensure directory exists before trying to open it
    if !app_data_dir.exists() {
        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let folder_path = app_data_dir.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    info!("Opened database folder: {}", folder_path);
    Ok(())
}

// ============================================================================
// Meeting Custom Notes Commands
// ============================================================================

/// Save a custom note for a meeting during live recording
#[tauri::command]
pub async fn save_meeting_note(
    app: AppHandle,
    meeting_id: String,
    timestamp: i64,
    text: String,
) -> Result<serde_json::Value, String> {
    use crate::state::AppState;
    use crate::database::repositories::meeting_custom_notes::MeetingCustomNotesRepository;

    let state = app
        .state::<AppState>()
        .inner();

    let note = MeetingCustomNotesRepository::save_note(
        state.db_manager.pool(),
        &meeting_id,
        timestamp,
        &text,
    )
    .await
    .map_err(|e| format!("Failed to save meeting note: {}", e))?;

    Ok(serde_json::json!(note))
}

/// Get all custom notes for a meeting
#[tauri::command]
pub async fn get_meeting_notes(
    app: AppHandle,
    meeting_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    use crate::state::AppState;
    use crate::database::repositories::meeting_custom_notes::MeetingCustomNotesRepository;

    let state = app
        .state::<AppState>()
        .inner();

    let notes = MeetingCustomNotesRepository::get_notes_for_meeting(
        state.db_manager.pool(),
        &meeting_id,
    )
    .await
    .map_err(|e| format!("Failed to get meeting notes: {}", e))?;

    Ok(notes
        .into_iter()
        .map(|n| serde_json::json!(n))
        .collect())
}

/// Delete a specific meeting note by ID
#[tauri::command]
pub async fn delete_meeting_note(
    app: AppHandle,
    note_id: i64,
) -> Result<(), String> {
    use crate::state::AppState;
    use crate::database::repositories::meeting_custom_notes::MeetingCustomNotesRepository;

    let state = app
        .state::<AppState>()
        .inner();

    MeetingCustomNotesRepository::delete_note(
        state.db_manager.pool(),
        note_id,
    )
    .await
    .map_err(|e| format!("Failed to delete meeting note: {}", e))?;

    Ok(())
}

// ============================================================================
// Meeting Markdown Notes Commands (single note per meeting, editable)
// ============================================================================

/// Save a markdown note for a meeting (upsert — one note per meeting)
#[tauri::command]
pub async fn save_meeting_markdown_note(
    app: AppHandle,
    meeting_id: String,
    markdown: String,
) -> Result<(), String> {
    use crate::state::AppState;
    use crate::database::repositories::meeting_markdown_notes::MeetingMarkdownNotesRepository;

    let state = app.state::<AppState>().inner();

    MeetingMarkdownNotesRepository::save_note(
        state.db_manager.pool(),
        &meeting_id,
        &markdown,
    )
    .await
    .map_err(|e| format!("Failed to save markdown note: {}", e))
}

/// Get the markdown note for a meeting
#[tauri::command]
pub async fn get_meeting_markdown_note(
    app: AppHandle,
    meeting_id: String,
) -> Result<Option<String>, String> {
    use crate::state::AppState;
    use crate::database::repositories::meeting_markdown_notes::MeetingMarkdownNotesRepository;

    let state = app.state::<AppState>().inner();

    MeetingMarkdownNotesRepository::get_note(
        state.db_manager.pool(),
        &meeting_id,
    )
    .await
    .map_err(|e| format!("Failed to get markdown note: {}", e))
}

// ============================================================================
// Meeting Chat Commands (persistent chat history)
// ============================================================================

/// Save a chat message for a meeting
#[tauri::command]
pub async fn save_chat_message(
    app: AppHandle,
    meeting_id: String,
    role: String,
    content: String,
    session_id: String,
) -> Result<serde_json::Value, String> {
    use crate::state::AppState;
    use crate::database::repositories::meeting_chat::MeetingChatRepository;

    let state = app.state::<AppState>().inner();
    let msg = MeetingChatRepository::save_message(
        state.db_manager.pool(),
        &meeting_id,
        &role,
        &content,
        &session_id,
    )
    .await
    .map_err(|e| format!("Failed to save chat message: {}", e))?;

    Ok(serde_json::json!(msg))
}

/// Get chat messages for a meeting session
#[tauri::command]
pub async fn get_chat_session_messages(
    app: AppHandle,
    meeting_id: String,
    session_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    use crate::state::AppState;
    use crate::database::repositories::meeting_chat::MeetingChatRepository;

    let state = app.state::<AppState>().inner();
    let msgs = MeetingChatRepository::get_session_messages(
        state.db_manager.pool(),
        &meeting_id,
        &session_id,
    )
    .await
    .map_err(|e| format!("Failed to get chat messages: {}", e))?;

    Ok(msgs.into_iter().map(|m| serde_json::json!(m)).collect())
}

/// Get all chat sessions for a meeting
#[tauri::command]
pub async fn get_chat_sessions(
    app: AppHandle,
    meeting_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    use crate::state::AppState;
    use crate::database::repositories::meeting_chat::MeetingChatRepository;

    let state = app.state::<AppState>().inner();
    let sessions = MeetingChatRepository::get_sessions(
        state.db_manager.pool(),
        &meeting_id,
    )
    .await
    .map_err(|e| format!("Failed to get chat sessions: {}", e))?;

    Ok(sessions
        .into_iter()
        .map(|(id, count)| serde_json::json!({ "session_id": id, "message_count": count }))
        .collect())
}

/// Delete a chat session
#[tauri::command]
pub async fn delete_chat_session(
    app: AppHandle,
    meeting_id: String,
    session_id: String,
) -> Result<(), String> {
    use crate::state::AppState;
    use crate::database::repositories::meeting_chat::MeetingChatRepository;

    let state = app.state::<AppState>().inner();
    MeetingChatRepository::delete_session(
        state.db_manager.pool(),
        &meeting_id,
        &session_id,
    )
    .await
    .map_err(|e| format!("Failed to delete chat session: {}", e))
}

