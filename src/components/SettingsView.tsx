import { useState, useCallback } from "react";
import { Trash2 } from "lucide-react";
import { useSettingsStore } from "../stores/useSettingsStore";
import { useServerStore } from "../stores/useServerStore";
import { usePeersStore } from "../stores/usePeersStore";
import { useTranslation } from "react-i18next";
import { pickFolder, setDownloadDirBackend } from "../lib/tauri-bridge";
import { useDeviceContext } from "../hooks/useDeviceContext";
import { useToast } from "./toast/Toast";

function TrustedPeersSection() {
  const toast = useToast();
  const peers = usePeersStore((s) => s.peers);
  const removePeer = usePeersStore((s) => s.removePeer);
  const [oubliEnCours, setOubliEnCours] = useState<string | null>(null);
  const apparies = peers.filter((p) => p.trusted);

  const oublier = useCallback(async (peerId: string, nom: string) => {
    setOubliEnCours(peerId);
    try {
      const { forgetPeer } = await import("../lib/tauri-bridge");
      await forgetPeer(peerId);
      removePeer(peerId);
      toast.success("Appareil oublié", `${nom} ne pourra plus envoyer sans nouvel appairage.`);
    } catch {
      toast.error("Oubli impossible", "Reessayez dans un instant.");
    } finally {
      setOubliEnCours(null);
    }
  }, [removePeer, toast]);

  const toutOublier = useCallback(async () => {
    try {
      const { forgetAllPeers } = await import("../lib/tauri-bridge");
      const n = await forgetAllPeers();
      for (const p of apparies) removePeer(p.id);
      toast.success("Confiance revoquee", `${n} appareil(s) oublie(s).`);
    } catch {
      toast.error("Oubli impossible", "Reessayez dans un instant.");
    }
  }, [apparies, removePeer, toast]);

  return (
    <div className="pt-2 border-t border-[var(--border)]">
      <div className="flex items-center justify-between">
        <p className="text-sm font-medium">Appareils apparies ({apparies.length})</p>
        {apparies.length > 0 && (
          <button onClick={toutOublier} className="text-xs text-[var(--text-tertiary)] hover:text-red-600">
            Tout oublier
          </button>
        )}
      </div>
      {apparies.length === 0 ? (
        <p className="mt-1 text-xs text-[var(--text-secondary)]">
          Aucun. Appariez un appareil depuis l'onglet Decouverte : vous pourrez ensuite envoyer sans ressaisir de code.
        </p>
      ) : (
        <ul className="mt-2 space-y-1.5">
          {apparies.map((p) => (
            <li key={p.id} className="flex items-center gap-2 rounded-lg border border-[var(--border)] bg-[var(--background)] px-3 py-2">
              <span className="h-2 w-2 shrink-0 rounded-full bg-emerald-500" />
              <span className="min-w-0 flex-1 truncate text-xs font-medium">{p.name}</span>
              <span className="mono shrink-0 text-xs text-[var(--text-tertiary)]">{p.ip}</span>
              <button
                onClick={() => oublier(p.id, p.name)}
                disabled={oubliEnCours === p.id}
                title={`Oublier ${p.name}`}
                aria-label={`Oublier ${p.name}`}
                className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full text-[var(--text-tertiary)] hover:bg-red-50 hover:text-red-600 dark:hover:bg-red-950/30 disabled:opacity-40"
              >
                <Trash2 className="h-3.5 w-3.5" />
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function SettingsView() {
  const { t, i18n } = useTranslation();
  const { isMobile } = useDeviceContext();
  const { downloadDir, setDownloadDir, darkMode, toggleDarkMode, language, setLanguage, notifications, setNotifications } = useSettingsStore();
  const tls = useServerStore((s) => s.tls);
  const empreinte = useServerStore((s) => s.empreinte);
  const toast = useToast();

  const browseDir = async () => {
    try {
      const selected = await pickFolder();
      if (selected) {
        setDownloadDir(selected);
        // Transmet le dossier au backend pour les prochaines réceptions.
        try {
          await setDownloadDirBackend(selected);
        } catch (err) {
          console.error("[Réglages] set_download_dir a échoué :", err);
        }
        toast.success("Dossier modifié", selected);
      }
    } catch (err) {
      console.error("[Réglages] choix du dossier impossible :", err);
      toast.error("Erreur", "Impossible d'ouvrir le sélecteur de dossier");
    }
  };

  return (
    <div className="space-y-6">
      <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-6 space-y-4">
        <h3 className="text-sm font-semibold">{t("settings")}</h3>
        <div>
          <label className="text-xs font-medium text-[var(--text-secondary)]">{t("downloadDir")}</label>
          <div className="mt-1 flex gap-2">
            <input value={downloadDir} onChange={(e) => setDownloadDir(e.target.value)} readOnly={isMobile} className="min-w-0 flex-1 rounded-lg border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm mono" />
            {!isMobile && (
              <button onClick={browseDir} className="rounded-lg bg-[var(--accent)] px-4 py-2 text-sm font-medium text-white">Parcourir</button>
            )}
          </div>
          {isMobile && (
            <p className="mt-1 text-xs text-[var(--text-tertiary)]">Dossier géré par l'application (pas de sélecteur natif sur mobile).</p>
          )}
        </div>
        <div className="flex items-center justify-between pt-2 border-t border-[var(--border)]">
          <span className="text-sm">{t("darkMode")}</span>
          <button onClick={toggleDarkMode} role="switch" aria-checked={darkMode} aria-label="Activer le mode sombre" className={`relative h-7 w-12 rounded-full transition-colors ${darkMode ? "bg-[var(--accent)]" : "bg-[var(--border-strong)] dark:bg-zinc-600"}`}>
            <span className={`absolute top-0.5 h-6 w-6 rounded-full bg-white shadow-sm transition-all ${darkMode ? "left-5" : "left-0.5"}`} />
          </button>
        </div>
        <div>
          <label className="text-xs font-medium text-[var(--text-secondary)]">{t("language")}</label>
          <select
            value={language}
            onChange={(e) => {
              const lang = e.target.value as "fr" | "en" | "es";
              setLanguage(lang);
              i18n.changeLanguage(lang);
            }}
            className="mt-1 w-full rounded-lg border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm"
          >
            <option value="fr">Français</option>
            <option value="en">English</option>
            <option value="es">Español</option>
          </select>
        </div>
        <div className="flex items-center justify-between pt-2 border-t border-[var(--border)]">
          <span className="text-sm">Notifications</span>
          <button onClick={() => setNotifications(!notifications)} role="switch" aria-checked={notifications} aria-label="Activer les notifications" className={`relative h-7 w-12 rounded-full transition-colors ${notifications ? "bg-[var(--accent)]" : "bg-[var(--border-strong)] dark:bg-zinc-600"}`}>
            <span className={`absolute top-0.5 h-6 w-6 rounded-full bg-white shadow-sm transition-all ${notifications ? "left-5" : "left-0.5"}`} />
          </button>
        </div>
        <div className="pt-2 border-t border-[var(--border)]">
          <p className="text-sm font-medium">{t("tlsTitle")}</p>
          <p className="mt-1 text-xs text-[var(--text-secondary)]">
            {tls ? t("tlsActive") : t("tlsInactive")}
          </p>
          {empreinte && (
            <p className="mono mt-2 break-all text-xs" title={empreinte}>
              {t("tlsFingerprint")} : {empreinte.slice(0, 32)}…
            </p>
          )}
          <p className="mt-1 text-xs text-[var(--text-tertiary)]">{t("tlsHint")}</p>
        </div>
        <TrustedPeersSection />
      </div>
    </div>
  );
}
