export const endpointKeys = {
  mempoolSpace: {
    prices: 'mempoolSpacePrices',
  },
  userWallet: {
    get: 'userWallet',
    balance: 'userWalletBalance',
    lnAddress: { get: 'userWalletGetAddress' },
    btcAddresses: { list: 'userWalletListBtcAddresses' },
    payments: { list: 'userWalletListPayments', get: 'userWalletGetPayment' },
    invoices: { list: 'userWalletListInvoices', get: 'userWalletGetInvoice' },
    contacts: { list: 'userWalletListContacts' },
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
