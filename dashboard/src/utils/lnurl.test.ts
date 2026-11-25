import { bech32 } from 'bech32';
import { it, expect, describe } from 'vitest';

import { encodeLNURL, displayLnAddress } from './lnurl';

describe('public receiving identity', () => {
  it('encodes the actual discovery endpoint on a separately configured host', () => {
    const host = 'x'.repeat(56) + '.onion';
    const base = 'http://' + host + '/.well-known/lnurlp/';
    const encoded = encodeLNURL('user+tag', base);
    const decoded = bech32.decode(encoded, 2000);
    expect(decoded.prefix).toBe('lnurl');
    expect(new TextDecoder().decode(Uint8Array.from(bech32.fromWords(decoded.words)))).toBe(
      base + 'user%2Btag'
    );
    expect(displayLnAddress('user', 'wallet.example')).toBe('user@wallet.example');
  });

  it('omits unconfigured domains and discovery endpoints', () => {
    expect(displayLnAddress('user', null)).toBe('');
    expect(encodeLNURL('user', null)).toBe('');
    expect(encodeLNURL(undefined, 'https://wallet.example/.well-known/lnurlp/')).toBe('');
  });
});
