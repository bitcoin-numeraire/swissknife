export const endpointKeys = {
  mempoolSpace: {
    prices: 'mempoolSpacePrices',
  },
  userWallet: {
    get: 'userWallet',
    balance: (walletId: string) => ['userWalletBalance', walletId] as const,
    lnAddress: { get: 'userWalletGetAddress' },
    btcAddresses: { list: 'userWalletListBtcAddresses' },
    payments: {
      list: (walletId: string, limit?: number, offset?: number) =>
        ['userWalletListPayments', walletId, limit, offset] as const,
      get: (walletId: string, id: string) => ['userWalletGetPayment', walletId, id] as const,
    },
    invoices: {
      list: (walletId: string) => ['userWalletListInvoices', walletId] as const,
      get: (walletId: string, id: string) => ['userWalletGetInvoice', walletId, id] as const,
    },
    contacts: { list: (walletId: string) => ['userWalletListContacts', walletId] as const },
    apiKeys: { list: 'userWalletListApiKeys' },
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
