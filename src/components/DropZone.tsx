import { useState, useCallback } from "react";
import { Upload, Sparkles, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";

interface Props {
  onFilesSelected: (files: File[]) => void;
}

export function DropZone({ onFilesSelected }: Props) {
  const { t } = useTranslation();
  const [dragging, setDragging] = useState(false);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setDragging(false);
      const files = Array.from(e.dataTransfer.files);
      if (files.length > 0) onFilesSelected(files);
    },
    [onFilesSelected]
  );

  const handleInput = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files ? Array.from(e.target.files) : [];
      if (files.length > 0) onFilesSelected(files);
    },
    [onFilesSelected]
  );

  return (
    <div
      onDragOver={(e) => {
        e.preventDefault();
        setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={handleDrop}
      className={`rivaldsend-dropzone group relative overflow-hidden rounded-[24px] p-8 sm:p-10 ${dragging ? "rivaldsend-dropzone-active" : ""}`}
    >
      <div className="absolute inset-0 opacity-[0.03] dark:opacity-[0.05]" style={{ backgroundImage: `url(/assets/Images/concept-transfer.png)`, backgroundSize: `280px`, backgroundRepeat: `no-repeat`, backgroundPosition: `right -20px center` }} />
      <div className="absolute top-4 right-4 hidden sm:flex items-center gap-1.5 rounded-full bg-[var(--accent-light)] px-3 py-1 text-xs font-medium text-[var(--accent)]">
        <Sparkles className="h-3 w-3" /> Glisser-déposer premium
      </div>
      <div className="relative flex flex-col items-center text-center">
        <div className="flex h-20 w-20 items-center justify-center rounded-[20px] bg-[var(--accent)] text-white shadow-lg shadow-blue-500/20 transition-transform group-hover:scale-105">
          <Upload className="h-9 w-9" strokeWidth={1.75} />
        </div>
        <h3 className="mt-5 text-[22px] font-bold tracking-tight text-[var(--text-primary)]">{t("dropTitle")}</h3>
        <p className="mt-1.5 max-w-md text-sm leading-relaxed text-[var(--text-secondary)]">{t("dropSubtitle")} — transfert local chiffré TLS 1.3 + BLAKE3, zéro serveur</p>
        <label className="mt-6 inline-flex cursor-pointer items-center gap-2 rounded-full bg-[var(--accent)] px-7 py-3 text-sm font-semibold text-white shadow-md shadow-blue-500/20 transition-all hover:bg-[var(--accent-hover)] hover:shadow-lg hover:shadow-blue-500/25 active:scale-[0.98]">
          <Upload className="h-4 w-4" />
          {t("selectFiles")}
          <input type="file" multiple className="hidden" onChange={handleInput} />
        </label>
        <div className="mt-4 flex flex-wrap items-center justify-center gap-2 text-xs">
          <span className="inline-flex items-center gap-1.5 rounded-full border border-[var(--border)] bg-[var(--surface-hover)] px-3 py-1 font-medium">
            <ShieldCheck className="h-3.5 w-3.5 text-[var(--success)]" /> Aucun octet via l&apos;IPC
          </span>
          <span className="text-[var(--text-tertiary)]">• Fichiers & dossiers • Jusqu&apos;à 20 Go testés</span>
        </div>
      </div>
    </div>
  );
}
