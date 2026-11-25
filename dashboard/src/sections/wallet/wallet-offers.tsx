'use client';

import useSWR from 'swr';
import { useRef, useState } from 'react';
import { QRCode } from 'react-qrcode-logo';

import {
  Box,
  Alert,
  Stack,
  Button,
  Checkbox,
  TextField,
  Typography,
  FormControlLabel,
} from '@mui/material';

import { handleActionError } from 'src/utils/errors';

import { useTranslate } from 'src/locales';
import { useAccountContext } from 'src/contexts/account';
import {
  listOffers,
  setupCheck,
  createOffer,
  disableOffer,
  listWalletOffers,
  type WalletOffer,
  createWalletOffer,
  disableWalletOffer,
} from 'src/lib/swissknife';

import { CopyButton } from 'src/components/copy';
import { SatsWithIcon } from 'src/components/bitcoin';
import { ConfirmDialog } from 'src/components/custom-dialog';

import { requestId } from './request-id';

const pageSize = 10;

export function WalletOffers({ walletId, isAdmin }: { walletId: string; isAdmin: boolean }) {
  const { t } = useTranslate();
  const { account } = useAccountContext();
  const [description, setDescription] = useState('');
  const [sats, setSats] = useState('');
  const [singleUse, setSingleUse] = useState(false);
  const [page, setPage] = useState(0);
  const [showQr, setShowQr] = useState<string>();
  const [disabling, setDisabling] = useState<WalletOffer>();
  const [busy, setBusy] = useState(false);
  const sending = useRef(false);
  const scope = JSON.stringify(['wallet-offers', account?.id, walletId, isAdmin]);
  const capability = useSWR(
    'swissknife-offer-capability',
    async () => (await setupCheck({ throwOnError: true })).data.bolt12_offers
  );
  const { data, error, mutate } = useSWR(
    account?.id ? [scope, page] : null,
    async () => {
      const query = { offset: page * pageSize, limit: pageSize + 1 };
      const result = isAdmin
        ? await listOffers({ query: { ...query, wallet_id: walletId }, throwOnError: true })
        : await listWalletOffers({ path: { wallet_id: walletId }, query, throwOnError: true });
      return result.data;
    },
    { refreshInterval: 10000 }
  );
  const msat = sats ? Number(sats) * 1000 : null;
  const amountValid =
    msat === null || (/^\d+$/.test(sats) && Number.isSafeInteger(msat) && msat > 0);
  const create = async () => {
    if (sending.current || !description.trim() || !amountValid) return;
    sending.current = true;
    setBusy(true);
    try {
      const terms = { description: description.trim(), amount_msat: msat, single_use: singleUse };
      const fingerprint = JSON.stringify(terms);
      const previous = JSON.parse(sessionStorage.getItem(scope) ?? 'null') as {
        fingerprint: string;
        key: string;
      } | null;
      const key = previous?.fingerprint === fingerprint ? previous.key : requestId();
      sessionStorage.setItem(scope, JSON.stringify({ fingerprint, key }));
      const body = { ...terms, idempotency_key: key };
      const result = isAdmin
        ? await createOffer({ body: { ...body, wallet_id: walletId }, throwOnError: true })
        : await createWalletOffer({ path: { wallet_id: walletId }, body, throwOnError: true });
      sessionStorage.removeItem(scope);
      setPage(0);
      setShowQr(result.data.id);
      setDescription('');
      setSats('');
      setSingleUse(false);
      await mutate();
    } catch (e) {
      handleActionError(e);
    } finally {
      sending.current = false;
      setBusy(false);
    }
  };
  const disable = async () => {
    if (!disabling || sending.current) return;
    sending.current = true;
    setBusy(true);
    try {
      const id = disabling.id;
      if (isAdmin) await disableOffer({ path: { id }, throwOnError: true });
      else await disableWalletOffer({ path: { wallet_id: walletId, id }, throwOnError: true });
      setDisabling(undefined);
      await mutate();
    } catch (e) {
      handleActionError(e);
    } finally {
      sending.current = false;
      setBusy(false);
    }
  };

  return (
    <Stack spacing={2}>
      {capability.data === false && <Alert severity="info">{t('offers.requires_cln')}</Alert>}
      {capability.error && <Alert severity="error">{t('offers.load_error')}</Alert>}
      {capability.data && (
        <>
          <TextField
            label={t('offers.description')}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            slotProps={{ htmlInput: { maxLength: 256 } }}
          />
          <TextField
            label={t('offers.amount')}
            value={sats}
            onChange={(e) => setSats(e.target.value)}
            error={!amountValid}
            helperText={t('offers.amount_help')}
            slotProps={{ htmlInput: { inputMode: 'numeric' } }}
          />
          <FormControlLabel
            control={
              <Checkbox checked={singleUse} onChange={(e) => setSingleUse(e.target.checked)} />
            }
            label={t('offers.single_use')}
          />
          <Button
            variant="contained"
            color="inherit"
            disabled={!description.trim() || !amountValid}
            loading={busy}
            onClick={create}
          >
            {t('offers.create')}
          </Button>
        </>
      )}
      {error && <Alert severity="error">{t('offers.load_error')}</Alert>}
      {data?.slice(0, pageSize).map((offer) => (
        <Box key={offer.id} sx={{ p: 2, border: 1, borderColor: 'divider', borderRadius: 1 }}>
          <Stack spacing={1}>
            <Typography variant="subtitle2" sx={{ overflowWrap: 'anywhere' }}>
              {offer.description}
            </Typography>
            <Stack
              direction="row"
              spacing={1}
              sx={{ alignItems: 'center', justifyContent: 'space-between' }}
            >
              {offer.amount_msat != null ? (
                <SatsWithIcon amountMSats={offer.amount_msat} showMillisatsTooltip />
              ) : (
                <Typography variant="body2">{t('offers.variable')}</Typography>
              )}
              <Typography variant="caption" color="text.secondary">
                {t('offers.' + offer.state)}
              </Typography>
            </Stack>
            {offer.bolt12 && offer.state === 'active' && (
              <>
                <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                  <Button
                    size="small"
                    onClick={() => setShowQr(showQr === offer.id ? undefined : offer.id)}
                  >
                    {t('offers.qr')}
                  </Button>
                  <CopyButton value={offer.bolt12} title={t('copy')} />
                  <Button size="small" color="error" onClick={() => setDisabling(offer)}>
                    {t('offers.disable')}
                  </Button>
                </Stack>
                {showQr === offer.id && (
                  <Box
                    sx={{
                      alignSelf: 'center',
                      maxWidth: 1,
                      lineHeight: 0,
                      bgcolor: 'common.white',
                      '& canvas': { maxWidth: '100%', height: 'auto !important' },
                    }}
                  >
                    <QRCode value={offer.bolt12.toUpperCase()} size={300} ecLevel="L" />
                  </Box>
                )}
              </>
            )}
          </Stack>
        </Box>
      ))}
      {(page > 0 || (data?.length ?? 0) > pageSize) && (
        <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
          <Button disabled={page === 0} onClick={() => setPage((p) => p - 1)}>
            {t('offers.previous')}
          </Button>
          <Button disabled={(data?.length ?? 0) <= pageSize} onClick={() => setPage((p) => p + 1)}>
            {t('offers.next')}
          </Button>
        </Stack>
      )}
      <ConfirmDialog
        open={Boolean(disabling)}
        onClose={() => setDisabling(undefined)}
        title={t('offers.disable')}
        content={t('offers.disable_help', { description: disabling?.description })}
        action={
          <Button variant="contained" color="error" loading={busy} onClick={disable}>
            {t('offers.disable')}
          </Button>
        }
      />
    </Stack>
  );
}
