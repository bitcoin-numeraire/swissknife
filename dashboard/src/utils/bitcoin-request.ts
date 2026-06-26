const SATS_PER_BITCOIN = 100_000_000;

export type ParsedBitcoinUri = {
  address: string;
  amountInvalid: boolean;
  amountSats?: number;
  lightning?: string;
  message?: string;
  label?: string;
  isUri: boolean;
  requiredParams: string[];
  unknownParams: string[];
};

export function satsToBtcAmount(sats: number) {
  return (sats / SATS_PER_BITCOIN).toFixed(8).replace(/0+$/, '').replace(/\.$/, '');
}

function getParam(params: URLSearchParams, name: string) {
  const normalizedName = name.toLowerCase();

  for (const [key, value] of params.entries()) {
    if (key.toLowerCase() === normalizedName) return value;
  }

  return undefined;
}

function safeDecode(value: string) {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

function parseBtcAmountToSats(value?: string) {
  if (!value) return { amountSats: undefined, amountInvalid: false };

  const normalized = value.trim();

  if (!/^\d+(\.\d{1,8})?$/.test(normalized)) {
    return { amountSats: undefined, amountInvalid: true };
  }

  const [wholePart, decimalPart = ''] = normalized.split('.');
  const wholeSats = Number(wholePart) * SATS_PER_BITCOIN;
  const decimalSats = Number(decimalPart.padEnd(8, '0'));
  const amountSats = wholeSats + decimalSats;

  if (!Number.isSafeInteger(amountSats)) {
    return { amountSats: undefined, amountInvalid: true };
  }

  return {
    amountSats: amountSats > 0 ? amountSats : undefined,
    amountInvalid: false,
  };
}

export function parseBitcoinUri(input: string): ParsedBitcoinUri {
  const value = input.trim();

  if (!/^bitcoin:/i.test(value)) {
    return {
      address: value,
      amountInvalid: false,
      isUri: false,
      requiredParams: [],
      unknownParams: [],
    };
  }

  const rest = value.replace(/^bitcoin:/i, '').replace(/^\/\//, '');
  const queryIndex = rest.indexOf('?');
  const rawAddress = queryIndex >= 0 ? rest.slice(0, queryIndex) : rest;
  const query = queryIndex >= 0 ? rest.slice(queryIndex + 1) : '';
  const params = new URLSearchParams(query);
  const amountParam = getParam(params, 'amount');
  const { amountSats, amountInvalid } = parseBtcAmountToSats(amountParam);
  const knownParams = new Set(['amount', 'label', 'message', 'lightning']);
  const requiredParams: string[] = [];
  const unknownParams: string[] = [];

  for (const [key] of params.entries()) {
    const normalizedKey = key.toLowerCase();

    if (knownParams.has(normalizedKey)) continue;
    if (normalizedKey.startsWith('req-')) {
      requiredParams.push(key);
    } else {
      unknownParams.push(key);
    }
  }

  return {
    address: safeDecode(rawAddress),
    amountInvalid,
    isUri: true,
    lightning: getParam(params, 'lightning'),
    message: getParam(params, 'message'),
    label: getParam(params, 'label'),
    amountSats,
    requiredParams,
    unknownParams,
  };
}

export function composeBip21(
  address?: string | null,
  bolt11?: string | null,
  amountSats?: number | null
) {
  if (!address) return bolt11 ?? '';

  const params = new URLSearchParams();
  if (amountSats) params.set('amount', satsToBtcAmount(amountSats));
  if (bolt11) params.set('lightning', bolt11);

  const suffix = params.toString();
  return `bitcoin:${address}${suffix ? `?${suffix}` : ''}`;
}
