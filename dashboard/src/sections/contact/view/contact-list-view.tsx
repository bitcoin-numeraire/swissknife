'use client';

import { paths } from 'src/routes/paths';

import { shouldFail } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { DashboardContent } from 'src/layouts/dashboard';
import { useFetchFiatPrices } from 'src/actions/mempool-space';
import { useListWalletContacts } from 'src/actions/user-wallet';

import { ErrorView } from 'src/components/error/error-view';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { ContactList } from '../contact-list';

// ----------------------------------------------------------------------

export function ContactListView() {
  const { t } = useTranslate();

  const { contacts, contactsLoading, contactsError } = useListWalletContacts();
  const { fiatPrices, fiatPricesLoading, fiatPricesError } = useFetchFiatPrices();

  const errors = [contactsError, fiatPricesError];
  const data = [contacts, fiatPrices];
  const isLoading = [contactsLoading, fiatPricesLoading];

  const failed = shouldFail(errors, data, isLoading);

  return (
    <DashboardContent>
      {failed ? (
        <ErrorView errors={errors} isLoading={isLoading} data={data} />
      ) : (
        <>
          <CustomBreadcrumbs
            heading={t('contacts')}
            links={[
              {
                name: t('wallet'),
                href: paths.wallet.root,
              },
              {
                name: t('contacts'),
              },
            ]}
            sx={{
              mb: { xs: 3, md: 5 },
            }}
          />

          <ContactList data={contacts!} fiatPrices={fiatPrices!} />
        </>
      )}
    </DashboardContent>
  );
}
