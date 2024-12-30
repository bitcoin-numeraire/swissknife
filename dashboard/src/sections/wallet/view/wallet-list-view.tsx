'use client';

import type { TFunction } from 'i18next';

import { mutate } from 'swr';
import { useBoolean } from 'minimal-shared/hooks';

import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';

import { shouldFail } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { Permission } from 'src/lib/swissknife';
import { DashboardContent } from 'src/layouts/dashboard';
import { useListWalletOverviews } from 'src/actions/wallet';
import { useFetchFiatPrices } from 'src/actions/mempool-space';

import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error/error-view';
import { RegisterWalletDialog } from 'src/components/wallet';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { RoleBasedGuard } from 'src/auth/guard';

import { WalletList } from '../wallet-list';

// ----------------------------------------------------------------------

const tableHead = (t: TFunction) => [
  { id: 'user_id', label: t('wallet_list.user') },
  { id: 'ln_address_username', label: t('wallet_list.ln_address') },
  { id: 'balance.available_msat', label: t('wallet_list.balance') },
  { id: 'n_invoices', label: t('wallet_list.invoices') },
  { id: 'n_payments', label: t('wallet_list.payments') },
  { id: 'n_contacts', label: t('wallet_list.contacts') },
  { id: 'created_at', label: t('wallet_list.created') },
  { id: '' },
];

// ----------------------------------------------------------------------

export function WalletListView() {
  const newWallet = useBoolean();
  const { t } = useTranslate();

  const { walletOverviews, walletOverviewsLoading, walletOverviewsError } =
    useListWalletOverviews();
  const { fiatPrices, fiatPricesLoading, fiatPricesError } = useFetchFiatPrices();

  const errors = [walletOverviewsError, fiatPricesError];
  const data = [walletOverviews, fiatPrices];
  const isLoading = [walletOverviewsLoading, fiatPricesLoading];

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent>
      <RoleBasedGuard permissions={[Permission['READ:WALLET']]} hasContent>
        {failed ? (
          <ErrorView errors={errors} isLoading={isLoading} data={data} />
        ) : (
          <>
            <CustomBreadcrumbs
              heading={t('wallets')}
              links={[
                {
                  name: t('admin'),
                },
                {
                  name: t('wallets'),
                },
              ]}
              action={
                <Stack direction="row" spacing={1}>
                  <Button
                    onClick={newWallet.onTrue}
                    variant="contained"
                    startIcon={<Iconify icon="mingcute:add-line" />}
                  >
                    {t('new')}
                  </Button>
                </Stack>
              }
              sx={{
                mb: { xs: 3, md: 5 },
              }}
            />

            <WalletList data={walletOverviews!} tableHead={tableHead(t)} fiatPrices={fiatPrices!} />

            <RegisterWalletDialog
              open={newWallet.value}
              onClose={newWallet.onFalse}
              onSuccess={() => mutate(endpointKeys.wallets.listOverviews)}
            />
          </>
        )}
      </RoleBasedGuard>
    </DashboardContent>
  );
}
