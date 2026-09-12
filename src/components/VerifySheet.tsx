import { useState } from "react";
import { ShieldCheck, X } from "lucide-react";
import type { Peer } from "../types";

const EMOJIS = ["🔵","🟢","🟡","🟣","🔴","🟠","⚪","🟤","🔷","🔶","🟩","🟦","⭐","🌟","🔸","🔹"];

function fpEmojis(fp: string): string {
  let s = "";
  for (let i = 0; i < 4; i++) s += EMOJIS[(fp.charCodeAt(i) || 0) % EMOJIS.length] ?? "🔵";
  return s;
}

export function VerifySheet({ peer, onApprove, onReject }: { peer: Peer; onApprove: (code: string) => void; onReject: () => void }) {
  const [code, setCode] = useState("");
  const [erreur, setErreur] = useState<string | null>(null);

  const valider = () => {
    const normalise = code.trim().replace("-", "").toUpperCase();
    if (!/^[A-HJ-NP-Z2-9]{6}$/.test(normalise) && !/^[0-9]{6}$/.test(normalise)) {
      setErreur("Code à 6 caractères requis (affiché sur l'autre appareil).");
      return;
    }
    setErreur(null);
    onApprove(normalise);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4" onClick={onReject}>
      <div className="max-h-[calc(100dvh-2rem)] w-full max-w-sm overflow-y-auto rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-6 shadow-xl" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-bold flex items-center gap-2"><ShieldCheck className="h-4 w-4 text-emerald-600 dark:text-emerald-400" /> Vérifier l'appareil</h3>
          <button onClick={onReject} className="rounded-full p-1 hover:bg-[var(--surface-hover)]"><X className="h-4 w-4" /></button>
        </div>
        <p className="mt-2 text-xs text-[var(--text-secondary)]">Comparez l'empreinte affichée sur l'autre appareil.</p>
        <div className="mt-4 rounded-xl border border-[var(--border)] bg-[var(--surface-hover)] p-4 text-center">
          <p className="text-sm font-semibold truncate">{peer.name}</p>
          <p className="mono mt-1 text-xs text-[var(--text-secondary)]">{peer.ip}:{peer.port}</p>
          <p className="mono mt-3 text-lg tracking-widest font-bold">{peer.fingerprintShort}</p>
          <p className="mt-2 text-2xl">{fpEmojis(peer.fingerprintShort)}</p>
          <p className="mono mt-2 text-xs text-[var(--text-tertiary)]">{peer.fingerprint.slice(0,16) || peer.fingerprintShort}</p>
        </div>
        <div className="mt-4">
          <label className="text-xs font-medium text-[var(--text-secondary)]">Code d'appairage affiché sur cet appareil</label>
          <input
            value={code}
            onChange={(e) => setCode(e.target.value)}
            placeholder=" ex : AB3-9XZ"
            maxLength={7}
            className="mono mt-1 w-full rounded-lg border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm uppercase tracking-widest"
          />
          {erreur && <p className="mt-1 text-xs text-red-500">{erreur}</p>}
        </div>
        <div className="mt-4 flex gap-2">
          <button onClick={onReject} className="flex-1 rounded-full border border-[var(--border)] px-4 py-2.5 text-sm font-medium hover:bg-[var(--surface-hover)]">Rejeter</button>
          <button onClick={valider} className="flex-1 rounded-full bg-emerald-600 px-4 py-2.5 text-sm font-semibold text-white hover:bg-emerald-700">Approuver</button>
        </div>
      </div>
    </div>
  );
}
