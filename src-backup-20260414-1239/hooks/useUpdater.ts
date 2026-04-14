import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

export type UpdateStatus = "idle" | "checking" | "available" | "downloading" | "ready" | "error";

export interface UpdateState {
  status: UpdateStatus;
  version: string;
  progress: number;
  error: string;
  retriesLeft: number;
}

interface UpdateCheckResult {
  available: boolean;
  version: string | null;
  date: string | null;
  body: string | null;
}

interface DownloadProgressPayload {
  chunk_length: number;
  content_length: number | null;
}

const SKIPPED_KEY = "memora_skipped_version";
const LAST_CHECK_KEY = "memora_last_check";
const CHECK_INTERVAL_MS = 60 * 60 * 1000; // 1h
const CHECK_TIMEOUT_MS = 20_000;
const MAX_DOWNLOAD_RETRIES = 2;

function getSkipped(): string {
  return localStorage.getItem(SKIPPED_KEY) ?? "";
}

function getLastCheck(): number {
  return Number(localStorage.getItem(LAST_CHECK_KEY)) || 0;
}

function withTimeout<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`${label} timed out after ${ms / 1000}s`)), ms);
    promise.then(
      (v) => {
        clearTimeout(timer);
        resolve(v);
      },
      (e) => {
        clearTimeout(timer);
        reject(e);
      },
    );
  });
}

export function useUpdater() {
  const [state, setState] = useState<UpdateState>({
    status: "idle",
    version: "",
    progress: 0,
    error: "",
    retriesLeft: MAX_DOWNLOAD_RETRIES,
  });

  const checkingRef = useRef(false);
  const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const mapUpdaterError = useCallback((e: unknown): string => {
    const msg = e instanceof Error ? e.message : String(e);
    if (/could not fetch|update check failed|timed out/i.test(msg)) {
      return "Network error or failed to fetch release metadata.";
    }
    return msg;
  }, []);

  const check = useCallback(async (): Promise<{ found: boolean; version?: string; error?: boolean }> => {
    if (checkingRef.current) return { found: false };
    checkingRef.current = true;

    try {
      setState((s) => ({ ...s, status: "checking", error: "" }));

      const result = await withTimeout(invoke<UpdateCheckResult>("check_app_update"), CHECK_TIMEOUT_MS, "Update check");

      if (!result.available || !result.version) {
        setState((s) => ({ ...s, status: "idle", version: "", progress: 0, error: "" }));
        localStorage.setItem(LAST_CHECK_KEY, String(Date.now()));
        return { found: false };
      }

      if (result.version === getSkipped()) {
        setState((s) => ({ ...s, status: "idle", version: "", progress: 0, error: "" }));
        localStorage.setItem(LAST_CHECK_KEY, String(Date.now()));
        return { found: false };
      }

      setState({
        status: "available",
        version: result.version,
        progress: 0,
        error: "",
        retriesLeft: MAX_DOWNLOAD_RETRIES,
      });
      localStorage.setItem(LAST_CHECK_KEY, String(Date.now()));
      return { found: true, version: result.version };
    } catch (e) {
      setState((s) => ({
        ...s,
        status: "error",
        version: "",
        progress: 0,
        error: mapUpdaterError(e),
      }));
      return { found: false, error: true };
    } finally {
      checkingRef.current = false;
    }
  }, [mapUpdaterError]);

  const download = useCallback(async () => {
    try {
      setState((s) => ({ ...s, status: "downloading", progress: 0, error: "" }));

      let downloaded = 0;
      let contentLength = 0;

      const unlisten = await listen<DownloadProgressPayload>("updater://download-progress", (event) => {
        if (event.payload.content_length) {
          contentLength = event.payload.content_length;
        }
        downloaded += event.payload.chunk_length;
        const pct =
          contentLength > 0
            ? Math.min(100, Math.round((downloaded / contentLength) * 100))
            : Math.min(95, downloaded > 0 ? Math.round(Math.log2(downloaded / 1024)) : 1);
        setState((s) => ({ ...s, progress: pct }));
      });

      try {
        await invoke("download_and_install_update");
        setState((s) => ({ ...s, status: "ready", progress: 100 }));
      } finally {
        unlisten();
      }
    } catch (e) {
      setState((prev) => {
        const retriesLeft = prev.retriesLeft - 1;
        if (retriesLeft > 0) {
          retryTimerRef.current = setTimeout(async () => {
            try {
              const res = await invoke<UpdateCheckResult>("check_app_update");
              if (res.available) {
                download();
              } else {
                setState((s) => ({ ...s, status: "idle", version: "", progress: 0, error: "" }));
              }
            } catch {
              setState((s) => ({ ...s, status: "error", progress: 0, error: mapUpdaterError(e) }));
            }
          }, 3000);
          return {
            ...prev,
            status: "downloading",
            progress: 0,
            error: "",
            retriesLeft,
          };
        }
        return {
          ...prev,
          status: "error",
          progress: 0,
          error: mapUpdaterError(e),
          retriesLeft: 0,
        };
      });
    }
  }, [mapUpdaterError]);

  const apply = useCallback(async () => {
    try {
      await invoke("restart_after_update");
    } catch (e) {
      setState((s) => ({
        ...s,
        status: "error",
        error: mapUpdaterError(e),
      }));
    }
  }, [mapUpdaterError]);

  const skip = useCallback(() => {
    if (state.version) {
      localStorage.setItem(SKIPPED_KEY, state.version);
    }
    setState({ status: "idle", version: "", progress: 0, error: "", retriesLeft: MAX_DOWNLOAD_RETRIES });
    if (retryTimerRef.current) {
      clearTimeout(retryTimerRef.current);
      retryTimerRef.current = null;
    }
  }, [state.version]);

  const dismiss = useCallback(() => {
    setState({ status: "idle", version: "", progress: 0, error: "", retriesLeft: MAX_DOWNLOAD_RETRIES });
    if (retryTimerRef.current) {
      clearTimeout(retryTimerRef.current);
      retryTimerRef.current = null;
    }
  }, []);

  const retry = useCallback(async () => {
    if (retryTimerRef.current) {
      clearTimeout(retryTimerRef.current);
      retryTimerRef.current = null;
    }
    setState((s) => ({ ...s, retriesLeft: MAX_DOWNLOAD_RETRIES }));
    await check();
  }, [check]);

  useEffect(() => {
    const lastCheck = getLastCheck();
    const elapsed = Date.now() - lastCheck;
    const firstDelay = elapsed >= CHECK_INTERVAL_MS ? 500 : CHECK_INTERVAL_MS - elapsed;

    const firstTimer = setTimeout(() => {
      check();
    }, firstDelay);

    const interval = setInterval(check, CHECK_INTERVAL_MS);

    return () => {
      clearTimeout(firstTimer);
      clearInterval(interval);
    };
  }, [check]);

  useEffect(() => {
    return () => {
      if (retryTimerRef.current) {
        clearTimeout(retryTimerRef.current);
      }
    };
  }, []);

  return { state, check, download, apply, skip, dismiss, retry };
}
