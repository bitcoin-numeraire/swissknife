import { appTitle } from 'src/utils/format-string';

import { PaymentsView } from 'src/sections/transaction/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Payments'),
};

export default function PaymentListPage() {
  return <PaymentsView />;
}
