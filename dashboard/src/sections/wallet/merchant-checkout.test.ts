import type { Invoice } from 'src/lib/swissknife';

import { it, expect } from 'vitest';

import { Ledger, InvoiceStatus } from 'src/lib/swissknife';

import {
  checkoutAmount,
  formatCheckoutSats,
  checkoutSettlement,
  restoreMerchantSale,
  checkoutPaymentRequest,
  restoreCheckoutReceipts,
} from './merchant-checkout';

const sale = {
  walletId: 'merchant',
  amountSats: 27,
  description: 'order',
  invoiceId: 'ln',
  bolt11: 'lnbcrt',
  address: 'bcrt1sale',
  expiresAt: 1234,
};
function invoice(
  id: string,
  ledger: Ledger,
  status: InvoiceStatus,
  msat: number,
  address?: string
) {
  return {
    id,
    wallet_id: sale.walletId,
    ledger,
    status,
    amount_msat: msat,
    bitcoin_output: address ? { address } : null,
  } as Invoice;
}
it.each([Ledger.INTERNAL, Ledger.LIGHTNING])(
  'recognizes %s settlement and ignores replays',
  (ledger) => {
    const paid = invoice('ln', ledger, InvoiceStatus.SETTLED, 27000);
    expect(checkoutSettlement(sale, [paid, paid], 0)).toMatchObject({
      status: 'paid',
      receivedMsat: BigInt(27000),
      invoiceIds: ['ln'],
    });
  }
);
it('tracks pending outputs and combines distinct settled outputs only once', () => {
  const pending = invoice('deposit', Ledger.ONCHAIN, InvoiceStatus.PENDING, 15000, sale.address);
  const confirmed = { ...pending, status: InvoiceStatus.SETTLED };
  const rest = { ...confirmed, id: 'second', amount_msat: 12000 };
  expect(checkoutSettlement(sale, [pending], 1235)).toMatchObject({
    status: 'incoming',
    pendingMsat: BigInt(15000),
    receivedMsat: BigInt(0),
  });
  expect(checkoutSettlement(sale, [confirmed, pending], 1235)).toMatchObject({
    status: 'partial',
    pendingMsat: BigInt(0),
    receivedMsat: BigInt(15000),
  });
  expect(checkoutSettlement(sale, [confirmed, rest, confirmed], 1235)).toMatchObject({
    status: 'paid',
    receivedMsat: BigInt(27000),
  });
});
it('isolates cancelled, subsequent and other wallets’ sales on replay', () => {
  const previous = invoice('old-ln', Ledger.ONCHAIN, InvoiceStatus.SETTLED, 27000, 'old-address');
  const foreign = {
    ...invoice('ln', Ledger.INTERNAL, InvoiceStatus.SETTLED, 27000),
    wallet_id: 'other',
  };
  expect(checkoutSettlement(sale, [previous, foreign], 0).status).toBe('waiting');
  expect(checkoutSettlement(sale, [previous, foreign], sale.expiresAt).status).toBe('expired');
});
it('combines an internal Bitcoin receipt with a chain deposit at the same sale address', () => {
  const local = {
    ...invoice('internal', Ledger.INTERNAL, InvoiceStatus.SETTLED, 12000),
    internal_btc_address: sale.address,
  };
  const other = { ...local, id: 'other', internal_btc_address: 'bcrt1different' };
  const foreign = { ...local, id: 'foreign', wallet_id: 'another wallet' };
  const deposit = invoice('deposit', Ledger.ONCHAIN, InvoiceStatus.SETTLED, 15000, sale.address);
  expect(checkoutSettlement(sale, [other, foreign, local, local], 0)).toMatchObject({
    status: 'partial',
    receivedMsat: BigInt(12000),
  });
  expect(checkoutSettlement(sale, [deposit, local, local], 0)).toMatchObject({
    status: 'paid',
    receivedMsat: BigInt(27000),
    invoiceIds: ['deposit', 'internal'],
  });
});
it('restores only a valid sale for the active wallet', () => {
  expect(restoreMerchantSale(JSON.stringify(sale), sale.walletId)).toEqual(sale);
  expect(restoreMerchantSale(JSON.stringify(sale), 'different')).toBeUndefined();
  expect(restoreMerchantSale('oops', sale.walletId)).toBeUndefined();
  expect(
    restoreMerchantSale(JSON.stringify({ ...sale, address: null, bolt11: null }), sale.walletId)
  ).toBeUndefined();
});
it('requires whole positive sats representable in millisatoshis', () => {
  expect(checkoutAmount('123')).toBe(123);
  for (const invalid of ['0', '2.5', '-3', '', '1e3', '9007199254741'])
    expect(checkoutAmount(invalid)).toBeUndefined();
});

it('offers only the unpaid whole-sat balance after a confirmed deposit', () => {
  const pending = invoice('part', Ledger.ONCHAIN, InvoiceStatus.PENDING, 15000, sale.address);
  const confirmed = { ...pending, status: InvoiceStatus.SETTLED };
  const request = (invoices: Invoice[], now = 1) =>
    checkoutPaymentRequest(sale, checkoutSettlement(sale, invoices, now), now);

  expect(request([])?.uri).toBe('bitcoin:bcrt1sale?amount=0.00000027&lightning=lnbcrt');
  expect(request([pending])).toBeUndefined();
  expect(request([confirmed, pending, confirmed])).toEqual({
    uri: 'bitcoin:bcrt1sale?amount=0.00000012',
    method: 'pos.onchain',
    amountSats: 12,
  });
  const rest = { ...pending, id: 'rest', amount_msat: 12000 };
  expect(request([confirmed, rest])).toBeUndefined();
  expect(request([confirmed, { ...rest, status: InvoiceStatus.SETTLED }])).toBeUndefined();
  expect(request([confirmed], sale.expiresAt)).toBeUndefined();
  expect(request([{ ...confirmed, amount_msat: 15001 }])).toBeUndefined();
});

it('retains the original rail for an unpaid single-method sale', () => {
  const onchain = { ...sale, bolt11: undefined, invoiceId: undefined };
  const lightning = { ...sale, address: undefined };
  expect(checkoutPaymentRequest(onchain, checkoutSettlement(onchain, [], 1), 1)).toEqual({
    uri: 'bitcoin:bcrt1sale?amount=0.00000027',
    method: 'pos.onchain',
    amountSats: 27,
  });
  expect(checkoutPaymentRequest(lightning, checkoutSettlement(lightning, [], 1), 1)).toEqual({
    uri: 'lnbcrt',
    method: 'pos.lightning',
    amountSats: 27,
  });
});

it('restores split receipts and previous single-receipt sessions', () => {
  const a = '00000000-0000-4000-8000-000000000001';
  const b = '00000000-0000-4000-8000-000000000002';
  expect(restoreCheckoutReceipts(a)).toEqual([a]);
  expect(restoreCheckoutReceipts(JSON.stringify([a, b, a]))).toEqual([a, b]);
  for (const invalid of [null, '', '{', '[]', '["invalid"]', '42']) {
    expect(restoreCheckoutReceipts(invalid)).toEqual([]);
  }
});

it('reports only confirmed excess, including a fraction of a sat, across replay', () => {
  const bill = invoice('ln', Ledger.LIGHTNING, InvoiceStatus.SETTLED, 27000);
  const deposit = invoice('extra', Ledger.ONCHAIN, InvoiceStatus.PENDING, 1000, sale.address);
  const settled = { ...deposit, status: InvoiceStatus.SETTLED };
  expect(checkoutSettlement(sale, [bill, deposit], 0).excessMsat).toBe(BigInt(0));
  const extra = checkoutSettlement(
    sale,
    [{ ...bill, amount_received_msat: 27001 }, settled, settled],
    0
  );
  expect(extra.excessMsat).toBe(BigInt(1001));
  expect(formatCheckoutSats(extra.excessMsat, 'fr-FR')).toBe('1,001');
});

it('keeps the exact excess when individually safe receipts sum above the JavaScript integer limit', () => {
  const bill = invoice('ln', Ledger.LIGHTNING, InvoiceStatus.SETTLED, Number.MAX_SAFE_INTEGER);
  const deposit = invoice(
    'extra',
    Ledger.ONCHAIN,
    InvoiceStatus.SETTLED,
    Number.MAX_SAFE_INTEGER,
    sale.address
  );
  const result = checkoutSettlement(sale, [bill, deposit], 0);
  expect(formatCheckoutSats(result.excessMsat, 'en-US')).toBe('18,014,398,509,454.982');
});
