use crate::error::AppError;
use serde::Serialize;
use tauri::menu::MenuBuilder;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;

const TRAY_ID: &str = "main";
const MENU_SHOW_ID: &str = "tray-show";
const MENU_QUIT_ID: &str = "tray-quit";

#[tauri::command]
#[tracing::instrument(skip(app), err)]
pub async fn send_notification(
    app: AppHandle,
    title: String,
    body: String,
) -> Result<(), AppError> {
    show_notification(&app, &title, &body)
}

#[tauri::command]
#[tracing::instrument(skip(app), err)]
pub async fn trigger_proactive_test(
    app: AppHandle,
    persona_id: String,
) -> Result<(), AppError> {
    let (name, _, _, _) = {
        let pool = crate::infra::db::memora_pool();
        let conn = pool.get()?;
        crate::repo::persona_repo::get_persona_data(&conn, &persona_id)?
    };

    send_proactive_notification(
        &app,
        &persona_id,
        format!("{name} 想和你说句话"),
        "这是主动触达测试通知".to_string(),
    )?;

    #[cfg(target_os = "macos")]
    {
        tracing::warn!("macOS 桌面通知无原生点击回调，trigger_proactive_test 直接发 proactive-trigger 事件用于联调");
        emit_proactive_trigger(&app, &persona_id)?;
    }

    Ok(())
}

pub fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), AppError> {
    let tray = app
        .tray_by_id(TRAY_ID)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("未找到系统托盘: {}", TRAY_ID)))?;

    let menu = MenuBuilder::new(app)
        .text(MENU_SHOW_ID, "显示 Memora")
        .separator()
        .text(MENU_QUIT_ID, "退出")
        .build()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("创建托盘菜单失败: {}", e)))?;

    tray.set_menu(Some(menu))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("设置托盘菜单失败: {}", e)))?;
    tray.set_tooltip(Some("Memora"))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("设置托盘提示失败: {}", e)))?;
    let _ = tray.set_show_menu_on_left_click(false);
    Ok(())
}

pub fn handle_tray_menu<R: Runtime>(app: &AppHandle<R>, menu_id: &str) {
    match menu_id {
        MENU_SHOW_ID => {
            if let Err(err) = reveal_main_window(app) {
                tracing::error!("显示主窗口失败: {}", err);
            }
        }
        MENU_QUIT_ID => app.exit(0),
        _ => {}
    }
}

pub fn handle_tray_click<R: Runtime>(app: &AppHandle<R>) {
    if let Err(err) = reveal_main_window(app) {
        tracing::error!("托盘点击恢复窗口失败: {}", err);
    }
}

fn reveal_main_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), AppError> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("未找到主窗口 main")))?;

    window
        .show()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("显示窗口失败: {}", e)))?;
    let _ = window.unminimize();
    let _ = window.set_focus();
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct ProactiveTriggerPayload {
    pub persona_id: String,
}

pub fn emit_proactive_trigger<R: Runtime>(
    app: &AppHandle<R>,
    persona_id: &str,
) -> Result<(), AppError> {
    app.emit(
        "proactive-trigger",
        ProactiveTriggerPayload {
            persona_id: persona_id.to_string(),
        },
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("发送 proactive-trigger 事件失败: {}", e)))
}

pub fn send_proactive_notification<R: Runtime>(
    app: &AppHandle<R>,
    persona_id: &str,
    title: String,
    body: String,
) -> Result<(), AppError> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .extra("persona_id", persona_id)
        .show()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("发送主动通知失败: {}", e)))
}

fn show_notification<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: &str,
) -> Result<(), AppError> {
    app.notification()
        .builder()
        .title(title.to_string())
        .body(body.to_string())
        .show()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("发送通知失败: {}", e)))
}
