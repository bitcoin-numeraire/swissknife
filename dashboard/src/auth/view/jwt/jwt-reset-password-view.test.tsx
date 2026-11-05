import { vi, it, expect, describe, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

import { resetLocalPassword } from 'src/lib/swissknife';

import { clearSession } from 'src/auth/context/jwt';

import { JwtResetPasswordView } from './jwt-reset-password-view';

vi.mock('src/locales', () => ({ useTranslate: () => ({ t: (key: string) => key }) }));
vi.mock('src/lib/swissknife', () => ({ resetLocalPassword: vi.fn() }));
vi.mock('src/auth/context/jwt', () => ({ clearSession: vi.fn() }));
vi.mock('src/utils/errors', () => ({ handleActionError: vi.fn() }));

beforeEach(() => vi.clearAllMocks());

function fillPassword(password: string, confirmation = password) {
  fireEvent.change(screen.getByLabelText('local_login.code', { exact: false }), {
    target: { value: 'c'.repeat(43) },
  });
  fireEvent.change(screen.getByLabelText('local_login.new_password', { exact: false }), {
    target: { value: password },
  });
  fireEvent.change(screen.getByLabelText('local_login.confirm_password', { exact: false }), {
    target: { value: confirmation },
  });
}

describe('local password activation and reset', () => {
  it('requires a matching passphrase before submitting', () => {
    render(<JwtResetPasswordView />);
    fillPassword('short');
    expect(screen.getByRole('button', { name: 'local_login.set_password' })).toBeDisabled();
    fillPassword('a long password phrase', 'different password');
    expect(screen.getByRole('button', { name: 'local_login.set_password' })).toBeDisabled();
    expect(resetLocalPassword).not.toHaveBeenCalled();
  });

  it('redeems the code, clears the old session, and asks the user to sign in', async () => {
    vi.mocked(resetLocalPassword).mockResolvedValue({ data: undefined } as Awaited<
      ReturnType<typeof resetLocalPassword>
    >);
    render(<JwtResetPasswordView />);
    fillPassword('a long password phrase');
    fireEvent.click(screen.getByRole('button', { name: 'local_login.set_password' }));
    await waitFor(() => expect(clearSession).toHaveBeenCalledOnce());
    expect(resetLocalPassword).toHaveBeenCalledWith({
      body: { code: 'c'.repeat(43), new_password: 'a long password phrase', display_name: null },
    });
    expect(screen.getByText('local_login.reset_success')).toBeInTheDocument();
    expect(screen.queryByLabelText('local_login.code', { exact: false })).not.toBeInTheDocument();
  });

  it('includes a chosen display name and lets each password be revealed independently', async () => {
    vi.mocked(resetLocalPassword).mockResolvedValue({ data: undefined } as Awaited<
      ReturnType<typeof resetLocalPassword>
    >);
    render(<JwtResetPasswordView />);
    fillPassword('a long password phrase');
    fireEvent.change(screen.getByLabelText('local_login.display_name'), {
      target: { value: '  Alice & Bob  ' },
    });
    const password = screen.getByLabelText('local_login.new_password', { exact: false });
    const confirmation = screen.getByLabelText('local_login.confirm_password', { exact: false });
    const toggles = screen.getAllByRole('button', { name: 'settings_view.show_password' });
    fireEvent.click(toggles[0]);
    expect(password).toHaveAttribute('type', 'text');
    expect(confirmation).toHaveAttribute('type', 'password');
    fireEvent.click(toggles[1]);
    expect(confirmation).toHaveAttribute('type', 'text');
    expect(resetLocalPassword).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'local_login.set_password' }));
    await waitFor(() =>
      expect(resetLocalPassword).toHaveBeenCalledWith({
        body: {
          code: 'c'.repeat(43),
          new_password: 'a long password phrase',
          display_name: 'Alice & Bob',
        },
      })
    );
    await waitFor(() => expect(screen.getByText('local_login.reset_success')).toBeInTheDocument());
  });

  it('keeps the form available when a code is rejected', async () => {
    vi.mocked(resetLocalPassword).mockRejectedValue(new Error('Invalid code'));
    render(<JwtResetPasswordView />);
    fillPassword('a long password phrase');
    fireEvent.click(screen.getByRole('button', { name: 'local_login.set_password' }));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'local_login.set_password' })).toBeEnabled()
    );
    expect(clearSession).not.toHaveBeenCalled();
    expect(screen.queryByText('local_login.reset_success')).not.toBeInTheDocument();
  });
});
