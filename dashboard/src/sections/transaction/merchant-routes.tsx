'use client';

import type { WalletMerchantRoute, WalletMerchantCatalog } from 'src/lib/swissknife';

import useSWR from 'swr';
import { useState } from 'react';

import {
  Alert,
  Stack,
  Button,
  Switch,
  MenuItem,
  TextField,
  Typography,
  LinearProgress,
  FormControlLabel,
} from '@mui/material';

import { useTranslate } from 'src/locales';
import { CONFIG } from 'src/global-config';
import { endpointKeys } from 'src/actions/keys';
import { setWalletMerchantRoute, listWalletMerchantRoutes } from 'src/lib/swissknife';

import { toast } from 'src/components/snackbar';

export function MerchantRoutes({ walletId }: { walletId: string }) {
  const { t } = useTranslate();
  const [selected, select] = useState('');
  const catalog = useSWR(
    endpointKeys.accountWallet.merchantRoutes(walletId),
    async () =>
      (await listWalletMerchantRoutes<true>({ path: { wallet_id: walletId }, throwOnError: true }))
        .data
  );
  if (catalog.error) return <Alert severity="error">{t('merchant_tools.load_error')}</Alert>;
  if (!catalog.data) return <LinearProgress aria-label={t('merchant_tools.routes')} />;
  const { origins, routes } = catalog.data;
  if (!origins.length && !routes.length)
    return <Typography color="text.secondary">{t('merchant_tools.no_origins')}</Typography>;
  const selection = selected || (origins.length ? '' : routes[0].name);
  const route = routes.find((item) => item.name === selection);
  return (
    <Stack spacing={3}>
      <TextField
        select
        label={t('merchant_tools.choose')}
        value={selection}
        onChange={(e) => select(e.target.value)}
        slotProps={{ select: { displayEmpty: true }, inputLabel: { shrink: true } }}
      >
        {!!origins.length && <MenuItem value="">{t('merchant_tools.new')}</MenuItem>}
        {routes.map((item) => (
          <MenuItem key={item.name} value={item.name}>
            {item.name}
          </MenuItem>
        ))}
      </TextField>
      <RouteForm
        key={selection}
        walletId={walletId}
        catalog={catalog.data}
        route={route}
        onSave={async (saved) => {
          await catalog.mutate(
            (current) =>
              current && {
                ...current,
                routes: [...current.routes.filter((item) => item.name !== saved.name), saved].sort(
                  (a, b) => a.name.localeCompare(b.name)
                ),
              },
            { revalidate: false }
          );
          select(saved.name);
        }}
      />
    </Stack>
  );
}

function RouteForm({
  walletId,
  catalog,
  route,
  onSave,
}: {
  walletId: string;
  catalog: WalletMerchantCatalog;
  route?: WalletMerchantRoute;
  onSave: (value: WalletMerchantRoute) => Promise<void>;
}) {
  const { t, i18n } = useTranslate();
  const [name, setName] = useState(route?.name ?? '');
  const [origin, setOrigin] = useState(route?.terms.origin ?? catalog.origins[0]?.name ?? '');
  const [price, setPrice] = useState(route?.terms.price_msat ?? '');
  const [enabled, setEnabled] = useState(route?.terms.enabled ?? true);
  const [saving, setSaving] = useState(false);
  const [failed, setFailed] = useState(false);
  const source = catalog.origins.find((item) => item.name === origin);
  const validName = /^[a-z0-9-]{1,57}$/.test(name);
  const duplicate = !route && catalog.routes.some((item) => item.name === name);
  const validPrice =
    !!source &&
    /^[1-9][0-9]{0,18}$/.test(price) &&
    BigInt(price) <= BigInt(source.maximum_price_msat);
  const amount = (value: string) => new Intl.NumberFormat(i18n.language).format(BigInt(value));
  async function save(event: React.FormEvent) {
    event.preventDefault();
    if (!validName || duplicate || !validPrice || saving) return;
    const terms = { origin, price_msat: price, enabled };
    setSaving(true);
    setFailed(false);
    try {
      await setWalletMerchantRoute<true>({
        path: { wallet_id: walletId, name },
        body: terms,
        throwOnError: true,
      });
      await onSave({ name, terms });
      toast.success(t('merchant_tools.saved'));
    } catch {
      setFailed(true);
    } finally {
      setSaving(false);
    }
  }
  return (
    <Stack component="form" onSubmit={save} spacing={2.5}>
      {route && (
        <TextField
          label={t('merchant_tools.public_url')}
          value={
            new URL(
              '/l402/w/' + walletId + '/' + route.name,
              CONFIG.serverUrl || window.location.origin
            ).href
          }
          slotProps={{ input: { readOnly: true } }}
          onFocus={(e) => e.target.select()}
        />
      )}
      {!source && route && <Alert severity="warning">{t('merchant_tools.withdrawn')}</Alert>}
      {!route && (
        <TextField
          required
          label={t('merchant_tools.name')}
          value={name}
          onChange={(e) => setName(e.target.value)}
          error={!!name && (!validName || duplicate)}
          helperText={duplicate ? t('merchant_tools.duplicate') : t('merchant_tools.name_help')}
        />
      )}
      <TextField
        select
        label={t('merchant_tools.origin')}
        value={origin}
        disabled={!!route || saving}
        onChange={(e) => setOrigin(e.target.value)}
      >
        {!source && route && <MenuItem value={origin}>{origin}</MenuItem>}
        {catalog.origins.map((item) => (
          <MenuItem key={item.name} value={item.name}>
            {item.name}
          </MenuItem>
        ))}
      </TextField>
      <TextField
        required
        label={t(source?.uses_quote ? 'merchant_tools.quote_limit' : 'merchant_tools.price')}
        value={price}
        disabled={!source || saving}
        onChange={(e) => setPrice(e.target.value)}
        error={!!price && !!source && !validPrice}
        helperText={
          source && t('merchant_tools.maximum', { value: amount(source.maximum_price_msat) })
        }
        slotProps={{ htmlInput: { inputMode: 'numeric', maxLength: 19 } }}
      />
      {source && (
        <Typography variant="body2" color="text.secondary">
          {t('merchant_tools.duration', { value: amount(String(source.access_seconds)) })}
          {source.requests_per_payment &&
            ' ' +
              t('merchant_tools.requests', { value: amount(String(source.requests_per_payment)) })}
        </Typography>
      )}
      <FormControlLabel
        control={
          <Switch
            checked={enabled}
            onChange={(e) => setEnabled(e.target.checked)}
            disabled={!source || saving}
          />
        }
        label={t('merchant_tools.sales')}
      />
      {failed && <Alert severity="error">{t('merchant_tools.save_error')}</Alert>}
      <Button
        type="submit"
        variant="contained"
        loading={saving}
        disabled={!validName || duplicate || !validPrice}
        sx={{ alignSelf: 'flex-start' }}
      >
        {t(route ? 'merchant_tools.save' : 'merchant_tools.publish')}
      </Button>
    </Stack>
  );
}
