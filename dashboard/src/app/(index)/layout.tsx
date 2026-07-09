import { CONFIG } from 'src/global-config';
import { AccountProvider } from 'src/contexts/account';
import { DashboardLayout } from 'src/layouts/dashboard';

import { AuthGuard } from 'src/auth/guard';

// ----------------------------------------------------------------------

type Props = {
  children: React.ReactNode;
};

export default function Layout({ children }: Props) {
  if (CONFIG.auth.skip) {
    return (
      <AccountProvider>
        <DashboardLayout>{children}</DashboardLayout>
      </AccountProvider>
    );
  }

  return (
    <AuthGuard>
      <AccountProvider>
        <DashboardLayout>{children}</DashboardLayout>
      </AccountProvider>
    </AuthGuard>
  );
}
