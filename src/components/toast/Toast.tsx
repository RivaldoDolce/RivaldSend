import {
  createContext,
  useContext,
  useState,
  useCallback,
  type ReactNode,
} from "react";
import { CheckCircle, XCircle, AlertTriangle, Info, X } from "lucide-react";

type ToastType = "success" | "error" | "warning" | "info";

interface Toast {
  id: string;
  type: ToastType;
  title: string;
  message?: string;
  action?: { label: string; onClick: () => void };
  duration?: number;
}

interface ToastContextValue {
  show: (toast: Omit<Toast, "id">) => void;
  success: (title: string, message?: string) => void;
  error: (title: string, message?: string) => void;
  warning: (title: string, message?: string) => void;
  info: (title: string, message?: string) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

const ICONS: Record<ToastType, typeof CheckCircle> = {
  success: CheckCircle,
  error: XCircle,
  warning: AlertTriangle,
  info: Info,
};

const ICON_COLORS: Record<ToastType, string> = {
  success: "text-emerald-500",
  error: "text-red-500",
  warning: "text-amber-500",
  info: "text-blue-500",
};

function ToastContent({ toast }: { toast: Toast }) {
  const Icon = ICONS[toast.type];
  return (
    <div className="flex items-start gap-3">
      <Icon
        className={`h-5 w-5 shrink-0 mt-0.5 ${ICON_COLORS[toast.type]}`}
      />
      <div className="flex-1 min-w-0">
        <p className="text-sm font-semibold">{toast.title}</p>
        {toast.message && (
          <p className="mt-0.5 text-xs text-[var(--text-secondary)]">
            {toast.message}
          </p>
        )}
        {toast.action && (
          <button
            onClick={toast.action.onClick}
            className="mt-2 text-xs font-medium text-[var(--accent)] hover:underline"
          >
            {toast.action.label}
          </button>
        )}
      </div>
    </div>
  );
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const dismiss = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const show = useCallback(
    (toast: Omit<Toast, "id">) => {
      const id = crypto.randomUUID();
      setToasts((prev) => [...prev, { ...toast, id }]);
      setTimeout(() => dismiss(id), toast.duration ?? 4000);
    },
    [dismiss]
  );

  const success = useCallback(
    (title: string, message?: string) => show({ type: "success", title, message }),
    [show]
  );
  const error = useCallback(
    (title: string, message?: string) => show({ type: "error", title, message }),
    [show]
  );
  const warning = useCallback(
    (title: string, message?: string) => show({ type: "warning", title, message }),
    [show]
  );
  const info = useCallback(
    (title: string, message?: string) => show({ type: "info", title, message }),
    [show]
  );

  return (
    <ToastContext.Provider value={{ show, success, error, warning, info }}>
      {children}
      <div className="fixed bottom-4 right-4 z-50 space-y-2 max-w-sm">
        {toasts.map((t) => (
          <div
            key={t.id}
            className="animate-slide-in-right rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4 shadow-xl"
          >
            <div className="flex items-start gap-3">
              <ToastContent toast={t} />
              <button
                onClick={() => dismiss(t.id)}
                aria-label="Fermer la notification"
                className="shrink-0 rounded-full p-0.5 hover:bg-[var(--surface-hover)]"
              >
                <X className="h-3.5 w-3.5 text-[var(--text-tertiary)]" />
              </button>
            </div>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used within ToastProvider");
  return ctx;
}
