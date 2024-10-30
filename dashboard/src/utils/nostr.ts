import { bech32 } from 'bech32';

export function npub(hex?: string | null): string {
  if (!hex) {
    return '';
  }

  const words = bech32.toWords(Buffer.from(hex, 'hex'));

  return bech32.encode('npub', words);
}
