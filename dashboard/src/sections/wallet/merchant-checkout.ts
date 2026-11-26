import type { Invoice } from 'src/lib/swissknife';

import { z } from 'zod';

import { composeBip21 } from 'src/utils/bitcoin-request';

import { Ledger, InvoiceStatus } from 'src/lib/swissknife';

const saleSchema = z
  .object({
    walletId: z.string().min(1),
    amountSats: z
      .number()
      .int()
      .positive()
      .max(Math.floor(Number.MAX_SAFE_INTEGER / 1000)),
    description: z.string(),
    invoiceId: z.string().optional(),
    bolt11: z.string().optional(),
    address: z.string().optional(),
    expiresAt: z.number().finite(),
  })
  .refine((sale) => Boolean(sale.address || (sale.invoiceId && sale.bolt11)));

export type MerchantSale = z.infer<typeof saleSchema>;

export function restoreMerchantSale(value: string | null, walletId: string) {
  try {
    const result = saleSchema.safeParse(JSON.parse(value ?? 'null'));
    if (result.success && result.data.walletId === walletId) return result.data;
  } catch {
    // A malformed or obsolete stored sale can be replaced with a new one.
  }
  return undefined;
}

export function restoreCheckoutReceipts(value: string | null): string[] {
  if (!value) return [];
  // Earlier checkouts stored the first receipt UUID as plain text.
  if (z.uuid().safeParse(value).success) return [value];
  try {
    const result = z.array(z.uuid()).safeParse(JSON.parse(value));
    return result.success ? [...new Set(result.data)] : [];
  } catch {
    return [];
  }
}

export function checkoutAmount(input: string) {
  if (!/^\d+$/.test(input)) return undefined;
  const sats = Number(input);
  return sats > 0 && Number.isSafeInteger(sats * 1000) ? sats : undefined;
}

export function checkoutSettlement(sale: MerchantSale, invoices: Invoice[], now: number) {
  const confirmed = new Map<string, number>();
  const pending = new Map<string, number>();
  for (const invoice of invoices) {
    if (invoice.wallet_id !== sale.walletId) continue;
    const matchesAddress = Boolean(
      sale.address &&
      ((invoice.ledger === Ledger.ONCHAIN && invoice.bitcoin_output?.address === sale.address) ||
        (invoice.ledger === Ledger.INTERNAL && invoice.internal_btc_address === sale.address))
    );
    if (!matchesAddress && invoice.id !== sale.invoiceId) continue;
    const amount = invoice.amount_received_msat ?? invoice.amount_msat ?? 0;
    if (amount <= 0 || !Number.isSafeInteger(amount)) continue;
    if (invoice.status === InvoiceStatus.SETTLED) confirmed.set(invoice.id, amount);
    if (matchesAddress && invoice.status === InvoiceStatus.PENDING) pending.set(invoice.id, amount);
  }
  let receivedMsat = 0;
  let pendingMsat = 0;
  for (const [id, amount] of confirmed) {
    pending.delete(id);
    receivedMsat += amount;
  }
  for (const amount of pending.values()) pendingMsat += amount;
  const status =
    receivedMsat >= sale.amountSats * 1000
      ? 'paid'
      : pendingMsat > 0
        ? 'incoming'
        : receivedMsat > 0
          ? 'partial'
          : now >= sale.expiresAt
            ? 'expired'
            : 'waiting';
  return { status, receivedMsat, pendingMsat, invoiceIds: Array.from(confirmed.keys()) };
}

export function checkoutPaymentRequest(
  sale: MerchantSale,
  settlement: ReturnType<typeof checkoutSettlement>,
  now: number
) {
  if (now >= sale.expiresAt) return undefined;
  if (settlement.status === 'waiting') {
    return {
      uri: composeBip21(sale.address, sale.bolt11, sale.amountSats),
      method:
        sale.address && sale.bolt11
          ? 'pos.unified'
          : sale.address
            ? 'pos.onchain'
            : 'pos.lightning',
      amountSats: sale.amountSats,
    };
  }
  if (settlement.status !== 'partial' || !sale.address) return undefined;
  const remainingSats = (sale.amountSats * 1000 - settlement.receivedMsat) / 1000;
  if (!Number.isSafeInteger(remainingSats) || remainingSats <= 0) return undefined;
  return {
    uri: composeBip21(sale.address, undefined, remainingSats),
    method: 'pos.onchain',
    amountSats: remainingSats,
  };
}
