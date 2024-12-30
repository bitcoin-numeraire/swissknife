import type { InvoiceResponse, PaymentResponse } from 'src/lib/swissknife';

import type { IDatePickerControl } from './common';

export type ITransaction =
  | (PaymentResponse & { transaction_type?: TransactionType })
  | (InvoiceResponse & { transaction_type?: TransactionType });

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
