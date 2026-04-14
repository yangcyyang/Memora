//! Backup and export commands for persona data.

use crate::error::AppError;
use crate::infra::db::memora_pool;
use crate::repo::{chat_repo, persona_repo, session_repo};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use zip::write::FileOptions;

/// Session export data structure
#[derive(Serialize, Deserialize)]
struct SessionExport {
    session_id: String,
    messages: Vec<MessageExport>,
    summary: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct MessageExport {
    role: String,
    content: String,
    created_at: String,
}

/// Export persona data to a ZIP file
///
/// # Arguments
/// * `persona_id` - The persona ID to export
/// * `output_path` - Full path where the ZIP file should be written
///
/// # Returns
/// * `Ok(String)` - Success message with output path
#[tauri::command]
pub fn export_persona(persona_id: String, output_path: String) -> Result<String, AppError> {
    let pool = memora_pool();
    let conn = pool.get().map_err(AppError::Pool)?;
    
    // Get persona data
    let persona = persona_repo::find_by_id(&conn, &persona_id)?;
    
    // Get all sessions for this persona
    let sessions = session_repo::list_sessions(&conn, &persona_id)?;
    
    // Create ZIP file
    let file = File::create(&output_path).map_err(AppError::Io)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    
    // Add persona metadata as JSON
    let metadata = serde_json::json!({
        "id": persona.id,
        "slug": persona.slug,
        "name": persona.name,
        "avatar_emoji": persona.avatar_emoji,
        "description": persona.description,
        "tags_json": persona.tags_json,
        "version": persona.version,
        "created_at": persona.created_at,
        "updated_at": persona.updated_at,
    });
    zip.start_file("persona.json", options).map_err(|e| AppError::Zip(e.to_string()))?;
    zip.write_all(metadata.to_string().as_bytes()).map_err(AppError::Io)?;
    
    // Add persona_md
    zip.start_file("persona.md", options).map_err(|e| AppError::Zip(e.to_string()))?;
    zip.write_all(persona.persona_md.as_bytes()).map_err(AppError::Io)?;
    
    // Add memories_md
    zip.start_file("memories.md", options).map_err(|e| AppError::Zip(e.to_string()))?;
    zip.write_all(persona.memories_md.as_bytes()).map_err(AppError::Io)?;
    
    // Export each session
    let mut session_exports = Vec::new();
    for session in &sessions {
        // Get all messages for this session (no limit)
        let messages = chat_repo::get_history(&conn, &persona_id, &session.session_id, 10000)?;
        
        let message_exports: Vec<MessageExport> = messages
            .into_iter()
            .map(|m| MessageExport {
                role: m.role,
                content: m.content,
                created_at: m.created_at,
            })
            .collect();
        
        // Get session summary if exists
        let summary = session_repo::get_summary(&conn, &session.session_id)?
            .map(|(s, _)| s);
        
        session_exports.push(SessionExport {
            session_id: session.session_id.clone(),
            messages: message_exports,
            summary,
        });
    }
    
    // Add sessions.json with all chat history
    zip.start_file("sessions.json", options).map_err(|e| AppError::Zip(e.to_string()))?;
    let sessions_json = serde_json::to_string_pretty(&session_exports).map_err(|e| AppError::Serialize(e.to_string()))?;
    zip.write_all(sessions_json.as_bytes()).map_err(AppError::Io)?;
    
    // Finish ZIP
    zip.finish().map_err(|e| AppError::Zip(e.to_string()))?;
    
    tracing::info!("Exported persona {} to {}", persona_id, output_path);
    Ok(format!("Persona exported to: {}", output_path))
}
