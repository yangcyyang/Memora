import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
// Note: ScrollArea component not available, using native scroll
import { Plus, Settings, MessageCircle, User } from 'lucide-react';
import { cn } from '@/lib/utils';
import { listPersonas } from '@/lib/tauri';

interface Persona {
  id: string;
  name: string;
  avatar_emoji?: string;
}

interface SidebarProps {
  currentPersonaId?: string;
  onSelectPersona: (id: string) => void;
  onCreatePersona: () => void;
  onOpenSettings: () => void;
}

export function Sidebar({
  currentPersonaId,
  onSelectPersona,
  onCreatePersona,
  onOpenSettings,
}: SidebarProps) {
  const [personas, setPersonas] = useState<Persona[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadPersonas();
  }, []);

  const loadPersonas = async () => {
    try {
      const list = await listPersonas();
      setPersonas(list);
    } catch (error) {
      console.error('Failed to load personas:', error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="w-[280px] h-full bg-[#1A1A23] border-r border-[#2D2D3D] flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-[#2D2D3D]">
        <div className="flex items-center gap-2 mb-4">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-[#6366F1] to-[#8B5CF6] flex items-center justify-center">
            <MessageCircle className="w-5 h-5 text-white" />
          </div>
          <span className="text-lg font-semibold text-[#F8FAFC]">Memora</span>
        </div>
        <Button
          onClick={onCreatePersona}
          className="w-full bg-[#6366F1] hover:bg-[#4F46E5] text-white"
        >
          <Plus className="w-4 h-4 mr-2" />
          新建角色
        </Button>
      </div>

      {/* Persona List */}
      <div className="flex-1 px-3 py-4 overflow-y-auto">
        <div className="space-y-1">
          <div className="text-xs font-medium text-[#64748B] px-3 py-2">
            我的角色
          </div>
          {loading ? (
            <div className="text-sm text-[#64748B] px-3 py-2">加载中...</div>
          ) : personas.length === 0 ? (
            <div className="text-sm text-[#64748B] px-3 py-2">
              还没有角色，点击上方新建
            </div>
          ) : (
            personas.map((persona) => (
              <button
                key={persona.id}
                onClick={() => onSelectPersona(persona.id)}
                className={cn(
                  'w-full flex items-center gap-3 px-3 py-2.5 rounded-lg transition-colors text-left',
                  currentPersonaId === persona.id
                    ? 'bg-[#6366F1]/20 text-[#F8FAFC]'
                    : 'text-[#94A3B8] hover:bg-[#252532] hover:text-[#F8FAFC]'
                )}
              >
                <span className="text-xl">{persona.avatar_emoji || '💬'}</span>
                <span className="flex-1 truncate text-sm">{persona.name}</span>
                {currentPersonaId === persona.id && (
                  <div className="w-2 h-2 rounded-full bg-[#22C55E]" />
                )}
              </button>
            ))
          )}
        </div>
      </div>

      {/* Footer */}
      <div className="p-3 border-t border-[#2D2D3D]">
        <Button
          variant="ghost"
          onClick={onOpenSettings}
          className="w-full justify-start text-[#94A3B8] hover:text-[#F8FAFC] hover:bg-[#252532]"
        >
          <Settings className="w-4 h-4 mr-3" />
          设置
        </Button>
      </div>
    </div>
  );
}
