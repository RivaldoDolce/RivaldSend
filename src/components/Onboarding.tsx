import { useState } from "react";
import { Package, Users, Lock, ChevronRight, ChevronLeft } from "lucide-react";
import { useSettingsStore } from "../stores/useSettingsStore";

const STEPS = [
  {
    icon: Package,
    title: "Transférez en un clin d'œil",
    description:
      "Déposez un fichier, choisissez un appareil, c'est envoyé. Moins de 3 secondes.",
    color: "#0A84FF",
  },
  {
    icon: Users,
    title: "Vos appareils, partout",
    description:
      "RivaldSend détecte automatiquement vos appareils sur le réseau local. Pas de configuration.",
    color: "#30D158",
  },
  {
    icon: Lock,
    title: "100% local, 100% privé",
    description:
      "Aucun serveur, aucun cloud. Vos fichiers restent entre vos appareils, chiffrés de bout en bout.",
    color: "#AF52DE",
  },
];

export function Onboarding({ onComplete }: { onComplete: () => void }) {
  const [step, setStep] = useState(0);
  const isLast = step === STEPS.length - 1;
  const current = STEPS[step]!;

  return (
    <div className="fixed inset-0 z-50 bg-[var(--background)] flex flex-col items-center justify-center p-8">
      <div className="text-center max-w-md">
        <div
          className="w-24 h-24 mx-auto mb-8 rounded-[28px] flex items-center justify-center"
          style={{ backgroundColor: `${current.color}15` }}
        >
          <current.icon
            className="h-12 w-12"
            style={{ color: current.color }}
            strokeWidth={1.5}
          />
        </div>
        <h1 className="text-3xl font-bold tracking-tight">
          {current.title}
        </h1>
        <p className="mt-4 text-[var(--text-secondary)] leading-relaxed">
          {current.description}
        </p>
      </div>

      <div className="mt-12 flex gap-2">
        {STEPS.map((_, i) => (
          <div
            key={i}
            className={`h-2 rounded-full transition-all duration-300 ${
              i === step
                ? "w-8 bg-[var(--accent)]"
                : "w-2 bg-[var(--border)]"
            }`}
          />
        ))}
      </div>

      <div className="mt-12 flex gap-3">
        {step > 0 && (
          <button
            onClick={() => setStep(step - 1)}
            className="inline-flex items-center gap-2 rounded-full border border-[var(--border)] px-6 py-3 text-sm font-medium hover:bg-[var(--surface-hover)]"
          >
            <ChevronLeft className="h-4 w-4" /> Retour
          </button>
        )}
        <button
          onClick={() => (isLast ? onComplete() : setStep(step + 1))}
          className="inline-flex items-center gap-2 rounded-full bg-[var(--accent)] px-8 py-3 text-sm font-semibold text-white shadow-md hover:bg-[var(--accent-hover)]"
        >
          {isLast ? "Commencer" : "Suivant"}{" "}
          {!isLast && <ChevronRight className="h-4 w-4" />}
        </button>
      </div>
    </div>
  );
}
