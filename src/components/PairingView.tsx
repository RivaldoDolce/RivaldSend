import { useState } from "react";
import { QrCode, Copy, Check } from "lucide-react";
import { useTranslation } from "react-i18next";

export function PairingView() {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const code = "742 981";
  const ip = "192.168.1.42:7420";

  const copy = async () => {
    await navigator.clipboard.writeText(code.replace(" ", ""));
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-6">
      <h3 className="text-sm font-semibold text-[var(--text-primary)] flex items-center gap-2">
        <QrCode className="h-4 w-4" /> {t("pairingCode")}
      </h3>
      <div className="mt-4 flex gap-6">
        <div className="flex h-32 w-32 items-center justify-center rounded-xl bg-white border border-[var(--border)] text-xs text-black">QR {ip}</div>
        <div className="flex-1">
          <p className="mono text-3xl font-bold tracking-widest text-[var(--text-primary)]">{code}</p>
          <p className="mt-1 text-xs text-[var(--text-secondary)]">TTL 2 min · usage unique · dérivation HKDF-SHA256</p>
          <button onClick={copy} className="mt-3 inline-flex items-center gap-2 rounded-full border border-[var(--border)] px-4 py-1.5 text-xs font-medium hover:bg-[var(--surface-hover)]">
            {copied ? <Check className="h-4 w-4 text-green-600" /> : <Copy className="h-4 w-4" />} {copied ? "Copié" : "Copier le code"}
          </button>
          <p className="mt-4 text-xs text-[var(--text-secondary)]">{t("enterIp")}</p>
          <div className="mt-2 flex gap-2">
            <input placeholder="192.168.1.10" className="flex-1 rounded-lg border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm" />
            <input placeholder="7420" className="w-20 rounded-lg border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm" />
          </div>
        </div>
      </div>
    </div>
  );
}
