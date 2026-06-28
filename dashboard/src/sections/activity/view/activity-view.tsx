'use client';

import type { TFunction } from 'i18next';
import type { LabelColor } from 'src/components/label';
import type { Invoice, Payment } from 'src/lib/swissknife';
import type { ITransaction, ITransactionTableFilters } from 'src/types/transaction';

import { mutate } from 'swr';
import { sumBy } from 'es-toolkit';
import { useMemo, useState, useEffect, useCallback } from 'react';
import { useBoolean, usePopover, useSetState, useCopyToClipboard } from 'minimal-shared/hooks';

import Tab from '@mui/material/Tab';
import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Tabs from '@mui/material/Tabs';
import Table from '@mui/material/Table';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Drawer from '@mui/material/Drawer';
import Divider from '@mui/material/Divider';
import Tooltip from '@mui/material/Tooltip';
import MenuItem from '@mui/material/MenuItem';
import MenuList from '@mui/material/MenuList';
import TableRow from '@mui/material/TableRow';
import Checkbox from '@mui/material/Checkbox';
import ButtonBase from '@mui/material/ButtonBase';
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
import { useRouter, useSearchParams } from 'src/routes/hooks';

import { composeBip21, compactBitcoinAddress } from 'src/utils/bitcoin-request';
import { shouldFail, handleActionError } from 'src/utils/errors';
import { fDate, fTime, fIsAfter, fDateTime, fIsBetween } from 'src/utils/format-time';
import {
  LEDGERS,
  getLedgerLabel,
  getCumulativeSeries,
  mergeAndSortTransactions,
} from 'src/utils/transactions';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { useListPayments } from 'src/actions/payments';
import { useListInvoices } from 'src/actions/invoices';
import { DashboardContent } from 'src/layouts/dashboard';
import { useGetUserWallet } from 'src/actions/user-wallet';
import { Permission, deleteInvoice, deletePayment } from 'src/lib/swissknife';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { Scrollbar } from 'src/components/scrollbar';
import { SatsWithIcon } from 'src/components/bitcoin';
import { Chart, useChart } from 'src/components/chart';
import { ItemAnalytic } from 'src/components/analytic';
import { ErrorView } from 'src/components/error/error-view';
import { EmptyContent } from 'src/components/empty-content';
import { ConfirmDialog } from 'src/components/custom-dialog';
import { CustomPopover } from 'src/components/custom-popover';
import { CopyMenuItem } from 'src/components/copy';
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

import { RoleBasedGuard } from 'src/auth/guard';

import { TransactionType } from 'src/types/transaction';

// ----------------------------------------------------------------------

type ActivityLens = 'all' | 'received' | 'paid' | 'Pending' | 'Expired' | 'Failed';
type FlowLens = 'income' | 'expenses';
type ActivityScope = 'wallet' | 'admin';
type ActivityTransactionKind = 'all' | 'payment' | 'invoice';

type ActivityRow = ITransaction & {
  row_key: string;
  direction: 'in' | 'out';
  detail_page_href: string;
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

const drawerSx = {
  width: { xs: 1, sm: 520 },
  maxWidth: 1,
};

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

function txExplorerUrl(txid?: string | null) {
  if (!txid) return undefined;

  const explorerBaseUrl = CONFIG.mempoolSpace.replace(/\/api\/v1\/?$/, '');
  return `${explorerBaseUrl}/tx/${txid}`;
}

function txidFromOutpoint(outpoint?: string | null) {
  if (!outpoint) return undefined;

  const [txid] = outpoint.split(':');
  return txid || undefined;
}

function composeInvoiceBip21(invoice?: Invoice | null) {
  const address = invoice?.bitcoin_output?.address;
  const bolt11 = invoice?.ln_invoice?.bolt11;

  if (!address || !bolt11) return undefined;

  const amountSats = invoice?.amount_msat ? invoice.amount_msat / 1000 : 0;

  return composeBip21(address, bolt11, amountSats);
}

function detailPageHrefForScope(tx: ITransaction, scope: ActivityScope) {
  if (tx.transaction_type === TransactionType.INVOICE) {
    return scope === 'admin' ? paths.admin.invoice(tx.id) : paths.wallet.invoice(tx.id);
  }

  return scope === 'admin' ? paths.admin.payment(tx.id) : paths.wallet.payment(tx.id);
}

function compactIdentifier(value: string) {
  if (value.length <= 18) return value;

  return `${value.slice(0, 8)}...${value.slice(-6)}`;
}

function isOpenAmountRequest(tx: ITransaction) {
  return (
    tx.transaction_type === TransactionType.INVOICE && tx.status === 'Pending' && !tx.amount_msat
  );
}

function makeRow(tx: ITransaction, scope: ActivityScope): ActivityRow {
  const direction = txDirection(tx);

  return {
    ...tx,
    direction,
    row_key: `${tx.transaction_type}-${tx.id}`,
    detail_page_href: detailPageHrefForScope(tx, scope),
    amount_total_msat: txAmount(tx),
    description_label:
      tx.description || (direction === 'in' ? 'Incoming payment' : 'Outgoing payment'),
  };
}

function isNeedsAction(tx: ActivityRow) {
  return tx.status === 'Failed' || tx.status === 'Expired';
}

function canDeleteRow(tx: ActivityRow, scope: ActivityScope) {
  return scope === 'admin' || tx.status === 'Failed' || tx.status === 'Expired';
}

function txMatchesStatus(tx: ActivityRow, status: ActivityLens) {
  if (status === 'all') return true;
  if (status === 'received') return tx.direction === 'in' && tx.status === 'Settled';
  if (status === 'paid') return tx.direction === 'out' && tx.status === 'Settled';
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

function normalizeTransactionKind(value: string | null): ActivityTransactionKind {
  if (value === 'payment' || value === 'payments') return 'payment';
  if (value === 'invoice' || value === 'invoices') return 'invoice';
  return 'all';
}

function transactionTypeForKind(kind: ActivityTransactionKind) {
  if (kind === 'payment') return TransactionType.PAYMENT;
  if (kind === 'invoice') return TransactionType.INVOICE;
  return undefined;
}

// ----------------------------------------------------------------------

export function ActivityView() {
  const searchParams = useSearchParams();
  const id = searchParams.get('id');
  const scope: ActivityScope = searchParams.get('scope') === 'admin' ? 'admin' : 'wallet';
  const kind = normalizeTransactionKind(searchParams.get('type'));

  return scope === 'admin' ? (
    <AdminActivityLedger kind={kind} initialDetailId={id} />
  ) : (
    <WalletActivityLedger kind={kind} initialDetailId={id} />
  );
}

function WalletActivityLedger({
  kind,
  initialDetailId,
}: {
  kind: ActivityTransactionKind;
  initialDetailId?: string | null;
}) {
  const { wallet, walletLoading, walletError } = useGetUserWallet();

  return (
    <ActivityLedger
      scope="wallet"
      kind={kind}
      invoices={wallet?.invoices || []}
      payments={wallet?.payments || []}
      errors={[walletError]}
      data={[wallet]}
      isLoading={[walletLoading]}
      initialDetailId={initialDetailId}
      onCleanSuccess={() => mutate(endpointKeys.userWallet.get)}
    />
  );
}

function AdminActivityLedger({
  kind,
  initialDetailId,
}: {
  kind: ActivityTransactionKind;
  initialDetailId?: string | null;
}) {
  const { invoices, invoicesLoading, invoicesError } = useListInvoices();
  const { payments, paymentsLoading, paymentsError } = useListPayments();

  return (
    <RoleBasedGuard permissions={[Permission.READ_TRANSACTION]} hasContent>
      <ActivityLedger
        scope="admin"
        kind={kind}
        invoices={invoices || []}
        payments={payments || []}
        errors={[invoicesError, paymentsError]}
        data={[invoices, payments]}
        isLoading={[invoicesLoading, paymentsLoading]}
        initialDetailId={initialDetailId}
        onCleanSuccess={() => {
          mutate(endpointKeys.invoices.list);
          mutate(endpointKeys.payments.list);
        }}
      />
    </RoleBasedGuard>
  );
}

function ActivityLedger({
  scope,
  kind,
  invoices,
  payments,
  errors,
  data,
  isLoading,
  initialDetailId,
  onCleanSuccess,
}: {
  scope: ActivityScope;
  kind: ActivityTransactionKind;
  invoices: Invoice[];
  payments: Payment[];
  errors: unknown[];
  data: (object | null | undefined)[];
  isLoading: boolean[];
  initialDetailId?: string | null;
  onCleanSuccess: VoidFunction;
}) {
  const { t } = useTranslate();
  const theme = useTheme();
  const router = useRouter();
  const table = useTable({
    defaultOrder: 'desc',
    defaultOrderBy: 'created_at',
    defaultRowsPerPage: 25,
  });

  const [flowLens, setFlowLens] = useState<FlowLens>(kind === 'payment' ? 'expenses' : 'income');
  const [detailRow, setDetailRow] = useState<ActivityRow | null>(null);
  const filters = useSetState<ITransactionTableFilters>({
    name: '',
    ledger: [],
    status: 'all',
    startDate: null,
    endDate: null,
  });

  const failed = shouldFail(errors, data, isLoading);
  const dateError = fIsAfter(filters.state.startDate, filters.state.endDate);

  const visibleInvoices = useMemo(() => (kind === 'payment' ? [] : invoices), [invoices, kind]);
  const visiblePayments = useMemo(() => (kind === 'invoice' ? [] : payments), [kind, payments]);

  const rows = useMemo(
    () =>
      mergeAndSortTransactions(visibleInvoices, visiblePayments).map((tx) => makeRow(tx, scope)),
    [scope, visibleInvoices, visiblePayments]
  );

  useEffect(() => {
    if (!initialDetailId) return;

    setDetailRow(rows.find((row) => row.id === initialDetailId) ?? null);
  }, [initialDetailId, rows]);

  const incomeSeries = useMemo(() => getCumulativeSeries(visibleInvoices), [visibleInvoices]);
  const expensesSeries = useMemo(() => getCumulativeSeries(visiblePayments), [visiblePayments]);

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
      title: t('activity_view.all'),
      value: 'all',
      label: t('activity_view.all'),
      color: 'default',
      suffix: t('activity_view.transactions_suffix'),
      icon: 'solar:bill-list-bold-duotone',
      analyticColor: theme.palette.info.main,
    },
    {
      title: t('activity_view.received'),
      value: 'received',
      label: t('activity_view.received'),
      color: 'success',
      suffix: t('activity_view.transactions_suffix'),
      icon: 'solar:download-minimalistic-bold-duotone',
      analyticColor: theme.palette.success.main,
    },
    {
      title: t('transaction_list.tabs.paid'),
      value: 'paid',
      label: t('transaction_list.tabs.paid'),
      color: 'warning',
      suffix: t('activity_view.transactions_suffix'),
      icon: 'solar:upload-minimalistic-bold-duotone',
      analyticColor: theme.palette.warning.main,
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
  const analyticTabs = activityTabs.filter((tab) => tab.value !== 'all');

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
    (status: ActivityLens) =>
      rows.length ? (getTransactionLength(status) / rows.length) * 100 : 0,
    [getTransactionLength, rows.length]
  );

  const handleFilterStatus = useCallback(
    (event: React.SyntheticEvent, newValue: ActivityLens) => {
      table.onResetPage();
      filters.setState({ status: newValue });
    },
    [filters, table]
  );

  const handleDeleteRow = useCallback(
    async (row: ActivityRow) => {
      try {
        if (row.transaction_type === TransactionType.INVOICE) {
          await deleteInvoice({ path: { id: row.id } });
        } else {
          await deletePayment({ path: { id: row.id } });
        }

        toast.success(t('transaction_list.delete_success'));
        setDetailRow((currentRow) => (currentRow?.row_key === row.row_key ? null : currentRow));
        onCleanSuccess();
      } catch (error) {
        handleActionError(error);
      }
    },
    [onCleanSuccess, t]
  );

  return (
    <DashboardContent maxWidth="xl">
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} data={data} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={t('activity')}
            links={[
              { name: scope === 'admin' ? t('accounts') : t('money') },
              { name: t('activity') },
            ]}
            action={
              <Tooltip title={t('recent_transactions.clean_failed_expired')} placement="top" arrow>
                <span>
                  <CleanTransactionsButton
                    onSuccess={onCleanSuccess}
                    transactionType={transactionTypeForKind(kind)}
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
                  divider={
                    <Divider orientation="vertical" flexItem sx={{ borderStyle: 'dashed' }} />
                  }
                  sx={{ py: 2 }}
                >
                  {analyticTabs.map((tab) => (
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
                          scope={scope}
                          selected={table.selected.includes(row.row_key)}
                          onSelectRow={() => table.onSelectRow(row.row_key)}
                          onOpenRow={() => setDetailRow(row)}
                          canDelete={canDeleteRow(row, scope)}
                          onDeleteRow={() => handleDeleteRow(row)}
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

          <ActivityDetailDrawer
            row={detailRow}
            canDelete={!!detailRow && canDeleteRow(detailRow, scope)}
            onDeleteRow={detailRow ? () => handleDeleteRow(detailRow) : undefined}
            onClose={() => {
              setDetailRow(null);
              if (initialDetailId) {
                router.push(paths.activityList(kind === 'all' ? undefined : kind, scope));
              }
            }}
          />
        </>
      )}
    </DashboardContent>
  );
}

// ----------------------------------------------------------------------

function ActivityTableRow({
  row,
  scope,
  selected,
  onSelectRow,
  onOpenRow,
  canDelete,
  onDeleteRow,
}: {
  row: ActivityRow;
  scope: ActivityScope;
  selected: boolean;
  onSelectRow: VoidFunction;
  onOpenRow: VoidFunction;
  canDelete: boolean;
  onDeleteRow: () => Promise<void>;
}) {
  const { t } = useTranslate();
  const router = useRouter();
  const popover = usePopover();
  const confirm = useBoolean();
  const isDeleting = useBoolean();
  const invoice = row.transaction_type === TransactionType.INVOICE ? (row as Invoice) : null;
  const payment = row.transaction_type === TransactionType.PAYMENT ? (row as Payment) : null;
  const invoiceBolt11 = invoice?.ln_invoice?.bolt11;
  const invoiceAddress = invoice?.bitcoin_output?.address;
  const invoiceOutpoint = invoice?.bitcoin_output?.outpoint;
  const invoiceUnified = composeInvoiceBip21(invoice);
  const paymentAddress =
    payment?.bitcoin?.address || payment?.internal?.btc_address || payment?.internal?.ln_address;
  const explorerUrl = txExplorerUrl(payment?.bitcoin?.txid || txidFromOutpoint(invoiceOutpoint));
  const methodLabel = getLedgerLabel(row.ledger, t);
  const isOpenAmount = isOpenAmountRequest(row);

  return (
    <>
      <TableRow hover selected={selected} onClick={onOpenRow} sx={{ cursor: 'pointer' }}>
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
            secondary={
              scope === 'admin'
                ? `${row.wallet_id} · ${fDateTime(row.created_at)} · ${methodLabel}`
                : `${fDateTime(row.created_at)} · ${methodLabel}`
            }
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
          {isOpenAmount ? (
            <Typography variant="body2" color="text.secondary" sx={{ whiteSpace: 'nowrap' }}>
              {t('wallet_view.open_amount')}
            </Typography>
          ) : (
            <Stack
              direction="row"
              spacing={0.25}
              sx={{
                alignItems: 'center',
                color: row.direction === 'in' ? 'success.main' : 'warning.main',
                whiteSpace: 'nowrap',
              }}
            >
              <Typography variant="body2" color="inherit">
                {row.direction === 'in' ? '+' : '-'}
              </Typography>
              <SatsWithIcon amountMSats={row.amount_total_msat} variant="body2" color="inherit" />
            </Stack>
          )}
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
            {methodLabel}
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
              router.push(row.detail_page_href);
              popover.onClose();
            }}
          >
            <Iconify icon="solar:bill-list-bold-duotone" />
            {t('wallet_view.open_details')}
          </MenuItem>

          <CopyMenuItem value={row.id} title={t('activity_view.copy_transaction_id')} />

          {invoiceUnified && (
            <CopyMenuItem value={invoiceUnified} title={t('transaction_actions.copy_unified')} />
          )}
          {invoiceBolt11 && (
            <CopyMenuItem value={invoiceBolt11} title={t('transaction_actions.copy_bolt11')} />
          )}
          {invoiceAddress && (
            <CopyMenuItem
              value={invoiceAddress}
              title={t('transaction_actions.copy_onchain_address')}
            />
          )}
          {invoiceOutpoint && (
            <CopyMenuItem value={invoiceOutpoint} title={t('transaction_actions.copy_outpoint')} />
          )}
          {paymentAddress && (
            <CopyMenuItem
              value={paymentAddress}
              title={t('transaction_actions.copy_destination')}
            />
          )}
          {explorerUrl && (
            <MenuItem component="a" href={explorerUrl} target="_blank" rel="noopener noreferrer">
              <Iconify icon="solar:map-arrow-right-bold" />
              {t('transaction_actions.open_explorer')}
            </MenuItem>
          )}

          {canDelete && (
            <>
              <Divider sx={{ borderStyle: 'dashed' }} />

              <MenuItem
                onClick={() => {
                  confirm.onTrue();
                  popover.onClose();
                }}
                sx={{ color: 'error.main' }}
              >
                <Iconify icon="solar:trash-bin-trash-bold" />
                {t('delete')}
              </MenuItem>
            </>
          )}
        </MenuList>
      </CustomPopover>

      <ConfirmDialog
        open={confirm.value}
        onClose={confirm.onFalse}
        title={t('delete')}
        content={t('confirm_delete')}
        action={
          <Button
            variant="contained"
            color="error"
            loading={isDeleting.value}
            onClick={async () => {
              isDeleting.onTrue();
              await onDeleteRow();
              isDeleting.onFalse();
              confirm.onFalse();
            }}
          >
            {t('delete')}
          </Button>
        }
      />
    </>
  );
}

function ActivityDetailDrawer({
  row,
  canDelete,
  onDeleteRow,
  onClose,
}: {
  row: ActivityRow | null;
  canDelete: boolean;
  onDeleteRow?: () => Promise<void>;
  onClose: VoidFunction;
}) {
  const { t } = useTranslate();
  const confirm = useBoolean();
  const isDeleting = useBoolean();

  const invoice = row?.transaction_type === TransactionType.INVOICE ? (row as Invoice) : null;
  const payment = row?.transaction_type === TransactionType.PAYMENT ? (row as Payment) : null;
  const invoiceOutpoint = invoice?.bitcoin_output?.outpoint;
  const explorerUrl = txExplorerUrl(payment?.bitcoin?.txid || txidFromOutpoint(invoiceOutpoint));
  const methodLabel = getLedgerLabel(row?.ledger, t);
  const isIncoming = row?.direction === 'in';
  const amountOnly = row?.amount_msat || 0;
  const feeAmount = row?.fee_msat || 0;
  const totalAmount = row?.amount_total_msat || 0;
  const isOpenAmount = row ? isOpenAmountRequest(row) : false;
  const bitcoinDestination =
    payment?.internal?.btc_address || payment?.bitcoin?.address || invoice?.bitcoin_output?.address;
  const destination =
    payment?.lightning?.ln_address ||
    payment?.internal?.ln_address ||
    bitcoinDestination ||
    payment?.bitcoin?.txid ||
    invoice?.ln_invoice?.bolt11;
  const destinationLabel =
    destination === bitcoinDestination && destination
      ? compactBitcoinAddress(destination)
      : destination;

  return (
    <Drawer
      anchor="right"
      open={!!row}
      onClose={onClose}
      transitionDuration={{ enter: 225, exit: 195 }}
      ModalProps={{ keepMounted: true }}
      slotProps={{ paper: { sx: drawerSx } }}
    >
      {row && (
        <>
          <Stack
            direction="row"
            spacing={2}
            sx={{ alignItems: 'center', justifyContent: 'space-between', px: 3, py: 2 }}
          >
            <Stack sx={{ minWidth: 0 }}>
              <Typography variant="h6" noWrap>
                {row.description_label}
              </Typography>
              <Typography variant="caption" color="text.secondary">
                {row.direction === 'in' ? t('activity_view.received') : t('activity_view.sent')} ·{' '}
                {methodLabel}
              </Typography>
            </Stack>

            <IconButton onClick={onClose} aria-label={t('close')}>
              <Iconify icon="mingcute:close-line" />
            </IconButton>
          </Stack>

          <Divider />

          <Stack spacing={3} sx={{ p: 3 }}>
            <Box
              sx={[
                (theme) => ({
                  p: 2,
                  borderRadius: 1,
                  bgcolor: 'background.neutral',
                  border: `1px solid ${theme.vars.palette.divider}`,
                }),
              ]}
            >
              <Stack spacing={1.5}>
                <Stack
                  direction="row"
                  sx={{ alignItems: 'center', justifyContent: 'space-between' }}
                >
                  <Label color={isIncoming ? 'success' : 'warning'}>
                    {isIncoming ? t('wallet_view.direction_in') : t('wallet_view.direction_out')}
                  </Label>
                  <Label variant="soft" color={statusColor(row.status)}>
                    {row.status}
                  </Label>
                </Stack>

                {isOpenAmount ? (
                  <Typography variant="h4">{t('wallet_view.open_amount')}</Typography>
                ) : (
                  <SatsWithIcon amountMSats={totalAmount} variant="h4" sx={{ fontWeight: 400 }} />
                )}
              </Stack>
            </Box>

            <Grid container spacing={{ xs: 1, sm: 1.5 }}>
              <Grid size={{ xs: 4 }}>
                <ActivityDrawerMetric
                  label={t('wallet_view.amount')}
                  amountMSats={isOpenAmount ? undefined : amountOnly}
                  value={isOpenAmount ? t('wallet_view.open_amount') : undefined}
                />
              </Grid>
              <Grid size={{ xs: 4 }}>
                <ActivityDrawerMetric
                  label={t('wallet_view.fee')}
                  amountMSats={feeAmount}
                  showMillisatsTooltip
                />
              </Grid>
              <Grid size={{ xs: 4 }}>
                <ActivityDrawerMetric
                  label={t('wallet_view.total')}
                  amountMSats={isOpenAmount ? undefined : totalAmount}
                  value={isOpenAmount ? t('wallet_view.open_amount') : undefined}
                />
              </Grid>
            </Grid>

            <Stack spacing={1.5}>
              <ActivityDrawerRow label={t('wallet_view.rail')} value={methodLabel} />
              <ActivityDrawerRow
                label={t('wallet_view.created')}
                value={fDateTime(row.created_at)}
              />
              <ActivityDrawerRow
                label={t('wallet_view.settled')}
                value={
                  row.payment_time ? fDateTime(row.payment_time) : t('wallet_view.not_settled')
                }
              />
              {destination && (
                <ActionCopyButton
                  label={t('wallet_view.destination')}
                  value={destination}
                  displayValue={destinationLabel || destination}
                  copyLabel={t('transaction_actions.copy_destination')}
                />
              )}
              <ActionCopyButton
                label={t('wallet_view.transaction_id')}
                value={row.id}
                displayValue={compactIdentifier(row.id)}
                copyLabel={t('activity_view.copy_transaction_id')}
              />
            </Stack>

            {explorerUrl && (
              <Button
                component="a"
                href={explorerUrl}
                target="_blank"
                rel="noopener noreferrer"
                color="inherit"
                variant="outlined"
                startIcon={<Iconify icon="solar:map-arrow-right-bold" />}
              >
                {t('transaction_actions.open_explorer')}
              </Button>
            )}

            <Button
              component="a"
              href={row.detail_page_href}
              color="inherit"
              variant="outlined"
              startIcon={<Iconify icon="solar:bill-list-bold-duotone" />}
            >
              {t('wallet_view.open_details')}
            </Button>

            {canDelete && onDeleteRow && (
              <Button
                color="error"
                variant="outlined"
                onClick={confirm.onTrue}
                startIcon={<Iconify icon="solar:trash-bin-trash-bold" />}
              >
                {t('delete')}
              </Button>
            )}
          </Stack>

          <ConfirmDialog
            open={confirm.value}
            onClose={confirm.onFalse}
            title={t('delete')}
            content={t('confirm_delete')}
            action={
              <Button
                variant="contained"
                color="error"
                loading={isDeleting.value}
                onClick={async () => {
                  if (!onDeleteRow) return;
                  isDeleting.onTrue();
                  await onDeleteRow();
                  isDeleting.onFalse();
                  confirm.onFalse();
                }}
              >
                {t('delete')}
              </Button>
            }
          />
        </>
      )}
    </Drawer>
  );
}

function ActivityDrawerRow({
  label,
  value,
  mono,
}: {
  label: string;
  value?: string | null;
  mono?: boolean;
}) {
  if (!value) return null;

  return (
    <Stack direction="row" spacing={1.5} sx={{ justifyContent: 'space-between', minWidth: 0 }}>
      <Typography variant="body2" color="text.secondary">
        {label}
      </Typography>
      <Typography
        variant="body2"
        sx={{
          maxWidth: '68%',
          textAlign: 'right',
          wordBreak: 'break-word',
          fontFamily: mono ? 'monospace' : undefined,
        }}
      >
        {value}
      </Typography>
    </Stack>
  );
}

function ActivityDrawerMetric({
  label,
  amountMSats,
  value,
  showMillisatsTooltip = false,
}: {
  label: string;
  amountMSats?: number;
  value?: string;
  showMillisatsTooltip?: boolean;
}) {
  return (
    <Box
      sx={[
        (theme) => ({
          p: 1.5,
          height: 1,
          borderRadius: 1,
          bgcolor: 'background.neutral',
          border: `1px solid ${theme.vars.palette.divider}`,
        }),
      ]}
    >
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      {value ? (
        <Typography variant="subtitle2">{value}</Typography>
      ) : (
        <SatsWithIcon
          amountMSats={amountMSats ?? 0}
          variant="subtitle2"
          showMillisatsTooltip={showMillisatsTooltip}
        />
      )}
    </Box>
  );
}

function ActionCopyButton({
  value,
  displayValue,
  label,
  copyLabel,
}: {
  value: string;
  displayValue?: string;
  label: string;
  copyLabel: string;
}) {
  const { t } = useTranslate();
  const { copy } = useCopyToClipboard();

  const handleCopy = async () => {
    if (await copy(value)) {
      toast.success(t('copied_to_clipboard'));
    }
  };

  return (
    <ButtonBase
      type="button"
      title={copyLabel}
      aria-label={copyLabel}
      onClick={handleCopy}
      sx={[
        (theme) => ({
          py: 0.5,
          gap: 1.5,
          width: 1,
          display: 'flex',
          minWidth: 0,
          borderRadius: 0.75,
          textAlign: 'left',
          alignItems: 'center',
          justifyContent: 'space-between',
          transition: theme.transitions.create('background-color'),
          '&:hover': {
            bgcolor: 'action.hover',
          },
        }),
      ]}
    >
      <Typography variant="body2" color="text.secondary">
        {label}
      </Typography>
      <Stack direction="row" spacing={0.75} sx={{ alignItems: 'center', minWidth: 0 }}>
        <Typography
          variant="body2"
          sx={{ typography: 'caption', fontFamily: 'monospace', color: 'text.primary' }}
          noWrap
        >
          {displayValue || value}
        </Typography>
        <Iconify icon="eva:copy-fill" width={16} sx={{ flexShrink: 0, color: 'text.disabled' }} />
      </Stack>
    </ButtonBase>
  );
}
