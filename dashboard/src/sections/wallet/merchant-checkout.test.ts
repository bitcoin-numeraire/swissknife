import type { Invoice } from 'src/lib/swissknife';

import { it, expect } from 'vitest';

import { Ledger, InvoiceStatus } from 'src/lib/swissknife';

import { checkoutAmount, checkoutSettlement, restoreMerchantSale } from './merchant-checkout';

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
      receivedMsat: 27000,
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
    pendingMsat: 15000,
    receivedMsat: 0,
  });
  expect(checkoutSettlement(sale, [confirmed, pending], 1235)).toMatchObject({
    status: 'partial',
    pendingMsat: 0,
    receivedMsat: 15000,
  });
  expect(checkoutSettlement(sale, [confirmed, rest, confirmed], 1235)).toMatchObject({
    status: 'paid',
    receivedMsat: 27000,
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
