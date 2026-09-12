import { useSettingsStore } from "../stores/useSettingsStore";
import { useServerStore } from "../stores/useServerStore";
import { useTranslation } from "react-i18next";
import { pickFolder, setDownloadDirBackend } from "../lib/tauri-bridge";
import { useToast } from "./toast/Toast";

export function SettingsView() {
  const { t, i18n } = useTranslation();
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
            <input value={downloadDir} onChange={(e) => setDownloadDir(e.target.value)} className="min-w-0 flex-1 rounded-lg border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm mono" />
            <button onClick={browseDir} className="rounded-lg bg-[var(--accent)] px-4 py-2 text-sm font-medium text-white">Parcourir</button>
          </div>
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
      </div>
    </div>
  );
}
