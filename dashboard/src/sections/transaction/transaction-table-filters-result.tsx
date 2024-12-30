import type { Theme, SxProps } from '@mui/material';
import type { UseSetStateReturn } from 'minimal-shared/hooks';
import type { ITransactionTableFilters } from 'src/types/transaction';

import { useCallback } from 'react';

import Chip from '@mui/material/Chip';

import { fDateRangeShortLabel } from 'src/utils/format-time';

import { useTranslate } from 'src/locales';

import { chipProps, FiltersBlock, FiltersResult } from 'src/components/filters-result';

// ----------------------------------------------------------------------

type Props = {
  filters: UseSetStateReturn<ITransactionTableFilters>;
  onResetPage: () => void;
  totalResults: number;
  sx?: SxProps<Theme>;
};

export function TransactionTableFiltersResult({ filters, totalResults, onResetPage, sx }: Props) {
  const { t } = useTranslate();

  const handleRemoveKeyword = useCallback(() => {
    onResetPage();
    filters.setState({ name: '' });
  }, [filters, onResetPage]);

  const handleRemoveLedger = useCallback(
    (inputValue: string) => {
      const newValue = filters.state.ledger.filter((item) => item !== inputValue);

      onResetPage();
      filters.setState({ ledger: newValue });
    },
    [filters, onResetPage]
  );

  const handleRemoveStatus = useCallback(() => {
    onResetPage();
    filters.setState({ status: 'all' });
  }, [filters, onResetPage]);

  const handleRemoveDate = useCallback(() => {
    onResetPage();
    filters.setState({ startDate: null, endDate: null });
  }, [filters, onResetPage]);

  return (
    <FiltersResult totalResults={totalResults} onReset={() => filters.resetState()} sx={sx}>
      <FiltersBlock label={t('transaction_filters.ledger')} isShow={!!filters.state.ledger.length}>
        {filters.state.ledger.map((item) => (
          <Chip {...chipProps} key={item} label={item} onDelete={() => handleRemoveLedger(item)} />
        ))}
      </FiltersBlock>

      <FiltersBlock label={t('transaction_filters.status')} isShow={filters.state.status !== 'all'}>
        <Chip
          {...chipProps}
          label={filters.state.status}
          onDelete={handleRemoveStatus}
          sx={{ textTransform: 'capitalize' }}
        />
      </FiltersBlock>

      <FiltersBlock
        label={t('transaction_filters.date')}
        isShow={Boolean(filters.state.startDate && filters.state.endDate)}
      >
        <Chip
          {...chipProps}
          label={fDateRangeShortLabel(filters.state.startDate, filters.state.endDate)}
          onDelete={handleRemoveDate}
        />
      </FiltersBlock>

      <FiltersBlock label={t('transaction_filters.keyword')} isShow={!!filters.state.name}>
        <Chip {...chipProps} label={filters.state.name} onDelete={handleRemoveKeyword} />
      </FiltersBlock>
    </FiltersResult>
  );
}
