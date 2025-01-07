import { AuthSplitLayout } from 'src/layouts/auth-split';

import { OnboardingGuard } from 'src/auth/guard/onboarding-guard';

// ----------------------------------------------------------------------

type Props = {
  children: React.ReactNode;
};

export default function Layout({ children }: Props) {
  return (
    <OnboardingGuard>
      <AuthSplitLayout>{children}</AuthSplitLayout>
    </OnboardingGuard>
  );
}
