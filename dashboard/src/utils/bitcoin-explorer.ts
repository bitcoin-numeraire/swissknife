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

export function txidFromOutpoint(outpoint?: string | null) {
  if (!outpoint) return undefined;

  return outpoint.split(':')[0];
}
