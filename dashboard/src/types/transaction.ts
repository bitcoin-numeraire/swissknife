import type { IDatePickerControl } from './common';
import type { Invoice, Payment } from 'src/lib/swissknife';

export type ITransaction =
  | (Payment & { transaction_type?: TransactionType })
  | (Invoice & { transaction_type?: TransactionType });

export enum TransactionType {
  INVOICE = 'INVOICE',
  PAYMENT = 'PAYMENT',
}

export type ITransactionTableFilters = {
  name: string;
  ledger: string[];
  status: string;
  startDate: IDatePickerControl;
  endDate: IDatePickerControl;
};
