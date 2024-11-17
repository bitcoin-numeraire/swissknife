import { appTitle } from 'src/utils/format-string';

import { InvoiceDetailsView } from 'src/sections/transaction/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Invoice Details'),
};

type Props = {
  params: {
    id: string;
  };
};

export default function InvoiceDetailsPage({ params }: Props) {
  const { id } = params;

  return <InvoiceDetailsView id={id} />;
}
