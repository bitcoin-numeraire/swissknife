// This file is auto-generated by @hey-api/openapi-ts

import type {
  PayResponse,
  GetApiKeyResponse,
  WalletPayResponse,
  GetWalletResponse,
  GetInvoiceResponse,
  GetAddressResponse,
  GetPaymentResponse,
  ListApiKeysResponse,
  ListWalletsResponse,
  CreateApiKeyResponse,
  ListInvoicesResponse,
  ListContactsResponse,
  ListPaymentsResponse,
  ListAddressesResponse,
  UpdateAddressResponse,
  GetUserWalletResponse,
  RegisterWalletResponse,
  GenerateInvoiceResponse,
  RegisterAddressResponse,
  GetWalletApiKeyResponse,
  NewWalletInvoiceResponse,
  GetWalletInvoiceResponse,
  GetWalletAddressResponse,
  GetWalletPaymentResponse,
  ListWalletApiKeysResponse,
  CreateWalletApiKeyResponse,
  ListWalletInvoicesResponse,
  ListWalletPaymentsResponse,
  UpdateWalletAddressResponse,
  ListWalletOverviewsResponse,
  RegisterWalletAddressResponse,
} from './types.gen';

const apiKeyResponseSchemaResponseTransformer = (data: any) => {
  data.created_at = new Date(data.created_at);
  if (data.expires_at) {
    data.expires_at = new Date(data.expires_at);
  }
  return data;
};

export const listApiKeysResponseTransformer = async (data: any): Promise<ListApiKeysResponse> => {
  data = data.map((item: any) => apiKeyResponseSchemaResponseTransformer(item));
  return data;
};

export const createApiKeyResponseTransformer = async (data: any): Promise<CreateApiKeyResponse> => {
  data = apiKeyResponseSchemaResponseTransformer(data);
  return data;
};

export const getApiKeyResponseTransformer = async (data: any): Promise<GetApiKeyResponse> => {
  data = apiKeyResponseSchemaResponseTransformer(data);
  return data;
};

const lnInvoiceResponseSchemaResponseTransformer = (data: any) => {
  data.expires_at = new Date(data.expires_at);
  return data;
};

const invoiceResponseSchemaResponseTransformer = (data: any) => {
  data.created_at = new Date(data.created_at);
  if (data.ln_invoice) {
    data.ln_invoice = lnInvoiceResponseSchemaResponseTransformer(data.ln_invoice);
  }
  if (data.payment_time) {
    data.payment_time = new Date(data.payment_time);
  }
  data.timestamp = new Date(data.timestamp);
  if (data.updated_at) {
    data.updated_at = new Date(data.updated_at);
  }
  return data;
};

export const listInvoicesResponseTransformer = async (data: any): Promise<ListInvoicesResponse> => {
  data = data.map((item: any) => invoiceResponseSchemaResponseTransformer(item));
  return data;
};

export const generateInvoiceResponseTransformer = async (
  data: any
): Promise<GenerateInvoiceResponse> => {
  data = invoiceResponseSchemaResponseTransformer(data);
  return data;
};

export const getInvoiceResponseTransformer = async (data: any): Promise<GetInvoiceResponse> => {
  data = invoiceResponseSchemaResponseTransformer(data);
  return data;
};

const lnAddressSchemaResponseTransformer = (data: any) => {
  data.created_at = new Date(data.created_at);
  if (data.updated_at) {
    data.updated_at = new Date(data.updated_at);
  }
  return data;
};

export const listAddressesResponseTransformer = async (
  data: any
): Promise<ListAddressesResponse> => {
  data = data.map((item: any) => lnAddressSchemaResponseTransformer(item));
  return data;
};

export const registerAddressResponseTransformer = async (
  data: any
): Promise<RegisterAddressResponse> => {
  data = lnAddressSchemaResponseTransformer(data);
  return data;
};

export const getAddressResponseTransformer = async (data: any): Promise<GetAddressResponse> => {
  data = lnAddressSchemaResponseTransformer(data);
  return data;
};

export const updateAddressResponseTransformer = async (
  data: any
): Promise<UpdateAddressResponse> => {
  data = lnAddressSchemaResponseTransformer(data);
  return data;
};

const contactSchemaResponseTransformer = (data: any) => {
  data.contact_since = new Date(data.contact_since);
  return data;
};

const paymentResponseSchemaResponseTransformer = (data: any) => {
  data.created_at = new Date(data.created_at);
  if (data.payment_time) {
    data.payment_time = new Date(data.payment_time);
  }
  if (data.updated_at) {
    data.updated_at = new Date(data.updated_at);
  }
  return data;
};

const walletResponseSchemaResponseTransformer = (data: any) => {
  data.contacts = data.contacts.map((item: any) => contactSchemaResponseTransformer(item));
  data.created_at = new Date(data.created_at);
  data.invoices = data.invoices.map((item: any) => invoiceResponseSchemaResponseTransformer(item));
  if (data.ln_address) {
    data.ln_address = lnAddressSchemaResponseTransformer(data.ln_address);
  }
  data.payments = data.payments.map((item: any) => paymentResponseSchemaResponseTransformer(item));
  if (data.updated_at) {
    data.updated_at = new Date(data.updated_at);
  }
  return data;
};

export const getUserWalletResponseTransformer = async (
  data: any
): Promise<GetUserWalletResponse> => {
  data = walletResponseSchemaResponseTransformer(data);
  return data;
};

export const listWalletApiKeysResponseTransformer = async (
  data: any
): Promise<ListWalletApiKeysResponse> => {
  data = data.map((item: any) => apiKeyResponseSchemaResponseTransformer(item));
  return data;
};

export const createWalletApiKeyResponseTransformer = async (
  data: any
): Promise<CreateWalletApiKeyResponse> => {
  data = apiKeyResponseSchemaResponseTransformer(data);
  return data;
};

export const getWalletApiKeyResponseTransformer = async (
  data: any
): Promise<GetWalletApiKeyResponse> => {
  data = apiKeyResponseSchemaResponseTransformer(data);
  return data;
};

export const listContactsResponseTransformer = async (data: any): Promise<ListContactsResponse> => {
  data = data.map((item: any) => contactSchemaResponseTransformer(item));
  return data;
};

export const listWalletInvoicesResponseTransformer = async (
  data: any
): Promise<ListWalletInvoicesResponse> => {
  data = data.map((item: any) => invoiceResponseSchemaResponseTransformer(item));
  return data;
};

export const newWalletInvoiceResponseTransformer = async (
  data: any
): Promise<NewWalletInvoiceResponse> => {
  data = invoiceResponseSchemaResponseTransformer(data);
  return data;
};

export const getWalletInvoiceResponseTransformer = async (
  data: any
): Promise<GetWalletInvoiceResponse> => {
  data = invoiceResponseSchemaResponseTransformer(data);
  return data;
};

const walletLnAddressResponseSchemaResponseTransformer = (data: any) => {
  if (data.ln_address) {
    data.ln_address = lnAddressSchemaResponseTransformer(data.ln_address);
  }
  return data;
};

export const getWalletAddressResponseTransformer = async (
  data: any
): Promise<GetWalletAddressResponse> => {
  data = walletLnAddressResponseSchemaResponseTransformer(data);
  return data;
};

export const registerWalletAddressResponseTransformer = async (
  data: any
): Promise<RegisterWalletAddressResponse> => {
  data = lnAddressSchemaResponseTransformer(data);
  return data;
};

export const updateWalletAddressResponseTransformer = async (
  data: any
): Promise<UpdateWalletAddressResponse> => {
  data = lnAddressSchemaResponseTransformer(data);
  return data;
};

export const listWalletPaymentsResponseTransformer = async (
  data: any
): Promise<ListWalletPaymentsResponse> => {
  data = data.map((item: any) => paymentResponseSchemaResponseTransformer(item));
  return data;
};

export const walletPayResponseTransformer = async (data: any): Promise<WalletPayResponse> => {
  data = paymentResponseSchemaResponseTransformer(data);
  return data;
};

export const getWalletPaymentResponseTransformer = async (
  data: any
): Promise<GetWalletPaymentResponse> => {
  data = paymentResponseSchemaResponseTransformer(data);
  return data;
};

export const listPaymentsResponseTransformer = async (data: any): Promise<ListPaymentsResponse> => {
  data = data.map((item: any) => paymentResponseSchemaResponseTransformer(item));
  return data;
};

export const payResponseTransformer = async (data: any): Promise<PayResponse> => {
  data = paymentResponseSchemaResponseTransformer(data);
  return data;
};

export const getPaymentResponseTransformer = async (data: any): Promise<GetPaymentResponse> => {
  data = paymentResponseSchemaResponseTransformer(data);
  return data;
};

export const listWalletsResponseTransformer = async (data: any): Promise<ListWalletsResponse> => {
  data = data.map((item: any) => walletResponseSchemaResponseTransformer(item));
  return data;
};

export const registerWalletResponseTransformer = async (
  data: any
): Promise<RegisterWalletResponse> => {
  data = walletResponseSchemaResponseTransformer(data);
  return data;
};

const walletOverviewSchemaResponseTransformer = (data: any) => {
  data.created_at = new Date(data.created_at);
  if (data.ln_address) {
    data.ln_address = lnAddressSchemaResponseTransformer(data.ln_address);
  }
  if (data.updated_at) {
    data.updated_at = new Date(data.updated_at);
  }
  return data;
};

export const listWalletOverviewsResponseTransformer = async (
  data: any
): Promise<ListWalletOverviewsResponse> => {
  data = data.map((item: any) => walletOverviewSchemaResponseTransformer(item));
  return data;
};

export const getWalletResponseTransformer = async (data: any): Promise<GetWalletResponse> => {
  data = walletResponseSchemaResponseTransformer(data);
  return data;
};