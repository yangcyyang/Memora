import { useEffect, useState } from "react";
import { Outlet, useNavigate, useLocation } from "@tanstack/react-router";
import { Toaster } from "sonner";
import { getSettings, listPersonas } from "@/lib/tauri";
import { useTheme } from "@/hooks/useTheme";
import { Sidebar } from "@/components/Sidebar";
import { cn } from "@/lib/utils";

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
      <div className="flex items-center justify-center w-full h-screen bg-[#0F0F14]">
        <span className="animate-pulse text-2xl text-[#6366F1] font-semibold">
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
            background: "#1A1A23",
            color: "#F8FAFC",
            border: "1px solid #2D2D3D",
          },
        }}
      />
      {isWelcomePage ? (
        // 欢迎页面不需要侧边栏
        <Outlet />
      ) : (
        // 主应用布局：侧边栏 + 内容区
        <div className="flex h-screen bg-[#0F0F14]">
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
