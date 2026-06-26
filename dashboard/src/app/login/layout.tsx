import { AuthCenteredLayout } from 'src/layouts/auth-centered';

import { GuestGuard } from 'src/auth/guard';

// ----------------------------------------------------------------------

type Props = {
  children: React.ReactNode;
};

export default function Layout({ children }: Props) {
  return (
    <GuestGuard>
      <AuthCenteredLayout
        cssVars={{ '--layout-auth-content-width': '440px' }}
        slotProps={{
          content: {
            sx: {
              p: { xs: 3, sm: 4.5 },
              overflow: 'hidden',
              borderRadius: 1,
              border: '1px solid',
              borderColor: 'divider',
              boxShadow: '0 28px 90px rgba(0,0,0,0.28)',
            },
          },
        }}
      >
        {children}
      </AuthCenteredLayout>
    </GuestGuard>
  );
}
