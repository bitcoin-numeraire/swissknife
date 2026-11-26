'use client';

import type { Wallet } from 'src/lib/swissknife';
import type { MerchantSale } from '../merchant-checkout';

import { QRCode } from 'react-qrcode-logo';
import { useRef, useState, useEffect } from 'react';

import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import Stack from '@mui/material/Stack';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import Switch from '@mui/material/Switch';
import Dialog from '@mui/material/Dialog';
import Toolbar from '@mui/material/Toolbar';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import FormControlLabel from '@mui/material/FormControlLabel';

import { paths } from 'src/routes/paths';

import { fSats } from 'src/utils/format-number';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { newWalletInvoice, newWalletBtcAddress } from 'src/lib/swissknife';
import {
  useActiveWallet,
  useGetWalletInvoice,
  useListWalletInvoices,
} from 'src/actions/account-wallet';

import { CopyButton } from 'src/components/copy';
import { LoadingScreen } from 'src/components/loading-screen';

import {
  checkoutAmount,
  checkoutSettlement,
  restoreMerchantSale,
  checkoutPaymentRequest,
} from '../merchant-checkout';

const saleDuration = 15 * 60;

function Checkout({ wallet }: { wallet: Wallet }) {
  const { t } = useTranslate();
  const sessionKey = 'swissknife.pos.' + wallet.id;
  const feedbackKey = 'swissknife.pos.feedback.' + wallet.account_id;
  const [ready, setReady] = useState(false);
  const [sale, setSale] = useState<MerchantSale>();
  const [amount, setAmount] = useState('');
  const [description, setDescription] = useState('');
  const [now, setNow] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const [previous, setPrevious] = useState<string>();
  const [sound, setSound] = useState(false);
  const [haptics, setHaptics] = useState(false);
  const audio = useRef<AudioContext | undefined>(undefined);
  const lastFeedback = useRef<string | undefined>(undefined);
  const creating = useRef(false);

  useEffect(() => {
    try {
      setSale(restoreMerchantSale(sessionStorage.getItem(sessionKey), wallet.id));
      setPrevious(sessionStorage.getItem(sessionKey + '.last') || undefined);
      lastFeedback.current = sessionStorage.getItem(sessionKey + '.feedback') || undefined;
      const prefs = JSON.parse(localStorage.getItem(feedbackKey) ?? '{}');
      setSound(prefs?.sound === true);
      setHaptics(prefs?.haptics === true);
    } catch {
      /* Storage is optional in private browsing and embedded views. */
    }
    setNow(Date.now());
    setReady(true);
    return () => {
      if (audio.current) void audio.current.close().catch(() => undefined);
    };
  }, [wallet.id, sessionKey, feedbackKey]);

  useEffect(() => {
    if (!sale) return undefined;
    const wake = () => setNow(Date.now());
    wake();
    const timer = window.setTimeout(wake, Math.max(0, sale.expiresAt - Date.now()));
    document.addEventListener('visibilitychange', wake);
    return () => {
      window.clearTimeout(timer);
      document.removeEventListener('visibilitychange', wake);
    };
  }, [sale]);

  const { invoice, invoiceError, invoiceValidating } = useGetWalletInvoice(
    sale?.invoiceId ?? '',
    wallet.id
  );
  const deposits = useListWalletInvoices(
    { bitcoin_address: sale?.address },
    wallet.id,
    Boolean(sale?.address)
  );
  const settlement = sale
    ? checkoutSettlement(sale, [...(deposits.invoices ?? []), ...(invoice ? [invoice] : [])], now)
    : undefined;
  const receipt = settlement?.status === 'paid' ? settlement.invoiceIds[0] : undefined;

  const saleId = sale?.invoiceId || sale?.address;
  useEffect(() => {
    if (!receipt || !saleId || lastFeedback.current === saleId) return;
    lastFeedback.current = saleId;
    setPrevious(receipt);
    try {
      sessionStorage.setItem(sessionKey + '.last', receipt);
      sessionStorage.setItem(sessionKey + '.feedback', saleId);
    } catch {
      /* The current tab still retains its receipt. */
    }
    if (haptics) navigator.vibrate?.(120);
    const context = audio.current;
    if (sound && context?.state === 'running') {
      const tone = context.createOscillator();
      const gain = context.createGain();
      const start = context.currentTime;
      gain.gain.setValueAtTime(0.08, start);
      gain.gain.exponentialRampToValueAtTime(0.001, start + 0.16);
      tone.frequency.value = 660;
      tone.connect(gain).connect(context.destination);
      tone.start(start);
      tone.stop(start + 0.16);
    }
  }, [receipt, saleId, sessionKey, haptics, sound]);

  const unlockSound = () => {
    if (typeof window.AudioContext !== 'function') return;
    try {
      audio.current ??= new AudioContext();
      void audio.current.resume().catch(() => undefined);
    } catch {
      /* The visual receipt remains available. */
    }
  };

  const changeFeedback = (nextSound: boolean, nextHaptics: boolean) => {
    setSound(nextSound);
    setHaptics(nextHaptics);
    if (nextSound) unlockSound();
    try {
      localStorage.setItem(feedbackKey, JSON.stringify({ sound: nextSound, haptics: nextHaptics }));
    } catch {
      /* Optional preference storage. */
    }
  };

  const reset = () => {
    try {
      sessionStorage.removeItem(sessionKey);
    } catch {
      /* In-memory checkout still resets. */
    }
    setSale(undefined);
    setAmount('');
    setDescription('');
    setError(false);
  };

  const start = async () => {
    const amountSats = checkoutAmount(amount);
    if (!amountSats || creating.current) return;
    creating.current = true;
    setBusy(true);
    setError(false);
    if (sound) unlockSound();
    try {
      const path = { wallet_id: wallet.id };
      const memo = description.trim();
      const [lightning, onchain] = await Promise.allSettled([
        newWalletInvoice<true>({
          path,
          body: {
            amount_msat: amountSats * 1000,
            description: memo,
            expiry: saleDuration,
          },
        }),
        newWalletBtcAddress<true>({
          path,
          body: { dedicated: true, description: memo || undefined },
        }),
      ]);
      const bill = lightning.status === 'fulfilled' ? lightning.value.data : undefined;
      const output = onchain.status === 'fulfilled' ? onchain.value.data : undefined;
      const address =
        output?.dedicated && output.wallet_id === wallet.id ? output.address : undefined;
      const bolt11 = bill?.wallet_id === wallet.id ? bill.ln_invoice?.bolt11 : undefined;
      if (!bolt11 && !address) {
        setError(true);
        return;
      }
      const next = {
        walletId: wallet.id,
        amountSats,
        description: memo,
        address,
        bolt11: bolt11 || undefined,
        invoiceId: bolt11 ? bill?.id : undefined,
        expiresAt: bill?.ln_invoice?.expires_at
          ? new Date(bill.ln_invoice.expires_at).getTime()
          : Date.now() + saleDuration * 1000,
      };
      try {
        sessionStorage.setItem(sessionKey, JSON.stringify(next));
      } catch {
        /* An in-memory sale also works. */
      }
      setNow(Date.now());
      setSale(next);
    } finally {
      creating.current = false;
      setBusy(false);
    }
  };

  if (!ready) return <LoadingScreen />;
  const receiptsReady =
    (!sale?.invoiceId || (invoice?.id === sale.invoiceId && !invoiceError && !invoiceValidating)) &&
    (!sale?.address ||
      (deposits.invoices && !deposits.invoicesError && !deposits.invoicesValidating));
  const paymentRequest =
    sale && settlement && receiptsReady ? checkoutPaymentRequest(sale, settlement, now) : undefined;
  return (
    <Stack spacing={3} sx={{ width: '100%', maxWidth: 480, mx: 'auto', px: 2.5, pb: 4 }}>
      <Typography variant="overline" color="text.secondary">
        {wallet.label || wallet.asset?.display_ticker}
      </Typography>
      {!sale ? (
        <Stack
          component="form"
          spacing={2.5}
          onSubmit={(event) => {
            event.preventDefault();
            void start();
          }}
        >
          <Typography component="h1" variant="h4">
            {t('pos.title')}
          </Typography>
          <TextField
            autoFocus
            required
            label={t('pos.amount')}
            value={amount}
            onChange={(event) => setAmount(event.target.value)}
            slotProps={{ htmlInput: { inputMode: 'numeric', pattern: '[0-9]+' } }}
          />
          <TextField
            label={t('pos.reference')}
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            slotProps={{ htmlInput: { maxLength: 100 } }}
          />
          <Stack direction="row" sx={{ flexWrap: 'wrap' }}>
            <FormControlLabel
              control={
                <Switch
                  checked={sound}
                  onChange={(event) => changeFeedback(event.target.checked, haptics)}
                />
              }
              label={t('pos.sound')}
            />
            <FormControlLabel
              control={
                <Switch
                  checked={haptics}
                  onChange={(event) => changeFeedback(sound, event.target.checked)}
                />
              }
              label={t('pos.haptics')}
            />
          </Stack>
          {error && <Alert severity="error">{t('pos.issue_error')}</Alert>}
          <Button
            size="large"
            variant="contained"
            type="submit"
            loading={busy}
            disabled={!checkoutAmount(amount)}
          >
            {t('pos.start')}
          </Button>
          {previous && <Button href={paths.wallet.invoice(previous)}>{t('pos.previous')}</Button>}
        </Stack>
      ) : (
        <Stack spacing={2.5}>
          <Card
            sx={{
              p: { xs: 2.5, sm: 4 },
              textAlign: 'center',
              ...(receipt && { bgcolor: 'success.darker', color: 'common.white' }),
            }}
          >
            <Stack spacing={2} sx={{ alignItems: 'center' }} aria-live="polite" aria-atomic="true">
              {receipt && (
                <Box
                  component="img"
                  alt={CONFIG.appName}
                  src={CONFIG.assetsDir + '/logo/logo_square_negative.svg'}
                  sx={{ width: 56, height: 56 }}
                />
              )}
              <Typography component="h1" variant="h4">
                {t('pos.' + settlement?.status)}
              </Typography>
              <Typography variant="h3">{fSats(sale.amountSats)} sats</Typography>
              {sale.description && (
                <Typography sx={{ overflowWrap: 'anywhere' }}>{sale.description}</Typography>
              )}
              {settlement?.status === 'incoming' && (
                <Typography>
                  {t('pos.pending_amount', { amount: fSats(settlement.pendingMsat / 1000) })}
                </Typography>
              )}
              {settlement && settlement.receivedMsat > 0 && (
                <Typography>
                  {t('pos.received', {
                    amount: fSats(settlement.receivedMsat / 1000, { maximumFractionDigits: 3 }),
                  })}
                </Typography>
              )}
              {paymentRequest && (
                <>
                  <Box
                    sx={{
                      width: '100%',
                      maxWidth: 300,
                      bgcolor: 'common.white',
                      p: 1.5,
                      '& canvas': {
                        width: '100% !important',
                        height: 'auto !important',
                        display: 'block',
                      },
                    }}
                  >
                    <QRCode value={paymentRequest.uri} size={384} quietZone={12} />
                  </Box>
                  <Stack direction="row" sx={{ alignItems: 'center' }}>
                    <Typography variant="body2">
                      {t(paymentRequest.method)}
                      {settlement?.status === 'partial' &&
                        ` · ${fSats(paymentRequest.amountSats)} sats`}
                    </Typography>
                    <CopyButton value={paymentRequest.uri} title={t('copy')} />
                  </Stack>
                </>
              )}
            </Stack>
          </Card>
          {(invoiceError || deposits.invoicesError) && (
            <Alert severity="warning">{t('pos.sync_error')}</Alert>
          )}
          {settlement?.invoiceIds.map((id) => (
            <Button key={id} href={paths.wallet.invoice(id)}>
              {t('pos.receipt')}
            </Button>
          ))}
          <Button
            size="large"
            color="inherit"
            variant={receipt ? 'contained' : 'outlined'}
            onClick={reset}
          >
            {t(receipt || settlement?.status === 'expired' ? 'pos.next' : 'pos.cancel')}
          </Button>
          {!receipt && (
            <Typography variant="caption" color="text.secondary">
              {t('pos.stop_note')}
            </Typography>
          )}
        </Stack>
      )}
    </Stack>
  );
}

export function MerchantCheckoutView() {
  const { t } = useTranslate();
  const { wallet, walletLoading } = useActiveWallet();
  return (
    <Dialog open fullScreen aria-label={t('pos.title')}>
      <Toolbar sx={{ justifyContent: 'flex-end', mb: { xs: 2, sm: 5 } }}>
        <Button href={paths.wallet.root} color="inherit">
          {t('pos.back')}
        </Button>
      </Toolbar>
      {walletLoading ? (
        <LoadingScreen />
      ) : wallet?.asset?.protocol === 'bitcoin' && wallet.asset.asset_ref === 'native' ? (
        <Checkout key={wallet.id} wallet={wallet} />
      ) : (
        <Alert severity="info">{t('pos.select_bitcoin')}</Alert>
      )}
    </Dialog>
  );
}
