// Memora Chrome Extension — Background Service Worker
// Handles context menu and keyboard shortcut events.

const DEFAULT_PORT = 17394;

// ── Context Menu ────────────────────────────────────────────────────

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: 'memora-push-selection',
    title: '推送选中文本到 Memora 💜',
    contexts: ['selection'],
  });
});

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  if (info.menuItemId === 'memora-push-selection' && info.selectionText) {
    try {
      await pushTextToMemora(info.selectionText);
      // Notify via content script
      if (tab?.id) {
        chrome.tabs.sendMessage(tab.id, {
          type: 'MEMORA_PUSH_RESULT',
          success: true,
          count: info.selectionText.split('\n').filter(l => l.trim()).length,
        });
      }
    } catch (e) {
      if (tab?.id) {
        chrome.tabs.sendMessage(tab.id, {
          type: 'MEMORA_PUSH_RESULT',
          success: false,
          error: e.message,
        });
      }
    }
  }
});

// ── WebSocket Sender ────────────────────────────────────────────────

function pushTextToMemora(text) {
  return new Promise((resolve, reject) => {
    try {
      const ws = new WebSocket(`ws://127.0.0.1:${DEFAULT_PORT}`);
      const timeout = setTimeout(() => {
        ws.close();
        reject(new Error('连接超时'));
      }, 5000);

      ws.onopen = () => {
        ws.send(JSON.stringify({
          action: 'push_text',
          text: text,
        }));
      };

      ws.onmessage = (event) => {
        clearTimeout(timeout);
        try {
          const resp = JSON.parse(event.data);
          if (resp.ok) {
            resolve(resp);
          } else {
            reject(new Error(resp.error || '推送失败'));
          }
        } catch {
          reject(new Error('无效响应'));
        }
        ws.close();
      };

      ws.onerror = () => {
        clearTimeout(timeout);
        reject(new Error('无法连接到 Memora'));
      };
    } catch (e) {
      reject(e);
    }
  });
}
