import { useEffect, useState } from "react";
import { Outlet, useNavigate, useLocation } from "@tanstack/react-router";
import { Toaster } from "sonner";
import { listen } from "@tauri-apps/api/event";
import { getSettings, listPersonas } from "@/lib/tauri";
import { useTheme } from "@/hooks/useTheme";
import { Sidebar } from "@/components/Sidebar";

export default function App() {
  const navigate = useNavigate();
  const location = useLocation();
  const [ready, setReady] = useState(false);
  const [currentPersonaId, setCurrentPersonaId] = useState<string>();
  const { resolved } = useTheme();

  // 判断是否在欢迎/登录页面（这些页面不需要侧边栏）
  const isWelcomePage = location.pathname === "/welcome" || location.pathname === "/onboarding";

  useEffect(() => {
    (async () => {
      try {
        const settings = await getSettings();
        if (settings.has_api_key) {
          const list = await listPersonas();
          if (list.length > 0) {
            setCurrentPersonaId(list[0].id);
            if (location.pathname === "/") {
              navigate({ to: "/chat/$personaId", params: { personaId: list[0].id } });
            }
          } else {
            navigate({ to: "/welcome" });
          }
        } else {
          navigate({ to: "/welcome" });
        }
      } catch {
        navigate({ to: "/welcome" });
      }
      setReady(true);
    })();
  }, [navigate]);

  // 监听主动触达通知点击事件
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    
    listen<{ persona_id: string }>("proactive-trigger", (event) => {
      const { persona_id } = event.payload;
      setCurrentPersonaId(persona_id);
      navigate({ to: "/chat/$personaId", params: { personaId: persona_id } });
    }).then(fn => { unlisten = fn; });
    
    return () => { unlisten?.(); };
  }, [navigate]);

  const handleSelectPersona = (id: string) => {
    setCurrentPersonaId(id);
    navigate({ to: "/chat/$personaId", params: { personaId: id } });
  };

  const handleCreatePersona = () => {
    navigate({ to: "/create" });
  };

  const handleOpenSettings = () => {
    navigate({ to: "/settings" });
  };

  if (!ready) {
    return (
      <div className="flex items-center justify-center w-full h-screen bg-[var(--color-cream-50)]">
        <span className="animate-pulse text-2xl text-[var(--color-rose-500)] font-semibold">
          Memora
        </span>
      </div>
    );
  }

  return (
    <>
      <Toaster
        position="top-center"
        theme={resolved}
        toastOptions={{
          style: {
            background: resolved === "dark" ? "#1A1A23" : "#FFFFFF",
            color: resolved === "dark" ? "#F8FAFC" : "#1A1A1A",
            border: resolved === "dark" ? "1px solid #2D2D3D" : "1px solid #E0E0E0",
          },
        }}
      />
      {isWelcomePage ? (
        // 欢迎页面不需要侧边栏
        <Outlet />
      ) : (
        // 主应用布局：侧边栏 + 内容区
        <div className="flex h-screen bg-[var(--color-cream-50)]">
          <Sidebar
            currentPersonaId={currentPersonaId}
            onSelectPersona={handleSelectPersona}
            onCreatePersona={handleCreatePersona}
            onOpenSettings={handleOpenSettings}
          />
          <main className="flex-1 overflow-hidden">
            <Outlet />
          </main>
        </div>
      )}
    </>
  );
}
