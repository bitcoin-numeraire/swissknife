import { AuthCenteredLayout } from 'src/layouts/auth-centered';

import { OnboardingGuard } from 'src/auth/guard/onboarding-guard';

// ----------------------------------------------------------------------

type Props = {
  children: React.ReactNode;
};

export default function Layout({ children }: Props) {
  return (
    <OnboardingGuard>
      <AuthCenteredLayout>{children}</AuthCenteredLayout>
    </OnboardingGuard>
  );
}
