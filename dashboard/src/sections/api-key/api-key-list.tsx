'use client';

import type { IApiKeyTableFilters } from 'src/types/apikey';
import type { ApiKeyResponse, ListApiKeysResponse } from 'src/lib/swissknife';

import { useState, useEffect, useCallback } from 'react';
import { useBoolean, useSetState } from 'minimal-shared/hooks';

import Card from '@mui/material/Card';
import Table from '@mui/material/Table';
import Stack from '@mui/material/Stack';
import { LoadingButton } from '@mui/lab';
import Tooltip from '@mui/material/Tooltip';
import TableBody from '@mui/material/TableBody';
import IconButton from '@mui/material/IconButton';
import TableContainer from '@mui/material/TableContainer';

import { handleActionError } from 'src/utils/errors';
import { fIsAfter, fIsBetween } from 'src/utils/format-time';

import { useTranslate } from 'src/locales';
import { revokeApiKey, revokeApiKeys } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { Scrollbar } from 'src/components/scrollbar';
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

import { ApiKeyTableRow } from './api-key-table-row';
import { ApiKeyTableToolbar } from './api-key-table-toolbar';
import { ApiKeyTableFiltersResult } from './api-key-table-filters-result';

// ----------------------------------------------------------------------

type Props = {
  data: ListApiKeysResponse;
  tableHead: TableHeadProps[];
};

type TableHeadProps = { id: string; label?: string };

export function ApiKeyList({ data: wallets, tableHead }: Props) {
  const { t } = useTranslate();
  const table = useTable({ defaultOrderBy: 'created_at', defaultRowsPerPage: 25 });
  const confirm = useBoolean();
  const isDeleting = useBoolean();

  const [tableData, setTableData] = useState(wallets);

  const filters = useSetState<IApiKeyTableFilters>({
    name: '',
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
  const canReset = !!filters.state.name || (!!filters.state.startDate && !!filters.state.endDate);
  const notFound = (!dataFiltered.length && canReset) || !dataFiltered.length;

  useEffect(() => {
    setTableData(wallets);
  }, [wallets]);

  const handleDeleteRow = useCallback(
    async (id: string) => {
      const deleteRow = tableData.filter((row) => row.id !== id);

      try {
        await revokeApiKey({ path: { id } });

        toast.success(t('api_key_list.delete_success'));
        setTableData(deleteRow);
        table.onUpdatePageDeleteRow(dataInPage.length);
      } catch (error) {
        handleActionError(error);
      }
    },
    [dataInPage.length, table, tableData, t]
  );

  const handleDeleteRows = useCallback(async () => {
    const deleteRows = tableData.filter((row) => !table.selected.includes(row.id));

    try {
      const { data } = await revokeApiKeys({ query: { ids: table.selected } });

      toast.success(t('api_key_list.delete_multiple_success', { count: data }));
      setTableData(deleteRows);
      table.onUpdatePageDeleteRows(dataInPage.length, dataFiltered.length);
    } catch (error) {
      handleActionError(error);
    }
  }, [dataFiltered.length, dataInPage.length, table, tableData, t]);

  return (
    <>
      <Card>
        <ApiKeyTableToolbar
          filters={filters}
          onResetPage={table.onResetPage}
          dateError={dateError}
        />

        {canReset && (
          <ApiKeyTableFiltersResult
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

                <Tooltip title={t('delete')}>
                  <IconButton color="primary" onClick={confirm.onTrue}>
                    <Iconify icon="solar:trash-bin-trash-bold" />
                  </IconButton>
                </Tooltip>
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
                    <ApiKeyTableRow
                      key={row.id}
                      row={row}
                      selected={table.selected.includes(row.id)}
                      onSelectRow={() => table.onSelectRow(row.id)}
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
        content={t('confirm_delete_content', { count: table.selected.length })}
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

function applyFilter({
  inputData,
  comparator,
  filters,
  dateError,
}: {
  inputData: ApiKeyResponse[];
  comparator: (a: any, b: any) => number;
  filters: IApiKeyTableFilters;
  dateError: boolean;
}) {
  const { name, startDate, endDate } = filters;

  const stabilizedThis = inputData.map((el, index) => [el, index] as const);

  stabilizedThis.sort((a, b) => {
    const order = comparator(a[0], b[0]);
    if (order !== 0) return order;
    return a[1] - b[1];
  });

  inputData = stabilizedThis.map((el) => el[0]);

  if (name) {
    inputData = inputData.filter(
      (apiKey) =>
        apiKey.user_id.toLowerCase().indexOf(name.toLowerCase()) !== -1 ||
        apiKey.id.toLowerCase().indexOf(name.toLowerCase()) !== -1 ||
        apiKey.name.toLowerCase().indexOf(name.toLowerCase()) !== -1
    );
  }

  if (!dateError) {
    if (startDate && endDate) {
      inputData = inputData.filter((wallet) => fIsBetween(wallet.created_at, startDate, endDate));
    }
  }

  return inputData;
}
