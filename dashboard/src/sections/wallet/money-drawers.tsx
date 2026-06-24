'use client';

import type { IFiatPrices } from 'src/types/bitcoin';
import type { Contact, Invoice, Payment, LnAddress, BtcAddress } from 'src/lib/swissknife';

import { bech32, bech32m } from 'bech32';
import { QRCode } from 'react-qrcode-logo';
import { decode } from 'light-bolt11-decoder';
import { useMemo, useState, useCallback } from 'react';
import { useCopyToClipboard } from 'minimal-shared/hooks';

import Box from '@mui/material/Box';
import Chip from '@mui/material/Chip';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Drawer from '@mui/material/Drawer';
import Divider from '@mui/material/Divider';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';

import { satsToFiat } from 'src/utils/fiat';
import { displayLnAddress } from 'src/utils/lnurl';
import { fCurrency } from 'src/utils/format-number';
import { handleActionError } from 'src/utils/errors';
import { truncateText } from 'src/utils/format-string';

import { useTranslate } from 'src/locales';
import { walletPay, BtcAddressType, newWalletInvoice, newWalletBtcAddress } from 'src/lib/swissknife';

import { Label } from 'src/components/label';
import { toast } from 'src/components/snackbar';
import { Iconify } from 'src/components/iconify';
import { CopyButton } from 'src/components/copy';
import { SatsWithIcon } from 'src/components/bitcoin';
import { useSettingsContext } from 'src/components/settings';

// ----------------------------------------------------------------------

type SendMoneyDrawerProps = {
  open: boolean;
  balance?: number;
  contacts: Contact[];
  fiatPrices: IFiatPrices;
  onClose: VoidFunction;
  onSuccess?: VoidFunction;
};

type ReceiveMoneyDrawerProps = {
  open: boolean;
  fiatPrices: IFiatPrices;
  lnAddress?: LnAddress | null;
  onClose: VoidFunction;
  onSuccess?: VoidFunction;
};

type RecipientKind = 'bolt11' | 'lightning-address' | 'lnurl' | 'bitcoin' | 'unknown';
type ReceiveRail = 'any' | 'lightning' | 'onchain';

const drawerSx = {
  width: { xs: 1, sm: 520 },
  maxWidth: 1,
};

const addressTypeOptions = [
  {
    value: BtcAddressType.P2TR,
    labelKey: 'bitcoin_address_type.taproot',
    helperKey: 'bitcoin_address_type.taproot_helper',
  },
  {
    value: BtcAddressType.P2WPKH,
    labelKey: 'bitcoin_address_type.native_segwit',
    helperKey: 'bitcoin_address_type.native_segwit_helper',
  },
] as const;

function satsToBtc(sats: number) {
  return (sats / 100_000_000).toFixed(8).replace(/0+$/, '').replace(/\.$/, '');
}

function extractBitcoinInput(input: string) {
  const value = input.trim();
  if (value.toLowerCase().startsWith('bitcoin:')) {
    return value.slice('bitcoin:'.length).split('?')[0];
  }

  return value;
}

function isBech32BitcoinAddress(value: string) {
  const normalized = value.toLowerCase();
  const decoder = normalized.startsWith('bc1p') || normalized.startsWith('tb1p') ? bech32m : bech32;

  try {
    const decoded = decoder.decode(normalized, 1023);
    return ['bc', 'tb', 'bcrt'].includes(decoded.prefix);
  } catch {
    return false;
  }
}

function detectRecipientKind(input: string): RecipientKind {
  const value = input.trim();
  const normalized = value.toLowerCase();

  if (!value) return 'unknown';

  try {
    decode(value);
    return 'bolt11';
  } catch {
    // Fall through to non-Bolt11 classifiers.
  }

  if (normalized.startsWith('lnurl')) return 'lnurl';
  if (isBech32BitcoinAddress(extractBitcoinInput(value))) return 'bitcoin';
  if (/^[13mn2][a-km-zA-HJ-NP-Z1-9]{25,90}$/.test(value)) return 'bitcoin';
  if (/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) return 'lightning-address';

  return 'unknown';
}

function decodedBolt11Amount(input: string) {
  try {
    const decoded = decode(input);
    const amountSection = decoded.sections.find((section: any) => section.name === 'amount');

    return amountSection ? Number(amountSection.value) / 1000 : 0;
  } catch {
    return 0;
  }
}

function composeBip21(address?: string, bolt11?: string, amountSats?: number) {
  if (!address) return bolt11 ?? '';

  const params = new URLSearchParams();
  if (amountSats) params.set('amount', satsToBtc(amountSats));
  if (bolt11) params.set('lightning', bolt11);

  const suffix = params.toString();
  return `bitcoin:${address}${suffix ? `?${suffix}` : ''}`;
}

function railLabel(kind: RecipientKind) {
  if (kind === 'bitcoin') return 'On-chain';
  if (kind === 'bolt11') return 'Lightning invoice';
  if (kind === 'lnurl') return 'LNURL';
  if (kind === 'lightning-address') return 'Lightning address';
  return 'Paste a payment request';
}

function drawerTitle(title: string, onClose: VoidFunction) {
  return (
    <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between', px: 3, py: 2 }}>
      <Typography variant="h6">{title}</Typography>
      <IconButton onClick={onClose}>
        <Iconify icon="mingcute:close-line" />
      </IconButton>
    </Stack>
  );
}

// ----------------------------------------------------------------------

export function SendMoneyDrawer({
  open,
  balance,
  contacts,
  fiatPrices,
  onClose,
  onSuccess,
}: SendMoneyDrawerProps) {
  const { t } = useTranslate();
  const { state } = useSettingsContext();
  const { copy } = useCopyToClipboard();

  const [input, setInput] = useState('');
  const [amount, setAmount] = useState(0);
  const [comment, setComment] = useState('');
  const [payment, setPayment] = useState<Payment>();
  const [isSubmitting, setIsSubmitting] = useState(false);

  const kind = useMemo(() => detectRecipientKind(input), [input]);
  const bolt11Amount = useMemo(() => decodedBolt11Amount(input), [input]);
  const amountSats = bolt11Amount || amount;

  const requestBody = useMemo(
    () => ({
      input,
      comment: comment || null,
      amount_msat: amountSats ? amountSats * 1000 : null,
    }),
    [amountSats, comment, input]
  );

  const curlSnippet = `curl -X POST "$SWISSKNIFE_URL/v1/me/payments" \\
  -H "Authorization: Bearer $SWISSKNIFE_TOKEN" \\
  -H "Content-Type: application/json" \\
  -d '${JSON.stringify(requestBody)}'`;

  const canSubmit = input.trim().length > 4 && (bolt11Amount > 0 || amount > 0);

  const handleClose = useCallback(() => {
    setInput('');
    setAmount(0);
    setComment('');
    setPayment(undefined);
    onClose();
  }, [onClose]);

  const handlePay = async () => {
    try {
      setIsSubmitting(true);
      const { data } = await walletPay({ body: requestBody });
      setPayment(data);
      onSuccess?.();
    } catch (error) {
      handleActionError(error);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCopySnippet = () => {
    copy(curlSnippet);
    toast.success(t('copied_to_clipboard'));
  };

  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={handleClose}
      slotProps={{ paper: { sx: drawerSx } }}
    >
      {drawerTitle(t('send_money.title'), handleClose)}
      <Divider />

      <Stack spacing={3} sx={{ p: 3 }}>
        {payment ? (
          <Stack spacing={3} sx={{ textAlign: 'center', py: 4 }}>
            <Box
              sx={{
                width: 72,
                height: 72,
                display: 'grid',
                borderRadius: 1,
                placeItems: 'center',
                mx: 'auto',
                color: 'success.main',
                bgcolor: 'success.lighter',
              }}
            >
              <Iconify icon="solar:check-circle-bold" width={42} />
            </Box>
            <Stack spacing={1}>
              <Typography variant="h5">{t('send_money.sent')}</Typography>
              <SatsWithIcon amountMSats={payment.amount_msat} variant="h4" />
              <Typography variant="body2" color="text.secondary">
                {truncateText(input, 42)}
              </Typography>
            </Stack>
            <Button variant="contained" color="inherit" onClick={handleClose}>
              {t('done')}
            </Button>
          </Stack>
        ) : (
          <>
            <Stack spacing={1}>
              <Typography variant="overline" color="text.secondary">
                {t('send_money.to')}
              </Typography>
              <TextField
                multiline
                minRows={3}
                value={input}
                onChange={(event) => setInput(event.target.value)}
                placeholder={t('send_money.recipient_placeholder')}
              />
            </Stack>

            {contacts.length > 0 && (
              <Stack direction="row" spacing={1} useFlexGap sx={{ flexWrap: 'wrap' }}>
                {contacts.slice(0, 6).map((contact) => (
                  <Chip
                    key={contact.ln_address}
                    label={contact.ln_address.split('@')[0]}
                    onClick={() => setInput(contact.ln_address)}
                    icon={<Iconify icon="solar:user-rounded-bold" />}
                    variant="outlined"
                  />
                ))}
              </Stack>
            )}

            <Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>
              <TextField
                fullWidth
                type="number"
                label={t('send_money.amount_sats')}
                value={bolt11Amount || amount || ''}
                disabled={bolt11Amount > 0}
                onChange={(event) => setAmount(Number(event.target.value))}
              />
              <Stack sx={{ minWidth: 120 }}>
                <Typography variant="caption" color="text.secondary">
                  {state.currency}
                </Typography>
                <Typography variant="subtitle2">
                  {fCurrency(satsToFiat(amountSats, fiatPrices, state.currency), {
                    currency: state.currency,
                  })}
                </Typography>
              </Stack>
            </Stack>

            <TextField
              label={t('send_money.note')}
              value={comment}
              onChange={(event) => setComment(event.target.value)}
            />

            <Box
              sx={[
                (theme) => ({
                  p: 2,
                  borderRadius: 1,
                  bgcolor: 'background.neutral',
                  border: `1px solid ${theme.vars.palette.divider}`,
                }),
              ]}
            >
              <Stack spacing={1.5}>
                <Stack direction="row" sx={{ alignItems: 'center', justifyContent: 'space-between' }}>
                  <Typography variant="subtitle2">{t('send_money.review')}</Typography>
                  <Label color={kind === 'bitcoin' ? 'warning' : 'info'}>{railLabel(kind)}</Label>
                </Stack>

                {kind === 'bitcoin' ? (
                  <Alert severity="warning" variant="outlined">
                    {t('send_money.onchain_fee_note')}
                  </Alert>
                ) : (
                  <Alert severity="info" variant="outlined">
                    {t('send_money.lightning_fee_note')}
                  </Alert>
                )}

                {balance != null && (
                  <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                    <Typography variant="body2" color="text.secondary">
                      {t('send_money.available')}
                    </Typography>
                    <SatsWithIcon amountMSats={balance} />
                  </Stack>
                )}
              </Stack>
            </Box>

            <Box
              component="pre"
              sx={{
                m: 0,
                p: 2,
                overflow: 'auto',
                borderRadius: 1,
                typography: 'caption',
                bgcolor: 'grey.900',
                color: 'grey.100',
              }}
            >
              {curlSnippet}
            </Box>

            <Stack direction="row" spacing={1.5}>
              <Button color="inherit" variant="outlined" onClick={handleCopySnippet}>
                {t('send_money.copy_curl')}
              </Button>
              <Button
                fullWidth
                color="inherit"
                variant="contained"
                loading={isSubmitting}
                disabled={!canSubmit}
                onClick={handlePay}
              >
                {kind === 'bitcoin' ? t('send_money.send_onchain') : t('send_money.send')}
              </Button>
            </Stack>
          </>
        )}
      </Stack>
    </Drawer>
  );
}

// ----------------------------------------------------------------------

export function ReceiveMoneyDrawer({
  open,
  lnAddress,
  fiatPrices,
  onClose,
  onSuccess,
}: ReceiveMoneyDrawerProps) {
  const { t } = useTranslate();
  const { state } = useSettingsContext();

  const [rail, setRail] = useState<ReceiveRail>('any');
  const [amount, setAmount] = useState(0);
  const [description, setDescription] = useState('');
  const [invoice, setInvoice] = useState<Invoice>();
  const [btcAddress, setBtcAddress] = useState<BtcAddress>();
  const [addressType, setAddressType] = useState<BtcAddressType>(BtcAddressType.P2TR);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [addressError, setAddressError] = useState<string>();

  const bolt11 = invoice?.ln_invoice?.bolt11;
  const bip21 = composeBip21(btcAddress?.address, bolt11, amount);
  const qrValue = rail === 'lightning' ? (bolt11 ?? '') : rail === 'onchain' ? (btcAddress?.address ?? '') : bip21;

  const handleClose = useCallback(() => {
    setRail('any');
    setAmount(0);
    setDescription('');
    setInvoice(undefined);
    setBtcAddress(undefined);
    setAddressError(undefined);
    onClose();
  }, [onClose]);

  const handleGenerate = async () => {
    try {
      setIsSubmitting(true);
      setAddressError(undefined);
      setInvoice(undefined);
      setBtcAddress(undefined);

      const shouldGenerateInvoice = rail !== 'onchain';
      const shouldGenerateAddress = rail !== 'lightning';

      if (shouldGenerateInvoice) {
        const invoiceResult = await newWalletInvoice({
          body: {
            description,
            amount_msat: amount * 1000,
          },
        });

        setInvoice(invoiceResult.data);
      }

      if (shouldGenerateAddress) {
        const addressResult = await newWalletBtcAddress({
          body: { type: addressType },
        }).catch((error) => {
          setAddressError(error?.message || t('receive_money.address_unavailable'));
          return null;
        });

        if (addressResult?.data) {
          setBtcAddress(addressResult.data);
        }
      }

      onSuccess?.();
    } catch (error) {
      handleActionError(error);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={handleClose}
      slotProps={{ paper: { sx: drawerSx } }}
    >
      {drawerTitle(t('receive_money.title'), handleClose)}
      <Divider />

      <Stack spacing={3} sx={{ p: 3 }}>
        <ToggleButtonGroup
          exclusive
          fullWidth
          size="small"
          value={rail}
          onChange={(_, value) => value && setRail(value)}
        >
          <ToggleButton value="any">{t('receive_money.any')}</ToggleButton>
          <ToggleButton value="lightning">Lightning</ToggleButton>
          <ToggleButton value="onchain">On-chain</ToggleButton>
        </ToggleButtonGroup>

        <Stack direction="row" spacing={2} sx={{ alignItems: 'center' }}>
          <TextField
            fullWidth
            type="number"
            label={t('receive_money.amount_sats')}
            value={amount || ''}
            onChange={(event) => setAmount(Number(event.target.value))}
          />
          <Stack sx={{ minWidth: 120 }}>
            <Typography variant="caption" color="text.secondary">
              {state.currency}
            </Typography>
            <Typography variant="subtitle2">
              {fCurrency(satsToFiat(amount, fiatPrices, state.currency), {
                currency: state.currency,
              })}
            </Typography>
          </Stack>
        </Stack>

        <TextField
          label={t('receive_money.memo')}
          value={description}
          onChange={(event) => setDescription(event.target.value)}
        />

        {rail !== 'lightning' && (
          <TextField
            select
            size="small"
            label={t('receive_money.address_type')}
            value={addressType}
            onChange={(event) => setAddressType(event.target.value as BtcAddressType)}
            helperText={t(
              addressTypeOptions.find((option) => option.value === addressType)?.helperKey ??
                'bitcoin_address_type.taproot_helper'
            )}
          >
            {addressTypeOptions.map((option) => (
              <MenuItem key={option.value} value={option.value}>
                {t(option.labelKey)}
              </MenuItem>
            ))}
          </TextField>
        )}

        <Button
          color="inherit"
          variant="contained"
          loading={isSubmitting}
          onClick={handleGenerate}
        >
          {invoice ? t('receive_money.regenerate') : t('receive_money.generate')}
        </Button>

        {addressError && <Alert severity="warning">{addressError}</Alert>}

        {qrValue ? (
          <Stack spacing={2}>
            <Box
              sx={[
                (theme) => ({
                  p: 2,
                  borderRadius: 1,
                  bgcolor: 'common.white',
                  border: `1px solid ${theme.vars.palette.divider}`,
                  '& canvas': { width: '100% !important', height: 'auto !important' },
                }),
              ]}
            >
              <QRCode
                value={qrValue}
                size={420}
                eyeRadius={5}
                logoPadding={3}
                removeQrCodeBehindLogo
                logoPaddingStyle="circle"
                logoImage="/logo/logo_square_negative.svg"
              />
            </Box>

            <Stack spacing={1}>
              {rail === 'any' && (
                <Label color={btcAddress && bolt11 ? 'success' : 'warning'}>
                  {btcAddress && bolt11 ? t('receive_money.bip21_ready') : t('receive_money.partial_qr')}
                </Label>
              )}

              {bolt11 && (
                <Stack direction="row" sx={{ alignItems: 'center', gap: 1 }}>
                  <Typography variant="body2" sx={{ flex: 1 }} noWrap>
                    {bolt11}
                  </Typography>
                  <CopyButton value={bolt11} title={t('copy')} />
                </Stack>
              )}

              {btcAddress && (
                <Stack direction="row" sx={{ alignItems: 'center', gap: 1 }}>
                  <Typography variant="body2" sx={{ flex: 1 }} noWrap>
                    {btcAddress.address}
                  </Typography>
                  <Label color={btcAddress.used ? 'warning' : 'success'}>
                    {btcAddress.used ? t('receive_money.used') : t('receive_money.fresh')}
                  </Label>
                  <CopyButton value={btcAddress.address} title={t('copy')} />
                </Stack>
              )}
            </Stack>

            {btcAddress?.used && (
              <Alert severity="warning" variant="outlined">
                {t('receive_money.reuse_warning')}
              </Alert>
            )}
          </Stack>
        ) : (
          <Box
            sx={[
              (theme) => ({
                p: 3,
                borderRadius: 1,
                textAlign: 'center',
                bgcolor: 'background.neutral',
                border: `1px dashed ${theme.vars.palette.divider}`,
              }),
            ]}
          >
            <Iconify icon="solar:qr-code-bold-duotone" width={52} sx={{ color: 'text.disabled' }} />
            <Typography variant="subtitle2" sx={{ mt: 1 }}>
              {t('receive_money.empty_qr')}
            </Typography>
          </Box>
        )}

        {lnAddress && (
          <Alert severity="info" variant="outlined">
            {t('receive_money.identity')} {displayLnAddress(lnAddress.username)}
          </Alert>
        )}
      </Stack>
    </Drawer>
  );
}
