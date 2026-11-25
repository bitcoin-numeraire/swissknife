import type { IDatePickerControl } from './common';
import type { Invoice, Payment, TransactionWalletDisplay } from 'src/lib/swissknife';

export type ITransaction = (Payment | Invoice) & {
  transaction_type?: TransactionType;
  wallet_display?: TransactionWalletDisplay | null;
};

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
