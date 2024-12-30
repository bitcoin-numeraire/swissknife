const ROOTS = {
  AUTH: '/auth',
};

export const paths = {
  wallet: {
    root: '/wallet',
    payments: '/wallet/payments',
    payment: (id: string) => `/wallet/payments/${id}`,
    invoices: '/wallet/invoices',
    invoice: (id: string) => `/wallet/invoices/${id}`,
    lightningAddress: '/wallet/lightning-address',
    nostrAddress: '/wallet/nostr-address',
    contacts: '/wallet/contacts',
  },
  admin: {
    wallets: '/admin/wallets',
    payments: '/admin/payments',
    payment: (id: string) => `/admin/payments/${id}`,
    invoices: '/admin/invoices',
    invoice: (id: string) => `/admin/invoices/${id}`,
    node: '/admin/lightning-node',
    lnAddresses: '/admin/lightning-addresses',
    lnAddress: (id: string) => `/admin/lightning-addresses/${id}`,
    apiKeys: '/admin/api-keys',
  },
  settings: {
    root: '/settings',
  },
  // AUTH
  auth: {
    jwt: {
      signIn: '/login',
      signUp: `${ROOTS.AUTH}/jwt/sign-up`,
    },
    auth0: {
      signIn: '/login',
    },
    supabase: {
      signIn: `${ROOTS.AUTH}/supabase/sign-in`,
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
    },
  },
};
