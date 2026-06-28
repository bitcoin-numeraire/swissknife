import { redirect } from 'next/navigation';

import { paths } from 'src/routes/paths';

import { appTitle } from 'src/utils/format-string';

import { AdminInvoiceDetailsView } from 'src/sections/transaction/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Invoice details'),
};

type Props = {
  searchParams?: Promise<{ id?: string | string[] }>;
};

export default async function AdminInvoicePage({ searchParams }: Props) {
  const params = await searchParams;
  const id = Array.isArray(params?.id) ? params.id[0] : params?.id;

  if (!id) {
    redirect(paths.admin.transactionList('invoice'));
  }

  return <AdminInvoiceDetailsView id={id} />;
}
