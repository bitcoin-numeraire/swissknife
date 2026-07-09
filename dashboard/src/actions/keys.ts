export const endpointKeys = {
  mempoolSpace: {
    prices: 'mempoolSpacePrices',
  },
  account: {
    get: 'account',
    wallets: 'accountWallets',
    lnAddress: { get: 'accountLnAddress' },
    apiKeys: { list: 'accountApiKeys' },
  },
  accountWallet: {
    get: (walletId: string) => ['accountWallet', walletId] as const,
    balance: (walletId: string) => ['accountWalletBalance', walletId] as const,
    btcAddresses: { list: 'accountWalletBtcAddresses' },
    payments: {
      list: (walletId: string, limit?: number, offset?: number) =>
        ['accountWalletPayments', walletId, limit, offset] as const,
      get: (walletId: string, id: string) => ['accountWalletPayment', walletId, id] as const,
    },
    invoices: {
      list: (walletId: string) => ['accountWalletInvoices', walletId] as const,
      get: (walletId: string, id: string) => ['accountWalletInvoice', walletId, id] as const,
    },
    contacts: { list: (walletId: string) => ['accountWalletContacts', walletId] as const },
  },
  system: {
    health: 'systemHealth',
  },
  wallets: {
    list: 'listWallets',
    get: (id: string) => ['getWallet', id] as const,
    listOverviews: 'listWalletOverviews',
  },
  invoices: {
    get: 'getInvoice',
    list: 'listInvoices',
  },
  payments: {
    get: 'getPayment',
    list: 'listPayments',
  },
  lightning: {
    node: {
      info: 'nodeInfo',
      lspInfo: 'lspIinfo',
      lsps: 'lsps',
    },
    addresses: {
      list: 'listLnAddresses',
      get: 'getLnAddress',
    },
  },
  bitcoin: {
    addresses: { list: 'listBtcAddresses' },
  },
  apiKeys: {
    list: 'listApiKeys',
  },
};
