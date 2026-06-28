const ROOTS = {
  AUTH: '/auth',
};

type ActivityScope = 'wallet' | 'admin';
type ActivityTransactionKind = 'payment' | 'invoice';

function activityHref(
  kind?: ActivityTransactionKind,
  id?: string,
  scope: ActivityScope = 'wallet'
) {
  const params = new URLSearchParams();

  if (kind) {
    params.set('type', kind);
  }

  if (id) {
    params.set('id', id);
  }

  if (scope === 'admin') {
    params.set('scope', scope);
  }

  const query = params.toString();

  return query ? `/activity?${query}` : '/activity';
}

function adminTransactionsHref(kind?: ActivityTransactionKind, id?: string) {
  const params = new URLSearchParams();

  if (kind) {
    params.set('type', kind);
  }

  if (id) {
    params.set('id', id);
  }

  const query = params.toString();

  return query ? `/admin/transactions?${query}` : '/admin/transactions';
}

export const paths = {
  overview: '/',
  activity: '/activity',
  activityList: (kind?: ActivityTransactionKind, scope: ActivityScope = 'wallet') =>
    activityHref(kind, undefined, scope),
  activityPayment: (id: string, scope: ActivityScope = 'wallet') =>
    activityHref('payment', id, scope),
  activityInvoice: (id: string, scope: ActivityScope = 'wallet') =>
    activityHref('invoice', id, scope),
  identity: '/identity',
  accounts: '/accounts',
  account: (id: string) => `/accounts?id=${id}`,
  nodeHealth: '/node-health',
  build: {
    apiKeys: '/build/api-keys',
  },
  onboarding: {
    welcome: '/welcome',
  },
  wallet: {
    root: '/wallet',
    payments: '/wallet/payments',
    payment: (id: string) => `/wallet/payments?id=${id}`,
    invoices: '/wallet/invoices',
    invoice: (id: string) => `/wallet/invoices?id=${id}`,
    lightningAddress: '/wallet/lightning-address',
    nostrAddress: '/wallet/nostr-address',
    contacts: '/wallet/contacts',
  },
  admin: {
    transactions: '/admin/transactions',
    transactionList: (kind?: ActivityTransactionKind) => adminTransactionsHref(kind),
    transactionPayment: (id: string) => adminTransactionsHref('payment', id),
    transactionInvoice: (id: string) => adminTransactionsHref('invoice', id),
    payments: '/admin/payments',
    payment: (id: string) => `/admin/payments?id=${id}`,
    invoices: '/admin/invoices',
    invoice: (id: string) => `/admin/invoices?id=${id}`,
    node: '/admin/lightning-node',
    lnAddresses: '/admin/lightning-addresses',
    lnAddress: (id: string) => `/admin/lightning-addresses?id=${id}`,
    btcAddresses: '/admin/bitcoin-addresses',
    apiKeys: '/admin/api-keys',
  },
  settings: {
    root: '/settings',
  },
  // AUTH
  auth: {
    login: '/login',
    signUp: '/sign-up',
    supabase: {
      verify: `${ROOTS.AUTH}/supabase/verify`,
      signUp: `${ROOTS.AUTH}/supabase/sign-up`,
      updatePassword: `${ROOTS.AUTH}/supabase/update-password`,
      resetPassword: `${ROOTS.AUTH}/supabase/reset-password`,
    },
  },
  // EXTERNAL
  external: {
    numeraire: {
      home: 'https://numeraire.tech',
      contact: 'https://numeraire.tech/contact',
      docs: 'https://docs.numeraire.tech',
      privacy: 'https://numeraire.tech/privacy',
    },
  },
};
