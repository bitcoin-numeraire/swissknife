import { it, expect, describe } from 'vitest';

import { hasAllPermissions } from './permissions';

describe('hasAllPermissions', () => {
  it('allows empty requirements', () => {
    expect(hasAllPermissions([], [])).toBe(true);
  });

  it('requires every requested permission', () => {
    expect(hasAllPermissions(['read:wallet'], ['read:wallet', 'write:wallet'])).toBe(true);
    expect(hasAllPermissions(['read:wallet', 'write:wallet'], ['read:wallet'])).toBe(false);
  });

  it('treats a missing user permission list as empty', () => {
    expect(hasAllPermissions(['read:wallet'])).toBe(false);
  });
});
