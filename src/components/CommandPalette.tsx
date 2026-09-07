import { useEffect, useState, useMemo } from "react";
import { Search, Send, Wifi, History, Settings, Shield } from "lucide-react";
import { usePeersStore } from "../stores/usePeersStore";
import { useNavStore } from "../stores/useNavStore";

export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const [q, setQ] = useState("");
  const peers = usePeersStore((s) => s.peers);
  const setView = useNavStore((s) => s.setView);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen((v) => !v);
      }
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const filtered = useMemo(() => {
    const n = q.toLowerCase().trim();
    if (!n) return peers.slice(0, 5);
    return peers.filter((p) => p.name.toLowerCase().includes(n) || p.ip.includes(n)).slice(0, 5);
  }, [q, peers]);

  const actions = [
    { id: "discovery", label: "Rechercher des appareils", icon: Wifi, fn: () => { setView("discovery"); setOpen(false); } },
    { id: "pairing", label: "Appairage QR", icon: Shield, fn: () => { setView("pairing"); setOpen(false); } },
    { id: "history", label: "Historique", icon: History, fn: () => { setView("history"); setOpen(false); } },
    { id: "settings", label: "Paramètres", icon: Settings, fn: () => { setView("settings"); setOpen(false); } },
  ];

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh] bg-black/40 backdrop-blur-sm" onClick={() => setOpen(false)}>
      <div className="w-full max-w-lg mx-4 rounded-2xl border border-[var(--border)] bg-[var(--surface)] shadow-xl overflow-hidden" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center gap-3 border-b border-[var(--border)] px-4 py-3">
          <Search className="h-4 w-4 text-[var(--text-tertiary)]" />
          <input autoFocus value={q} onChange={(e) => setQ(e.target.value)} placeholder="Rechercher un appareil, IP ou action... (Ctrl+K)" className="flex-1 bg-transparent text-sm outline-none placeholder:text-[var(--text-tertiary)]" />
          <span className="rounded border border-[var(--border)] bg-[var(--surface-hover)] px-1.5 py-0.5 text-xs mono">ESC</span>
        </div>
        <div className="max-h-80 overflow-y-auto p-2 space-y-1">
          {filtered.map((p) => (
            <button key={p.id} onClick={() => { usePeersStore.getState().selectPeer(p.id); setOpen(false); }} className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left hover:bg-[var(--surface-hover)]">
              <Send className="h-4 w-4 text-[var(--accent)]" />
              <span className="flex-1 truncate text-sm font-medium">{p.name}</span>
              <span className="mono text-xs text-[var(--text-secondary)]">{p.ip}</span>
            </button>
          ))}
          {filtered.length === 0 && q && <p className="px-3 py-6 text-center text-sm text-[var(--text-secondary)]">Aucun appareil trouvé</p>}
          <div className="pt-2 border-t border-[var(--border)] mt-2">
            {actions.filter((a) => !q || a.label.toLowerCase().includes(q.toLowerCase())).map((a) => (
              <button key={a.id} onClick={a.fn} className="flex w-full items-center gap-3 rounded-xl px-3 py-2 text-left hover:bg-[var(--surface-hover)]">
                <a.icon className="h-4 w-4 text-[var(--text-secondary)]" />
                <span className="text-sm">{a.label}</span>
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
