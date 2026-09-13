import { waitFor, renderHook } from '@testing-library/react';
import { it, vi, expect, describe, afterEach } from 'vitest';

import { streamAccountEvents } from 'src/lib/swissknife';

import { useAccountEventStream } from './account-event-stream';

vi.mock('src/lib/swissknife', () => ({ streamAccountEvents: vi.fn() }));

afterEach(() => {
  vi.clearAllMocks();
});

describe('account event authentication', () => {
  it('connects once an account is authenticated and closes on sign-out', async () => {
    vi.mocked(streamAccountEvents).mockImplementation(async (options) => ({
      stream: (async function* () {
        await new Promise<void>((resolve) => {
          options?.signal?.addEventListener('abort', () => resolve(), { once: true });
        });
        yield* [];
      })(),
    }));
    const { rerender, unmount } = renderHook(
      ({ accountId }: { accountId: string | undefined }) => useAccountEventStream(accountId),
      { initialProps: { accountId: undefined as string | undefined } }
    );
    expect(streamAccountEvents).not.toHaveBeenCalled();

    // Account identity is sufficient; administrative permissions are not an input.
    rerender({ accountId: 'ordinary-account' });
    await waitFor(() => expect(streamAccountEvents).toHaveBeenCalledOnce());
    const signal = vi.mocked(streamAccountEvents).mock.calls[0][0]?.signal;
    expect(signal?.aborted).toBe(false);

    rerender({ accountId: undefined });
    expect(signal?.aborted).toBe(true);
    unmount();
  });
});
