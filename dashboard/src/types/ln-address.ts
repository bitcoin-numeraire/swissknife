import type { IDatePickerControl } from './common';

export type ILnAddressTableFilterValue = string | string[] | Date | null;
export type ILnAddressTableFilters = {
  name: string;
  status: string;
  startDate: IDatePickerControl;
  endDate: IDatePickerControl;
};
