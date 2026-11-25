import { appTitle } from 'src/utils/format-string';

import { MerchantCheckoutView } from 'src/sections/wallet/view/merchant-checkout-view';

export const metadata = { title: appTitle('Point of sale') };

export default function MerchantCheckoutPage() {
  return <MerchantCheckoutView />;
}
