export function requestId() {
  if (typeof crypto.randomUUID === 'function') return crypto.randomUUID();
  // Self-hosted dashboards on local HTTP still support getRandomValues.
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  bytes[6] = 64 + (bytes[6] % 16);
  bytes[8] = 128 + (bytes[8] % 64);
  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20),
  ].join('-');
}
