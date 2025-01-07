import { SimpleLayout } from 'src/layouts/simple';

import { GuestGuard } from 'src/auth/guard';

// ----------------------------------------------------------------------

type Props = {
  children: React.ReactNode;
};

export default function Layout({ children }: Props) {
  return (
    <GuestGuard>
      <SimpleLayout>{children}</SimpleLayout>
    </GuestGuard>
  );
}
