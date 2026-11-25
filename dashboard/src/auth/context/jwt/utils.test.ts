import { it, vi, expect } from 'vitest';

import { getAccount } from 'src/lib/swissknife';

import { JWT_STORAGE_KEY } from './constant';
import { setSession, clearSession } from './utils';

it('keeps a JWT session on 404 and respects the request error mode', async () => {
  const error = { message: 'No account record' };
  const fetch = vi.fn(async () => Response.json(error, { status: 404 }));
  const options = { baseUrl: 'http://localhost', fetch };
  await setSession('test-session');
  try {
    const result = await getAccount({ ...options, throwOnError: false });
    expect(result.response?.status).toBe(404);
    expect(result.error).toEqual(error);
    expect(sessionStorage.getItem(JWT_STORAGE_KEY)).toBe('test-session');
    await expect(getAccount({ ...options, throwOnError: true })).rejects.toEqual(error);
  } finally {
    clearSession();
  }
});
