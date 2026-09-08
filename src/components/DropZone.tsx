import { useState, useCallback, useEffect, useMemo, useRef } from "react";
import { Upload, ShieldCheck, FolderOpen, FileText, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { pickFiles, pickFolder } from "../lib/tauri-bridge";
import { formatBytes } from "../lib/utils";
import { usePeersStore } from "../stores/usePeersStore";

interface Props {
  onFilesSelected: (files: File[]) => void;
  onPathsSelected?: (paths: string[]) => void;
}

export function DropZone({ onFilesSelected, onPathsSelected }: Props) {
  const { t } = useTranslation();
  const [dragging, setDragging] = useState(false);
  const pending = usePeersStore((s) => s.pendingFiles);
  const totalSize = useMemo(() => pending.reduce((a, f) => a + f.size, 0), [pending]);
  const totalLabel = useMemo(() => formatBytes(totalSize), [totalSize]);

  const lastOpenRef = useRef(0);
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    import("@tauri-apps/api/webview").then(({ getCurrentWebview }) => {
      getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "over") setDragging(true);
        else if (event.payload.type === "drop") {
          setDragging(false);
          const paths = event.payload.paths;
          if (paths.length === 0) return;
          const now = Date.now();
          if (now - lastOpenRef.current < 300) return;
          lastOpenRef.current = now;
          if (onPathsSelected) onPathsSelected(paths);
          else onFilesSelected([]);
        } else setDragging(false);
      }).then((fn) => { unlisten = fn; }).catch(() => {});
    }).catch(() => {});
    return () => { unlisten?.(); };
  }, [onFilesSelected, onPathsSelected, lastOpenRef]);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setDragging(false);
      const now = Date.now();
      if (now - lastOpenRef.current < 300) return;
      lastOpenRef.current = now;
      const files = Array.from(e.dataTransfer.files);
      if (files.length > 0) onFilesSelected(files);
    },
    [onFilesSelected, lastOpenRef]
  );

  const handlePick = useCallback(async () => {
    const paths = await pickFiles();
    if (paths && paths.length > 0 && onPathsSelected) onPathsSelected(paths);
  }, [onPathsSelected]);

  const handlePickFolder = useCallback(async () => {
    const folder = await pickFolder();
    if (folder && onPathsSelected) onPathsSelected([folder]);
  }, [onPathsSelected]);

  const clearPending = useCallback(() => usePeersStore.getState().closeSendModal(), []);

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
      <div className="absolute inset-0 opacity-[0.04] dark:opacity-[0.05] pointer-events-none" style={{ backgroundImage: `url(/assets/concept-transfer.webp)`, backgroundSize: `280px`, backgroundRepeat: `no-repeat`, backgroundPosition: `right -20px center` }} aria-hidden="true" />
      {pending.length > 0 && (
        <button onClick={clearPending} className="absolute right-3 top-3 rounded-full bg-[var(--surface)] p-1.5 shadow border border-[var(--border)] hover:bg-[var(--surface-hover)]">
          <X className="h-4 w-4" />
        </button>
      )}
      <div className="relative flex flex-col items-center text-center">
        <div className="flex h-20 w-20 items-center justify-center rounded-[20px] bg-[var(--accent)] text-white shadow-lg shadow-blue-500/20 transition-transform group-hover:scale-105 group-hover:rotate-1">
          <Upload className="h-9 w-9" strokeWidth={1.75} />
        </div>
        <h3 className="mt-5 text-[22px] font-bold tracking-tight text-[var(--text-primary)]">{t("dropTitle")}</h3>
        <p className="mt-1.5 max-w-md text-sm leading-relaxed text-[var(--text-secondary)]">{t("dropSubtitle")}</p>
        {pending.length > 0 ? (
          <div className="mt-4 w-full max-w-md rounded-xl border border-[var(--border)] bg-[var(--surface-hover)] p-3 text-left">
            <div className="flex items-center justify-between text-xs font-semibold">
              <span>{pending.length} fichier{pending.length>1?"s":""} · {totalLabel}</span>
              <span className="text-[var(--text-tertiary)]">Prêt à envoyer</span>
            </div>
            <div className="mt-2 max-h-20 overflow-y-auto space-y-1">
              {pending.slice(0,4).map((f) => (
                <div key={f.path} className="flex items-center gap-2 text-xs truncate">
                  <FileText className="h-3.5 w-3.5 text-[var(--text-tertiary)] shrink-0" />
                  <span className="truncate">{f.path.split(/[\\/]/).pop()}</span>
                </div>
              ))}
              {pending.length>4 && <p className="text-xs text-[var(--text-tertiary)]">+{pending.length-4} autres</p>}
            </div>
          </div>
        ) : null}
        <div className="mt-6 flex flex-wrap justify-center gap-2">
          <button type="button" onClick={handlePick} className="inline-flex items-center gap-2 rounded-full bg-[var(--accent)] px-7 py-3 text-sm font-semibold text-white shadow-md shadow-blue-500/20 hover:bg-[var(--accent-hover)] active:scale-[0.98]">
            <Upload className="h-4 w-4" />
            {t("selectFiles")}
          </button>
          <button type="button" onClick={handlePickFolder} className="inline-flex items-center gap-2 rounded-full border border-[var(--border)] bg-[var(--surface)] px-5 py-3 text-sm font-medium hover:bg-[var(--surface-hover)]">
            <FolderOpen className="h-4 w-4" />
            Dossier
          </button>
        </div>
        <div className="mt-4 flex flex-wrap items-center justify-center gap-2 text-xs">
          <span className="inline-flex items-center gap-1.5 rounded-full border border-[var(--border)] bg-[var(--surface-hover)] px-3 py-1 font-medium">
            <ShieldCheck className="h-3.5 w-3.5 text-[var(--success)]" /> Transfert local chiffré
          </span>
          <span className="text-[var(--text-tertiary)]">• Jusqu&apos;à 20 Go • Reprise BLAKE3</span>
        </div>
      </div>
    </div>
  );
}
