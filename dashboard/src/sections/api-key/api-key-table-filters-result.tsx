import type { Theme, SxProps } from '@mui/material';
import type { StackProps } from '@mui/material/Stack';
import type { IApiKeyTableFilters } from 'src/types/apikey';
import type { UseSetStateReturn } from 'minimal-shared/hooks';

import { useCallback } from 'react';

import Chip from '@mui/material/Chip';

import { fDateRangeShortLabel } from 'src/utils/format-time';

import { useTranslate } from 'src/locales';

import { chipProps, FiltersBlock, FiltersResult } from 'src/components/filters-result';

// ----------------------------------------------------------------------

type Props = StackProps & {
  filters: UseSetStateReturn<IApiKeyTableFilters>;
  onResetPage: () => void;
  totalResults: number;
  sx?: SxProps<Theme>;
};

export function ApiKeyTableFiltersResult({ filters, totalResults, onResetPage, sx }: Props) {
  const { t } = useTranslate();

  const handleRemoveKeyword = useCallback(() => {
    onResetPage();
    filters.setState({ name: '' });
  }, [filters, onResetPage]);

  const handleRemoveDate = useCallback(() => {
    onResetPage();
    filters.setState({ startDate: null, endDate: null });
  }, [filters, onResetPage]);

  return (
    <FiltersResult totalResults={totalResults} onReset={() => filters.resetState()} sx={sx}>
      <FiltersBlock
        label={t('date_filter')}
        isShow={Boolean(filters.state.startDate && filters.state.endDate)}
      >
        <Chip
          {...chipProps}
          label={fDateRangeShortLabel(filters.state.startDate, filters.state.endDate)}
          onDelete={handleRemoveDate}
        />
      </FiltersBlock>

      <FiltersBlock label={t('keyword')} isShow={!!filters.state.name}>
        <Chip {...chipProps} label={filters.state.name} onDelete={handleRemoveKeyword} />
      </FiltersBlock>
    </FiltersResult>
  );
}
