import useSWR from 'swr';

import {
  getLocalLogin,
  resetLocalLogin,
  createLocalLogin,
  updateLocalLogin,
} from 'src/lib/swissknife';

export function useLocalLogin(id: string) {
  return useSWR(['localLogin', id], () => getLocalLogin<true>({ path: { id } }));
}

export async function addLocalLogin(id: string, username: string) {
  return createLocalLogin<true>({ path: { id }, body: { username } });
}

export async function setLocalLoginEnabled(id: string, enabled: boolean) {
  return updateLocalLogin<true>({ path: { id }, body: { enabled } });
}

export async function issueLocalLoginReset(id: string) {
  return resetLocalLogin<true>({ path: { id } });
}
