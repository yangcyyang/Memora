// Memora Chrome Extension — Popup Script
// Handles connection state, text selection push, and full-page push.

const DEFAULT_PORT = 17394;

const $ = (id) => document.getElementById(id);
const statusDot = $('statusDot');
const statusText = $('statusText');
const portInput = $('portInput');
const btnPushSelected = $('btnPushSelected');
const btnPushPage = $('btnPushPage');
const resultArea = $('resultArea');

let wsPort = DEFAULT_PORT;
let connected = false;

// ── Connection Check ────────────────────────────────────────────────

async function checkConnection() {
  const port = parseInt(portInput.value) || DEFAULT_PORT;
  wsPort = port;

  return new Promise((resolve) => {
    try {
      const ws = new WebSocket(`ws://127.0.0.1:${port}`);
      const timeout = setTimeout(() => {
        ws.close();
        setStatus(false);
        resolve(false);
      }, 2000);

      ws.onopen = () => {
        clearTimeout(timeout);
        // Send a ping to verify
        ws.send(JSON.stringify({ action: 'ping' }));
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          if (data.ok !== undefined) {
            setStatus(true);
            resolve(true);
          }
        } catch {
          setStatus(false);
          resolve(false);
        }
        ws.close();
      };

      ws.onerror = () => {
        clearTimeout(timeout);
        setStatus(false);
        resolve(false);
      };
    } catch {
      setStatus(false);
      resolve(false);
    }
  });
}

function setStatus(isConnected) {
  connected = isConnected;
  if (isConnected) {
    statusDot.className = 'status-dot connected';
    statusText.textContent = `已连接 (端口 ${wsPort})`;
    btnPushSelected.disabled = false;
  } else {
    statusDot.className = 'status-dot disconnected';
    statusText.textContent = '未连接 — 请启动 Memora';
    btnPushSelected.disabled = true;
  }
}

// ── Push Functions ──────────────────────────────────────────────────

async function sendToMemora(action, data) {
  const port = parseInt(portInput.value) || DEFAULT_PORT;

  return new Promise((resolve, reject) => {
    try {
      const ws = new WebSocket(`ws://127.0.0.1:${port}`);
      const timeout = setTimeout(() => {
        ws.close();
        reject(new Error('连接超时'));
      }, 5000);

      ws.onopen = () => {
        ws.send(JSON.stringify({ action, ...data }));
      };

      ws.onmessage = (event) => {
        clearTimeout(timeout);
        try {
          const resp = JSON.parse(event.data);
          resolve(resp);
        } catch (e) {
          reject(new Error('无效响应'));
        }
        ws.close();
      };

      ws.onerror = (err) => {
        clearTimeout(timeout);
        reject(new Error('WebSocket 连接失败'));
      };
    } catch (e) {
      reject(e);
    }
  });
}

async function getSelectedText() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!tab?.id) return '';

  const results = await chrome.scripting.executeScript({
    target: { tabId: tab.id },
    func: () => window.getSelection()?.toString() || '',
  });

  return results?.[0]?.result || '';
}

async function getPageText() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!tab?.id) return '';

  const results = await chrome.scripting.executeScript({
    target: { tabId: tab.id },
    func: () => document.body.innerText || '',
  });

  return results?.[0]?.result || '';
}

// ── Event Handlers ──────────────────────────────────────────────────

btnPushSelected.addEventListener('click', async () => {
  resultArea.textContent = '获取选中文本…';
  const text = await getSelectedText();

  if (!text.trim()) {
    resultArea.textContent = '⚠️ 未检测到选中文本，请先在页面上选中聊天内容';
    return;
  }

  resultArea.textContent = `正在推送 ${text.length} 字…`;

  try {
    const resp = await sendToMemora('push_text', { text });
    if (resp.ok) {
      resultArea.textContent = `✅ 推送成功！提取了 ${resp.message_count} 条消息`;
    } else {
      resultArea.textContent = `❌ 推送失败: ${resp.error}`;
    }
  } catch (e) {
    resultArea.textContent = `❌ 错误: ${e.message}`;
  }
});

btnPushPage.addEventListener('click', async () => {
  resultArea.textContent = '获取页面文本…';
  const text = await getPageText();

  if (!text.trim()) {
    resultArea.textContent = '⚠️ 页面内容为空';
    return;
  }

  // Truncate very large pages
  const MAX = 50000;
  const truncated = text.length > MAX ? text.substring(0, MAX) : text;

  resultArea.textContent = `正在推送 ${truncated.length} 字…`;

  try {
    const resp = await sendToMemora('push_text', { text: truncated });
    if (resp.ok) {
      resultArea.textContent = `✅ 推送成功！提取了 ${resp.message_count} 条消息`;
    } else {
      resultArea.textContent = `❌ 推送失败: ${resp.error}`;
    }
  } catch (e) {
    resultArea.textContent = `❌ 错误: ${e.message}`;
  }
});

portInput.addEventListener('change', () => {
  checkConnection();
});

// ── Init ────────────────────────────────────────────────────────────

checkConnection();
