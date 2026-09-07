import { useState, useMemo, useCallback, useEffect } from "react";
import { Search, MonitorSmartphone, Smartphone, Globe, RefreshCw, Radar } from "lucide-react";
import { usePeersStore } from "../stores/usePeersStore";
import { VerifySheet } from "./VerifySheet";
import { useToast } from "./toast/Toast";
import { useTranslation } from "react-i18next";
import type { Peer, PeerPlatform } from "../types";

function useDebouncedValue(value: string, delay: number): string {
  const [debounced, setDebounced] = useState(value);

  useEffect(() => {
    const t = setTimeout(() => setDebounced(value), delay);
    return () => clearTimeout(t);
  }, [value, delay]);

  return debounced;
}

function LatencyBadge({ ms }: { ms?: number }) {
  if (ms === undefined) return null;
  const color =
    ms < 20 ? "bg-emerald-100 text-emerald-700" :
    ms < 60 ? "bg-amber-100 text-amber-700" :
    "bg-red-100 text-red-700";
  return (
    <span className={`shrink-0 rounded-full px-2.5 py-0.5 text-xs font-semibold mono ${color}`}>
      {ms} ms
    </span>
  );
}

function PlatformIcon({ platform }: { platform: PeerPlatform }) {
  switch (platform) {
    case "macos":
    case "windows":
    case "linux":
      return <MonitorSmartphone className="h-5 w-5" strokeWidth={1.5} />;
    case "android":
    case "ios":
      return <Smartphone className="h-5 w-5" strokeWidth={1.5} />;
    default:
      return <Globe className="h-5 w-5" strokeWidth={1.5} />;
  }
}

function RadarView() {
  const { t } = useTranslation();
  return (
    <div className="radar mx-auto" aria-label={t("discoverySearching")}>
      <span className="sr-only">{t("discoveryScanSr")}</span>
    </div>
  );
}

function usePeerLatency(ip: string, port: number) {
  const [ms, setMs] = useState<number | undefined>(undefined);
  useEffect(() => {
    let alive = true;
    const ping = async () => {
      try {
        const { pingPeer } = await import("../lib/tauri-bridge");
        const v = await pingPeer(ip, port);
        if (alive) setMs(v);
      } catch { if (alive) setMs(undefined); }
    };
    ping();
    const t = setInterval(ping, 5000);
    return () => { alive = false; clearInterval(t); };
  }, [ip, port]);
  return ms;
}

function ManualConnectRow() {
  const [ip, setIp] = useState("");
  const [port, setPort] = useState("7420");
  const [busy, setBusy] = useState(false);
  const toast = useToast();

  const handleConnect = useCallback(async () => {
    if (!ip.trim()) return;
    setBusy(true);
    try {
      const { connectByIp } = await import("../lib/tauri-bridge");
      const peer = await connectByIp(ip.trim(), Number(port) || 7420);
      usePeersStore.getState().addPeer({ id: peer.id, name: peer.name, ip: peer.ip, port: peer.port, fingerprint: peer.fingerprintShort, fingerprintShort: peer.fingerprintShort, status: peer.trusted ? "paired" : "discovered", platform: peer.platform as never, latencyMs: undefined, trusted: peer.trusted });
      toast.success("Appareil ajouté", peer.name);
    } catch (err) {
      console.error("[discovery] connect_by_ip:", err);
      toast.error("Appareil injoignable", "Vérifiez IP, port et pare-feu.");
    } finally { setBusy(false); }
  }, [ip, port, toast]);

  return (
    <div className="flex items-center gap-2 rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-3">
      <Globe className="h-4 w-4 shrink-0 text-[var(--text-tertiary)]" />
      <input
        value={ip}
        onChange={(e) => setIp(e.target.value)}
        placeholder="IP manuelle (ex: 192.168.1.10)"
        className="flex-1 bg-transparent text-sm outline-none placeholder:text-[var(--text-tertiary)]"
        onKeyDown={(e) => e.key === "Enter" && handleConnect()}
      />
      <input
        value={port}
        onChange={(e) => setPort(e.target.value)}
        placeholder="Port"
        className="w-16 bg-transparent text-right text-sm mono outline-none placeholder:text-[var(--text-tertiary)]"
      />
      <button
        onClick={handleConnect}
        disabled={!ip.trim() || busy}
        className="shrink-0 rounded-full bg-[var(--accent)] px-4 py-1.5 text-xs font-semibold text-white disabled:opacity-40"
      >
        {busy ? "..." : "Connecter"}
      </button>
    </div>
  );
}

export function DiscoveryView() {
  const { t } = useTranslation();
  const peers = usePeersStore((s) => s.peers);
  const isDiscovering = usePeersStore((s) => s.isDiscovering);
  const setDiscovering = usePeersStore((s) => s.setDiscovering);
  const [query, setQuery] = useState("");
  const q = useDebouncedValue(query, 120);

  const handleScan = useCallback(async () => {
    setDiscovering(true);
    try {
      const { rescanPeers } = await import("../lib/tauri-bridge");
      await rescanPeers();
    } catch { /* ignore */ }
    window.setTimeout(() => setDiscovering(false), 2000);
  }, [setDiscovering]);

  useEffect(() => {
    handleScan();
  }, [handleScan]);

  const filtered = useMemo(() => {
    const n = q.trim().toLowerCase();
    const list = !n
      ? peers
      : peers.filter(
          (p) =>
            p.name.toLowerCase().includes(n) ||
            p.ip.startsWith(n) ||
            p.platform.includes(n)
        );
    return [...list].sort((a, b) => (a.latencyMs ?? 999) - (b.latencyMs ?? 999));
  }, [peers, q]);

  return (
    <div className="space-y-4">
      <button
        onClick={handleScan}
        disabled={isDiscovering}
        className="w-full inline-flex items-center justify-center gap-2 rounded-full bg-[var(--accent)] px-6 py-3 text-sm font-semibold text-white shadow-md hover:bg-[var(--accent-hover)] disabled:opacity-60"
      >
        {isDiscovering ? <RefreshCw className="h-4 w-4 animate-spin" /> : <Radar className="h-4 w-4" />}
        {isDiscovering ? t("discoverySearching") : t("discoverySearchNearby")}
      </button>

      <div className="relative">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[var(--text-tertiary)]" />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t("discoverySearchPlaceholder")}
          className="w-full rounded-full border border-[var(--border)] bg-[var(--surface)] py-2.5 pl-9 pr-4 text-sm outline-none focus:border-[var(--accent)] focus:ring-2 focus:ring-[var(--accent-ring)]"
        />
      </div>

      {isDiscovering && peers.length === 0 && <RadarView />}

      <div className="space-y-3">
        {filtered.map((p) => (
          <DiscoveryCard key={p.id} peer={p} />
        ))}
      </div>

      {filtered.length === 0 && !isDiscovering && (
        <div className="flex flex-col items-center rounded-2xl border border-dashed border-[var(--border-strong)] bg-[var(--surface)] p-8 text-center">
          <img src="/assets/empty-no-peers.webp" alt="" width={80} height={80} decoding="async" loading="lazy" className="h-20 w-20 object-contain opacity-90" onError={(e) => ((e.target as HTMLImageElement).style.display = "none")} />
          <p className="mt-3 text-sm font-medium">{t("discoveryNoPeers")}</p>
          <p className="mt-1 text-xs text-[var(--text-secondary)]">
            {t("discoveryNoPeersDesc")}
          </p>
        </div>
      )}

      <ManualConnectRow />
    </div>
  );
}

function DiscoveryCard({ peer }: { peer: Peer }) {
  const openSendModal = usePeersStore((s) => s.openSendModal);
  const [connState, setConnState] = useState<"discovered" | "connecting" | "verifying" | "paired" | "failed">(
    peer.trusted ? "paired" : "discovered"
  );
  const [showVerify, setShowVerify] = useState(false);
  const latency = usePeerLatency(peer.ip, peer.port);

  const handleConnect = useCallback(async () => {
    setConnState("connecting");
    try {
      const { connectByIp } = await import("../lib/tauri-bridge");
      await connectByIp(peer.ip, peer.port);
      setConnState("verifying");
      setShowVerify(true);
    } catch {
      setConnState("failed");
      setTimeout(() => setConnState("discovered"), 2500);
    }
  }, [peer.ip, peer.port]);

  const handleApprove = useCallback(async () => {
    try {
      const { approvePeer } = await import("../lib/tauri-bridge");
      await approvePeer(peer.id);
      usePeersStore.getState().updatePeer(peer.id, { trusted: true, status: "paired" });
      setShowVerify(false);
      setConnState("paired");
    } catch {
      setShowVerify(false);
      setConnState("failed");
    }
  }, [peer.id]);

  const handleReject = useCallback(() => {
    setShowVerify(false);
    setConnState("discovered");
  }, []);

  const handleSend = useCallback(() => {
    openSendModal([]);
  }, [openSendModal]);

  return (
    <div className="card-premium flex items-center gap-3 rounded-[20px] p-4">
      <div className={`flex h-12 w-12 shrink-0 items-center justify-center rounded-[14px] border ${
        connState === "paired"
          ? "bg-white border-[var(--accent)]/20 text-[var(--accent)]"
          : "bg-[var(--surface-hover)] border-[var(--border)] text-[var(--text-secondary)]"
      }`}>
        <PlatformIcon platform={peer.platform} />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-semibold truncate">{peer.name}</p>
        <p className="mono text-xs text-[var(--text-secondary)]">
          {peer.ip}:{peer.port} · {peer.platform}
        </p>
      </div>
      <LatencyBadge ms={latency ?? peer.latencyMs} />
      {connState === "paired" ? (
        <button
          onClick={handleSend}
          className="shrink-0 rounded-full bg-[var(--accent)] px-4 py-1.5 text-xs font-semibold text-white hover:bg-[var(--accent-hover)]"
        >
          Envoyer
        </button>
      ) : (
        <button
          onClick={handleConnect}
          disabled={connState === "connecting" || connState === "verifying"}
          className="shrink-0 rounded-full border border-[var(--border)] px-4 py-1.5 text-xs font-semibold hover:bg-[var(--surface-hover)] disabled:opacity-60"
        >
          {connState === "connecting"
            ? "Connexion..."
            : connState === "verifying"
            ? "Vérification..."
            : connState === "failed"
            ? "Réessayer"
            : "Connecter"}
        </button>
      )}
      {showVerify && <VerifySheet peer={peer} onApprove={handleApprove} onReject={handleReject} />}
    </div>
  );
}
