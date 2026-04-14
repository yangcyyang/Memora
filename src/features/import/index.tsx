import { useState, useCallback } from "react";
import { useNavigate } from "@tanstack/react-router";
import { detectAndParse } from "@/lib/tauri";
import { toast } from "sonner";
import { Upload, FileText, X, Check, ChevronRight, Loader2 } from "lucide-react";
import type { ParsedContent } from "@/types";

interface FileWithPreview {
  file: File;
  path: string;
  status: "pending" | "parsing" | "success" | "error";
  result?: ParsedContent;
  error?: string;
}

export function DataImport() {
  const navigate = useNavigate();
  const [files, setFiles] = useState<FileWithPreview[]>([]);
  const [isDragging, setIsDragging] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    const droppedFiles = Array.from(e.dataTransfer.files).filter(
      f => f.name.endsWith('.txt') || f.name.endsWith('.html') || f.name.endsWith('.csv')
    );
    addFiles(droppedFiles);
  }, []);

  const handleFileSelect = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const selectedFiles = Array.from(e.target.files || []);
    addFiles(selectedFiles);
  }, []);

  const addFiles = (newFiles: File[]) => {
    const filePreviews: FileWithPreview[] = newFiles.map(file => ({
      file,
      path: file.name,
      status: "pending",
    }));
    setFiles(prev => [...prev, ...filePreviews]);
  };

  const removeFile = (index: number) => {
    setFiles(prev => prev.filter((_, i) => i !== index));
  };

  const parseFiles = async () => {
    if (files.length === 0) return;
    
    setIsProcessing(true);
    const pendingFiles = files.filter(f => f.status === "pending");
    
    // Update status to parsing
    setFiles(prev => prev.map(f => 
      f.status === "pending" ? { ...f, status: "parsing" } : f
    ));

    try {
      // In real Tauri app, we'd use the file path, here we simulate with text content
      // For actual implementation, you'd use @tauri-apps/plugin-dialog to select files
      const fileContents = await Promise.all(
        pendingFiles.map(async (f) => {
          const text = await f.file.text();
          return { name: f.file.name, text };
        })
      );

      // Call detect_and_parse with the content
      // Note: In actual implementation, this would be file paths
      const paths = pendingFiles.map(f => f.path);
      const results = await detectAndParse(paths);

      // Update file statuses based on results
      setFiles(prev => prev.map((f, idx) => {
        const result = results[idx];
        if (result && result.parsed) {
          return { ...f, status: "success", result: result.parsed };
        }
        return { ...f, status: "error", error: "解析失败" };
      }));

      const successCount = results.filter(r => r.parsed).length;
      if (successCount > 0) {
        toast.success(`成功解析 ${successCount} 个文件`);
      }
    } catch (error) {
      toast.error(`解析失败: ${error}`);
      setFiles(prev => prev.map(f => 
        f.status === "parsing" ? { ...f, status: "error", error: "解析失败" } : f
      ));
    } finally {
      setIsProcessing(false);
    }
  };

  const handleContinue = () => {
    const parsedFiles = files.filter(f => f.status === "success" && f.result);
    if (parsedFiles.length === 0) {
      toast.error("请至少成功解析一个文件");
      return;
    }
    
    // Store parsed content in session storage for CreateWizard to use
    sessionStorage.setItem('imported_data', JSON.stringify(
      parsedFiles.map(f => f.result)
    ));
    
    navigate({ to: "/create" });
  };

  const handleSkip = () => {
    navigate({ to: "/create" });
  };

  const totalParsed = files.filter(f => f.status === "success").length;

  return (
    <div style={styles.container}>
      <div style={styles.content}>
        <h1 className="text-hero" style={{ marginBottom: 8 }}>导入聊天记录</h1>
        <p className="text-body" style={styles.subtitle}>
          支持微信导出的 txt/html/csv 和 QQ 导出的 txt 格式
        </p>

        {/* Drop Zone */}
        <div
          style={{
            ...styles.dropZone,
            borderColor: isDragging ? "var(--color-rose-500)" : "var(--color-cream-300)",
            background: isDragging ? "var(--color-rose-500)/10" : "var(--color-cream-100)",
          }}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
        >
          <input
            type="file"
            multiple
            accept=".txt,.html,.csv"
            onChange={handleFileSelect}
            style={{ display: "none" }}
            id="file-input"
          />
          <label htmlFor="file-input" style={styles.dropZoneContent}>
            <Upload size={48} style={{ color: "var(--color-rose-500)", marginBottom: 16 }} />
            <p className="text-body" style={{ marginBottom: 8 }}>
              拖拽文件到这里，或点击选择
            </p>
            <p className="text-caption" style={{ color: "var(--color-earth-500)" }}>
              支持 .txt, .html, .csv 格式
            </p>
          </label>
        </div>

        {/* File List */}
        {files.length > 0 && (
          <div style={styles.fileList}>
            {files.map((f, index) => (
              <div key={index} style={styles.fileItem}>
                <div style={styles.fileInfo}>
                  <FileText size={18} style={{ color: "var(--color-earth-500)" }} />
                  <span className="text-small" style={{ flex: 1 }}>{f.file.name}</span>
                  {f.status === "success" && (
                    <span style={{ ...styles.badge, background: "var(--color-sage-400)", color: "white" }}>
                      <Check size={12} />
                      已解析
                    </span>
                  )}
                  {f.status === "parsing" && (
                    <span style={{ ...styles.badge, background: "var(--color-cream-300)" }}>
                      <Loader2 size={12} className="animate-spin" />
                      解析中
                    </span>
                  )}
                  {f.status === "error" && (
                    <span style={{ ...styles.badge, background: "var(--color-coral-400)", color: "white" }}>
                      失败
                    </span>
                  )}
                </div>
                <button
                  type="button"
                  onClick={() => removeFile(index)}
                  style={styles.removeBtn}
                  disabled={isProcessing}
                >
                  <X size={16} />
                </button>
              </div>
            ))}
          </div>
        )}

        {/* Actions */}
        <div style={styles.actions}>
          {files.length > 0 && (
            <button
              type="button"
              onClick={parseFiles}
              disabled={isProcessing || !files.some(f => f.status === "pending")}
              style={{
                ...styles.secondaryBtn,
                opacity: isProcessing || !files.some(f => f.status === "pending") ? 0.5 : 1,
              }}
            >
              {isProcessing ? (
                <><Loader2 size={16} className="animate-spin" /> 解析中...</>
              ) : (
                <><Check size={16} /> 开始解析</>
              )}
            </button>
          )}
          
          {totalParsed > 0 && (
            <button type="button" onClick={handleContinue} style={styles.primaryBtn}>
              继续创建角色
              <ChevronRight size={18} />
            </button>
          )}
          
          <button type="button" onClick={handleSkip} style={styles.ghostBtn}>
            跳过导入，直接创建角色 →
          </button>
        </div>
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    width: "100%",
    minHeight: "100vh",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    background: "var(--color-cream-50)",
    padding: "40px 24px",
  },
  content: {
    maxWidth: 560,
    width: "100%",
  },
  subtitle: {
    color: "var(--color-earth-500)",
    marginBottom: 32,
  },
  dropZone: {
    border: "2px dashed var(--color-cream-300)",
    borderRadius: "var(--radius-lg)",
    padding: "40px 24px",
    background: "var(--color-cream-100)",
    transition: "all var(--duration-fast)",
    cursor: "pointer",
  },
  dropZoneContent: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    textAlign: "center",
    cursor: "pointer",
  },
  fileList: {
    marginTop: 24,
    display: "flex",
    flexDirection: "column",
    gap: 8,
  },
  fileItem: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: "12px 16px",
    background: "white",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--color-cream-300)",
  },
  fileInfo: {
    display: "flex",
    alignItems: "center",
    gap: 12,
    flex: 1,
  },
  badge: {
    display: "flex",
    alignItems: "center",
    gap: 4,
    padding: "4px 10px",
    borderRadius: "var(--radius-full)",
    fontSize: "0.75rem",
    fontWeight: 500,
  },
  removeBtn: {
    background: "none",
    border: "none",
    cursor: "pointer",
    padding: 4,
    color: "var(--color-earth-400)",
    display: "flex",
    alignItems: "center",
  },
  actions: {
    marginTop: 32,
    display: "flex",
    flexDirection: "column",
    gap: 12,
    alignItems: "center",
  },
  primaryBtn: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    padding: "14px 32px",
    background: "var(--color-rose-500)",
    color: "white",
    border: "none",
    borderRadius: "var(--radius-lg)",
    fontSize: "1rem",
    fontWeight: 500,
    cursor: "pointer",
    fontFamily: "var(--font-body)",
    boxShadow: "var(--shadow-md)",
  },
  secondaryBtn: {
    display: "flex",
    alignItems: "center",
    gap: 8,
    padding: "10px 24px",
    background: "var(--color-cream-100)",
    color: "var(--color-earth-700)",
    border: "1.5px solid var(--color-cream-300)",
    borderRadius: "var(--radius-md)",
    fontSize: "0.9rem",
    fontWeight: 500,
    cursor: "pointer",
    fontFamily: "var(--font-body)",
  },
  ghostBtn: {
    padding: "10px 20px",
    background: "none",
    border: "none",
    color: "var(--color-earth-500)",
    fontSize: "0.9rem",
    cursor: "pointer",
    fontFamily: "var(--font-body)",
  },
};
