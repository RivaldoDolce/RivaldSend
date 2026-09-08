import { useState, useEffect, useCallback } from 'react';
import { Copy, RefreshCw, Check } from 'lucide-react';
import { useToast } from './toast/Toast';
import { usePeersStore } from '../stores/usePeersStore';
import { getDeviceInfo, generatePairingQr } from '../lib/tauri-bridge';

interface DeviceInfo {
  name: string;
  ip: string;
  port: number;
  fingerprint: string;
  fingerprintShort: string;
}

function generatePin(): string {
  const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'; // sans 0/O/1/I
  let pin = '';
  for (let i = 0; i < 6; i++) {
    pin += chars[Math.floor(Math.random() * chars.length)];
  }
  return `${pin.slice(0, 3)}-${pin.slice(3)}`;
}

export function PairingView() {
  const toast = useToast();
  const [deviceInfo, setDeviceInfo] = useState<DeviceInfo | null>(null);
  const [pin, setPin] = useState(generatePin);
  const [qrSvg, setQrSvg] = useState<string>('');
  const [copied, setCopied] = useState(false);
  const [secondsLeft, setSecondsLeft] = useState(300); // 5 minutes

  const peers = usePeersStore((s) => s.peers);

  // Charger les infos de l'appareil
  useEffect(() => {
    getDeviceInfo().then(setDeviceInfo).catch(console.error);
  }, []);

  // Générer le QR à chaque changement de PIN ou device
  useEffect(() => {
    if (!deviceInfo) return;
    const code = pin.replace('-', '');
    const fingerprintShort = deviceInfo.fingerprintShort ?? '';
    generatePairingQr({ ip: deviceInfo.ip, port: deviceInfo.port, code, fingerprintShort })
      .then(setQrSvg)
      .catch(async () => {
        // Repli hors Tauri (prévisualisation web) : QR généré côté JS
        try {
          const QRCode = await import('qrcode');
          const payload = `rivaldsend://${deviceInfo.ip}:${deviceInfo.port}?code=${code}&fp=${fingerprintShort}`;
          setQrSvg(await QRCode.toString(payload, { type: 'svg', margin: 1, width: 200, color: { dark: '#000000', light: '#ffffff' } }));
        } catch (err) {
          console.error(err);
        }
      });
  }, [deviceInfo, pin]);

  // Countdown
  useEffect(() => {
    if (secondsLeft <= 0) {
      // Régénérer PIN et QR
      setPin(generatePin());
      setSecondsLeft(300);
      toast.success('Nouveau code généré', 'Le précédent a expiré');
      return;
    }
    const t = setTimeout(() => setSecondsLeft((s) => s - 1), 1000);
    return () => clearTimeout(t);
  }, [secondsLeft, toast]);

  const handleCopyLink = useCallback(async () => {
    if (!deviceInfo) return;
    const link = `rivaldsend://${deviceInfo.ip}:${deviceInfo.port}?code=${pin.replace('-', '')}&fp=${deviceInfo.fingerprintShort ?? ''}`;
    try {
      await navigator.clipboard.writeText(link);
      setCopied(true);
      toast.success('Lien copié', 'Partagez-le avec l\'autre appareil');
      setTimeout(() => setCopied(false), 2000);
    } catch {
      toast.error('Erreur', 'Impossible de copier le lien');
    }
  }, [deviceInfo, pin, toast]);

  const handleRefresh = useCallback(() => {
    setPin(generatePin());
    setSecondsLeft(300);
    toast.success('Code régénéré', 'Un nouveau code est disponible');
  }, [toast]);

  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  };

  // Rendu via data-URI plutôt que dangerouslySetInnerHTML (pas d'exécution de SVG)
  const qrSrc = qrSvg ? `data:image/svg+xml;utf8,${encodeURIComponent(qrSvg)}` : '';

  return (
    <div className="max-w-3xl mx-auto space-y-6">
      <div className="text-center">
        <h2 className="text-xl font-bold">Appairage sécurisé</h2>
        <p className="text-sm text-[var(--text-secondary)] mt-1">
          Scannez le QR code depuis l'autre appareil pour vous connecter
        </p>
      </div>

      <div className="grid md:grid-cols-2 gap-6">
        {/* QR Code */}
        <div className="card-premium p-6 text-center">
          <div className="inline-block p-4 bg-white rounded-2xl">
            {qrSrc ? (
              <img src={qrSrc} alt="QR code d'appairage" className="h-48 w-48" />
            ) : (
              <div className="flex h-48 w-48 items-center justify-center text-xs text-gray-500">
                Génération…
              </div>
            )}
          </div>
          <p className="mono text-2xl font-bold mt-4 tracking-widest text-[var(--text-primary)]">
            {pin}
          </p>
          <p className="text-xs text-[var(--text-tertiary)] mt-2">
            Expire dans <span className="font-semibold mono">{formatTime(secondsLeft)}</span>
          </p>
          <div className="flex gap-2 mt-4 justify-center">
            <button
              onClick={handleRefresh}
              className="inline-flex items-center gap-2 rounded-full border border-[var(--border)] px-4 py-2 text-xs font-medium hover:bg-[var(--surface-hover)]"
            >
              <RefreshCw className="h-3 w-3" /> Régénérer
            </button>
            <button
              onClick={handleCopyLink}
              className="inline-flex items-center gap-2 rounded-full bg-[var(--accent)] px-4 py-2 text-xs font-semibold text-white hover:bg-[var(--accent-hover)]"
            >
              {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
              {copied ? 'Copié' : 'Copier le lien'}
            </button>
          </div>
        </div>

        {/* Instructions */}
        <div className="card-premium p-6 space-y-4">
          <h3 className="font-semibold">Comment faire ?</h3>
          <ol className="space-y-3 text-sm">
            <li className="flex gap-3">
              <span className="shrink-0 flex h-6 w-6 items-center justify-center rounded-full bg-[var(--accent)] text-white text-xs font-bold">1</span>
              <span>Ouvrez RivaldSend sur l'autre appareil</span>
            </li>
            <li className="flex gap-3">
              <span className="shrink-0 flex h-6 w-6 items-center justify-center rounded-full bg-[var(--accent)] text-white text-xs font-bold">2</span>
              <span>Allez dans <strong>Découverte</strong> → <strong>Scanner QR</strong></span>
            </li>
            <li className="flex gap-3">
              <span className="shrink-0 flex h-6 w-6 items-center justify-center rounded-full bg-[var(--accent)] text-white text-xs font-bold">3</span>
              <span>Scannez le QR code ci-contre</span>
            </li>
            <li className="flex gap-3">
              <span className="shrink-0 flex h-6 w-6 items-center justify-center rounded-full bg-[var(--accent)] text-white text-xs font-bold">4</span>
              <span>Confirmez le code <strong className="mono">{pin}</strong> sur les deux appareils</span>
            </li>
          </ol>

          {peers.length > 0 && (
            <div className="mt-4 pt-4 border-t border-[var(--border)]">
              <p className="text-xs font-semibold text-[var(--text-secondary)] mb-2">
                Appareils appairés ({peers.filter(p => p.trusted).length})
              </p>
              <div className="space-y-1">
                {peers.filter(p => p.trusted).slice(0, 3).map(p => (
                  <div key={p.id} className="text-xs flex items-center gap-2">
                    <span className="h-2 w-2 rounded-full bg-emerald-500" />
                    <span className="truncate">{p.name}</span>
                    <span className="mono text-[var(--text-tertiary)]">{p.ip}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
