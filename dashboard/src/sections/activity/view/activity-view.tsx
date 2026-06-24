'use client';

import type { TFunction } from 'i18next';
import type { LabelColor } from 'src/components/label';
import type { ITransaction, ITransactionTableFilters } from 'src/types/transaction';

import { mutate } from 'swr';
import { sumBy } from 'es-toolkit';
import { useMemo, useState, useCallback } from 'react';
import { usePopover, useSetState } from 'minimal-shared/hooks';

import Tab from '@mui/material/Tab';
import Card from '@mui/material/Card';
import Tabs from '@mui/material/Tabs';
import Table from '@mui/material/Table';
import Stack from '@mui/material/Stack';
import Divider from '@mui/material/Divider';
import Tooltip from '@mui/material/Tooltip';
import MenuItem from '@mui/material/MenuItem';
import MenuList from '@mui/material/MenuList';
import TableRow from '@mui/material/TableRow';
import Checkbox from '@mui/material/Checkbox';
import TableCell from '@mui/material/TableCell';
import TableBody from '@mui/material/TableBody';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import ListItemText from '@mui/material/ListItemText';
import ToggleButton from '@mui/material/ToggleButton';
import { alpha, useTheme } from '@mui/material/styles';
import TableContainer from '@mui/material/TableContainer';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';

import { paths } from 'src/routes/paths';
import { useRouter } from 'src/routes/hooks';

import { shouldFail } from 'src/utils/errors';
import { fDate, fTime, fIsAfter, fDateTime, fIsBetween } from 'src/utils/format-time';
import { LEDGERS, getCumulativeSeries, mergeAndSortTransactions } from 'src/utils/transactions';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { DashboardContent } from 'src/layouts/dashboard';
import { useGetUserWallet } from 'src/actions/user-wallet';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CopyMenuItem } from 'src/components/copy';
import { Scrollbar } from 'src/components/scrollbar';
import { SatsWithIcon } from 'src/components/bitcoin';
import { Chart, useChart } from 'src/components/chart';
import { ItemAnalytic } from 'src/components/analytic';
import { ErrorView } from 'src/components/error/error-view';
import { EmptyContent } from 'src/components/empty-content';
import { CustomPopover } from 'src/components/custom-popover';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';
import { CleanTransactionsButton } from 'src/components/transactions';
import {
  useTable,
  emptyRows,
  TableNoData,
  getComparator,
  TableEmptyRows,
  TableHeadCustom,
  TablePaginationCustom,
} from 'src/components/table';

import { TransactionTableToolbar } from 'src/sections/transaction/transaction-table-toolbar';
import { TransactionTableFiltersResult } from 'src/sections/transaction/transaction-table-filters-result';

import { TransactionType } from 'src/types/transaction';

// ----------------------------------------------------------------------

type ActivityLens = 'all' | 'Settled' | 'Pending' | 'Expired' | 'Failed';
type FlowLens = 'income' | 'expenses';

type ActivityRow = ITransaction & {
  row_key: string;
  direction: 'in' | 'out';
  detail_href: string;
  amount_total_msat: number;
  description_label: string;
};

type ActivityTab = {
  title: string;
  value: ActivityLens;
  label: string;
  color: LabelColor;
  suffix: string;
  icon: string;
  analyticColor: string;
};

const tableHead = (t: TFunction) => [
  { id: 'direction', label: t('activity_view.direction'), width: 96 },
  { id: 'description_label', label: t('transaction_list.description') },
  { id: 'created_at', label: t('transaction_list.created') },
  { id: 'payment_time', label: t('transaction_list.settled') },
  { id: 'amount_total_msat', label: t('transaction_list.amount') },
  { id: 'ledger', label: t('transaction_details.ledger') },
  { id: 'status', label: t('transaction_list.status') },
  { id: '', width: 56 },
];

function txDirection(tx: ITransaction): 'in' | 'out' {
  return tx.transaction_type === TransactionType.INVOICE ? 'in' : 'out';
}

function txAmount(tx: ITransaction) {
  return (tx.amount_msat || 0) + (tx.fee_msat || 0);
}

function statusColor(status: string): LabelColor {
  if (status === 'Settled') return 'success';
  if (status === 'Failed' || status === 'Expired') return 'error';
  return 'warning';
}

function directionColor(direction: 'in' | 'out'): LabelColor {
  return direction === 'in' ? 'success' : 'warning';
}

function detailHref(tx: ITransaction) {
  if (tx.transaction_type === TransactionType.INVOICE) {
    return paths.wallet.invoice(tx.id);
  }

  return paths.wallet.payment(tx.id);
}

function makeRow(tx: ITransaction): ActivityRow {
  const direction = txDirection(tx);

  return {
    ...tx,
    direction,
    row_key: `${tx.transaction_type}-${tx.id}`,
    detail_href: detailHref(tx),
    amount_total_msat: txAmount(tx),
    description_label: tx.description || (direction === 'in' ? 'Incoming payment' : 'Outgoing payment'),
  };
}

function isNeedsAction(tx: ActivityRow) {
  return tx.status === 'Failed' || tx.status === 'Expired';
}

function txMatchesStatus(tx: ActivityRow, status: ActivityLens) {
  if (status === 'all') return true;
  return tx.status === status;
}

function applyFilter({
  inputData,
  comparator,
  filters,
  dateError,
}: {
  dateError: boolean;
  inputData: ActivityRow[];
  filters: ITransactionTableFilters;
  comparator: (a: any, b: any) => number;
}) {
  const { name, status, ledger, startDate, endDate } = filters;

  const stabilizedThis = inputData.map((el, index) => [el, index] as const);

  stabilizedThis.sort((a, b) => {
    const order = comparator(a[0], b[0]);
    if (order !== 0) return order;
    return a[1] - b[1];
  });

  inputData = stabilizedThis.map((el) => el[0]);

  if (name) {
    inputData = inputData.filter((tx) => {
      const value = name.toLowerCase();

      return (
        tx.description_label.toLowerCase().includes(value) ||
        tx.ledger.toLowerCase().includes(value) ||
        tx.status.toLowerCase().includes(value) ||
        tx.id.toLowerCase().includes(value) ||
        tx.wallet_id.toLowerCase().includes(value)
      );
    });
  }

  if (status !== 'all') {
    inputData = inputData.filter((tx) => txMatchesStatus(tx, status as ActivityLens));
  }

  if (ledger.length) {
    inputData = inputData.filter((tx) => ledger.includes(tx.ledger));
  }

  if (!dateError && startDate && endDate) {
    inputData = inputData.filter((tx) => fIsBetween(tx.created_at, startDate, endDate));
  }

  return inputData;
}

// ----------------------------------------------------------------------

export function ActivityView() {
  const { t } = useTranslate();
  const theme = useTheme();
  const table = useTable({
    defaultOrder: 'desc',
    defaultOrderBy: 'created_at',
    defaultRowsPerPage: 25,
  });

  const [flowLens, setFlowLens] = useState<FlowLens>('income');
  const filters = useSetState<ITransactionTableFilters>({
    name: '',
    ledger: [],
    status: 'all',
    startDate: null,
    endDate: null,
  });

  const { wallet, walletLoading, walletError } = useGetUserWallet();
  const errors = [walletError];
  const data = [wallet];
  const isLoading = [walletLoading];
  const failed = shouldFail(errors, data, isLoading);
  const dateError = fIsAfter(filters.state.startDate, filters.state.endDate);

  const rows = useMemo(
    () => mergeAndSortTransactions(wallet?.invoices || [], wallet?.payments || []).map(makeRow),
    [wallet?.invoices, wallet?.payments]
  );

  const incomeSeries = useMemo(() => getCumulativeSeries(wallet?.invoices || []), [wallet?.invoices]);
  const expensesSeries = useMemo(
    () => getCumulativeSeries(wallet?.payments || []),
    [wallet?.payments]
  );

  const chartSeries = useMemo(
    () => [
      {
        name: flowLens === 'income' ? t('wallet_view.income') : t('wallet_view.expenses'),
        data: (flowLens === 'income' ? incomeSeries : expensesSeries)[0].data,
      },
    ],
    [expensesSeries, flowLens, incomeSeries, t]
  );

  const chartOptions = useChart({
    colors: [flowLens === 'income' ? theme.palette.success.main : theme.palette.warning.main],
    xaxis: { type: 'datetime' },
    tooltip: {
      y: {
        formatter: (value) => `${Math.round(Number(value)).toLocaleString()} sats`,
      },
    },
    yaxis: {
      labels: {
        formatter: (value) => `${Math.round(Number(value)).toLocaleString()} sats`,
      },
    },
  });

  const activityTabs: ActivityTab[] = [
    {
      title: t('transaction_list.tabs.total'),
      value: 'all',
      label: t('activity_view.all'),
      color: 'default',
      suffix: t('activity_view.transactions_suffix'),
      icon: 'solar:bill-list-bold-duotone',
      analyticColor: theme.palette.info.main,
    },
    {
      title: t('transaction_list.tabs.paid'),
      value: 'Settled',
      label: t('transaction_list.tabs.paid'),
      color: 'success',
      suffix: t('activity_view.transactions_suffix'),
      icon: 'solar:bill-check-bold-duotone',
      analyticColor: theme.palette.success.main,
    },
    {
      title: t('transaction_list.tabs.pending'),
      value: 'Pending',
      label: t('transaction_list.tabs.pending'),
      color: 'warning',
      suffix: t('activity_view.transactions_suffix'),
      icon: 'solar:sort-by-time-bold-duotone',
      analyticColor: theme.palette.warning.main,
    },
    {
      title: t('transaction_list.tabs.expired'),
      value: 'Expired',
      label: t('transaction_list.tabs.expired'),
      color: 'error',
      suffix: t('activity_view.transactions_suffix'),
      icon: 'solar:bill-cross-bold-duotone',
      analyticColor: theme.palette.error.main,
    },
    {
      title: t('transaction_list.tabs.failed'),
      value: 'Failed',
      label: t('transaction_list.tabs.failed'),
      color: 'error',
      suffix: t('activity_view.transactions_suffix'),
      icon: 'solar:danger-triangle-bold-duotone',
      analyticColor: theme.palette.error.dark,
    },
  ];

  const dataFiltered = applyFilter({
    inputData: rows,
    comparator: getComparator(table.order, table.orderBy),
    filters: filters.state,
    dateError,
  });

  const dataInPage = dataFiltered.slice(
    table.page * table.rowsPerPage,
    table.page * table.rowsPerPage + table.rowsPerPage
  );

  const denseHeight = table.dense ? 56 : 76;
  const canReset =
    !!filters.state.name ||
    !!filters.state.ledger.length ||
    filters.state.status !== 'all' ||
    (!!filters.state.startDate && !!filters.state.endDate);
  const notFound = (!dataFiltered.length && canReset) || !dataFiltered.length;
  const hasFailedOrExpired = rows.some(isNeedsAction);

  const getTransactionLength = useCallback(
    (status: ActivityLens) => rows.filter((row) => txMatchesStatus(row, status)).length,
    [rows]
  );

  const getTotalAmount = useCallback(
    (status: ActivityLens) =>
      sumBy(
        rows.filter((row) => txMatchesStatus(row, status)),
        (row) => row.amount_total_msat
      ),
    [rows]
  );

  const getPercentByStatus = useCallback(
    (status: ActivityLens) => (rows.length ? (getTransactionLength(status) / rows.length) * 100 : 0),
    [getTransactionLength, rows.length]
  );

  const handleFilterStatus = useCallback(
    (event: React.SyntheticEvent, newValue: ActivityLens) => {
      table.onResetPage();
      filters.setState({ status: newValue });
    },
    [filters, table]
  );

  const handleCleanSuccess = () => {
    mutate(endpointKeys.userWallet.get);
  };

  return (
    <DashboardContent maxWidth="xl">
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} data={data} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={t('activity')}
            links={[{ name: t('money') }, { name: t('activity') }]}
            action={
              <Tooltip title={t('recent_transactions.clean_failed_expired')} placement="top" arrow>
                <span>
                  <CleanTransactionsButton
                    onSuccess={handleCleanSuccess}
                    buttonProps={{
                      color: 'error',
                      variant: 'outlined',
                      disabled: !hasFailedOrExpired,
                      startIcon: <Iconify icon="solar:trash-bin-trash-bold" />,
                    }}
                  >
                    {t('clean')}
                  </CleanTransactionsButton>
                </span>
              </Tooltip>
            }
            sx={{ mb: { xs: 3, md: 5 } }}
          />

          <Stack spacing={3}>
            <Card>
              <Scrollbar>
                <Stack
                  direction="row"
                  divider={<Divider orientation="vertical" flexItem sx={{ borderStyle: 'dashed' }} />}
                  sx={{ py: 2 }}
                >
                  {activityTabs.map((tab) => (
                    <ItemAnalytic
                      key={tab.title}
                      title={tab.title}
                      total={getTransactionLength(tab.value)}
                      percent={getPercentByStatus(tab.value)}
                      price={getTotalAmount(tab.value)}
                      icon={tab.icon}
                      color={tab.analyticColor}
                      countSuffix={tab.suffix}
                      compact
                    />
                  ))}
                </Stack>
              </Scrollbar>
            </Card>

            <Card sx={{ p: 3 }}>
              <Stack
                direction={{ xs: 'column', md: 'row' }}
                spacing={3}
                sx={{ alignItems: { md: 'center' }, justifyContent: 'space-between' }}
              >
                <Stack spacing={0.5}>
                  <Typography variant="h6">{t('activity_view.flow_title')}</Typography>
                  <Typography variant="body2" color="text.secondary">
                    {t('activity_view.flow_subheader')}
                  </Typography>
                </Stack>

                <ToggleButtonGroup
                  exclusive
                  size="small"
                  value={flowLens}
                  onChange={(_, value) => value && setFlowLens(value)}
                >
                  <ToggleButton value="income">{t('wallet_view.income')}</ToggleButton>
                  <ToggleButton value="expenses">{t('wallet_view.expenses')}</ToggleButton>
                </ToggleButtonGroup>
              </Stack>

              {chartSeries[0].data.length ? (
                <Chart
                  type="area"
                  series={chartSeries}
                  options={chartOptions}
                  sx={{ height: 300, mt: 3 }}
                />
              ) : (
                <EmptyContent
                  title={t('activity_view.empty_chart')}
                  description={t('activity_view.empty_chart_description')}
                  sx={{ py: 5 }}
                />
              )}
            </Card>

            <Card>
              <Tabs
                value={filters.state.status}
                onChange={handleFilterStatus}
                variant="scrollable"
                sx={{
                  px: 2.5,
                  boxShadow: `inset 0 -2px 0 0 ${alpha(theme.palette.grey[500], 0.08)}`,
                }}
              >
                {activityTabs.map((tab) => (
                  <Tab
                    key={tab.value}
                    value={tab.value}
                    label={tab.label}
                    iconPosition="end"
                    icon={
                      <Label
                        variant={
                          ((tab.value === 'all' || tab.value === filters.state.status) &&
                            'filled') ||
                          'soft'
                        }
                        color={tab.color}
                      >
                        {getTransactionLength(tab.value)}
                      </Label>
                    }
                  />
                ))}
              </Tabs>

              <TransactionTableToolbar
                filters={filters}
                onResetPage={table.onResetPage}
                dateError={dateError}
                invoiceLedgerOptions={LEDGERS}
              />

              {canReset && (
                <TransactionTableFiltersResult
                  filters={filters}
                  onResetPage={table.onResetPage}
                  totalResults={dataFiltered.length}
                  sx={{ p: 2.5, pt: 0 }}
                />
              )}

              <TableContainer sx={{ position: 'relative', overflow: 'unset' }}>
                <Scrollbar>
                  <Table size={table.dense ? 'small' : 'medium'} sx={{ minWidth: 960 }}>
                    <TableHeadCustom
                      order={table.order}
                      orderBy={table.orderBy}
                      headCells={tableHead(t)}
                      rowCount={dataFiltered.length}
                      numSelected={table.selected.length}
                      onSort={table.onSort}
                      onSelectAllRows={(checked) =>
                        table.onSelectAllRows(
                          checked,
                          dataFiltered.map((row) => row.row_key)
                        )
                      }
                    />

                    <TableBody>
                      {dataInPage.map((row) => (
                        <ActivityTableRow
                          key={row.row_key}
                          row={row}
                          selected={table.selected.includes(row.row_key)}
                          onSelectRow={() => table.onSelectRow(row.row_key)}
                        />
                      ))}

                      <TableEmptyRows
                        height={denseHeight}
                        emptyRows={emptyRows(table.page, table.rowsPerPage, dataFiltered.length)}
                      />

                      <TableNoData notFound={notFound} />
                    </TableBody>
                  </Table>
                </Scrollbar>
              </TableContainer>

              <TablePaginationCustom
                count={dataFiltered.length}
                page={table.page}
                rowsPerPage={table.rowsPerPage}
                onPageChange={table.onChangePage}
                onRowsPerPageChange={table.onChangeRowsPerPage}
                dense={table.dense}
                onChangeDense={table.onChangeDense}
              />
            </Card>
          </Stack>
        </>
      )}
    </DashboardContent>
  );
}

// ----------------------------------------------------------------------

function ActivityTableRow({
  row,
  selected,
  onSelectRow,
}: {
  row: ActivityRow;
  selected: boolean;
  onSelectRow: VoidFunction;
}) {
  const { t } = useTranslate();
  const router = useRouter();
  const popover = usePopover();

  return (
    <>
      <TableRow
        hover
        selected={selected}
        onClick={() => router.push(row.detail_href)}
        sx={{ cursor: 'pointer' }}
      >
        <TableCell padding="checkbox">
          <Checkbox
            checked={selected}
            onClick={(event) => {
              event.stopPropagation();
              onSelectRow();
            }}
          />
        </TableCell>

        <TableCell>
          <Label color={directionColor(row.direction)}>
            {row.direction === 'in' ? t('activity_view.in') : t('activity_view.out')}
          </Label>
        </TableCell>

        <TableCell>
          <ListItemText
            primary={row.description_label}
            secondary={`${fDateTime(row.created_at)} · ${row.ledger}`}
            slotProps={{
              primary: { noWrap: true, sx: { typography: 'body2' } },
              secondary: { sx: { typography: 'caption', color: 'text.disabled' } },
            }}
          />
        </TableCell>

        <TableCell>
          <ListItemText
            primary={fDate(row.created_at)}
            secondary={fTime(row.created_at)}
            slotProps={{
              primary: { noWrap: true, sx: { typography: 'body2' } },
              secondary: { sx: { typography: 'caption', color: 'text.disabled' } },
            }}
          />
        </TableCell>

        <TableCell>
          <ListItemText
            primary={fDate(row.payment_time)}
            secondary={fTime(row.payment_time)}
            slotProps={{
              primary: { noWrap: true, sx: { typography: 'body2' } },
              secondary: { sx: { typography: 'caption', color: 'text.disabled' } },
            }}
          />
        </TableCell>

        <TableCell>
          <SatsWithIcon amountMSats={row.amount_total_msat} />
        </TableCell>

        <TableCell>
          <Label
            variant="soft"
            color={
              (row.ledger === 'Lightning' && 'secondary') ||
              (row.ledger === 'Internal' && 'primary') ||
              'default'
            }
          >
            {row.ledger}
          </Label>
        </TableCell>

        <TableCell>
          <Label variant="soft" color={statusColor(row.status)}>
            {row.status}
          </Label>
        </TableCell>

        <TableCell align="right" sx={{ px: 1 }}>
          <IconButton
            color={popover.open ? 'inherit' : 'default'}
            onClick={(event) => {
              event.stopPropagation();
              popover.onOpen(event);
            }}
          >
            <Iconify icon="eva:more-vertical-fill" />
          </IconButton>
        </TableCell>
      </TableRow>

      <CustomPopover
        open={popover.open}
        anchorEl={popover.anchorEl}
        onClose={popover.onClose}
        slotProps={{ arrow: { placement: 'right-top' } }}
      >
        <MenuList>
          <MenuItem
            onClick={() => {
              router.push(row.detail_href);
              popover.onClose();
            }}
          >
            <Iconify icon="solar:eye-bold" />
            {t('details')}
          </MenuItem>

          <CopyMenuItem value={row.id} title={t('activity_view.copy_transaction_id')} />
        </MenuList>
      </CustomPopover>
    </>
  );
}
