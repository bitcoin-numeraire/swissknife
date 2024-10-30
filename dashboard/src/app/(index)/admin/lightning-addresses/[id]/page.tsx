import { appTitle } from 'src/utils/format-string';

import { AdminLnAddressDetailsView } from 'src/sections/ln-address/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Lightning Address Details'),
};

type Props = {
  params: {
    id: string;
  };
};

export default function AdminLnAddressDetailsPage({ params }: Props) {
  const { id } = params;

  return <AdminLnAddressDetailsView id={id} />;
}
