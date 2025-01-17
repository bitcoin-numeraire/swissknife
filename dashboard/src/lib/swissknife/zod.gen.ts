// This file is auto-generated by @hey-api/openapi-ts

import { z } from 'zod';

export const zApiKeyResponse = z.object({
  created_at: z.string().datetime(),
  description: z.unknown().optional(),
  expires_at: z.unknown().optional(),
  id: z.string().uuid(),
  key: z.unknown().optional(),
  name: z.string(),
  permissions: z.array(
    z.enum([
      'read:wallet',
      'write:wallet',
      'read:ln_address',
      'write:ln_address',
      'read:transaction',
      'write:transaction',
      'read:ln_node',
      'write:ln_node',
      'read:api_key',
      'write:api_key',
    ])
  ),
  user_id: z.string(),
});

export const zBalance = z.object({
  available_msat: z.number(),
  fees_paid_msat: z.number().gte(0),
  received_msat: z.number().gte(0),
  sent_msat: z.number().gte(0),
});

export const zCheckMessageRequest = z.object({
  message: z.string(),
  pubkey: z.string(),
  signature: z.string(),
});

export const zCheckMessageResponse = z.object({
  is_valid: z.boolean(),
});

export const zConnectLSPRequest = z.object({
  lsp_id: z.string(),
});

export const zContact = z.object({
  contact_since: z.string().datetime(),
  ln_address: z.string(),
});

export const zCreateApiKeyRequest = z.object({
  description: z.unknown().optional(),
  expiry: z.unknown().optional(),
  name: z.string(),
  permissions: z.array(
    z.enum([
      'read:wallet',
      'write:wallet',
      'read:ln_address',
      'write:ln_address',
      'read:transaction',
      'write:transaction',
      'read:ln_node',
      'write:ln_node',
      'read:api_key',
      'write:api_key',
    ])
  ),
  user_id: z.unknown().optional(),
});

export const zCurrency = z.enum(['Bitcoin', 'BitcoinTestnet', 'Regtest', 'Simnet', 'Signet']);

export const zErrorResponse = z.object({
  reason: z.string(),
  status: z.string(),
});

export const zHealthCheck = z.object({
  database: z.enum(['Operational', 'Unavailable', 'Maintenance']),
  is_healthy: z.boolean(),
  ln_provider: z.enum(['Operational', 'Unavailable', 'Maintenance']),
});

export const zHealthStatus = z.enum(['Operational', 'Unavailable', 'Maintenance']);

export const zInvoiceOrderBy = z.enum(['CreatedAt', 'PaymentTime', 'UpdatedAt']);

export const zInvoiceResponse = z.object({
  amount_msat: z.unknown().optional(),
  amount_received_msat: z.unknown().optional(),
  created_at: z.string().datetime(),
  currency: zCurrency,
  description: z.unknown().optional(),
  fee_msat: z.unknown().optional(),
  id: z.string().uuid(),
  ledger: z.enum(['Lightning', 'Internal', 'Onchain']),
  ln_address_id: z.unknown().optional(),
  ln_invoice: z.unknown().optional(),
  payment_time: z.unknown().optional(),
  status: z.enum(['Pending', 'Settled', 'Expired']),
  timestamp: z.string().datetime(),
  updated_at: z.unknown().optional(),
  wallet_id: z.string().uuid(),
});

export const zInvoiceStatus = z.enum(['Pending', 'Settled', 'Expired']);

export const zLedger = z.enum(['Lightning', 'Internal', 'Onchain']);

export const zLnAddress = z.object({
  active: z.boolean(),
  allows_nostr: z.boolean(),
  created_at: z.string().datetime(),
  id: z.string().uuid(),
  nostr_pubkey: z.unknown().optional(),
  updated_at: z.unknown().optional(),
  username: z.string(),
  wallet_id: z.string().uuid(),
});

export const zLnInvoiceResponse = z.object({
  bolt11: z.string(),
  description_hash: z.unknown().optional(),
  expires_at: z.string().datetime(),
  expiry: z.string(),
  min_final_cltv_expiry_delta: z.number().gte(0),
  payee_pubkey: z.string(),
  payment_hash: z.string(),
  payment_secret: z.string(),
});

export const zLnURLPayRequest = z.object({
  allowsNostr: z.boolean(),
  callback: z.string(),
  commentAllowed: z.number().gte(0),
  maxSendable: z.number().gte(0),
  metadata: z.string(),
  minSendable: z.number().gte(0),
  nostrPubkey: z.unknown().optional(),
  tag: z.string(),
});

export const zLnUrlCallbackResponse = z.object({
  disposable: z.unknown().optional(),
  pr: z.string(),
  routes: z.array(z.string()),
  successAction: z.unknown().optional(),
});

export const zLnUrlSuccessAction = z.object({
  description: z.unknown().optional(),
  message: z.unknown().optional(),
  tag: z.string(),
  url: z.unknown().optional(),
});

export const zNewInvoiceRequest = z.object({
  amount_msat: z.number().gte(0),
  description: z.unknown().optional(),
  expiry: z.unknown().optional(),
  wallet_id: z.unknown().optional(),
});

export const zNostrNIP05Response = z.object({
  names: z.object({}),
});

export const zOrderDirection = z.enum(['Asc', 'Desc']);

export const zPaymentResponse = z.object({
  amount_msat: z.number().gte(0),
  created_at: z.string().datetime(),
  currency: zCurrency,
  description: z.unknown().optional(),
  error: z.unknown().optional(),
  fee_msat: z.unknown().optional(),
  id: z.string().uuid(),
  ledger: zLedger,
  ln_address: z.unknown().optional(),
  metadata: z.unknown().optional(),
  payment_hash: z.unknown().optional(),
  payment_preimage: z.unknown().optional(),
  payment_time: z.unknown().optional(),
  status: z.enum(['Pending', 'Settled', 'Failed']),
  success_action: z.unknown().optional(),
  updated_at: z.unknown().optional(),
  wallet_id: z.string().uuid(),
});

export const zPaymentStatus = z.enum(['Pending', 'Settled', 'Failed']);

export const zPermission = z.enum([
  'read:wallet',
  'write:wallet',
  'read:ln_address',
  'write:ln_address',
  'read:transaction',
  'write:transaction',
  'read:ln_node',
  'write:ln_node',
  'read:api_key',
  'write:api_key',
]);

export const zRedeemOnchainRequest = z.object({
  feerate: z.number().gte(0),
  to_address: z.string(),
});

export const zRedeemOnchainResponse = z.object({
  txid: z.string(),
});

export const zRegisterLnAddressRequest = z.object({
  allows_nostr: z.boolean().optional(),
  nostr_pubkey: z.unknown().optional(),
  username: z.string(),
  wallet_id: z.unknown().optional(),
});

export const zRegisterWalletRequest = z.object({
  user_id: z.string(),
});

export const zSendOnchainPaymentRequest = z.object({
  amount_msat: z.number().gte(0),
  feerate: z.number().gte(0),
  recipient_address: z.string(),
});

export const zSendPaymentRequest = z.object({
  amount_msat: z.unknown().optional(),
  comment: z.unknown().optional(),
  input: z.string(),
  wallet_id: z.unknown().optional(),
});

export const zSetupInfo = z.object({
  sign_up_complete: z.boolean(),
  welcome_complete: z.boolean(),
});

export const zSignInRequest = z.object({
  password: z.string(),
});

export const zSignInResponse = z.object({
  token: z.string(),
});

export const zSignMessageRequest = z.object({
  message: z.string(),
});

export const zSignMessageResponse = z.object({
  signature: z.string(),
});

export const zSignUpRequest = z.object({
  password: z.string(),
});

export const zUpdateLnAddressRequest = z.object({
  active: z.unknown().optional(),
  allows_nostr: z.unknown().optional(),
  nostr_pubkey: z.unknown().optional(),
  username: z.unknown().optional(),
});

export const zVersionInfo = z.object({
  build_time: z.string(),
  version: z.string(),
});

export const zWalletLnAddressResponse = z.object({
  ln_address: z.unknown().optional(),
});

export const zWalletOverview = z.object({
  balance: zBalance,
  created_at: z.string().datetime(),
  id: z.string().uuid(),
  ln_address: z.unknown().optional(),
  n_contacts: z.number().gte(0),
  n_invoices: z.number().gte(0),
  n_payments: z.number().gte(0),
  updated_at: z.unknown().optional(),
  user_id: z.string(),
});

export const zWalletResponse = z.object({
  balance: zBalance,
  contacts: z.array(zContact),
  created_at: z.string().datetime(),
  id: z.string().uuid(),
  invoices: z.array(zInvoiceResponse),
  ln_address: z.unknown().optional(),
  payments: z.array(zPaymentResponse),
  updated_at: z.unknown().optional(),
  user_id: z.string(),
});
