import type { IdentitySecrets } from './keys';
import type { WalletIdentity } from 'src/lib/swissknife';

export type SealedIdentity = {
  version: 1;
  identity: WalletIdentity;
  salt: Uint8Array<ArrayBuffer>;
  nonce: Uint8Array<ArrayBuffer>;
  ciphertext: ArrayBuffer;
};

export type IdentityChange = {
  id: string;
  expected: WalletIdentity | null;
  sealed: SealedIdentity;
};

export type DeviceIdentity = {
  active?: SealedIdentity;
  pending?: IdentityChange;
};

export function sameIdentity(a?: WalletIdentity | null, b?: WalletIdentity | null) {
  return a?.recovery_pubkey === b?.recovery_pubkey && a?.nostr_pubkey === b?.nostr_pubkey;
}

async function encryptionKey(password: string, salt: Uint8Array<ArrayBuffer>) {
  const material = new TextEncoder().encode(password);
  try {
    const key = await crypto.subtle.importKey('raw', material, 'PBKDF2', false, ['deriveKey']);
    return await crypto.subtle.deriveKey(
      { name: 'PBKDF2', hash: 'SHA-256', iterations: 600_000, salt },
      key,
      { name: 'AES-GCM', length: 256 },
      false,
      ['encrypt', 'decrypt']
    );
  } finally {
    material.fill(0);
  }
}

function additionalData(scope: string, identity: WalletIdentity) {
  return new TextEncoder().encode(
    JSON.stringify([1, scope, identity.recovery_pubkey, identity.nostr_pubkey])
  );
}

export async function sealIdentity(
  scope: string,
  identity: WalletIdentity,
  secrets: IdentitySecrets,
  password: string
): Promise<SealedIdentity> {
  const salt = crypto.getRandomValues(new Uint8Array(16));
  const nonce = crypto.getRandomValues(new Uint8Array(12));
  const key = await encryptionKey(password, salt);
  const plain = new TextEncoder().encode(JSON.stringify(secrets));
  try {
    const ciphertext = await crypto.subtle.encrypt(
      { name: 'AES-GCM', iv: nonce, additionalData: additionalData(scope, identity) },
      key,
      plain
    );
    return { version: 1, identity, salt, nonce, ciphertext };
  } finally {
    plain.fill(0);
  }
}

export async function unlockIdentity(
  scope: string,
  sealed: SealedIdentity,
  password: string
): Promise<IdentitySecrets> {
  if (sealed.version !== 1) throw new Error('unsupported_vault');
  const key = await encryptionKey(password, sealed.salt);
  const plain = new Uint8Array(
    await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv: sealed.nonce, additionalData: additionalData(scope, sealed.identity) },
      key,
      sealed.ciphertext
    )
  );
  try {
    return JSON.parse(new TextDecoder().decode(plain)) as IdentitySecrets;
  } finally {
    plain.fill(0);
  }
}

export function deviceIdentity(
  scope: string,
  update?: (current: DeviceIdentity) => DeviceIdentity
): Promise<DeviceIdentity> {
  return new Promise((resolve, reject) => {
    const open = indexedDB.open('swissknife-wallet-identities', 1);
    open.onupgradeneeded = () => open.result.createObjectStore('wallets');
    open.onerror = () => reject(open.error);
    open.onsuccess = () => {
      const db = open.result;
      const transaction = db.transaction('wallets', update ? 'readwrite' : 'readonly');
      const store = transaction.objectStore('wallets');
      const request = store.get(scope);
      let result: DeviceIdentity;
      transaction.oncomplete = () => {
        db.close();
        resolve(result);
      };
      transaction.onabort = () => {
        db.close();
        reject(transaction.error ?? new Error('storage_failed'));
      };
      request.onsuccess = () => {
        result = request.result ?? {};
        if (!update) return;
        try {
          result = update(result);
          store.put(result, scope);
        } catch (error) {
          transaction.abort();
          reject(error);
        }
      };
    };
  });
}

// Persist the encrypted candidate before publishing either public key. Browser or API interruption
// leaves enough information to retry the exact change while keeping the previous device key.
export function stageIdentity(scope: string, change: IdentityChange) {
  return deviceIdentity(scope, (current) => {
    if (current.pending) throw new Error('pending_change');
    return { ...current, pending: change };
  });
}

export function finishIdentity(scope: string, changeId: string, accepted: boolean) {
  return deviceIdentity(scope, (current) => {
    if (current.pending?.id !== changeId) return current;
    return { active: accepted ? current.pending.sealed : current.active };
  });
}
