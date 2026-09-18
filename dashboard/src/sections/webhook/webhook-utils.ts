export function webhookHref(
  params: { toString: () => string },
  values: Record<string, string | undefined>
) {
  const next = new URLSearchParams(params.toString());
  next.set('tab', 'webhooks');
  Object.entries(values).forEach(([key, value]) =>
    value ? next.set(key, value) : next.delete(key)
  );
  return `/developers?${next}`;
}
