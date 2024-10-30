import type { IDatePickerControl } from './common';

export type IApiKeyTableFilterValue = string | string[] | Date | null;
export type IApiKeyTableFilters = {
  name: string;
  startDate: IDatePickerControl;
  endDate: IDatePickerControl;
};
