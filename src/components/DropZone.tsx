import { useState, useCallback } from "react";
import { Upload, File } from "lucide-react";
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
      className={`rivaldsend-dropzone flex flex-col items-center justify-center rounded-2xl p-10 text-center cursor-pointer ${dragging ? "rivaldsend-dropzone-active" : "bg-[var(--surface)] border-2 border-dashed"}`}
    >
      <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-[var(--accent-light)] text-[var(--accent)]">
        {dragging ? <File className="h-8 w-8" /> : <Upload className="h-8 w-8" />}
      </div>
      <h3 className="mt-4 text-xl font-semibold text-[var(--text-primary)]">{t("dropTitle")}</h3>
      <p className="mt-1 text-sm text-[var(--text-secondary)]">{t("dropSubtitle")}</p>
      <label className="mt-6 cursor-pointer rounded-full bg-[var(--accent)] px-6 py-2.5 text-sm font-medium text-white hover:bg-[var(--accent-hover)] transition-colors">
        {t("selectFiles")}
        <input type="file" multiple className="hidden" onChange={handleInput} />
      </label>
      <p className="mt-3 text-xs text-[var(--text-secondary)]">Glissez-déposez ou partage via le menu — aucun octet via l&apos;IPC</p>
    </div>
  );
}
