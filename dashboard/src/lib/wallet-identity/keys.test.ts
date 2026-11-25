import { bech32 } from 'bech32';
import { it, expect, describe } from 'vitest';
import { hexToBytes } from '@noble/hashes/utils.js';
import { secp256k1 } from '@noble/curves/secp256k1.js';

import { sealIdentity, unlockIdentity } from './vault';
import { nostrKeys, signLogin, linkingPath, reviewLogin, importIdentityPhrase } from './keys';

const phrase = 'leader monkey parrot ring guide accident before fence cannon height naive bean';
const secrets = { phrase, bip39Passphrase: '' };
const nonce = 'a1'.repeat(32);

describe('device wallet identity', () => {
  it('derives the NIP-06 vector and changes only the selected Nostr key', async () => {
    expect(await importIdentityPhrase(phrase)).toBe(phrase);
    await expect(importIdentityPhrase(phrase.replace('bean', 'monkey'))).rejects.toThrow();
    const original = await nostrKeys(secrets);
    expect(original.identity.nostr_pubkey).toBe(
      '17162c921dc4d2518f9a101db33695df1afb56ab82f5ff3e5da6eec3ca5cd917'
    );
    expect(original.nsec).toBe('nsec10allq0gjx7fddtzef0ax00mdps9t2kmtrldkyjfs8l5xruwvh2dq0lhhkp');
    const updated = await nostrKeys({ ...secrets, nostrSecret: '01'.padStart(64, '0') });
    expect(updated.identity.recovery_pubkey).toBe(original.identity.recovery_pubkey);
    expect(updated.identity.nostr_pubkey).not.toBe(original.identity.nostr_pubkey);
  });

  it('matches the LUD-05 path and signs the original 32 bytes for the reviewed domain', async () => {
    expect(
      linkingPath(
        hexToBytes('7d417a6a5e9a6a4a879aeaba11a11838764c8fa2b959c242d43dea682b3e409b'),
        'site.com'
      )
    ).toEqual([1588488367, 2659270754, 38110259, 4136336762]);
    const link = `https://shop.example/login?tag=login&action=link&k1=${nonce}`;
    const encoded = bech32.encode('lnurl', bech32.toWords(new TextEncoder().encode(link)), 8192);
    const review = reviewLogin(encoded.toUpperCase());
    expect(review).toMatchObject({ domain: 'shop.example', action: 'link', url: link });
    const signed = new URL(await signLogin(secrets, review.url)).searchParams;
    expect(
      secp256k1.verify(
        hexToBytes(signed.get('sig')!),
        hexToBytes(nonce),
        hexToBytes(signed.get('key')!),
        { prehash: false, format: 'der' }
      )
    ).toBe(true);
    const other = new URL(await signLogin(secrets, link.replace('shop.example', 'other.example')));
    expect(other.searchParams.get('key')).not.toBe(signed.get('key'));
    for (const bad of [
      link + '&sig=a',
      link + '&action=login',
      link + '&k1=' + nonce,
      link.replace('https:', 'http:'),
      link.replace('shop.', 'a:b@shop.'),
      link.replace('action=link', 'action=pay'),
    ]) {
      expect(() => reviewLogin(bad)).toThrow();
    }
  });

  it('authenticates the wallet, public keys and encrypted content on unlock', async () => {
    const { identity } = await nostrKeys(secrets);
    const password = 'test unlock password';
    const sealed = await sealIdentity('wallet-a', identity, secrets, password);
    expect(await unlockIdentity('wallet-a', sealed, password)).toEqual(secrets);
    await expect(unlockIdentity('wallet-b', sealed, password)).rejects.toThrow();
    await expect(unlockIdentity('wallet-a', sealed, 'incorrect')).rejects.toThrow();
    const changed = { ...sealed, identity: { ...identity, nostr_pubkey: '00'.repeat(32) } };
    await expect(unlockIdentity('wallet-a', changed, password)).rejects.toThrow();
  });
});
