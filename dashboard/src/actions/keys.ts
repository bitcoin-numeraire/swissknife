export const endpointKeys = {
  auth: {
    me: '/api/auth/me',
    signIn: 'signIn',
  },
  mempoolSpace: {
    prices: 'mempoolSpacePrices',
  },
  userWallet: {
    get: 'userWallet',
    balance: 'userWalletBalance',
    lnAddress: { get: 'userWalletGetAddress' },
    payments: { list: 'userWalletListPayments', get: 'userWalletGetPayment' },
    invoices: { list: 'userWalletListInvoices', get: 'userWalletGetInvoice' },
    contacts: { list: 'userWalletListContacts' },
    apiKeys: { list: 'userWalletListApiKeys' },
  },
  wallets: {
    list: 'listWallets',
    get: 'getWallet',
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
  apiKeys: {
    list: 'listApiKeys',
  },
};
