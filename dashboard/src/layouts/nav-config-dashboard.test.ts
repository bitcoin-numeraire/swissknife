import { it, expect, describe } from 'vitest';

import { Permission } from 'src/lib/swissknife';

import {
  navData,
  filterDashboardNavData,
  getDashboardModuleDiagnostics,
} from './nav-config-dashboard';

function itemTitles(data = navData) {
  return data.flatMap((group) => group.items.map((item) => item.title));
}

describe('dashboard permission gating', () => {
  it('keeps public modules visible without permissions', () => {
    const titles = itemTitles(filterDashboardNavData(navData, [], 'server'));

    expect(titles).toContain('overview');
    expect(titles).toContain('activity');
    expect(titles).toContain('identity_hub');
    expect(titles).toContain('contacts');
    expect(titles).toContain('api_keys');
    expect(titles).not.toContain('accounts_directory');
    expect(titles).not.toContain('admin_transactions');
  });

  it('shows modules when the user has the required permission', () => {
    const titles = itemTitles(
      filterDashboardNavData(
        navData,
        [Permission.READ_ACCOUNT, Permission.READ_TRANSACTION],
        'server'
      )
    );

    expect(titles).toContain('accounts_directory');
    expect(titles).toContain('admin_transactions');
  });

  it('keeps an api-key-only user out of wallet administration', () => {
    const titles = itemTitles(
      filterDashboardNavData(navData, [Permission.READ_API_KEY, Permission.WRITE_API_KEY], 'server')
    );

    expect(titles).toContain('api_keys');
    expect(titles).not.toContain('accounts_directory');
  });

  it('reports missing permissions in diagnostics', () => {
    const diagnostics = getDashboardModuleDiagnostics(navData, [], 'server', new Set());
    const adminTransactions = diagnostics.find((item) => item.title === 'admin_transactions');

    expect(adminTransactions?.visible).toBe(false);
    expect(adminTransactions?.reasons).toContainEqual({
      type: 'permissions',
      missing: [Permission.READ_TRANSACTION],
    });
  });
});
