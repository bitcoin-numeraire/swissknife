import { CONFIG } from 'src/global-config';

function explorerBaseUrl() {
  return CONFIG.mempoolSpace.replace(/\/api\/v1\/?$/, '');
}

export function bitcoinTransactionExplorerUrl(txid?: string | null) {
  if (!txid) return undefined;

  return `${explorerBaseUrl()}/tx/${txid}`;
}

export function bitcoinAddressExplorerUrl(address?: string | null) {
  if (!address) return undefined;

  return `${explorerBaseUrl()}/address/${address}`;
}

export function bitcoinOutpointExplorerUrl(outpoint?: string | null) {
  if (!outpoint) return undefined;

  const [txid, vout] = outpoint.split(':');
  if (!txid) return undefined;

  const fragment = vout ? `#vout=${vout}` : '';

  return `${bitcoinTransactionExplorerUrl(txid)}${fragment}`;
}

export function txidFromOutpoint(outpoint?: string | null) {
  if (!outpoint) return undefined;

  return outpoint.split(':')[0];
}
