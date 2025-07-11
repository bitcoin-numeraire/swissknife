const ROOTS = {
  AUTH: '/auth',
};

export const paths = {
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
    wallets: '/admin/wallets',
    payments: '/admin/payments',
    payment: (id: string) => `/admin/payments?id=${id}`,
    invoices: '/admin/invoices',
    invoice: (id: string) => `/admin/invoices?id=${id}`,
    node: '/admin/lightning-node',
    lnAddresses: '/admin/lightning-addresses',
    lnAddress: (id: string) => `/admin/lightning-addresses?id=${id}`,
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
