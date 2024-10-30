import { appTitle } from 'src/utils/format-string';

import { AdminInvoiceDetailsView } from 'src/sections/transaction/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Invoice Details'),
};

type Props = {
  params: {
    id: string;
  };
};

export default function AdminInvoiceDetailsPage({ params }: Props) {
  const { id } = params;

  return <AdminInvoiceDetailsView id={id} />;
}
