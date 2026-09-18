import type {
  ListWebhookSubscriptionsData,
  CreateWebhookSubscriptionRequest,
  UpdateWebhookSubscriptionRequest,
  ListWebhookSubscriptionDeliveriesData,
} from 'src/lib/swissknife';

import useSWR, { mutate } from 'swr';

import { useAccountContext } from 'src/contexts/account';
import {
  getWebhook,
  createWebhook,
  updateWebhook,
  deleteWebhook,
  sendWebhookTest,
  getWebhookDelivery,
  listAccountWebhooks,
  rotateWebhookSecret,
  retryWebhookDelivery,
  listWebhookDeliveries,
  getWebhookSubscription,
  listWebhookSubscriptions,
  createWebhookSubscription,
  updateWebhookSubscription,
  deleteWebhookSubscription,
  sendWebhookSubscriptionTest,
  getWebhookSubscriptionDelivery,
  rotateWebhookSubscriptionSecret,
  retryWebhookSubscriptionDelivery,
  listWebhookSubscriptionDeliveries,
} from 'src/lib/swissknife';

import { endpointKeys } from './keys';

export type WebhookTarget = { id: string; wallet_id: string };
export type WebhookFilter = NonNullable<ListWebhookSubscriptionsData['query']>;
export type DeliveryFilter = NonNullable<ListWebhookSubscriptionDeliveriesData['query']>;

export function useWebhooks(isAdmin: boolean, query: WebhookFilter) {
  const { account } = useAccountContext();
  return useSWR(
    account ? [endpointKeys.webhooks, account.id, isAdmin, 'list', query] : null,
    async () =>
      isAdmin
        ? (await listWebhookSubscriptions<true>({ query })).data
        : (await listAccountWebhooks<true>({ query })).data
  );
}

export function useWebhook(isAdmin: boolean, id?: string, walletId?: string) {
  const { account } = useAccountContext();
  return useSWR(
    account && id ? [endpointKeys.webhooks, account.id, isAdmin, 'detail', id, walletId] : null,
    async () => {
      if (isAdmin) return (await getWebhookSubscription<true>({ path: { id: id! } })).data;
      if (walletId)
        return (await getWebhook<true>({ path: { id: id!, wallet_id: walletId } })).data;
      const data = (await listAccountWebhooks<true>({ query: { ids: [id!] } })).data;
      if (!data[0]) throw new Error('Webhook not found.');
      return data[0];
    }
  );
}

export function useWebhookDeliveries(
  isAdmin: boolean,
  target: WebhookTarget,
  query: DeliveryFilter,
  refreshInterval = 5000
) {
  const { account } = useAccountContext();
  return useSWR(
    account ? [endpointKeys.webhooks, account.id, isAdmin, 'deliveries', target.id, query] : null,
    async () =>
      isAdmin
        ? (await listWebhookSubscriptionDeliveries<true>({ path: { id: target.id }, query })).data
        : (await listWebhookDeliveries<true>({ path: target, query })).data,
    { refreshInterval }
  );
}

export function useWebhookDelivery(isAdmin: boolean, target: WebhookTarget, deliveryId?: string) {
  const { account } = useAccountContext();
  return useSWR(
    account && deliveryId
      ? [endpointKeys.webhooks, account.id, isAdmin, 'delivery', target.id, deliveryId]
      : null,
    async () =>
      isAdmin
        ? (
            await getWebhookSubscriptionDelivery<true>({
              path: { id: target.id, delivery_id: deliveryId! },
            })
          ).data
        : (await getWebhookDelivery<true>({ path: { ...target, delivery_id: deliveryId! } })).data,
    { refreshInterval: 5000 }
  );
}

export async function refreshWebhooks() {
  await mutate((key) => Array.isArray(key) && key[0] === endpointKeys.webhooks, undefined, {
    populateCache: false,
    revalidate: true,
    throwOnError: false,
  });
}

export async function saveWebhook(
  isAdmin: boolean,
  walletId: string,
  body: CreateWebhookSubscriptionRequest,
  id?: string
) {
  const response = id
    ? isAdmin
      ? await updateWebhookSubscription<true>({ path: { id }, body })
      : await updateWebhook<true>({ path: { id, wallet_id: walletId }, body })
    : isAdmin
      ? await createWebhookSubscription<true>({ body: { ...body, wallet_id: walletId } })
      : await createWebhook<true>({ path: { wallet_id: walletId }, body });
  // Return one-time secrets immediately, even when background revalidation fails.
  // They are never stored in SWR.
  void refreshWebhooks();
  return response.data;
}

export async function changeWebhook(
  isAdmin: boolean,
  target: WebhookTarget,
  body: UpdateWebhookSubscriptionRequest
) {
  const result = isAdmin
    ? await updateWebhookSubscription<true>({ path: { id: target.id }, body })
    : await updateWebhook<true>({ path: target, body });
  await refreshWebhooks();
  return result.data;
}

export async function removeWebhook(isAdmin: boolean, target: WebhookTarget) {
  if (isAdmin) await deleteWebhookSubscription<true>({ path: { id: target.id } });
  else await deleteWebhook<true>({ path: target });
  await refreshWebhooks();
}

export async function rotateWebhook(isAdmin: boolean, target: WebhookTarget) {
  const result = isAdmin
    ? await rotateWebhookSubscriptionSecret<true>({ path: { id: target.id } })
    : await rotateWebhookSecret<true>({ path: target });
  void refreshWebhooks();
  return result.data.signing_secret;
}

export async function testWebhook(isAdmin: boolean, target: WebhookTarget) {
  const result = isAdmin
    ? await sendWebhookSubscriptionTest<true>({ path: { id: target.id } })
    : await sendWebhookTest<true>({ path: target });
  await refreshWebhooks();
  return result.data;
}

export async function retryDelivery(isAdmin: boolean, target: WebhookTarget, deliveryId: string) {
  const result = isAdmin
    ? await retryWebhookSubscriptionDelivery<true>({
        path: { id: target.id, delivery_id: deliveryId },
      })
    : await retryWebhookDelivery<true>({ path: { ...target, delivery_id: deliveryId } });
  await refreshWebhooks();
  return result.data;
}
