// Memora Chrome Extension — Content Script
// Shows a toast notification after push results.

chrome.runtime.onMessage.addListener((message) => {
  if (message.type === 'MEMORA_PUSH_RESULT') {
    showToast(message.success, message.count, message.error);
  }
});

function showToast(success, count, error) {
  // Remove existing toast if any
  const existing = document.getElementById('memora-toast');
  if (existing) existing.remove();

  const toast = document.createElement('div');
  toast.id = 'memora-toast';

  if (success) {
    toast.textContent = `💜 Memora: 已推送 ${count} 条语料`;
    toast.className = 'memora-toast memora-toast-success';
  } else {
    toast.textContent = `❌ Memora: ${error || '推送失败'}`;
    toast.className = 'memora-toast memora-toast-error';
  }

  document.body.appendChild(toast);

  // Animate in
  requestAnimationFrame(() => {
    toast.style.opacity = '1';
    toast.style.transform = 'translateY(0)';
  });

  // Auto-dismiss
  setTimeout(() => {
    toast.style.opacity = '0';
    toast.style.transform = 'translateY(-12px)';
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}
