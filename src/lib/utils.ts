export function formatBytes(n: number): string {
  if (n < 1024) return `${n} o`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} Ko`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} Mo`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} Go`;
}

// Taille d'un chunk de transfert — doit rester égale au backend
// (rivaldsend-core pipeline::chunker::CHUNK_SIZE).
export const CHUNK_SIZE_BYTES = 4 * 1024 * 1024;
