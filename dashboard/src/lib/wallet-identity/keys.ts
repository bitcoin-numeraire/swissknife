import { bech32 } from 'bech32';
import { HDKey } from '@scure/bip32';
import { hmac } from '@noble/hashes/hmac.js';
import { sha256 } from '@noble/hashes/sha2.js';
import { bytesToHex, hexToBytes } from '@noble/hashes/utils.js';
import { schnorr, secp256k1 } from '@noble/curves/secp256k1.js';
import { wordlist as english } from '@scure/bip39/wordlists/english.js';
import { mnemonicToSeed, generateMnemonic, validateMnemonic } from '@scure/bip39';

export type IdentitySecrets = {
  phrase: string;
  bip39Passphrase: string;
  nostrSecret?: string;
};

export function newIdentityPhrase() {
  return generateMnemonic(english, 256);
}

export async function importIdentityPhrase(value: string) {
  const phrase = value.normalize('NFKD').trim().split(/\s+/u).join(' ');
  if (validateMnemonic(phrase, english)) return phrase;

  // Load additional BIP39 wordlists only when importing a non-English phrase.
  for (const load of [
    () => import('@scure/bip39/wordlists/french.js'),
    () => import('@scure/bip39/wordlists/spanish.js'),
    () => import('@scure/bip39/wordlists/italian.js'),
    () => import('@scure/bip39/wordlists/portuguese.js'),
    () => import('@scure/bip39/wordlists/czech.js'),
    () => import('@scure/bip39/wordlists/japanese.js'),
    () => import('@scure/bip39/wordlists/korean.js'),
    () => import('@scure/bip39/wordlists/simplified-chinese.js'),
    () => import('@scure/bip39/wordlists/traditional-chinese.js'),
  ]) {
    if (validateMnemonic(phrase, (await load()).wordlist)) return phrase;
  }
  throw new Error('invalid_phrase');
}

export function parseNostrSecret(input: string): Uint8Array {
  const value = input.trim();
  let bytes;
  if (/^[\da-f]{64}$/i.test(value)) {
    bytes = hexToBytes(value);
  } else {
    const { prefix, words } = bech32.decode(value);
    if (prefix !== 'nsec') throw new Error('invalid_nsec');
    bytes = Uint8Array.from(bech32.fromWords(words));
  }
  if (!secp256k1.utils.isValidSecretKey(bytes)) throw new Error('invalid_nsec');
  return bytes;
}

async function rootKey(secrets: IdentitySecrets) {
  const seed = await mnemonicToSeed(secrets.phrase, secrets.bip39Passphrase);
  try {
    return HDKey.fromMasterSeed(seed);
  } finally {
    seed.fill(0);
  }
}

export async function nostrKeys(secrets: IdentitySecrets) {
  const root = await rootKey(secrets);
  const account = root.derive("m/44'/1237'/0'/0/0");
  const derived = account.privateKey!;
  const active = secrets.nostrSecret ? parseNostrSecret(secrets.nostrSecret) : derived;
  try {
    return {
      identity: {
        recovery_pubkey: bytesToHex(schnorr.getPublicKey(derived)),
        nostr_pubkey: bytesToHex(schnorr.getPublicKey(active)),
      },
      nsec: bech32.encode('nsec', bech32.toWords(active)),
    };
  } finally {
    active.fill(0);
    derived.fill(0);
    account.wipePrivateData();
    root.wipePrivateData();
  }
}

const actions = ['register', 'login', 'link', 'auth'] as const;
type LoginAction = (typeof actions)[number];

export function reviewLogin(input: string) {
  let raw = input.trim().replace(/^lightning:/i, '');
  if (raw.toLowerCase().startsWith('lnurl1')) {
    const decoded = bech32.decode(raw, 8192);
    if (decoded.prefix !== 'lnurl') throw new Error('invalid_login');
    raw = new TextDecoder('utf-8', { fatal: true }).decode(
      Uint8Array.from(bech32.fromWords(decoded.words))
    );
  }
  const url = new URL(raw);
  const params = url.searchParams;
  const k1 = params.get('k1') ?? '';
  const action = params.get('action') ?? 'login';
  if (
    url.protocol !== 'https:' ||
    url.username ||
    url.password ||
    url.hash ||
    params.getAll('tag').length !== 1 ||
    params.get('tag') !== 'login' ||
    params.getAll('k1').length !== 1 ||
    !/^[\da-f]{64}$/i.test(k1) ||
    params.getAll('action').length > 1 ||
    !actions.includes(action as LoginAction) ||
    params.has('key') ||
    params.has('sig')
  )
    throw new Error('invalid_login');
  return {
    url: url.href,
    domain: url.hostname.replace(/\.$/, ''),
    action: action as LoginAction,
    k1,
  };
}

export function linkingPath(hashingKey: Uint8Array, domain: string) {
  const digest = hmac(sha256, hashingKey, new TextEncoder().encode(domain));
  const parts = new DataView(digest.buffer, digest.byteOffset, 16);
  return [0, 4, 8, 12].map((offset) => parts.getUint32(offset, false));
}

export async function signLogin(secrets: IdentitySecrets, reviewedUrl: string) {
  const { url, domain, k1 } = reviewLogin(reviewedUrl);
  const root = await rootKey(secrets);
  const purpose = root.derive("m/138'");
  const hashing = purpose.deriveChild(0);
  let linking = purpose;
  try {
    for (const index of linkingPath(hashing.privateKey!, domain)) {
      const next = linking.deriveChild(index);
      if (linking !== purpose) linking.wipePrivateData();
      linking = next;
    }
    const signature = secp256k1.sign(hexToBytes(k1), linking.privateKey!, {
      prehash: false,
      format: 'der',
    });
    const callback = new URL(url);
    callback.searchParams.set('key', bytesToHex(linking.publicKey!));
    callback.searchParams.set('sig', bytesToHex(signature));
    return callback.href;
  } finally {
    linking.wipePrivateData();
    hashing.wipePrivateData();
    purpose.wipePrivateData();
    root.wipePrivateData();
  }
}
