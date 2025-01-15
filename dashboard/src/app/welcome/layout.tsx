import { SimpleLayout } from 'src/layouts/simple';

import { OnboardingGuard } from 'src/auth/guard/onboarding-guard';

// ----------------------------------------------------------------------

type Props = {
  children: React.ReactNode;
};

export default function Layout({ children }: Props) {
  return (
    <OnboardingGuard>
      <SimpleLayout>{children}</SimpleLayout>
    </OnboardingGuard>
  );
}
