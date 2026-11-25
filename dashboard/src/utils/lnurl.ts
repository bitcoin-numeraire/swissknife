import { bech32 } from 'bech32';

export function encodeLNURL(username?: string, baseUrl?: string | null): string {
  if (!username || !baseUrl) return '';
  const discovery = baseUrl + encodeURIComponent(username);
  return bech32
    .encode('lnurl', bech32.toWords(new TextEncoder().encode(discovery)), 2000)
    .toUpperCase();
}

export function displayLnAddress(username: string, domain?: string | null): string {
  return domain ? username + '@' + domain : '';
}
