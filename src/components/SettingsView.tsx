import { useAppStore } from "../stores/appStore";
import { useTranslation } from "react-i18next";

export function SettingsView() {
  const { t } = useTranslation();
  const { downloadDir, setDownloadDir, darkMode, toggleDarkMode } = useAppStore();
  return (
    <div className="space-y-6">
      <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-6 space-y-4">
        <h3 className="text-sm font-semibold">{t("settings")}</h3>
        <div>
          <label className="text-xs font-medium text-[var(--text-secondary)]">{t("downloadDir")}</label>
          <div className="mt-1 flex gap-2">
            <input value={downloadDir} onChange={(e) => setDownloadDir(e.target.value)} className="flex-1 rounded-lg border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm mono" />
            <button className="rounded-lg bg-[var(--accent)] px-4 py-2 text-sm font-medium text-white">Parcourir</button>
          </div>
        </div>
        <div className="flex items-center justify-between pt-2 border-t border-[var(--border)]">
          <span className="text-sm">{t("darkMode")}</span>
          <button onClick={toggleDarkMode} className={`relative h-7 w-12 rounded-full transition-colors ${darkMode ? "bg-[var(--accent)]" : "bg-gray-300"}`}>
            <span className={`absolute top-0.5 h-6 w-6 rounded-full bg-white transition-all ${darkMode ? "left-5" : "left-0.5"}`} />
          </button>
        </div>
        <div>
          <label className="text-xs font-medium text-[var(--text-secondary)]">{t("language")}</label>
          <select className="mt-1 w-full rounded-lg border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm">
            <option>Français</option>
            <option>English</option>
            <option>Español</option>
          </select>
        </div>
        <div className="pt-2 border-t border-[var(--border)]">
          <p className="text-xs font-medium text-[var(--text-secondary)]">{t("network")}</p>
          <p className="mt-1 text-xs mono text-[var(--text-secondary)]">Interfaces : en0 (192.168.1.42) · VPN désactivée par défaut (F5.3)</p>
        </div>
      </div>
    </div>
  );
}
