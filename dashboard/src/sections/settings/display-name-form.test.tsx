import { vi, it, expect, describe, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

import { handleActionError } from 'src/utils/errors';

import { updateCurrentAccount } from 'src/lib/swissknife';

import { DisplayNameForm } from './display-name-form';

const { refreshAccount } = vi.hoisted(() => ({ refreshAccount: vi.fn() }));

vi.mock('src/locales', () => ({ useTranslate: () => ({ t: (key: string) => key }) }));
vi.mock('src/lib/swissknife', () => ({ updateCurrentAccount: vi.fn() }));
vi.mock('src/contexts/account', () => ({
  useAccountContext: () => ({ account: { display_name: 'Alice' }, refreshAccount }),
}));
vi.mock('src/utils/errors', () => ({ handleActionError: vi.fn() }));
vi.mock('src/components/snackbar', () => ({ toast: { success: vi.fn() } }));

beforeEach(() => vi.resetAllMocks());

describe('display name settings', () => {
  it.each([
    ['  Alice & Bob  ', 'Alice & Bob'],
    ['   ', null],
  ])('saves %j and refreshes the shared account', async (input, saved) => {
    render(<DisplayNameForm />);
    expect(screen.getByRole('button', { name: 'save' })).toBeDisabled();
    fireEvent.change(screen.getByLabelText('accounts_view.display_name'), {
      target: { value: input },
    });
    fireEvent.click(screen.getByRole('button', { name: 'save' }));
    await waitFor(() => expect(refreshAccount).toHaveBeenCalledOnce());
    expect(updateCurrentAccount).toHaveBeenCalledWith({ body: { display_name: saved } });
  });

  it('keeps the edit available after an API error', async () => {
    const error = new Error('Update failed');
    vi.mocked(updateCurrentAccount).mockRejectedValue(error);
    render(<DisplayNameForm />);
    fireEvent.change(screen.getByLabelText('accounts_view.display_name'), {
      target: { value: 'New name' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'save' }));
    await waitFor(() => expect(handleActionError).toHaveBeenCalledWith(error));
    expect(refreshAccount).not.toHaveBeenCalled();
    expect(screen.getByLabelText('accounts_view.display_name')).toHaveValue('New name');
    expect(screen.getByRole('button', { name: 'save' })).toBeEnabled();
  });
});
