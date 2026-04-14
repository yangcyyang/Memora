use crate::error::AppError;
use tauri::menu::MenuBuilder;
use tauri::{AppHandle, Manager, Runtime};
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
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("发送通知失败: {}", e)))
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
