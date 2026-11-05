'use client';

import type { TFunction } from 'i18next';
import type { IFiatPrices } from 'src/types/bitcoin';
import type { LabelColor } from 'src/components/label';
import type { Wallet, Account, Invoice, Payment } from 'src/lib/swissknife';
import type { ITransaction, ITransactionTableFilters } from 'src/types/transaction';

import { mutate } from 'swr';
import { sumBy } from 'es-toolkit';
import { useMemo, useState, useEffect, useCallback } from 'react';
import { useBoolean, usePopover, useSetState } from 'minimal-shared/hooks';

import Tab from '@mui/material/Tab';
import Card from '@mui/material/Card';
import Link from '@mui/material/Link';
import Tabs from '@mui/material/Tabs';
import Alert from '@mui/material/Alert';
import Table from '@mui/material/Table';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
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
import { useRouter, useSearchParams } from 'src/routes/hooks';

import { composeBip21 } from 'src/utils/bitcoin-request';
import { shouldFail, handleActionError } from 'src/utils/errors';
import { fDate, fTime, fIsAfter, fIsBetween } from 'src/utils/format-time';
import {
  LEDGERS,
  getLedgerLabel,
  getCumulativeSeries,
  mergeAndSortTransactions,
} from 'src/utils/transactions';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { useListAccounts } from 'src/actions/account';
import { useListPayments } from 'src/actions/payments';
import { useListInvoices } from 'src/actions/invoices';
import { DashboardContent } from 'src/layouts/dashboard';
import { useActiveWallet } from 'src/actions/account-wallet';
import { useFetchFiatPrices } from 'src/actions/mempool-space';
import { Permission, deleteInvoice, deletePayment } from 'src/lib/swissknife';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { CopyMenuItem } from 'src/components/copy';
import { Scrollbar } from 'src/components/scrollbar';
import { SatsWithIcon } from 'src/components/bitcoin';
import { Chart, useChart } from 'src/components/chart';
import { ItemAnalytic } from 'src/components/analytic';
import { ErrorView } from 'src/components/error/error-view';
import { EmptyContent } from 'src/components/empty-content';
import { ConfirmDialog } from 'src/components/custom-dialog';
import { CustomPopover } from 'src/components/custom-popover';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';
import { CleanTransactionsButton } from 'src/components/transactions';
import { DeleteSelectedTransactionsButton } from 'src/components/transactions/delete-selected-transactions-button';
import {
  useTable,
  emptyRows,
  TableNoData,
  getComparator,
  TableEmptyRows,
  TableHeadCustom,
  TableSelectedAction,
  TablePaginationCustom,
} from 'src/components/table';

import { SendMoneyDrawer, ReceiveMoneyDrawer } from 'src/sections/wallet/money-drawers';
import { TransactionQuickDrawer } from 'src/sections/transaction/transaction-quick-drawer';
import { TransactionTableToolbar } from 'src/sections/transaction/transaction-table-toolbar';
import { TransactionTableFiltersResult } from 'src/sections/transaction/transaction-table-filters-result';

import { RoleBasedGuard } from 'src/auth/guard';
import { useAuthContext } from 'src/auth/hooks';
import { hasAllPermissions } from 'src/auth/permissions';

import { TransactionType } from 'src/types/transaction';

// ----------------------------------------------------------------------

type ActivityLens = 'all' | 'received' | 'paid' | 'Pending' | 'Expired' | 'Failed';
type FlowLens = 'income' | 'expenses';
type ActivityScope = 'wallet' | 'admin';
type ActivityTransactionKind = 'all' | 'payment' | 'invoice';
type ActivityRoute = 'activity' | 'adminTransactions';

type ActivityRow = ITransaction & {
  row_key: string;
  direction: 'in' | 'out';
  detail_page_href: string;
  amount_total_msat: number;
  description_label: string;
  account_label: string;
  wallet_label: string;
  owner?: { account: Account; wallet: Wallet };
};

const fallbackFiatPrices: IFiatPrices = { USD: 0, EUR: 0, CHF: 0 };

function compactId(id: string) {
  return id.length > 20 ? `${id.slice(0, 8)}...${id.slice(-6)}` : id;
}

type ActivityTab = {
  title: string;
  value: ActivityLens;
  label: string;
  color: LabelColor;
  suffix: string;
  icon: string;
  analyticColor: string;
};

const tableHead = (t: TFunction, scope: ActivityScope) => [
  { id: 'direction', label: t('activity_view.direction'), width: 96 },
  { id: 'description_label', label: t('transaction_list.description') },
  ...(scope === 'admin'
    ? [
        { id: 'account_label', label: t('activity_view.account') },
        { id: 'wallet_label', label: t('wallet') },
      ]
    : []),
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
    return scope === 'admin'
      ? paths.admin.transactionInvoiceDetail(tx.id)
      : paths.wallet.invoice(tx.id);
  }

  return scope === 'admin'
    ? paths.admin.transactionPaymentDetail(tx.id)
    : paths.wallet.payment(tx.id);
}

function isOpenAmountRequest(tx: ITransaction) {
  return (
    tx.transaction_type === TransactionType.INVOICE && tx.status === 'Pending' && !tx.amount_msat
  );
}

function makeRow(
  tx: ITransaction,
  scope: ActivityScope,
  owner?: ActivityRow['owner']
): ActivityRow {
  const direction = txDirection(tx);

  return {
    ...tx,
    owner,
    account_label: owner
      ? owner.account.display_name || owner.account.identity?.subject || owner.account.id
      : '',
    wallet_label: owner
      ? owner.wallet.label || owner.wallet.asset?.name || owner.wallet.id
      : tx.wallet_id,
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
        tx.wallet_id.toLowerCase().includes(value) ||
        tx.account_label.toLowerCase().includes(value) ||
        tx.wallet_label.toLowerCase().includes(value) ||
        !!tx.owner?.account.id.toLowerCase().includes(value) ||
        !!tx.owner?.account.identity?.subject.toLowerCase().includes(value) ||
        !!tx.owner?.wallet.ln_address?.username.toLowerCase().includes(value)
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

export function AdminTransactionsView() {
  const searchParams = useSearchParams();
  const id = searchParams.get('id');
  const walletId = searchParams.get('wallet_id');
  const kind = normalizeTransactionKind(searchParams.get('type'));

  return (
    <AdminActivityLedger
      kind={kind}
      initialDetailId={id}
      initialWalletId={walletId}
      route="adminTransactions"
    />
  );
}

function WalletActivityLedger({
  kind,
  initialDetailId,
}: {
  kind: ActivityTransactionKind;
  initialDetailId?: string | null;
}) {
  const sendDrawer = useBoolean();
  const receiveDrawer = useBoolean();
  const { wallet, walletLoading, walletError } = useActiveWallet();
  const { fiatPrices } = useFetchFiatPrices();

  const refreshWallet = () => {
    if (wallet?.id) mutate(endpointKeys.accountWallet.get(wallet.id));
  };

  return (
    <>
      <ActivityLedger
        scope="wallet"
        walletId={wallet?.id}
        kind={kind}
        invoices={wallet?.invoices || []}
        payments={wallet?.payments || []}
        errors={[walletError]}
        data={[wallet]}
        isLoading={[walletLoading]}
        initialDetailId={initialDetailId}
        onSend={sendDrawer.onTrue}
        onReceive={receiveDrawer.onTrue}
        onCleanSuccess={refreshWallet}
      />

      <SendMoneyDrawer
        open={sendDrawer.value}
        contacts={wallet?.contacts ?? []}
        fiatPrices={fiatPrices ?? fallbackFiatPrices}
        balance={wallet?.balance.available_msat}
        onClose={sendDrawer.onFalse}
        onSuccess={refreshWallet}
      />
      <ReceiveMoneyDrawer
        open={receiveDrawer.value}
        fiatPrices={fiatPrices ?? fallbackFiatPrices}
        lnAddress={wallet?.ln_address}
        onClose={receiveDrawer.onFalse}
        onSuccess={refreshWallet}
      />
    </>
  );
}

function AdminActivityLedger({
  kind,
  initialDetailId,
  initialWalletId,
  route = 'activity',
}: {
  kind: ActivityTransactionKind;
  initialDetailId?: string | null;
  initialWalletId?: string | null;
  route?: ActivityRoute;
}) {
  const { user } = useAuthContext();
  const canReadAccounts = hasAllPermissions([Permission.READ_ACCOUNT], user?.permissions);
  const { accounts, accountsError } = useListAccounts(canReadAccounts);
  const { invoices, invoicesLoading, invoicesError } = useListInvoices();
  const { payments, paymentsLoading, paymentsError } = useListPayments();

  return (
    <RoleBasedGuard permissions={[Permission.READ_TRANSACTION]} hasContent>
      <ActivityLedger
        scope="admin"
        accounts={accounts}
        ownershipError={!!accountsError}
        kind={kind}
        invoices={invoices || []}
        payments={payments || []}
        errors={[invoicesError, paymentsError]}
        data={[invoices, payments]}
        isLoading={[invoicesLoading, paymentsLoading]}
        initialDetailId={initialDetailId}
        initialWalletId={initialWalletId}
        route={route}
        onCleanSuccess={() => {
          mutate(endpointKeys.invoices.list);
          mutate(endpointKeys.payments.list);
        }}
      />
    </RoleBasedGuard>
  );
}

function ActivityLedger({
  accounts,
  ownershipError,
  scope,
  walletId,
  kind,
  invoices,
  payments,
  errors,
  data,
  isLoading,
  initialDetailId,
  initialWalletId,
  route = 'activity',
  onSend,
  onReceive,
  onCleanSuccess,
}: {
  scope: ActivityScope;
  accounts?: Account[];
  ownershipError?: boolean;
  walletId?: string;
  kind: ActivityTransactionKind;
  invoices: Invoice[];
  payments: Payment[];
  errors: unknown[];
  data: (object | null | undefined)[];
  isLoading: boolean[];
  initialDetailId?: string | null;
  initialWalletId?: string | null;
  route?: ActivityRoute;
  onSend?: VoidFunction;
  onReceive?: VoidFunction;
  onCleanSuccess: VoidFunction;
}) {
  const { t } = useTranslate();
  const theme = useTheme();
  const { user } = useAuthContext();
  const canWriteTransactions = hasAllPermissions([Permission.WRITE_TRANSACTION], user?.permissions);
  const router = useRouter();
  const table = useTable({
    defaultOrder: 'desc',
    defaultOrderBy: 'created_at',
    defaultRowsPerPage: 25,
  });

  const [flowLens, setFlowLens] = useState<FlowLens>(kind === 'payment' ? 'expenses' : 'income');
  const [detailRow, setDetailRow] = useState<ActivityRow | null>(null);
  const filters = useSetState<ITransactionTableFilters>({
    name: initialWalletId ?? '',
    ledger: [],
    status: 'all',
    startDate: null,
    endDate: null,
  });

  const failed = shouldFail(errors, data, isLoading);
  const dateError = fIsAfter(filters.state.startDate, filters.state.endDate);

  const visibleInvoices = useMemo(() => (kind === 'payment' ? [] : invoices), [invoices, kind]);
  const visiblePayments = useMemo(() => (kind === 'invoice' ? [] : payments), [kind, payments]);

  const owners = useMemo(
    () =>
      new Map(
        (accounts ?? []).flatMap((account) =>
          account.wallets.map((wallet) => [wallet.id, { account, wallet }] as const)
        )
      ),
    [accounts]
  );

  const rows = useMemo(
    () =>
      mergeAndSortTransactions(visibleInvoices, visiblePayments).map((tx) =>
        makeRow(tx, scope, owners.get(tx.wallet_id))
      ),
    [scope, visibleInvoices, visiblePayments, owners]
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

  const dataFiltered = useMemo(
    () =>
      applyFilter({
        inputData: rows,
        comparator: getComparator(table.order, table.orderBy),
        filters: filters.state,
        dateError,
      }),
    [rows, table.order, table.orderBy, filters.state, dateError]
  );

  const selectedRows = dataFiltered.filter((row) => table.selected.includes(row.row_key));
  const { setSelected, setPage } = table;
  useEffect(() => {
    const visible = new Set(dataFiltered.map((row) => row.row_key));
    setSelected((selected) =>
      selected.every((key) => visible.has(key))
        ? selected
        : selected.filter((key) => visible.has(key))
    );
    setPage((page) =>
      Math.min(page, Math.max(0, Math.ceil(dataFiltered.length / table.rowsPerPage) - 1))
    );
  }, [dataFiltered, setSelected, setPage, table.rowsPerPage]);

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
  const isAdminTransactionsRoute = route === 'adminTransactions';
  const currentKind = kind === 'all' ? undefined : kind;
  const listHref = isAdminTransactionsRoute
    ? paths.admin.transactionList(currentKind)
    : paths.activityList(currentKind, scope);

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
          await deleteInvoice<true>({ path: { id: row.id }, throwOnError: true });
        } else {
          await deletePayment<true>({ path: { id: row.id }, throwOnError: true });
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
            heading={isAdminTransactionsRoute ? t('transactions') : t('activity')}
            links={[
              { name: isAdminTransactionsRoute || scope === 'admin' ? t('accounts') : t('money') },
              { name: isAdminTransactionsRoute ? t('transactions') : t('activity') },
            ]}
            action={
              <Stack direction="row" spacing={1} useFlexGap sx={{ flexWrap: 'wrap' }}>
                {onSend && (
                  <Button
                    color="inherit"
                    variant="outlined"
                    onClick={onSend}
                    startIcon={<Iconify icon="solar:arrow-up-linear" />}
                  >
                    {t('wallet_view.send_money')}
                  </Button>
                )}
                {onReceive && (
                  <Button
                    color="inherit"
                    variant="contained"
                    onClick={onReceive}
                    startIcon={<Iconify icon="solar:arrow-down-linear" />}
                  >
                    {t('wallet_view.receive_money')}
                  </Button>
                )}
                <Tooltip
                  title={t('recent_transactions.clean_failed_expired')}
                  placement="top"
                  arrow
                >
                  <span>
                    <CleanTransactionsButton
                      scope={scope}
                      walletId={walletId}
                      onSuccess={onCleanSuccess}
                      transactionType={transactionTypeForKind(kind)}
                      buttonProps={{
                        color: 'error',
                        variant: 'outlined',
                        disabled:
                          !hasFailedOrExpired || (scope === 'admin' && !canWriteTransactions),
                        startIcon: <Iconify icon="solar:trash-bin-trash-bold" />,
                      }}
                    >
                      {t('clean')}
                    </CleanTransactionsButton>
                  </span>
                </Tooltip>
              </Stack>
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

              {ownershipError && (
                <Alert severity="warning" sx={{ m: 2 }}>
                  {t('activity_view.ownership_unavailable')}
                </Alert>
              )}
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
                {scope === 'admin' && canWriteTransactions && (
                  <TableSelectedAction
                    dense={table.dense}
                    numSelected={selectedRows.length}
                    rowCount={dataFiltered.length}
                    onSelectAllRows={(checked) =>
                      table.onSelectAllRows(
                        checked,
                        dataFiltered.map((row) => row.row_key)
                      )
                    }
                    action={
                      <DeleteSelectedTransactionsButton
                        transactions={selectedRows}
                        onDeleted={(completed) => {
                          const keys = new Set(
                            completed.map((tx) => `${tx.transaction_type}-${tx.id}`)
                          );
                          setSelected((selected) => selected.filter((key) => !keys.has(key)));
                          setDetailRow((current) =>
                            current && keys.has(current.row_key) ? null : current
                          );
                          onCleanSuccess();
                        }}
                      />
                    }
                  />
                )}
                <Scrollbar>
                  <Table
                    size={table.dense ? 'small' : 'medium'}
                    sx={{ minWidth: scope === 'admin' ? 1200 : 960 }}
                  >
                    <TableHeadCustom
                      order={table.order}
                      orderBy={table.orderBy}
                      headCells={tableHead(t, scope)}
                      rowCount={dataFiltered.length}
                      numSelected={selectedRows.length}
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
                          canDelete={
                            canDeleteRow(row, scope) && (scope !== 'admin' || canWriteTransactions)
                          }
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

          <TransactionQuickDrawer
            row={detailRow}
            title={detailRow?.description_label}
            detailHref={detailRow?.detail_page_href}
            canDelete={
              !!detailRow &&
              canDeleteRow(detailRow, scope) &&
              (scope !== 'admin' || canWriteTransactions)
            }
            onDeleteRow={detailRow ? () => handleDeleteRow(detailRow) : undefined}
            onClose={() => {
              setDetailRow(null);
              if (initialDetailId) {
                router.push(listHref);
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
            slotProps={{
              primary: { noWrap: true, sx: { typography: 'body2' } },
              secondary: { sx: { typography: 'caption', color: 'text.disabled' } },
            }}
          />
        </TableCell>

        {scope === 'admin' && (
          <>
            <TableCell>
              {row.owner ? (
                <ListItemText
                  primary={
                    <Link
                      href={paths.admin.account(row.owner.account.id)}
                      onClick={(event) => event.stopPropagation()}
                      title={row.owner.account.id}
                    >
                      {row.account_label}
                    </Link>
                  }
                  secondary={compactId(row.owner.account.id)}
                  slotProps={{
                    primary: { sx: { typography: 'body2', overflowWrap: 'anywhere' } },
                    secondary: { sx: { typography: 'caption' } },
                  }}
                />
              ) : (
                <Typography variant="body2" color="text.secondary">
                  {t('activity_view.account_unavailable')}
                </Typography>
              )}
            </TableCell>
            <TableCell>
              <ListItemText
                primary={
                  <Link
                    href={paths.admin.wallet(row.wallet_id)}
                    onClick={(event) => event.stopPropagation()}
                    title={row.wallet_id}
                  >
                    {row.wallet_label}
                  </Link>
                }
                secondary={
                  row.owner
                    ? [
                        row.owner.wallet.asset?.display_ticker,
                        row.owner.wallet.asset?.network,
                        compactId(row.wallet_id),
                      ]
                        .filter(Boolean)
                        .join(' · ')
                    : undefined
                }
                slotProps={{
                  primary: { sx: { typography: 'body2', overflowWrap: 'anywhere' } },
                  secondary: { sx: { typography: 'caption' } },
                }}
              />
            </TableCell>
          </>
        )}

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
