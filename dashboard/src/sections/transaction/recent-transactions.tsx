import type { TFunction } from 'i18next';
import type { CardProps } from '@mui/material/Card';
import type { ITransaction } from 'src/types/transaction';
import type { InvoiceResponse } from 'src/lib/swissknife';

import { mutate } from 'swr';
import Link from 'next/link';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Table from '@mui/material/Table';
import Avatar from '@mui/material/Avatar';
import Divider from '@mui/material/Divider';
import MenuItem from '@mui/material/MenuItem';
import TableRow from '@mui/material/TableRow';
import TableCell from '@mui/material/TableCell';
import TableBody from '@mui/material/TableBody';
import IconButton from '@mui/material/IconButton';
import CardHeader from '@mui/material/CardHeader';
import ListItemText from '@mui/material/ListItemText';
import Badge, { badgeClasses } from '@mui/material/Badge';
import TableContainer from '@mui/material/TableContainer';
import { Stack, Tooltip, MenuList, Typography, Link as MuiLink } from '@mui/material';

import { paths } from 'src/routes/paths';

import { fFromNow } from 'src/utils/format-time';
import { truncateText } from 'src/utils/format-string';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CopyMenuItem } from 'src/components/copy';
import { Scrollbar } from 'src/components/scrollbar';
import { SatsWithIcon } from 'src/components/bitcoin';
import { TableHeadCustom } from 'src/components/table';
import { CleanTransactionsButton } from 'src/components/transactions';
import { usePopover, CustomPopover } from 'src/components/custom-popover';

import { TransactionType } from 'src/types/transaction';

// ----------------------------------------------------------------------

const tableLabels = (t: TFunction) => [
  { id: 'description', label: t('transaction_list.description') },
  { id: 'amount_msat', label: t('transaction_list.amount') },
  { id: 'status', label: t('transaction_list.status') },
  { id: '' },
];

const adminTableLabels = (t: TFunction) => [
  { id: 'description', label: t('transaction_list.description') },
  { id: 'wallet_id', label: t('transaction_list.wallet') },
  { id: 'amount_msat', label: t('transaction_list.amount') },
  { id: 'status', label: t('transaction_list.status') },
  { id: '' },
];

const labels = (t: TFunction, isAdmin?: boolean) => (isAdmin ? adminTableLabels(t) : tableLabels(t));

interface Props extends CardProps {
  title?: string;
  tableData: ITransaction[];
  isAdmin?: boolean;
}

export function RecentTransactions({ title, tableData, isAdmin, ...other }: Props) {
  const { t } = useTranslate();

  return (
    <Card {...other}>
      <CardHeader
        title={title || t('recent_transactions.title')}
        subheader={t('recent_transactions.subheader', { count: tableData.length })}
        sx={{ mb: 3 }}
      />

      <TableContainer sx={{ overflow: 'unset' }}>
        <Scrollbar>
          <Table sx={{ minWidth: 720 }}>
            <TableHeadCustom headLabel={labels(t, isAdmin)} />

            <TableBody>
              {tableData.map((row) => (
                <RecentTransactionsRow key={row.id} row={row} isAdmin={isAdmin} />
              ))}
            </TableBody>
          </Table>
        </Scrollbar>
      </TableContainer>

      {!isAdmin && (
        <>
          <Divider />

          <Stack direction="row" spacing={2} justifyContent="flex-end" sx={{ p: 2 }}>
            <Tooltip title={t('recent_transactions.clean_failed_expired')} placement="top" arrow>
              <Box>
                <CleanTransactionsButton
                  onSuccess={() => mutate(endpointKeys.userWallet.get)}
                  buttonProps={{
                    size: 'small',
                    color: 'error',
                    variant: 'outlined',
                    endIcon: <Iconify icon="solar:trash-bin-trash-bold" width={18} />,
                  }}
                >
                  {t('clean')}
                </CleanTransactionsButton>
              </Box>
            </Tooltip>
          </Stack>
        </>
      )}
    </Card>
  );
}

// ----------------------------------------------------------------------

type RecentTransactionsRowProps = {
  row: ITransaction;
  isAdmin?: boolean;
};

function RecentTransactionsRow({ row, isAdmin }: RecentTransactionsRowProps) {
  const { t } = useTranslate();
  const { id, wallet_id, amount_msat, transaction_type, status, description, payment_time, fee_msat, created_at } = row;

  const popover = usePopover();

  const rowHref = (): string => {
    if (transaction_type === TransactionType.PAYMENT) {
      return isAdmin ? paths.admin.payment(id) : paths.wallet.payment(id);
    }

    return isAdmin ? paths.admin.invoice(id) : paths.wallet.invoice(id);
  };

  return (
    <>
      <TableRow>
        <TableCell sx={{ display: 'flex', alignItems: 'center' }}>
          <Box sx={{ position: 'relative', mr: 2 }}>
            {transaction_type === TransactionType.PAYMENT ? (
              <Badge
                overlap="circular"
                color="error"
                anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
                badgeContent={<Iconify icon="eva:diagonal-arrow-right-up-fill" width={16} />}
                sx={{
                  [`& .${badgeClasses.badge}`]: {
                    p: 0,
                    width: 20,
                  },
                }}
              >
                <Avatar
                  sx={{
                    width: 48,
                    height: 48,
                    color: 'text.secondary',
                    bgcolor: 'background.neutral',
                  }}
                />
              </Badge>
            ) : (
              <Iconify
                icon="eva:diagonal-arrow-left-down-fill"
                sx={{
                  p: 1.5,
                  width: 48,
                  height: 48,
                  borderRadius: '50%',
                  color: 'success.main',
                  bgcolor: 'background.neutral',
                }}
              />
            )}
          </Box>
          <ListItemText
            disableTypography
            primary={
              <MuiLink href={rowHref()} sx={{ color: 'text.primary', cursor: 'pointer' }}>
                <Typography variant="body2" sx={{ whiteSpace: 'normal', wordWrap: 'break-word' }}>
                  {description || t('recent_transactions.empty_description')}
                </Typography>
              </MuiLink>
            }
            secondary={
              <Typography variant="body2" noWrap sx={{ color: 'text.disabled' }}>
                {fFromNow(payment_time || created_at)}
              </Typography>
            }
          />
        </TableCell>

        {isAdmin && (
          <TableCell>
            <Typography variant="body2" noWrap>
              {truncateText(wallet_id, 10)}
            </Typography>
          </TableCell>
        )}

        <TableCell>
          <SatsWithIcon amountMSats={(amount_msat || 0) + (fee_msat || 0)} />
        </TableCell>

        <TableCell>
          <Label variant="soft" color={(status === 'Settled' && 'success') || (status === 'Pending' && 'warning') || 'error'}>
            {status}
          </Label>
        </TableCell>

        <TableCell align="right" sx={{ pr: 1 }}>
          <IconButton color={popover.open ? 'inherit' : 'default'} onClick={popover.onOpen}>
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
          <Link href={rowHref()} passHref legacyBehavior>
            <MenuItem>
              <Iconify icon="eva:eye-fill" />
              {t('details')}
            </MenuItem>
          </Link>
        </MenuList>

        {transaction_type === TransactionType.INVOICE && status === 'Pending' && (
          <CopyMenuItem value={(row as InvoiceResponse).ln_invoice?.bolt11!} />
        )}
      </CustomPopover>
    </>
  );
}
