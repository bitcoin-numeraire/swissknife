import type { IDatePickerControl } from './common';

export type IWalletTableFilterValue = string | string[] | Date | null;
export type IWalletTableFilters = {
  name: string;
  startDate: IDatePickerControl;
  endDate: IDatePickerControl;
};
