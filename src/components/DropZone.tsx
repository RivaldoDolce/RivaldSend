import { useState, useCallback, useEffect } from "react";
import { Upload, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { pickFiles } from "../lib/tauri-bridge";

interface Props {
  onFilesSelected: (files: File[]) => void;
  onPathsSelected?: (paths: string[]) => void;
}

export function DropZone({ onFilesSelected, onPathsSelected }: Props) {
  const { t } = useTranslation();
  const [dragging, setDragging] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    import("@tauri-apps/api/webview").then(({ getCurrentWebview }) => {
      getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "over") setDragging(true);
        else if (event.payload.type === "drop") {
          setDragging(false);
          const paths = event.payload.paths;
          if (paths.length > 0 && onPathsSelected) onPathsSelected(paths);
          else if (paths.length > 0) {
            const fakeFiles = paths.map((p) => ({ name: p.split("/").pop() ?? p, size: 0 } as unknown as File));
            onFilesSelected(fakeFiles);
          }
        } else setDragging(false);
      }).then((fn) => { unlisten = fn; }).catch(() => {});
    }).catch(() => {});
    return () => { unlisten?.(); };
  }, [onFilesSelected, onPathsSelected]);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setDragging(false);
      const files = Array.from(e.dataTransfer.files);
      if (files.length > 0) onFilesSelected(files);
    },
    [onFilesSelected]
  );

  const handlePick = useCallback(async () => {
    const paths = await pickFiles();
    if (paths && paths.length > 0 && onPathsSelected) onPathsSelected(paths);
    else if (paths && paths.length > 0) {
      const fakeFiles = paths.map((p) => ({ name: p.split("/").pop() ?? p, size: 0 } as unknown as File));
      onFilesSelected(fakeFiles);
    }
  }, [onFilesSelected, onPathsSelected]);

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
      <div className="absolute inset-0 opacity-[0.04] dark:opacity-[0.05]" style={{ backgroundImage: `url(/assets/concept-transfer.webp)`, backgroundSize: `280px`, backgroundRepeat: `no-repeat`, backgroundPosition: `right -20px center` }} aria-hidden="true" />
      <div className="relative flex flex-col items-center text-center">
        <div className="flex h-20 w-20 items-center justify-center rounded-[20px] bg-[var(--accent)] text-white shadow-lg shadow-blue-500/20 transition-transform group-hover:scale-105">
          <Upload className="h-9 w-9" strokeWidth={1.75} />
        </div>
        <h3 className="mt-5 text-[22px] font-bold tracking-tight text-[var(--text-primary)]">{t("dropTitle")}</h3>
        <p className="mt-1.5 max-w-md text-sm leading-relaxed text-[var(--text-secondary)]">{t("dropSubtitle")}</p>
        <div className="mt-6 flex gap-2">
          <button type="button" onClick={handlePick} className="inline-flex items-center gap-2 rounded-full bg-[var(--accent)] px-7 py-3 text-sm font-semibold text-white shadow-md shadow-blue-500/20 hover:bg-[var(--accent-hover)] active:scale-[0.98]">
            <Upload className="h-4 w-4" />
            {t("selectFiles")}
          </button>
          <label className="inline-flex cursor-pointer items-center gap-2 rounded-full border border-[var(--border)] px-5 py-3 text-sm font-medium hover:bg-[var(--surface-hover)]">
            Parcourir
            <input type="file" multiple className="hidden" onChange={handleInput} />
          </label>
        </div>
        <div className="mt-4 flex flex-wrap items-center justify-center gap-2 text-xs">
          <span className="inline-flex items-center gap-1.5 rounded-full border border-[var(--border)] bg-[var(--surface-hover)] px-3 py-1 font-medium">
            <ShieldCheck className="h-3.5 w-3.5 text-[var(--success)]" /> Transfert local chiffré
          </span>
          <span className="text-[var(--text-tertiary)]">• Fichiers & dossiers • Jusqu&apos;à 20 Go</span>
        </div>
      </div>
    </div>
  );
}
