import { bech32 } from 'bech32';

import { CONFIG } from 'src/global-config';

export function encodeLNURL(username?: string): string {
  if (!username) {
    return '';
  }

  const words = bech32.toWords(Buffer.from(`https://${CONFIG.domain}/lnurlp/${username}`, 'utf8'));

  return bech32.encode('lnurl', words).toUpperCase();
}

export function displayLnAddress(username: string): string {
  return `${username}@${CONFIG.domain}`;
}
