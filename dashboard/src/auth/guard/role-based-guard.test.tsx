import { it, vi, expect, describe } from 'vitest';
import { render, screen } from '@testing-library/react';

import { Permission } from 'src/lib/swissknife';

import { RoleBasedGuard } from './role-based-guard';
import { AuthContext } from '../context/auth-context';

vi.mock('src/assets/illustrations', () => ({
  ForbiddenIllustration: () => <div data-testid="forbidden-illustration" />,
}));

function renderGuard({
  permissions,
  requiredPermissions,
  hasContent,
}: {
  permissions: string[];
  requiredPermissions: string[];
  hasContent?: boolean;
}) {
  return render(
    <AuthContext.Provider
      value={{
        user: { permissions },
        loading: false,
        authenticated: true,
        unauthenticated: false,
      }}
    >
      <RoleBasedGuard permissions={requiredPermissions} hasContent={hasContent}>
        <span>Protected content</span>
      </RoleBasedGuard>
    </AuthContext.Provider>
  );
}

describe('RoleBasedGuard', () => {
  it('renders children when the user has every required permission', () => {
    renderGuard({
      permissions: [Permission.READ_WALLET, Permission.WRITE_WALLET],
      requiredPermissions: [Permission.READ_WALLET],
    });

    expect(screen.getByText('Protected content')).toBeInTheDocument();
  });

  it('hides children when permissions are missing and no fallback content is requested', () => {
    renderGuard({
      permissions: [Permission.READ_WALLET],
      requiredPermissions: [Permission.WRITE_WALLET],
    });

    expect(screen.queryByText('Protected content')).not.toBeInTheDocument();
  });

  it('renders a permission denied state when fallback content is requested', () => {
    renderGuard({
      hasContent: true,
      permissions: [Permission.READ_WALLET],
      requiredPermissions: [Permission.WRITE_WALLET],
    });

    expect(screen.getByText('Permission denied')).toBeInTheDocument();
  });
});
