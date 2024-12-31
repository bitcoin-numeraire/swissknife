// This file is auto-generated by @hey-api/openapi-ts

import { createClient, createConfig, type Options } from '@hey-api/client-fetch';

import {
  payResponseTransformer,
  getApiKeyResponseTransformer,
  walletPayResponseTransformer,
  getWalletResponseTransformer,
  getInvoiceResponseTransformer,
  getAddressResponseTransformer,
  getPaymentResponseTransformer,
  listApiKeysResponseTransformer,
  listWalletsResponseTransformer,
  createApiKeyResponseTransformer,
  listInvoicesResponseTransformer,
  listContactsResponseTransformer,
  listPaymentsResponseTransformer,
  listAddressesResponseTransformer,
  updateAddressResponseTransformer,
  getUserWalletResponseTransformer,
  registerWalletResponseTransformer,
  generateInvoiceResponseTransformer,
  registerAddressResponseTransformer,
  getWalletApiKeyResponseTransformer,
  newWalletInvoiceResponseTransformer,
  getWalletInvoiceResponseTransformer,
  getWalletAddressResponseTransformer,
  getWalletPaymentResponseTransformer,
  listWalletApiKeysResponseTransformer,
  createWalletApiKeyResponseTransformer,
  listWalletInvoicesResponseTransformer,
  listWalletPaymentsResponseTransformer,
  updateWalletAddressResponseTransformer,
  listWalletOverviewsResponseTransformer,
  registerWalletAddressResponseTransformer,
} from './transformers.gen';

import type {
  PayData,
  SwapData,
  SyncData,
  PayError,
  SwapError,
  SyncError,
  SignInData,
  BackupData,
  RedeemData,
  SignInError,
  BackupError,
  LspInfoData,
  RedeemError,
  PayResponse,
  CallbackData,
  NodeInfoData,
  LspInfoError,
  ListLspsData,
  WellKnownData,
  CallbackError,
  GetApiKeyData,
  NodeInfoError,
  ListLspsError,
  WalletPayData,
  GetWalletData,
  WellKnownError,
  GetApiKeyError,
  GetInvoiceData,
  GetAddressData,
  BackupResponse,
  ConnectLspData,
  RedeemResponse,
  WalletPayError,
  GetPaymentData,
  SetupCheckData,
  GetWalletError,
  ListApiKeysData,
  SignInResponse2,
  GetInvoiceError,
  GetAddressError,
  ConnectLspError,
  SignMessageData,
  GetPaymentError,
  HealthCheckData,
  SetupCheckError,
  ListWalletsData,
  CallbackResponse,
  ListApiKeysError,
  CreateApiKeyData,
  RevokeApiKeyData,
  ListInvoicesData,
  CheckMessageData,
  ListLspsResponse,
  SignMessageError,
  ListContactsData,
  ListPaymentsData,
  HealthCheckError,
  VersionCheckData,
  ListWalletsError,
  DeleteWalletData,
  WellKnownResponse,
  RevokeApiKeysData,
  CreateApiKeyError,
  RevokeApiKeyError,
  GetApiKeyResponse,
  ListInvoicesError,
  DeleteInvoiceData,
  ListAddressesData,
  DeleteAddressData,
  UpdateAddressData,
  CheckMessageError,
  GetUserWalletData,
  ListContactsError,
  WalletPayResponse,
  ListPaymentsError,
  DeletePaymentData,
  DeleteWalletsData,
  DeleteWalletError,
  GetWalletResponse,
  WellKnownNostrData,
  RevokeApiKeysError,
  DeleteInvoicesData,
  DeleteInvoiceError,
  GetInvoiceResponse,
  ListAddressesError,
  DeleteAddressError,
  GetAddressResponse,
  UpdateAddressError,
  GetUserWalletError,
  DeletePaymentsData,
  DeletePaymentError,
  GetPaymentResponse,
  ReadinessCheckData,
  SetupCheckResponse,
  DeleteWalletsError,
  RegisterWalletData,
  WellKnownNostrError,
  ListApiKeysResponse,
  DeleteInvoicesError,
  GenerateInvoiceData,
  DeleteAddressesData,
  RegisterAddressData,
  GetWalletApiKeyData,
  DeletePaymentsError,
  HealthCheckResponse,
  ListWalletsResponse,
  RegisterWalletError,
  CreateApiKeyResponse,
  ListInvoicesResponse,
  GenerateInvoiceError,
  DeleteAddressesError,
  RegisterAddressError,
  CloseLspChannelsData,
  SignMessageResponse2,
  GetWalletApiKeyError,
  GetWalletBalanceData,
  ListContactsResponse,
  NewWalletInvoiceData,
  GetWalletInvoiceData,
  GetWalletAddressData,
  GetWalletPaymentData,
  ListPaymentsResponse,
  VersionCheckResponse,
  RevokeApiKeysResponse,
  ListAddressesResponse,
  UpdateAddressResponse,
  CheckMessageResponse2,
  CloseLspChannelsError,
  GetUserWalletResponse,
  ListWalletApiKeysData,
  GetWalletBalanceError,
  NewWalletInvoiceError,
  GetWalletInvoiceError,
  GetWalletAddressError,
  GetWalletPaymentError,
  DeleteWalletsResponse,
  WellKnownNostrResponse,
  DeleteInvoicesResponse,
  ListWalletApiKeysError,
  CreateWalletApiKeyData,
  RevokeWalletApiKeyData,
  ListWalletInvoicesData,
  ListWalletPaymentsData,
  DeletePaymentsResponse,
  RegisterWalletResponse,
  GenerateInvoiceResponse,
  DeleteAddressesResponse,
  RegisterAddressResponse,
  RevokeWalletApiKeysData,
  CreateWalletApiKeyError,
  RevokeWalletApiKeyError,
  GetWalletApiKeyResponse,
  ListWalletInvoicesError,
  DeleteWalletAddressData,
  UpdateWalletAddressData,
  ListWalletPaymentsError,
  ListWalletOverviewsData,
  CloseLspChannelsResponse,
  RevokeWalletApiKeysError,
  GetWalletBalanceResponse,
  NewWalletInvoiceResponse,
  GetWalletInvoiceResponse,
  DeleteWalletAddressError,
  GetWalletAddressResponse,
  UpdateWalletAddressError,
  DeleteFailedPaymentsData,
  GetWalletPaymentResponse,
  ListWalletOverviewsError,
  ListWalletApiKeysResponse,
  DeleteExpiredInvoicesData,
  RegisterWalletAddressData,
  DeleteFailedPaymentsError,
  CreateWalletApiKeyResponse,
  DeleteExpiredInvoicesError,
  ListWalletInvoicesResponse,
  RegisterWalletAddressError,
  ListWalletPaymentsResponse,
  RevokeWalletApiKeysResponse,
  UpdateWalletAddressResponse,
  ListWalletOverviewsResponse,
  DeleteFailedPaymentsResponse,
  DeleteExpiredInvoicesResponse,
  RegisterWalletAddressResponse,
} from './types.gen';

export const client = createClient(createConfig());

/**
 * Well-known endpoint
 * Returns the LNURL payRequest for this LN Address (username). The returned payload contains information allowing the payer to generate an invoice. See [LUDS-06](https://github.com/lnurl/luds/blob/luds/06.md)
 */
export const wellKnown = <ThrowOnError extends boolean = false>(
  options: Options<WellKnownData, ThrowOnError>
) =>
  (options?.client ?? client).get<WellKnownResponse, WellKnownError, ThrowOnError>({
    ...options,
    url: '/.well-known/lnurlp/{username}',
  });

/**
 * Well-known endpoint
 * Returns the names known by this service given username. The returned payload contains public keys in hex format. See [NIP-05](https://github.com/nostr-protocol/nips/blob/master/05.md)
 */
export const wellKnownNostr = <ThrowOnError extends boolean = false>(
  options?: Options<WellKnownNostrData, ThrowOnError>
) =>
  (options?.client ?? client).get<WellKnownNostrResponse, WellKnownNostrError, ThrowOnError>({
    ...options,
    url: '/.well-known/nostr.json',
  });

/**
 * LNURL callback endpoint
 * Returns the callback response for this LN Address (username). Containing an invoice and information on how to behave upon success. See [LUDS-06](https://github.com/lnurl/luds/blob/luds/06.md)
 */
export const callback = <ThrowOnError extends boolean = false>(
  options: Options<CallbackData, ThrowOnError>
) =>
  (options?.client ?? client).get<CallbackResponse, CallbackError, ThrowOnError>({
    ...options,
    url: '/lnurlp/{username}/callback',
  });

/**
 * Revoke API Keys
 * Revokes all the API Keys given a filter. Returns the number of revoked keys.
 */
export const revokeApiKeys = <ThrowOnError extends boolean = false>(
  options?: Options<RevokeApiKeysData, ThrowOnError>
) =>
  (options?.client ?? client).delete<RevokeApiKeysResponse, RevokeApiKeysError, ThrowOnError>({
    ...options,
    url: '/v1/api-keys',
  });

/**
 * List API Keys
 * Returns all the API Keys given a filter
 */
export const listApiKeys = <ThrowOnError extends boolean = false>(
  options?: Options<ListApiKeysData, ThrowOnError>
) =>
  (options?.client ?? client).get<ListApiKeysResponse, ListApiKeysError, ThrowOnError>({
    ...options,
    url: '/v1/api-keys',
    responseTransformer: listApiKeysResponseTransformer,
  });

/**
 * Generate a new API Key
 * Returns the generated API Key for the given user. Users can create API keys with permissions as a subset of his current permissions.
 */
export const createApiKey = <ThrowOnError extends boolean = false>(
  options: Options<CreateApiKeyData, ThrowOnError>
) =>
  (options?.client ?? client).post<CreateApiKeyResponse, CreateApiKeyError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/api-keys',
    responseTransformer: createApiKeyResponseTransformer,
  });

/**
 * Revoke an API Key
 * Revokes an API Key by ID. Returns an empty body.
 */
export const revokeApiKey = <ThrowOnError extends boolean = false>(
  options: Options<RevokeApiKeyData, ThrowOnError>
) =>
  (options?.client ?? client).delete<unknown, RevokeApiKeyError, ThrowOnError>({
    ...options,
    url: '/v1/api-keys/{id}',
  });

/**
 * Find an API Key
 * Returns the API Key by its ID.
 */
export const getApiKey = <ThrowOnError extends boolean = false>(
  options: Options<GetApiKeyData, ThrowOnError>
) =>
  (options?.client ?? client).get<GetApiKeyResponse, GetApiKeyError, ThrowOnError>({
    ...options,
    url: '/v1/api-keys/{id}',
    responseTransformer: getApiKeyResponseTransformer,
  });

/**
 * Sign In
 * Returns a JWT token to be used for authentication. The JWT token contains authentication and permissions. Sign in is only available for `JWT` Auth provider.
 */
export const signIn = <ThrowOnError extends boolean = false>(
  options: Options<SignInData, ThrowOnError>
) =>
  (options?.client ?? client).post<SignInResponse2, SignInError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/auth/sign-in',
  });

/**
 * Delete invoices
 * Deletes all the invoices given a filter. Returns the number of deleted invoices. Deleting an invoice can have an effect on the user balance
 */
export const deleteInvoices = <ThrowOnError extends boolean = false>(
  options?: Options<DeleteInvoicesData, ThrowOnError>
) =>
  (options?.client ?? client).delete<DeleteInvoicesResponse, DeleteInvoicesError, ThrowOnError>({
    ...options,
    url: '/v1/invoices',
  });

/**
 * List invoices
 * Returns all the invoices given a filter
 */
export const listInvoices = <ThrowOnError extends boolean = false>(
  options?: Options<ListInvoicesData, ThrowOnError>
) =>
  (options?.client ?? client).get<ListInvoicesResponse, ListInvoicesError, ThrowOnError>({
    ...options,
    url: '/v1/invoices',
    responseTransformer: listInvoicesResponseTransformer,
  });

/**
 * Generate a new invoice
 * Returns the generated invoice for the given user
 */
export const generateInvoice = <ThrowOnError extends boolean = false>(
  options: Options<GenerateInvoiceData, ThrowOnError>
) =>
  (options?.client ?? client).post<GenerateInvoiceResponse, GenerateInvoiceError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/invoices',
    responseTransformer: generateInvoiceResponseTransformer,
  });

/**
 * Delete an invoice
 * Deletes an invoice by ID. Returns an empty body. Deleting an invoice has an effect on the user balance
 */
export const deleteInvoice = <ThrowOnError extends boolean = false>(
  options: Options<DeleteInvoiceData, ThrowOnError>
) =>
  (options?.client ?? client).delete<unknown, DeleteInvoiceError, ThrowOnError>({
    ...options,
    url: '/v1/invoices/{id}',
  });

/**
 * Find an invoice
 * Returns the invoice by its ID.
 */
export const getInvoice = <ThrowOnError extends boolean = false>(
  options: Options<GetInvoiceData, ThrowOnError>
) =>
  (options?.client ?? client).get<GetInvoiceResponse, GetInvoiceError, ThrowOnError>({
    ...options,
    url: '/v1/invoices/{id}',
    responseTransformer: getInvoiceResponseTransformer,
  });

/**
 * Delete LN Addresses
 * Deletes all the addresses given a filter. Returns the number of deleted addresses
 */
export const deleteAddresses = <ThrowOnError extends boolean = false>(
  options?: Options<DeleteAddressesData, ThrowOnError>
) =>
  (options?.client ?? client).delete<DeleteAddressesResponse, DeleteAddressesError, ThrowOnError>({
    ...options,
    url: '/v1/lightning-addresses',
  });

/**
 * List LN Addresses
 * Returns all the addresses given a filter
 */
export const listAddresses = <ThrowOnError extends boolean = false>(
  options?: Options<ListAddressesData, ThrowOnError>
) =>
  (options?.client ?? client).get<ListAddressesResponse, ListAddressesError, ThrowOnError>({
    ...options,
    url: '/v1/lightning-addresses',
    responseTransformer: listAddressesResponseTransformer,
  });

/**
 * Register a new LN Address
 * Registers an address. Returns the address details. LN Addresses are ready to receive funds through the LNURL protocol upon registration.
 */
export const registerAddress = <ThrowOnError extends boolean = false>(
  options: Options<RegisterAddressData, ThrowOnError>
) =>
  (options?.client ?? client).post<RegisterAddressResponse, RegisterAddressError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/lightning-addresses',
    responseTransformer: registerAddressResponseTransformer,
  });

/**
 * Delete a LN Address
 * Deletes an address by ID. Returns an empty body
 */
export const deleteAddress = <ThrowOnError extends boolean = false>(
  options: Options<DeleteAddressData, ThrowOnError>
) =>
  (options?.client ?? client).delete<unknown, DeleteAddressError, ThrowOnError>({
    ...options,
    url: '/v1/lightning-addresses/{id}',
  });

/**
 * Find a LN Address
 * Returns the address by its ID.
 */
export const getAddress = <ThrowOnError extends boolean = false>(
  options: Options<GetAddressData, ThrowOnError>
) =>
  (options?.client ?? client).get<GetAddressResponse, GetAddressError, ThrowOnError>({
    ...options,
    url: '/v1/lightning-addresses/{id}',
    responseTransformer: getAddressResponseTransformer,
  });

/**
 * Update a LN Address
 * Updates an address. Returns the address details.
 */
export const updateAddress = <ThrowOnError extends boolean = false>(
  options: Options<UpdateAddressData, ThrowOnError>
) =>
  (options?.client ?? client).put<UpdateAddressResponse, UpdateAddressError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/lightning-addresses/{id}',
    responseTransformer: updateAddressResponseTransformer,
  });

/**
 * Backup node channels
 * Returns the static channel backup file contaning the channel information needed to recover funds for a Core Lightning node. See [the documentation](https://docs.corelightning.org/docs/backup#static-channel-backup)
 */
export const backup = <ThrowOnError extends boolean = false>(
  options?: Options<BackupData, ThrowOnError>
) =>
  (options?.client ?? client).get<BackupResponse, BackupError, ThrowOnError>({
    ...options,
    url: '/v1/lightning-node/backup',
  });

/**
 * Verify Signature
 * Verifies the validity of a signature against a node's public key. Returns `true` if valid.
 */
export const checkMessage = <ThrowOnError extends boolean = false>(
  options: Options<CheckMessageData, ThrowOnError>
) =>
  (options?.client ?? client).post<CheckMessageResponse2, CheckMessageError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/lightning-node/check-message',
  });

/**
 * Close LSP channels
 * Returns the list of transaction IDs for the lightning channel closures. The funds are deposited in your on-chain addresses and can be redeemed
 */
export const closeLspChannels = <ThrowOnError extends boolean = false>(
  options?: Options<CloseLspChannelsData, ThrowOnError>
) =>
  (options?.client ?? client).post<CloseLspChannelsResponse, CloseLspChannelsError, ThrowOnError>({
    ...options,
    url: '/v1/lightning-node/close-channels',
  });

/**
 * Connect LSP
 * Connects to an LSP from the list of available LSPs by its ID. Returns an  empty body
 */
export const connectLsp = <ThrowOnError extends boolean = false>(
  options: Options<ConnectLspData, ThrowOnError>
) =>
  (options?.client ?? client).post<unknown, ConnectLspError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/lightning-node/connect-lsp',
  });

/**
 * Get node info
 * Returns the Core Lightning node info hosted on [Greenlight (Blockstream)](https://blockstream.com/lightning/greenlight/) infrastructure
 */
export const nodeInfo = <ThrowOnError extends boolean = false>(
  options?: Options<NodeInfoData, ThrowOnError>
) =>
  (options?.client ?? client).get<unknown, NodeInfoError, ThrowOnError>({
    ...options,
    url: '/v1/lightning-node/info',
  });

/**
 * Get LSP info
 * Returns the info of the current Breez partner LSP connected to the Core Lightning node.
 */
export const lspInfo = <ThrowOnError extends boolean = false>(
  options?: Options<LspInfoData, ThrowOnError>
) =>
  (options?.client ?? client).get<unknown, LspInfoError, ThrowOnError>({
    ...options,
    url: '/v1/lightning-node/lsp-info',
  });

/**
 * List LSPs
 * Returns the list of available LSPs for the node.
 */
export const listLsps = <ThrowOnError extends boolean = false>(
  options?: Options<ListLspsData, ThrowOnError>
) =>
  (options?.client ?? client).get<ListLspsResponse, ListLspsError, ThrowOnError>({
    ...options,
    url: '/v1/lightning-node/lsps',
  });

/**
 * Redeem BTC
 * Redeems your whole on-chain BTC balance to an address of your choice. Returns the transaction ID.
 */
export const redeem = <ThrowOnError extends boolean = false>(
  options: Options<RedeemData, ThrowOnError>
) =>
  (options?.client ?? client).post<RedeemResponse, RedeemError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/lightning-node/redeem',
  });

/**
 * Sign message
 * Signs a message using the node's key. Returns a zbase encoded signature
 */
export const signMessage = <ThrowOnError extends boolean = false>(
  options: Options<SignMessageData, ThrowOnError>
) =>
  (options?.client ?? client).post<SignMessageResponse2, SignMessageError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/lightning-node/sign-message',
  });

/**
 * Swap BTC
 * Pays BTC on-chain via Swap service. Meaning that the funds are sent through Lightning and swaps to the recipient on-chain address
 */
export const swap = <ThrowOnError extends boolean = false>(
  options: Options<SwapData, ThrowOnError>
) =>
  (options?.client ?? client).post<unknown, SwapError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/lightning-node/swap',
  });

/**
 * Sync node
 * Syncs the local state with the remote node state.
 */
export const sync = <ThrowOnError extends boolean = false>(
  options?: Options<SyncData, ThrowOnError>
) =>
  (options?.client ?? client).post<unknown, SyncError, ThrowOnError>({
    ...options,
    url: '/v1/lightning-node/sync',
  });

/**
 * Get wallet
 * Returns the user wallet.
 */
export const getUserWallet = <ThrowOnError extends boolean = false>(
  options?: Options<GetUserWalletData, ThrowOnError>
) =>
  (options?.client ?? client).get<GetUserWalletResponse, GetUserWalletError, ThrowOnError>({
    ...options,
    url: '/v1/me',
    responseTransformer: getUserWalletResponseTransformer,
  });

/**
 * Revoke API Keys
 * Revokes all the API Keys given a filter. Returns the number of revoked keys.
 */
export const revokeWalletApiKeys = <ThrowOnError extends boolean = false>(
  options?: Options<RevokeWalletApiKeysData, ThrowOnError>
) =>
  (options?.client ?? client).delete<
    RevokeWalletApiKeysResponse,
    RevokeWalletApiKeysError,
    ThrowOnError
  >({
    ...options,
    url: '/v1/me/api-keys',
  });

/**
 * List API Keys
 * Returns all the API Keys given a filter
 */
export const listWalletApiKeys = <ThrowOnError extends boolean = false>(
  options?: Options<ListWalletApiKeysData, ThrowOnError>
) =>
  (options?.client ?? client).get<ListWalletApiKeysResponse, ListWalletApiKeysError, ThrowOnError>({
    ...options,
    url: '/v1/me/api-keys',
    responseTransformer: listWalletApiKeysResponseTransformer,
  });

/**
 * Generate a new API Key
 * Returns the generated API Key for the given user. Users can create API keys with permissions as a subset of his current permissions.
 */
export const createWalletApiKey = <ThrowOnError extends boolean = false>(
  options: Options<CreateWalletApiKeyData, ThrowOnError>
) =>
  (options?.client ?? client).post<
    CreateWalletApiKeyResponse,
    CreateWalletApiKeyError,
    ThrowOnError
  >({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/me/api-keys',
    responseTransformer: createWalletApiKeyResponseTransformer,
  });

/**
 * Revoke an API Key
 * Revokes an API Key by ID. Returns an empty body.
 */
export const revokeWalletApiKey = <ThrowOnError extends boolean = false>(
  options: Options<RevokeWalletApiKeyData, ThrowOnError>
) =>
  (options?.client ?? client).delete<unknown, RevokeWalletApiKeyError, ThrowOnError>({
    ...options,
    url: '/v1/me/api-keys/{id}',
  });

/**
 * Find an API Key
 * Returns the API Key by its ID.
 */
export const getWalletApiKey = <ThrowOnError extends boolean = false>(
  options: Options<GetWalletApiKeyData, ThrowOnError>
) =>
  (options?.client ?? client).get<GetWalletApiKeyResponse, GetWalletApiKeyError, ThrowOnError>({
    ...options,
    url: '/v1/me/api-keys/{id}',
    responseTransformer: getWalletApiKeyResponseTransformer,
  });

/**
 * Get wallet balance
 * Returns the wallet balance.
 */
export const getWalletBalance = <ThrowOnError extends boolean = false>(
  options?: Options<GetWalletBalanceData, ThrowOnError>
) =>
  (options?.client ?? client).get<GetWalletBalanceResponse, GetWalletBalanceError, ThrowOnError>({
    ...options,
    url: '/v1/me/balance',
  });

/**
 * List contacts
 * Returns all the contacts
 */
export const listContacts = <ThrowOnError extends boolean = false>(
  options?: Options<ListContactsData, ThrowOnError>
) =>
  (options?.client ?? client).get<ListContactsResponse, ListContactsError, ThrowOnError>({
    ...options,
    url: '/v1/me/contacts',
    responseTransformer: listContactsResponseTransformer,
  });

/**
 * Delete expired invoices
 * Deletes all the invoices with status `Èxpired`. Returns the number of deleted invoices
 */
export const deleteExpiredInvoices = <ThrowOnError extends boolean = false>(
  options?: Options<DeleteExpiredInvoicesData, ThrowOnError>
) =>
  (options?.client ?? client).delete<
    DeleteExpiredInvoicesResponse,
    DeleteExpiredInvoicesError,
    ThrowOnError
  >({
    ...options,
    url: '/v1/me/invoices',
  });

/**
 * List invoices
 * Returns all the invoices given a filter
 */
export const listWalletInvoices = <ThrowOnError extends boolean = false>(
  options?: Options<ListWalletInvoicesData, ThrowOnError>
) =>
  (options?.client ?? client).get<
    ListWalletInvoicesResponse,
    ListWalletInvoicesError,
    ThrowOnError
  >({
    ...options,
    url: '/v1/me/invoices',
    responseTransformer: listWalletInvoicesResponseTransformer,
  });

/**
 * Generate a new invoice
 * Returns the generated invoice
 */
export const newWalletInvoice = <ThrowOnError extends boolean = false>(
  options: Options<NewWalletInvoiceData, ThrowOnError>
) =>
  (options?.client ?? client).post<NewWalletInvoiceResponse, NewWalletInvoiceError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/me/invoices',
    responseTransformer: newWalletInvoiceResponseTransformer,
  });

/**
 * Find an invoice
 * Returns the invoice by its ID
 */
export const getWalletInvoice = <ThrowOnError extends boolean = false>(
  options: Options<GetWalletInvoiceData, ThrowOnError>
) =>
  (options?.client ?? client).get<GetWalletInvoiceResponse, GetWalletInvoiceError, ThrowOnError>({
    ...options,
    url: '/v1/me/invoices/{id}',
    responseTransformer: getWalletInvoiceResponseTransformer,
  });

/**
 * Delete LN Address
 * Deletes an address. Returns an empty body. Once the address is deleted, it will no longer be able to receive funds and its username can be claimed by another user.
 */
export const deleteWalletAddress = <ThrowOnError extends boolean = false>(
  options?: Options<DeleteWalletAddressData, ThrowOnError>
) =>
  (options?.client ?? client).delete<unknown, DeleteWalletAddressError, ThrowOnError>({
    ...options,
    url: '/v1/me/lightning-address',
  });

/**
 * Get LN Address
 * Returns the registered address
 */
export const getWalletAddress = <ThrowOnError extends boolean = false>(
  options?: Options<GetWalletAddressData, ThrowOnError>
) =>
  (options?.client ?? client).get<GetWalletAddressResponse, GetWalletAddressError, ThrowOnError>({
    ...options,
    url: '/v1/me/lightning-address',
    responseTransformer: getWalletAddressResponseTransformer,
  });

/**
 * Register LN Address
 * Registers an address. Returns the address details. LN Addresses are ready to receive funds through the LNURL protocol upon registration.
 */
export const registerWalletAddress = <ThrowOnError extends boolean = false>(
  options: Options<RegisterWalletAddressData, ThrowOnError>
) =>
  (options?.client ?? client).post<
    RegisterWalletAddressResponse,
    RegisterWalletAddressError,
    ThrowOnError
  >({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/me/lightning-address',
    responseTransformer: registerWalletAddressResponseTransformer,
  });

/**
 * Update LN Address
 * Updates the address. Returns the address details.
 */
export const updateWalletAddress = <ThrowOnError extends boolean = false>(
  options: Options<UpdateWalletAddressData, ThrowOnError>
) =>
  (options?.client ?? client).put<
    UpdateWalletAddressResponse,
    UpdateWalletAddressError,
    ThrowOnError
  >({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/me/lightning-address',
    responseTransformer: updateWalletAddressResponseTransformer,
  });

/**
 * Delete failed payments
 * Deletes all the payments with `Failed` status. Returns the number of deleted payments
 */
export const deleteFailedPayments = <ThrowOnError extends boolean = false>(
  options?: Options<DeleteFailedPaymentsData, ThrowOnError>
) =>
  (options?.client ?? client).delete<
    DeleteFailedPaymentsResponse,
    DeleteFailedPaymentsError,
    ThrowOnError
  >({
    ...options,
    url: '/v1/me/payments',
  });

/**
 * List payments
 * Returns all the payments given a filter
 */
export const listWalletPayments = <ThrowOnError extends boolean = false>(
  options?: Options<ListWalletPaymentsData, ThrowOnError>
) =>
  (options?.client ?? client).get<
    ListWalletPaymentsResponse,
    ListWalletPaymentsError,
    ThrowOnError
  >({
    ...options,
    url: '/v1/me/payments',
    responseTransformer: listWalletPaymentsResponseTransformer,
  });

/**
 * Send payment
 * Pay for a LN invoice, LNURL, LN Address, On-chain or internally to an other user on the same instance. Returns the payment details.
 */
export const walletPay = <ThrowOnError extends boolean = false>(
  options: Options<WalletPayData, ThrowOnError>
) =>
  (options?.client ?? client).post<WalletPayResponse, WalletPayError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/me/payments',
    responseTransformer: walletPayResponseTransformer,
  });

/**
 * Find a payment
 * Returns the payment by its ID
 */
export const getWalletPayment = <ThrowOnError extends boolean = false>(
  options: Options<GetWalletPaymentData, ThrowOnError>
) =>
  (options?.client ?? client).get<GetWalletPaymentResponse, GetWalletPaymentError, ThrowOnError>({
    ...options,
    url: '/v1/me/payments/{id}',
    responseTransformer: getWalletPaymentResponseTransformer,
  });

/**
 * Delete payments
 * Deletes all the payments given a filter. Returns the number of deleted payments. Deleting a payment can have an effect on the user balance
 */
export const deletePayments = <ThrowOnError extends boolean = false>(
  options?: Options<DeletePaymentsData, ThrowOnError>
) =>
  (options?.client ?? client).delete<DeletePaymentsResponse, DeletePaymentsError, ThrowOnError>({
    ...options,
    url: '/v1/payments',
  });

/**
 * List payments
 * Returns all the payments given a filter
 */
export const listPayments = <ThrowOnError extends boolean = false>(
  options?: Options<ListPaymentsData, ThrowOnError>
) =>
  (options?.client ?? client).get<ListPaymentsResponse, ListPaymentsError, ThrowOnError>({
    ...options,
    url: '/v1/payments',
    responseTransformer: listPaymentsResponseTransformer,
  });

/**
 * Send a payment
 * Pay for a LN invoice, LNURL, LN Address, On-chain or internally to an other user on the same instance. Returns the payment details.
 */
export const pay = <ThrowOnError extends boolean = false>(
  options: Options<PayData, ThrowOnError>
) =>
  (options?.client ?? client).post<PayResponse, PayError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/payments',
    responseTransformer: payResponseTransformer,
  });

/**
 * Delete a payment
 * Deletes a payment by ID. Returns an empty body. Deleting a payment has an effect on the user balance
 */
export const deletePayment = <ThrowOnError extends boolean = false>(
  options: Options<DeletePaymentData, ThrowOnError>
) =>
  (options?.client ?? client).delete<unknown, DeletePaymentError, ThrowOnError>({
    ...options,
    url: '/v1/payments/{id}',
  });

/**
 * Find a payment
 * Returns the payment by its ID.
 */
export const getPayment = <ThrowOnError extends boolean = false>(
  options: Options<GetPaymentData, ThrowOnError>
) =>
  (options?.client ?? client).get<GetPaymentResponse, GetPaymentError, ThrowOnError>({
    ...options,
    url: '/v1/payments/{id}',
    responseTransformer: getPaymentResponseTransformer,
  });

/**
 * Health Check
 * Returns the health of the system fine-grained by dependency.
 */
export const healthCheck = <ThrowOnError extends boolean = false>(
  options?: Options<HealthCheckData, ThrowOnError>
) =>
  (options?.client ?? client).get<HealthCheckResponse, HealthCheckError, ThrowOnError>({
    ...options,
    url: '/v1/system/health',
  });

/**
 * Readiness Check
 * Returns successfully if the server is reachable.
 */
export const readinessCheck = <ThrowOnError extends boolean = false>(
  options?: Options<ReadinessCheckData, ThrowOnError>
) =>
  (options?.client ?? client).get<unknown, unknown, ThrowOnError>({
    ...options,
    url: '/v1/system/ready',
  });

/**
 * Setup Status Check
 * Returns whether the application setup is complete.
 */
export const setupCheck = <ThrowOnError extends boolean = false>(
  options?: Options<SetupCheckData, ThrowOnError>
) =>
  (options?.client ?? client).get<SetupCheckResponse, SetupCheckError, ThrowOnError>({
    ...options,
    url: '/v1/system/setup',
  });

/**
 * Version Information
 * Returns the current version and build time of the system.
 */
export const versionCheck = <ThrowOnError extends boolean = false>(
  options?: Options<VersionCheckData, ThrowOnError>
) =>
  (options?.client ?? client).get<VersionCheckResponse, unknown, ThrowOnError>({
    ...options,
    url: '/v1/system/version',
  });

/**
 * Delete wallets
 * Deletes all the wallets given a filter. Returns the number of deleted wallets. Deleting a wallet removes all data related to that wallet.
 */
export const deleteWallets = <ThrowOnError extends boolean = false>(
  options?: Options<DeleteWalletsData, ThrowOnError>
) =>
  (options?.client ?? client).delete<DeleteWalletsResponse, DeleteWalletsError, ThrowOnError>({
    ...options,
    url: '/v1/wallets',
  });

/**
 * List wallets
 * Returns all the wallets without any linked data. Use the wallet ID to get the full wallet details.
 */
export const listWallets = <ThrowOnError extends boolean = false>(
  options?: Options<ListWalletsData, ThrowOnError>
) =>
  (options?.client ?? client).get<ListWalletsResponse, ListWalletsError, ThrowOnError>({
    ...options,
    url: '/v1/wallets',
    responseTransformer: listWalletsResponseTransformer,
  });

/**
 * Register a new wallet
 * Returns the generated wallet for the given user
 */
export const registerWallet = <ThrowOnError extends boolean = false>(
  options: Options<RegisterWalletData, ThrowOnError>
) =>
  (options?.client ?? client).post<RegisterWalletResponse, RegisterWalletError, ThrowOnError>({
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    url: '/v1/wallets',
    responseTransformer: registerWalletResponseTransformer,
  });

/**
 * List wallet overviews
 * Returns all the wallet overviews. A wallet overview is a summary of a wallet with the number of payments, invoices and contacts.
 */
export const listWalletOverviews = <ThrowOnError extends boolean = false>(
  options?: Options<ListWalletOverviewsData, ThrowOnError>
) =>
  (options?.client ?? client).get<
    ListWalletOverviewsResponse,
    ListWalletOverviewsError,
    ThrowOnError
  >({
    ...options,
    url: '/v1/wallets/overviews',
    responseTransformer: listWalletOverviewsResponseTransformer,
  });

/**
 * Delete a wallet
 * Deletes an wallet by ID. Returns an empty body. Deleting a wallet removes all data related to that wallet.
 */
export const deleteWallet = <ThrowOnError extends boolean = false>(
  options: Options<DeleteWalletData, ThrowOnError>
) =>
  (options?.client ?? client).delete<unknown, DeleteWalletError, ThrowOnError>({
    ...options,
    url: '/v1/wallets/{id}',
  });

/**
 * Find a wallet
 * Returns the wallet by its ID.
 */
export const getWallet = <ThrowOnError extends boolean = false>(
  options: Options<GetWalletData, ThrowOnError>
) =>
  (options?.client ?? client).get<GetWalletResponse, GetWalletError, ThrowOnError>({
    ...options,
    url: '/v1/wallets/{id}',
    responseTransformer: getWalletResponseTransformer,
  });