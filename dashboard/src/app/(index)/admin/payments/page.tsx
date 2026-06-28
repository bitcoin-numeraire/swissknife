import { redirect } from 'next/navigation';

import { paths } from 'src/routes/paths';

import { appTitle } from 'src/utils/format-string';

import { AdminPaymentDetailsView } from 'src/sections/transaction/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Payment details'),
};

type Props = {
  searchParams?: Promise<{ id?: string | string[] }>;
};

export default async function AdminPaymentPage({ searchParams }: Props) {
  const params = await searchParams;
  const id = Array.isArray(params?.id) ? params.id[0] : params?.id;

  if (!id) {
    redirect(paths.admin.transactionList('payment'));
  }

  return <AdminPaymentDetailsView id={id} />;
}
