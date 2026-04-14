import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/context-menu";
import { Brain, Edit3, CheckCircle } from "lucide-react";
import { toast } from "sonner";
import { reinforceMemory } from "@/lib/tauri";

interface MessageContextMenuProps {
  children: React.ReactNode;
  personaId: string;
  messageContent: string;
  onCorrect?: () => void;
}

export function MessageContextMenu({
  children,
  personaId,
  messageContent,
  onCorrect,
}: MessageContextMenuProps) {
  const handleReinforce = async () => {
    try {
      const result = await reinforceMemory(personaId, messageContent);
      if (result.success) {
        toast.success(`已强化记忆（版本 ${result.version}）`, {
          description: `提取了 ${result.rules.length} 条规则`,
        });
      } else {
        toast.error("记忆强化失败");
      }
    } catch (error) {
      toast.error(`强化失败: ${error}`);
    }
  };

  const handleCorrect = () => {
    onCorrect?.();
  };

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
      <ContextMenuContent className="w-48 bg-[#1A1A23] border-[#2D2D3D]">
        <ContextMenuItem
          onClick={handleReinforce}
          className="text-[#F8FAFC] hover:bg-[#252532] cursor-pointer"
        >
          <Brain className="w-4 h-4 mr-2 text-[#6366F1]" />
          记住这个
        </ContextMenuItem>
        <ContextMenuSeparator className="bg-[#2D2D3D]" />
        <ContextMenuItem
          onClick={handleCorrect}
          className="text-[#F8FAFC] hover:bg-[#252532] cursor-pointer"
        >
          <Edit3 className="w-4 h-4 mr-2 text-[#F59E0B]" />
          纠错
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
