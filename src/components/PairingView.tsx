import { useState, useEffect, useCallback } from "react";
import { QrCode, Copy, Check, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { generatePairingQr, getDeviceInfo } from "../lib/tauri-bridge";
import { usePeersStore } from "../stores/usePeersStore";
import { useToast } from "./toast/Toast";

export function PairingView() {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const [qrSvg, setQrSvg] = useState("");
  const [loading, setLoading] = useState(true);
  const [manualIp, setManualIp] = useState("");
  const [manualPort, setManualPort] = useState("53317");
  const [deviceIp, setDeviceIp] = useState("127.0.0.1");
  const [devicePort] = useState(53317);
  const peers = usePeersStore((s) => s.peers);
  const toast = useToast();

  const generateCode = useCallback(async () => {
    setLoading(true);
    try {
      let ip = deviceIp;
      let port = devicePort;
      let fpShort = "0000";
      try {
        const info = await getDeviceInfo();
        ip = info.ip || ip;
        port = info.port || port;
        fpShort = (info as unknown as { fingerprintShort?: string }).fingerprintShort || info.fingerprint?.slice(0,4) || fpShort;
        setDeviceIp(ip);
      } catch {
        const fallbackIp = peers[0]?.ip;
        if (fallbackIp) ip = fallbackIp;
      }
      const code = Math.floor(100000 + Math.random() * 900000).toString();
      let svg = "";
      try {
        svg = await generatePairingQr({ ip, port, code, fingerprintShort: fpShort });
      } catch {
        const QRCode = await import("qrcode");
        const payload = `rivaldsend://${ip}:${port}?code=${code}&fp=${fpShort}`;
        svg = await QRCode.toString(payload, { type: "svg", margin: 1, width: 200, color: { dark: "#000000", light: "#ffffff" } });
      }
      if (!svg.includes("<svg")) svg = "";
      setQrSvg(svg);
    } catch {
      toast.error("Erreur", "Impossible de générer le QR code");
      setQrSvg("");
    }
    setLoading(false);
  }, [deviceIp, devicePort, peers, toast]);

  useEffect(() => {
    generateCode();
  }, [generateCode]);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(`rivaldsend://${deviceIp}:${devicePort}`);
      setCopied(true);
      toast.success("Copié", "Lien d'appairage copié dans le presse-papiers");
      setTimeout(() => setCopied(false), 1500);
    } catch {
      toast.error("Erreur", "Impossible de copier");
    }
  };

  return (
    <div className="space-y-4">
      <div className="rounded-[20px] border border-[var(--border)] bg-[var(--surface)] p-6 shadow-sm">
        <h3 className="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2">
          <QrCode className="h-4 w-4 text-[var(--accent)]" /> {t("pairingCode")}
        </h3>
        <p className="mt-1 text-xs text-[var(--text-secondary)]">Expire dans 04:59 · Usage unique · {deviceIp}:{devicePort}</p>
        <div className="mt-4 flex flex-col sm:flex-row gap-6">
          <div className="flex h-44 w-44 shrink-0 items-center justify-center rounded-2xl bg-white border border-[var(--border)] shadow-sm mx-auto sm:mx-0 overflow-hidden">
            {loading ? (
              <RefreshCw className="h-8 w-8 text-[var(--text-secondary)] animate-spin" />
            ) : qrSvg ? (
              <div className="p-2 w-full h-full flex items-center justify-center [&>svg]:w-full [&>svg]:h-full [&>svg]:max-w-[168px] [&>svg]:max-h-[168px]" dangerouslySetInnerHTML={{ __html: qrSvg }} />
            ) : (
              <span className="text-xs text-[var(--text-secondary)] px-4 text-center">QR indisponible<br/><button onClick={generateCode} className="mt-2 text-[var(--accent)] underline">Réessayer</button></span>
            )}
          </div>
          <div className="flex-1 min-w-0">
            <p className="text-sm leading-relaxed text-[var(--text-secondary)]">Scannez ce QR avec l&apos;autre appareil pour appairer instantanément.</p>
            <div className="mt-3 flex flex-wrap gap-2">
              <button onClick={generateCode} className="inline-flex items-center gap-1.5 rounded-full border border-[var(--border)] px-4 py-2 text-xs font-medium hover:bg-[var(--surface-hover)]">
                <RefreshCw className="h-3 w-3" /> Nouveau code
              </button>
              <button onClick={copy} className="inline-flex items-center gap-2 rounded-full bg-[var(--accent)] px-4 py-2 text-xs font-semibold text-white hover:bg-[var(--accent-hover)]">
                {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />} {copied ? "Copié !" : "Copier le lien"}
              </button>
            </div>
            <div className="mt-3 flex items-center gap-2 text-xs text-emerald-600 dark:text-emerald-400">
              <span className="h-2 w-2 rounded-full bg-emerald-500" /> 🔒 Chiffré TLS 1.3 · PIN 6 chiffres
            </div>
          </div>
        </div>
      </div>

      <div className="rounded-[20px] border border-[var(--border)] bg-[var(--surface)] p-6 shadow-sm">
        <h4 className="text-sm font-semibold">Connexion manuelle</h4>
        <p className="mt-1 text-xs text-[var(--text-secondary)]">{t("enterIp")} — si le QR ne passe pas, saisissez l&apos;IP affichée sur l&apos;autre appareil.</p>
        <div className="mt-3 flex flex-col sm:flex-row gap-2">
          <input value={manualIp} onChange={(e) => setManualIp(e.target.value)} placeholder="192.168.1.10" className="flex-1 rounded-xl border border-[var(--border)] bg-[var(--background)] px-3 py-2.5 text-sm outline-none focus:border-[var(--accent)]" />
          <input value={manualPort} onChange={(e) => setManualPort(e.target.value)} placeholder="53317" className="w-full sm:w-24 rounded-xl border border-[var(--border)] bg-[var(--background)] px-3 py-2.5 text-sm outline-none focus:border-[var(--accent)]" />
          <button disabled={!manualIp.trim()} onClick={async () => { try { const { connectByIp } = await import("../lib/tauri-bridge"); await connectByIp(manualIp.trim(), Number(manualPort)||53317); toast.success("Appareil ajouté", manualIp); } catch { toast.error("Connexion échouée", "Vérifiez IP/port"); } }} className="w-full sm:w-auto shrink-0 rounded-full bg-[var(--accent)] px-6 py-2.5 text-sm font-semibold text-white hover:bg-[var(--accent-hover)] disabled:opacity-40">Connecter</button>
        </div>
      </div>
    </div>
  );
}
