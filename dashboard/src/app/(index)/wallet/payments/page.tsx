import { appTitle } from 'src/utils/format-string';

import { LegacyActivityRedirect } from 'src/sections/transaction/legacy-activity-redirect';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Payments'),
};

export default function PaymentListPage() {
  return <LegacyActivityRedirect kind="payment" />;
}
