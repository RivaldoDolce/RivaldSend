import { useState, useEffect } from "react";
import { QrCode, Copy, Check, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { generatePairingQr } from "../lib/tauri-bridge";
import { usePeersStore } from "../stores/usePeersStore";
import { useToast } from "./toast/Toast";

export function PairingView() {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const [qrSvg, setQrSvg] = useState("");
  const [loading, setLoading] = useState(true);
  const [manualIp, setManualIp] = useState("");
  const [manualPort, setManualPort] = useState("7420");
  const peers = usePeersStore((s) => s.peers);
  const toast = useToast();

  const generateCode = async () => {
    setLoading(true);
    try {
      const ip = peers[0]?.ip ?? "127.0.0.1";
      const port = peers[0]?.port ?? 7420;
      const code = Math.floor(100000 + Math.random() * 900000).toString();
      const fpShort = peers[0]?.fingerprintShort ?? "0000";
      const svg = await generatePairingQr({ ip, port, code, fingerprintShort: fpShort });
      setQrSvg(svg);
    } catch {
      toast.error("Erreur", "Impossible de générer le QR code");
      setQrSvg("");
    }
    setLoading(false);
  };

  useEffect(() => {
    generateCode();
  }, []);

  const copy = async () => {
    try {
      const ip = peers[0]?.ip ?? "127.0.0.1";
      const port = peers[0]?.port ?? 7420;
      await navigator.clipboard.writeText(`rivaldsend://${ip}:${port}`);
      setCopied(true);
      toast.success("Copié", "Lien d'appairage copié dans le presse-papiers");
      setTimeout(() => setCopied(false), 1500);
    } catch {
      toast.error("Erreur", "Impossible de copier");
    }
  };

  return (
    <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-6">
      <h3 className="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2">
        <QrCode className="h-4 w-4" /> {t("pairingCode")}
      </h3>
      <div className="mt-4 flex gap-6">
        <div className="flex h-40 w-40 shrink-0 items-center justify-center rounded-xl bg-white border border-[var(--border)]">
          {loading ? (
            <RefreshCw className="h-8 w-8 text-[var(--text-secondary)] animate-spin" />
          ) : qrSvg ? (
            <div className="p-2" dangerouslySetInnerHTML={{ __html: qrSvg }} />
          ) : (
            <span className="text-xs text-[var(--text-secondary)]">QR indisponible</span>
          )}
        </div>
        <div className="flex-1">
          <p className="text-xs text-[var(--text-secondary)]">Scannez ce QR avec l&apos;autre appareil pour appairer.</p>
          <button onClick={generateCode} className="mt-2 inline-flex items-center gap-1.5 text-xs text-[var(--accent)] hover:underline">
            <RefreshCw className="h-3 w-3" /> Nouveau code
          </button>
          <button onClick={copy} className="mt-3 inline-flex items-center gap-2 rounded-full border border-[var(--border)] px-4 py-1.5 text-xs font-medium hover:bg-[var(--surface-hover)]">
            {copied ? <Check className="h-4 w-4 text-green-600" /> : <Copy className="h-4 w-4" />} {copied ? "Copié" : "Copier le lien"}
          </button>
          <p className="mt-4 text-xs text-[var(--text-secondary)]">{t("enterIp")}</p>
          <div className="mt-2 flex gap-2">
            <input value={manualIp} onChange={(e) => setManualIp(e.target.value)} placeholder="192.168.1.10" className="flex-1 rounded-lg border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm" />
            <input value={manualPort} onChange={(e) => setManualPort(e.target.value)} placeholder="7420" className="w-20 rounded-lg border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm" />
            <button className="rounded-lg bg-[var(--accent)] px-4 py-2 text-sm font-medium text-white">Connecter</button>
          </div>
        </div>
      </div>
    </div>
  );
}
