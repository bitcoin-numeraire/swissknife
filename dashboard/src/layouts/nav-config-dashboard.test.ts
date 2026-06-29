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
    expect(titles).toContain('contacts');
    expect(titles).not.toContain('admin_transactions');
  });

  it('shows modules when the user has the required permission', () => {
    const titles = itemTitles(
      filterDashboardNavData(navData, [Permission.READ_WALLET, Permission.READ_TRANSACTION], 'server')
    );

    expect(titles).toContain('accounts_directory');
    expect(titles).toContain('admin_transactions');
  });

  it('reports missing permissions in diagnostics', () => {
    const diagnostics = getDashboardModuleDiagnostics(navData, [], 'server', new Set());
    const activity = diagnostics.find((item) => item.title === 'activity');

    expect(activity?.visible).toBe(false);
    expect(activity?.reasons).toContainEqual({
      type: 'permissions',
      missing: [Permission.READ_TRANSACTION],
    });
  });
});
