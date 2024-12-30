'use client';

import type { LabelColor } from 'src/components/label';
import type { ITransaction, ITransactionTableFilters } from 'src/types/transaction';

import { sumBy } from 'es-toolkit';
import { useState, useEffect, useCallback } from 'react';
import { useBoolean, useSetState } from 'minimal-shared/hooks';

import Tab from '@mui/material/Tab';
import Tabs from '@mui/material/Tabs';
import Card from '@mui/material/Card';
import Table from '@mui/material/Table';
import Stack from '@mui/material/Stack';
import { LoadingButton } from '@mui/lab';
import Divider from '@mui/material/Divider';
import Tooltip from '@mui/material/Tooltip';
import TableBody from '@mui/material/TableBody';
import IconButton from '@mui/material/IconButton';
import { alpha, useTheme } from '@mui/material/styles';
import TableContainer from '@mui/material/TableContainer';

import { LEDGERS } from 'src/utils/transactions';
import { handleActionError } from 'src/utils/errors';
import { fIsAfter, fIsBetween } from 'src/utils/format-time';

import { useTranslate } from 'src/locales';
import { deleteInvoice, deletePayment, deleteInvoices, deletePayments } from 'src/lib/swissknife';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { Scrollbar } from 'src/components/scrollbar';
import { ItemAnalytic } from 'src/components/analytic';
import { ConfirmDialog } from 'src/components/custom-dialog';
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

import { TransactionType } from 'src/types/transaction';

import { TransactionTableRow } from './transaction-table-row';
import { TransactionTableToolbar } from './transaction-table-toolbar';
import { TransactionTableFiltersResult } from './transaction-table-filters-result';

// ----------------------------------------------------------------------

type Props = {
  transactionType: TransactionType;
  data: ITransaction[];
  tableHead: TableHeadProps[];
  tabs: TabsProps[];
  isAdmin?: boolean;
  href: (id: string) => string;
};

type TabsProps = {
  title: string;
  value: string;
  label: string;
  color: LabelColor;
  suffix: string;
  icon: string;
  analyticColor: string;
};

type TableHeadProps = { id: string; label?: string };

export function TransactionList({
  data: transactions,
  isAdmin,
  tableHead,
  tabs,
  href,
  transactionType,
}: Props) {
  const { t } = useTranslate();
  const theme = useTheme();
  const table = useTable({ defaultOrderBy: 'created_at', defaultRowsPerPage: 25 });
  const confirm = useBoolean();
  const isDeleting = useBoolean();

  const [tableData, setTableData] = useState(transactions);

  const filters = useSetState<ITransactionTableFilters>({
    name: '',
    ledger: [],
    status: 'all',
    startDate: null,
    endDate: null,
  });

  const dateError = fIsAfter(filters.state.startDate, filters.state.endDate);

  const dataFiltered = applyFilter({
    inputData: tableData,
    comparator: getComparator(table.order, table.orderBy),
    filters: filters.state,
    dateError,
  });

  const dataInPage = dataFiltered.slice(
    table.page * table.rowsPerPage,
    table.page * table.rowsPerPage + table.rowsPerPage
  );
  const denseHeight = table.dense ? 56 : 56 + 20;
  const canReset =
    !!filters.state.name ||
    !!filters.state.ledger.length ||
    filters.state.status !== 'all' ||
    (!!filters.state.startDate && !!filters.state.endDate);
  const notFound = (!dataFiltered.length && canReset) || !dataFiltered.length;

  useEffect(() => {
    setTableData(transactions);
  }, [transactions]);

  const handleDeleteRow = useCallback(
    async (id: string) => {
      const deleteRow = tableData.filter((row) => row.id !== id);

      try {
        if (transactionType === TransactionType.INVOICE) {
          await deleteInvoice({ path: { id } });
        } else {
          await deletePayment({ path: { id } });
        }

        toast.success(t('transaction_list.delete_success'));
        setTableData(deleteRow);
        table.onUpdatePageDeleteRow(dataInPage.length);
      } catch (error) {
        handleActionError(error);
      }
    },
    [dataInPage.length, table, tableData, transactionType, t]
  );

  const handleDeleteRows = useCallback(async () => {
    const deleteRows = tableData.filter((row) => !table.selected.includes(row.id));

    try {
      const { data } =
        transactionType === TransactionType.INVOICE
          ? await deleteInvoices({ query: { ids: table.selected } })
          : await deletePayments({ query: { ids: table.selected } });

      toast.success(t('transaction_list.transactions_deleted', { count: data }));
      setTableData(deleteRows);
      table.onUpdatePageDeleteRows(dataInPage.length, dataFiltered.length);
    } catch (error) {
      handleActionError(error);
    }
  }, [dataFiltered.length, dataInPage.length, table, tableData, transactionType, t]);

  const handleFilterStatus = useCallback(
    (event: React.SyntheticEvent, newValue: string) => {
      table.onResetPage();
      filters.setState({ status: newValue });
    },
    [filters, table]
  );

  const getTransactionLength = (status: string) => {
    if (status === 'all') {
      return tableData.length;
    }

    return tableData.filter((item) => item.status === status).length;
  };

  const getTotalAmount = (status: string) => {
    if (status === 'all') {
      return sumBy(tableData, (tx) => (tx.amount_msat || 0) + (tx.fee_msat || 0));
    }

    return sumBy(
      tableData.filter((item) => item.status === status),
      (tx) => (tx.amount_msat || 0) + (tx.fee_msat || 0)
    );
  };

  const getPercentByStatus = (status: string) =>
    (getTransactionLength(status) / tableData.length) * 100;

  return (
    <>
      <Card
        sx={{
          mb: { xs: 3, md: 5 },
        }}
      >
        <Scrollbar>
          <Stack
            direction="row"
            divider={<Divider orientation="vertical" flexItem sx={{ borderStyle: 'dashed' }} />}
            sx={{ py: 2 }}
          >
            {tabs.map((tab) => (
              <ItemAnalytic
                key={tab.title}
                title={tab.title}
                total={getTransactionLength(tab.value)}
                percent={getPercentByStatus(tab.value)}
                price={getTotalAmount(tab.value)}
                icon={tab.icon}
                color={tab.analyticColor}
                countSuffix={tab.suffix}
              />
            ))}
          </Stack>
        </Scrollbar>
      </Card>

      <Card>
        <Tabs
          value={filters.state.status}
          onChange={handleFilterStatus}
          sx={{
            px: 2.5,
            boxShadow: `inset 0 -2px 0 0 ${alpha(theme.palette.grey[500], 0.08)}`,
          }}
        >
          {tabs.map((tab) => (
            <Tab
              key={tab.value}
              value={tab.value}
              label={tab.label}
              iconPosition="end"
              icon={
                <Label
                  variant={
                    ((tab.value === 'all' || tab.value === filters.state.status) && 'filled') ||
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
          <TableSelectedAction
            dense={table.dense}
            numSelected={table.selected.length}
            rowCount={dataFiltered.length}
            onSelectAllRows={(checked) => {
              table.onSelectAllRows(
                checked,
                dataFiltered.map((row) => row.id)
              );
            }}
            action={
              <Stack direction="row">
                <Tooltip title={t('download')}>
                  <IconButton color="primary" onClick={() => toast.info(t('coming_soon'))}>
                    <Iconify icon="eva:download-outline" />
                  </IconButton>
                </Tooltip>

                <Tooltip title={t('print')}>
                  <IconButton color="primary" onClick={() => toast.info(t('coming_soon'))}>
                    <Iconify icon="solar:printer-minimalistic-bold" />
                  </IconButton>
                </Tooltip>

                {isAdmin && (
                  <Tooltip title={t('delete')}>
                    <IconButton color="primary" onClick={confirm.onTrue}>
                      <Iconify icon="solar:trash-bin-trash-bold" />
                    </IconButton>
                  </Tooltip>
                )}
              </Stack>
            }
          />

          <Scrollbar>
            <Table size={table.dense ? 'small' : 'medium'} sx={{ minWidth: 800 }}>
              <TableHeadCustom
                order={table.order}
                orderBy={table.orderBy}
                headCells={tableHead}
                rowCount={dataFiltered.length}
                numSelected={table.selected.length}
                onSort={table.onSort}
                onSelectAllRows={(checked) =>
                  table.onSelectAllRows(
                    checked,
                    dataFiltered.map((row) => row.id)
                  )
                }
              />

              <TableBody>
                {dataFiltered
                  .slice(
                    table.page * table.rowsPerPage,
                    table.page * table.rowsPerPage + table.rowsPerPage
                  )
                  .map((row) => (
                    <TransactionTableRow
                      isAdmin={isAdmin}
                      transactionType={transactionType}
                      key={row.id}
                      row={row}
                      selected={table.selected.includes(row.id)}
                      onSelectRow={() => table.onSelectRow(row.id)}
                      href={href(row.id)}
                      onDeleteRow={() => handleDeleteRow(row.id)}
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

      <ConfirmDialog
        open={confirm.value}
        onClose={confirm.onFalse}
        title={t('confirm_delete_title')}
        content={<>{t('confirm_delete_content', { count: table.selected.length })}</>}
        action={
          <LoadingButton
            variant="contained"
            color="error"
            onClick={async () => {
              isDeleting.onTrue();
              await handleDeleteRows();
              confirm.onFalse();
              isDeleting.onFalse();
            }}
            loading={isDeleting.value}
          >
            {t('delete')}
          </LoadingButton>
        }
      />
    </>
  );
}

// ----------------------------------------------------------------------

type ApplyFilterProps = {
  dateError: boolean;
  inputData: ITransaction[];
  filters: ITransactionTableFilters;
  comparator: (a: any, b: any) => number;
};

function applyFilter({ inputData, comparator, filters, dateError }: ApplyFilterProps) {
  const { name, status, ledger, startDate, endDate } = filters;

  const stabilizedThis = inputData.map((el, index) => [el, index] as const);

  stabilizedThis.sort((a, b) => {
    const order = comparator(a[0], b[0]);
    if (order !== 0) return order;
    return a[1] - b[1];
  });

  inputData = stabilizedThis.map((el) => el[0]);

  if (name) {
    inputData = inputData.filter(
      (tx) =>
        tx.wallet_id.toLowerCase().indexOf(name.toLowerCase()) !== -1 ||
        (tx.description && tx.description.toLowerCase().indexOf(name.toLowerCase()) !== -1) ||
        tx.id.toLowerCase().indexOf(name.toLowerCase()) !== -1
    );
  }

  if (status !== 'all') {
    inputData = inputData.filter((tx) => tx.status === status);
  }

  if (ledger.length) {
    inputData = inputData.filter((tx) => ledger.includes(tx.ledger));
  }

  if (!dateError) {
    if (startDate && endDate) {
      inputData = inputData.filter((tx) => fIsBetween(tx.created_at, startDate, endDate));
    }
  }

  return inputData;
}
