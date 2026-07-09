import type { TFunction } from 'i18next';
import type { CardProps } from '@mui/material/Card';
import type { Invoice } from 'src/lib/swissknife';
import type { ITransaction } from 'src/types/transaction';

import { mutate } from 'swr';
import { usePopover } from 'minimal-shared/hooks';

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
import { useRouter } from 'src/routes/hooks';

import { fFromNow } from 'src/utils/format-time';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { useAccountContext } from 'src/contexts/account';

import { Label } from 'src/components/label';
import { Iconify } from 'src/components/iconify';
import { CopyMenuItem } from 'src/components/copy';
import { Scrollbar } from 'src/components/scrollbar';
import { SatsWithIcon } from 'src/components/bitcoin';
import { TableHeadCustom } from 'src/components/table';
import { CustomPopover } from 'src/components/custom-popover';
import { CleanTransactionsButton } from 'src/components/transactions';

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
  { id: 'wallet_id', label: t('recent_transactions.source') },
  { id: 'amount_msat', label: t('transaction_list.amount') },
  { id: 'status', label: t('transaction_list.status') },
  { id: '' },
];

const labels = (t: TFunction, isAdmin?: boolean) =>
  isAdmin ? adminTableLabels(t) : tableLabels(t);

interface Props extends CardProps {
  title?: string;
  tableData: ITransaction[];
  isAdmin?: boolean;
}

export function RecentTransactions({ title, tableData, isAdmin, ...other }: Props) {
  const { t } = useTranslate();
  const { activeWalletId } = useAccountContext();

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
            <TableHeadCustom headCells={labels(t, isAdmin)} />

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

          <Stack direction="row" spacing={2} sx={{ p: 2, justifyContent: 'flex-end' }}>
            <Tooltip title={t('recent_transactions.clean_failed_expired')} placement="top" arrow>
              <Box>
                <CleanTransactionsButton
                  onSuccess={() => {
                    if (activeWalletId) mutate(endpointKeys.accountWallet.get(activeWalletId));
                  }}
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
  const router = useRouter();
  const {
    id,
    amount_msat,
    transaction_type,
    status,
    description,
    payment_time,
    fee_msat,
    created_at,
  } = row;

  const popover = usePopover();
  const isPayment = transaction_type === TransactionType.PAYMENT;
  const amountColor = isPayment ? 'warning.main' : 'success.main';

  const rowHref = (): string => {
    if (isPayment) {
      return isAdmin ? paths.admin.transactionPayment(id) : paths.activityPayment(id);
    }

    return isAdmin ? paths.admin.transactionInvoice(id) : paths.activityInvoice(id);
  };

  return (
    <>
      <TableRow hover onClick={() => router.push(rowHref())} sx={{ cursor: 'pointer' }}>
        <TableCell sx={{ display: 'flex', alignItems: 'center' }}>
          <Box sx={{ position: 'relative', mr: 2 }}>
            {isPayment ? (
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
              {t('recent_transactions.wallet_account')}
            </Typography>
          </TableCell>
        )}

        <TableCell>
          <Stack direction="row" spacing={0.25} sx={{ alignItems: 'center' }}>
            <Typography component="span" variant="body2" sx={{ color: amountColor }}>
              {isPayment ? '-' : '+'}
            </Typography>
            <SatsWithIcon
              component="span"
              amountMSats={(amount_msat || 0) + (fee_msat || 0)}
              sx={{ color: amountColor }}
            />
          </Stack>
        </TableCell>

        <TableCell>
          <Label
            variant="soft"
            color={
              (status === 'Settled' && 'success') || (status === 'Pending' && 'warning') || 'error'
            }
          >
            {status}
          </Label>
        </TableCell>

        <TableCell align="right" sx={{ pr: 1 }}>
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
              router.push(rowHref());
              popover.onClose();
            }}
          >
            <Iconify icon="eva:eye-fill" />
            {t('details')}
          </MenuItem>

          {transaction_type === TransactionType.INVOICE &&
            status === 'Pending' &&
            (row as Invoice).ln_invoice && (
              <CopyMenuItem value={(row as Invoice).ln_invoice!.bolt11} />
            )}
        </MenuList>
      </CustomPopover>
    </>
  );
}
