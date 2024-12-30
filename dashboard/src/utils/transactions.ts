import type { ITransaction } from 'src/types/transaction';
import type { InvoiceResponse, PaymentResponse } from 'src/lib/swissknife';

import { sumBy } from 'es-toolkit';

import { TransactionType } from 'src/types/transaction';

export const LEDGERS = ['Lightning', 'Internal', 'Onchain'];

export function mergeAndSortTransactions(
  invoices: InvoiceResponse[],
  payments: PaymentResponse[]
): ITransaction[] {
  const invoicesWithType = invoices.map((invoice) => ({
    ...invoice,
    transaction_type: TransactionType.INVOICE,
  }));

  const paymentsWithType = payments.map((payment) => ({
    ...payment,
    transaction_type: TransactionType.PAYMENT,
  }));

  const mergedTransactions: ITransaction[] = [...invoicesWithType, ...paymentsWithType];
  return mergedTransactions.sort((a, b) => b.created_at.getTime() - a.created_at.getTime());
}

export function getPercentageChange(transactions: ITransaction[]): number {
  const now = new Date();
  const currentMonth = now.getMonth();
  const currentYear = now.getFullYear();

  const lastMonth = new Date(currentYear, currentMonth - 1, 1);
  const nextMonth = new Date(currentYear, currentMonth + 1, 1);

  const previousMonthStart = new Date(currentYear, currentMonth - 2, 1);
  const previousMonthEnd = lastMonth;

  const currentMonthTransactions = transactions.filter(
    (tx) => tx.created_at >= lastMonth && tx.created_at < nextMonth && tx.status === 'Settled'
  );

  const previousMonthTransactions = transactions.filter(
    (tx) =>
      tx.created_at >= previousMonthStart &&
      tx.created_at < previousMonthEnd &&
      tx.status === 'Settled'
  );

  const currentMonthTotal = currentMonthTransactions.reduce(
    (sum, tx) => sum + (tx.amount_msat || 0),
    0
  );
  const previousMonthTotal = previousMonthTransactions.reduce(
    (sum, tx) => sum + (tx.amount_msat || 0),
    0
  );

  if (previousMonthTotal === 0) {
    return currentMonthTotal === 0 ? 0 : 100;
  }

  const percentageChange = ((currentMonthTotal - previousMonthTotal) / previousMonthTotal) * 100;
  return percentageChange;
}

export function getTotal(transactions: ITransaction[]): number {
  return sumBy(
    transactions.filter((tx) => tx.status === 'Settled'),
    (tx) => (tx.amount_msat || 0) + (tx.fee_msat || 0)
  );
}

export function getCumulativeSeries(transactions: ITransaction[]) {
  let cumulativeSum = 0;
  const seriesData = transactions
    .filter((tx) => tx.status === 'Settled')
    .sort((a, b) => a.payment_time!.getTime() - b.payment_time!.getTime())
    .map((tx) => {
      const amount = ((tx.amount_msat || 0) + (tx.fee_msat || 0)) / 1000; // Convert from msat to sat
      cumulativeSum += amount;
      return { x: tx.payment_time!, y: cumulativeSum };
    });

  return [
    {
      data: seriesData,
    },
  ];
}
