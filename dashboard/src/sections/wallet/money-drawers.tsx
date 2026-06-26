'use client';

import type { Key, HTMLAttributes } from 'react';
import type { DialogProps } from '@mui/material/Dialog';
import type { IDetectedBarcode } from '@yudiel/react-qr-scanner';
import type { IFiatPrices } from 'src/types/bitcoin';

import { bech32, bech32m } from 'bech32';
import { QRCode } from 'react-qrcode-logo';
import { decode } from 'light-bolt11-decoder';
import { Scanner } from '@yudiel/react-qr-scanner';
import { useMemo, useState, useEffect, useCallback } from 'react';
import { useBoolean, useCopyToClipboard } from 'minimal-shared/hooks';

import Box from '@mui/material/Box';
import Chip from '@mui/material/Chip';
import Alert from '@mui/material/Alert';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Drawer from '@mui/material/Drawer';
import Dialog from '@mui/material/Dialog';
import Divider from '@mui/material/Divider';
import Accordion from '@mui/material/Accordion';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import Autocomplete from '@mui/material/Autocomplete';
import ToggleButton from '@mui/material/ToggleButton';
import DialogActions from '@mui/material/DialogActions';
import InputAdornment from '@mui/material/InputAdornment';
import AccordionDetails from '@mui/material/AccordionDetails';
import AccordionSummary from '@mui/material/AccordionSummary';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';

import { satsToFiat } from 'src/utils/fiat';
import { fCurrency } from 'src/utils/format-number';
import { handleActionError } from 'src/utils/errors';
import { truncateText } from 'src/utils/format-string';
import { fToNow, fDateTime } from 'src/utils/format-time';
import { encodeLNURL, displayLnAddress } from 'src/utils/lnurl';
import { composeBip21, parseBitcoinUri, compactBitcoinAddress } from 'src/utils/bitcoin-request';

import { CONFIG } from 'src/global-config';
import { useTranslate } from 'src/locales';
import { useListWallets } from 'src/actions/wallet';
import { useGetUserWallet } from 'src/actions/user-wallet';
import { useListBtcAddresses } from 'src/actions/btc-addresses';
import {
  pay,
  walletPay,
  type Wallet,
  type Contact,
  type Invoice,
  type Payment,
  InvoiceStatus,
  BtcAddressType,
  type LnAddress,
  generateInvoice,
  type BtcAddress,
  newWalletInvoice,
  generateBtcAddress,
  newWalletBtcAddress,
} from 'src/lib/swissknife';

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
  isAdmin?: boolean;
  walletId?: string;
  initialInput?: string;
};

type ReceiveMoneyDrawerProps = {
  open: boolean;
  fiatPrices: IFiatPrices;
  lnAddress?: LnAddress | null;
  onClose: VoidFunction;
  onSuccess?: VoidFunction;
  isAdmin?: boolean;
  walletId?: string;
};

type RecipientKind =
  | 'bip21'
  | 'bolt11'
  | 'lightning-address'
  | 'lnurl'
  | 'bitcoin'
  | 'internal'
  | 'unknown';
type ReceivePayload = 'unified' | 'lightning' | 'onchain' | 'identity';
type AmountUnit = 'sats' | 'btc' | 'fiat';

type DecodedBolt11 = {
  description?: string;
  sections?: Array<{ name: string; value?: string | number }>;
};

type Bolt11Details = {
  amountSats: number;
  description?: string;
  paymentHash?: string;
  expirySeconds?: number;
  expiresAt?: Date;
};

const drawerSx = {
  width: { xs: 1, sm: 520 },
  maxWidth: 1,
};

const SATS_PER_BITCOIN = 100_000_000;
const DEFAULT_BOLT11_EXPIRY_SECONDS = 3600;

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

function stripLightningScheme(input: string) {
  return input.trim().replace(/^lightning:/i, '');
}

function extractBitcoinInput(input: string) {
  return parseBitcoinUri(input).address;
}

function isLnurlPayload(input: string) {
  const value = stripLightningScheme(input);

  if (!value.toLowerCase().startsWith('lnurl1')) return false;

  try {
    return bech32.decode(value, 2000).prefix.toLowerCase() === 'lnurl';
  } catch {
    return false;
  }
}

function isBitcoinAddress(value: string) {
  return isBech32BitcoinAddress(value) || /^[13mn2][a-km-zA-HJ-NP-Z1-9]{25,90}$/.test(value.trim());
}

function looksLikeBitcoinAddress(value: string) {
  const normalized = value.trim();

  return (
    /^(bc|tb|bcrt)1[02-9ac-hj-np-z]{8,}$/i.test(normalized) ||
    /^[13mn2][a-km-zA-HJ-NP-Z1-9]{25,90}$/.test(normalized)
  );
}

function isBech32BitcoinAddress(value: string) {
  const normalized = value.toLowerCase();
  const decoder =
    normalized.startsWith('bc1p') ||
    normalized.startsWith('tb1p') ||
    normalized.startsWith('bcrt1p')
      ? bech32m
      : bech32;

  try {
    const decoded = decoder.decode(normalized, 1023);
    return ['bc', 'tb', 'bcrt'].includes(decoded.prefix);
  } catch {
    return false;
  }
}

function detectRecipientKind(input: string): RecipientKind {
  const value = input.trim();
  const bitcoinUri = parseBitcoinUri(value);
  const paymentRequest = bitcoinUri.lightning ?? value;

  if (!value) return 'unknown';
  if (
    bitcoinUri.isUri &&
    (isBitcoinAddress(bitcoinUri.address) || looksLikeBitcoinAddress(bitcoinUri.address))
  ) {
    return 'bip21';
  }

  try {
    decode(stripLightningScheme(paymentRequest));
    return 'bolt11';
  } catch {
    // Fall through to non-Bolt11 classifiers.
  }

  if (isBitcoinAddress(bitcoinUri.address) || looksLikeBitcoinAddress(bitcoinUri.address)) {
    return 'bitcoin';
  }
  if (isLnurlPayload(value)) return 'lnurl';
  if (isBitcoinAddress(extractBitcoinInput(value))) return 'bitcoin';
  if (/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) {
    return value.split('@')[1].toLowerCase() === CONFIG.domain.toLowerCase()
      ? 'internal'
      : 'lightning-address';
  }

  return 'unknown';
}

function decodedBolt11Details(input: string): Bolt11Details {
  const bitcoinUri = parseBitcoinUri(input);
  const paymentRequest = bitcoinUri.lightning ?? input;

  try {
    const decoded = decode(stripLightningScheme(paymentRequest)) as DecodedBolt11;
    const amountSection = decoded.sections?.find((section) => section.name === 'amount');
    const descriptionSection = decoded.sections?.find((section) => section.name === 'description');
    const paymentHashSection = decoded.sections?.find((section) => section.name === 'payment_hash');
    const expirySection = decoded.sections?.find((section) => section.name === 'expiry');
    const timestampSection = decoded.sections?.find((section) => section.name === 'timestamp');
    const timestampSeconds = Number(timestampSection?.value);
    const expirySeconds = Number(expirySection?.value ?? DEFAULT_BOLT11_EXPIRY_SECONDS);
    const expiresAt =
      Number.isFinite(timestampSeconds) && Number.isFinite(expirySeconds)
        ? new Date((timestampSeconds + expirySeconds) * 1000)
        : undefined;

    return {
      amountSats: amountSection?.value ? Number(amountSection.value) / 1000 : 0,
      description: descriptionSection?.value ? String(descriptionSection.value) : undefined,
      paymentHash: paymentHashSection?.value ? String(paymentHashSection.value) : undefined,
      expirySeconds,
      expiresAt,
    };
  } catch {
    return { amountSats: 0 };
  }
}

function amountValueToSats(
  value: string,
  unit: AmountUnit,
  fiatPrices: IFiatPrices,
  currency: string
) {
  const numericValue = Number(value.replaceAll(',', ''));
  if (!Number.isFinite(numericValue) || numericValue <= 0) return 0;

  if (unit === 'btc') return Math.round(numericValue * SATS_PER_BITCOIN);
  if (unit === 'fiat') {
    const price = fiatPrices[currency] ?? 0;
    return price > 0 ? Math.round((numericValue / price) * SATS_PER_BITCOIN) : 0;
  }

  return Math.round(numericValue);
}

function trimFixed(value: number, maximumFractionDigits: number) {
  return value.toFixed(maximumFractionDigits).replace(/0+$/, '').replace(/\.$/, '');
}

function satsToAmountValue(
  sats: number,
  unit: AmountUnit,
  fiatPrices: IFiatPrices,
  currency: string
) {
  if (!Number.isFinite(sats) || sats <= 0) return '';

  if (unit === 'btc') return trimFixed(sats / SATS_PER_BITCOIN, 8);

  if (unit === 'fiat') {
    const price = fiatPrices[currency] ?? 0;
    if (price <= 0) return '';

    return trimFixed((sats / SATS_PER_BITCOIN) * price, 8);
  }

  return String(Math.round(sats));
}

function fiatSymbol(currency: string) {
  try {
    const symbol = new Intl.NumberFormat(undefined, {
      style: 'currency',
      currency,
      currencyDisplay: 'narrowSymbol',
    })
      .formatToParts(0)
      .find((part) => part.type === 'currency')?.value;

    return symbol || currency;
  } catch {
    return currency;
  }
}

function amountUnitPrefix(unit: AmountUnit, currency: string, displayUnit: 'bip177' | 'sats') {
  if (unit === 'btc') return 'BTC';
  if (unit === 'fiat') return fiatSymbol(currency);
  return displayUnit === 'bip177' ? '₿' : 'sats';
}

function railLabel(kind: RecipientKind, t: (key: string) => string) {
  if (kind === 'bip21') return t('send_money.request_type_bip21');
  if (kind === 'bitcoin') return t('send_money.request_type_onchain');
  if (kind === 'internal') return t('send_money.request_type_internal');
  if (kind === 'bolt11') return t('send_money.request_type_bolt11');
  if (kind === 'lnurl') return t('send_money.request_type_lnurl');
  if (kind === 'lightning-address') return t('send_money.request_type_lightning_address');
  return t('send_money.request_type_unknown');
}

function railColor(kind: RecipientKind) {
  if (kind === 'bitcoin') return 'warning';
  if (kind === 'internal') return 'success';
  if (kind === 'unknown') return 'default';
  return 'info';
}

function drawerTitle(title: string, onClose: VoidFunction) {
  return (
    <Stack
      direction="row"
      sx={{ alignItems: 'center', justifyContent: 'space-between', px: 3, py: 2 }}
    >
      <Typography variant="h6">{title}</Typography>
      <IconButton onClick={onClose}>
        <Iconify icon="mingcute:close-line" />
      </IconButton>
    </Stack>
  );
}

function AmountEntryField({
  label,
  value,
  fiatLabel,
  amountUnit,
  currency,
  displayUnit,
  hasFiatPrice,
  error = false,
  onChange,
  onSwap,
}: {
  label: string;
  value: string;
  fiatLabel: string;
  amountUnit: AmountUnit;
  currency: string;
  displayUnit: 'bip177' | 'sats';
  hasFiatPrice: boolean;
  error?: boolean;
  onChange: (value: string) => void;
  onSwap: VoidFunction;
}) {
  return (
    <Box
      sx={[
        (theme) => ({
          p: 2,
          borderRadius: 1,
          bgcolor: 'background.neutral',
          border: `1px solid ${error ? theme.vars.palette.error.main : theme.vars.palette.divider}`,
          transition: theme.transitions.create(['border-color', 'box-shadow']),
          '&:focus-within': {
            borderColor: error ? theme.vars.palette.error.main : theme.vars.palette.primary.main,
            boxShadow: `0 0 0 1px ${
              error ? theme.vars.palette.error.main : theme.vars.palette.primary.main
            }`,
          },
        }),
      ]}
    >
      <Stack spacing={1}>
        <Typography variant="caption" color="text.secondary">
          {label}
        </Typography>

        <Stack direction="row" spacing={1.25} sx={{ alignItems: 'center' }}>
          <Typography
            variant="h4"
            color={error ? 'error.main' : 'text.secondary'}
            sx={{ minWidth: amountUnit === 'fiat' ? 28 : 'auto' }}
          >
            {amountUnitPrefix(amountUnit, currency, displayUnit)}
          </Typography>

          <TextField
            fullWidth
            type="number"
            variant="standard"
            value={value}
            placeholder="0"
            error={error}
            onChange={(event) => onChange(event.target.value)}
            sx={{
              '& .MuiInputBase-input': {
                p: 0,
                typography: 'h4',
                fontWeight: 400,
              },
              '& .MuiInput-root:before, & .MuiInput-root:after': {
                display: 'none',
              },
            }}
          />

          <IconButton size="small" disabled={!hasFiatPrice} onClick={onSwap}>
            <Iconify icon="solar:transfer-horizontal-bold" />
          </IconButton>
        </Stack>

        <Typography variant="subtitle1" color={error ? 'error.main' : 'text.secondary'}>
          {fiatLabel}
        </Typography>
      </Stack>
    </Box>
  );
}

function WalletPicker({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  const { t } = useTranslate();
  const { wallets, walletsError, walletsLoading } = useListWallets();

  const selectedWallet = wallets?.find((wallet) => wallet.id === value) ?? null;

  if (walletsError) {
    return (
      <Alert severity="warning" variant="outlined">
        {t('admin_wallet_picker.unavailable')}
      </Alert>
    );
  }

  return (
    <Autocomplete
      options={wallets ?? []}
      loading={walletsLoading}
      value={selectedWallet}
      onChange={(_, wallet: Wallet | null) => onChange(wallet?.id ?? '')}
      getOptionLabel={(wallet) => wallet.user_id || wallet.id}
      isOptionEqualToValue={(option, wallet) => option.id === wallet.id}
      renderOption={(props: HTMLAttributes<HTMLLIElement> & { key: Key }, wallet) => {
        const { key, ...optionProps } = props;

        return (
          <li key={key} {...optionProps}>
            <Stack sx={{ minWidth: 0 }}>
              <Typography variant="subtitle2" noWrap>
                {wallet.user_id}
              </Typography>
              <Typography variant="caption" color="text.secondary" noWrap>
                {wallet.id}
              </Typography>
            </Stack>
          </li>
        );
      }}
      renderInput={(params) => (
        <TextField
          {...params}
          label={t('admin_wallet_picker.label')}
          helperText={t('admin_wallet_picker.helper')}
        />
      )}
    />
  );
}

function SendDetailRow({ label, value }: { label: string; value?: string | number | null }) {
  if (value == null || value === '') return null;

  return (
    <Stack direction="row" spacing={1.5} sx={{ justifyContent: 'space-between', minWidth: 0 }}>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      <Typography variant="caption" sx={{ maxWidth: '68%', textAlign: 'right' }} noWrap>
        {value}
      </Typography>
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
  isAdmin = false,
  walletId,
  initialInput,
}: SendMoneyDrawerProps) {
  const { t } = useTranslate();
  const { state } = useSettingsContext();
  const scanQR = useBoolean();

  const [input, setInput] = useState(initialInput ?? '');
  const [amountValue, setAmountValue] = useState('');
  const [amountUnit, setAmountUnit] = useState<AmountUnit>('sats');
  const [comment, setComment] = useState('');
  const [payment, setPayment] = useState<Payment>();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [nowMs, setNowMs] = useState(0);
  const [selectedWalletId, setSelectedWalletId] = useState(walletId ?? '');

  const kind = useMemo(() => detectRecipientKind(input), [input]);
  const bolt11Details = useMemo(() => decodedBolt11Details(input), [input]);
  const bolt11Amount = bolt11Details.amountSats;
  const bitcoinRequest = useMemo(() => parseBitcoinUri(input), [input]);
  const hasLightningInvoice = kind === 'bolt11' || Boolean(bitcoinRequest.lightning);
  const bolt11ExpiresAt = bolt11Details.expiresAt ?? null;
  const bolt11ExpiryMs = bolt11ExpiresAt ? bolt11ExpiresAt.getTime() : null;
  const lightningInvoiceExpired =
    hasLightningInvoice &&
    typeof bolt11ExpiryMs === 'number' &&
    Number.isFinite(bolt11ExpiryMs) &&
    bolt11ExpiryMs <= nowMs;
  const hasUnsupportedBip21Params = kind === 'bip21' && bitcoinRequest.requiredParams.length > 0;
  const hasInvalidBip21Amount = kind === 'bip21' && bitcoinRequest.amountInvalid;
  const bitcoinAddressForValidation = bitcoinRequest.address || extractBitcoinInput(input);
  const hasInvalidBitcoinAddress =
    (kind === 'bitcoin' || kind === 'bip21') &&
    looksLikeBitcoinAddress(bitcoinAddressForValidation) &&
    !isBitcoinAddress(bitcoinAddressForValidation);
  const expiredInvoiceBlocksSubmit =
    lightningInvoiceExpired &&
    (kind === 'bolt11' || (kind === 'bip21' && !bitcoinRequest.amountSats));
  const manualAmountSats = useMemo(
    () => amountValueToSats(amountValue, amountUnit, fiatPrices, state.currency),
    [amountUnit, amountValue, fiatPrices, state.currency]
  );
  const embeddedAmountSats = bolt11Amount || bitcoinRequest.amountSats || 0;
  const amountSats = embeddedAmountSats || manualAmountSats;
  const amountMSats = amountSats * 1000;
  const amountExceedsAvailable =
    balance != null && amountSats > 0 && amountMSats > Math.max(balance, 0);
  const canEnterAmount =
    input.trim().length > 0 &&
    embeddedAmountSats === 0 &&
    kind !== 'unknown' &&
    !hasInvalidBip21Amount &&
    !hasUnsupportedBip21Params;
  const activeWalletId = walletId || selectedWalletId;
  const needsWallet = isAdmin && !activeWalletId;
  const hasFiatPrice = (fiatPrices[state.currency] ?? 0) > 0;
  const canSendComment = kind === 'lnurl' || kind === 'lightning-address' || kind === 'internal';
  const canShowParsedInput = input.trim().length > 0;
  const canSubmit =
    kind !== 'unknown' &&
    input.trim().length > 4 &&
    amountSats > 0 &&
    (amountUnit !== 'fiat' || hasFiatPrice || embeddedAmountSats > 0) &&
    !expiredInvoiceBlocksSubmit &&
    !hasUnsupportedBip21Params &&
    !hasInvalidBip21Amount &&
    !hasInvalidBitcoinAddress &&
    !amountExceedsAvailable &&
    !needsWallet;
  const canEstimateFee =
    kind !== 'unknown' &&
    kind !== 'internal' &&
    amountSats > 0 &&
    !amountExceedsAvailable &&
    !hasUnsupportedBip21Params &&
    !hasInvalidBip21Amount &&
    !hasInvalidBitcoinAddress;
  const submitLabel =
    (input.trim() && kind === 'unknown' && t('send_money.unsupported_request')) ||
    ((kind === 'bitcoin' || (kind === 'bip21' && !bitcoinRequest.lightning)) &&
      t('send_money.send_onchain')) ||
    t('send_money.send');
  const amountFiatLabel = amountExceedsAvailable
    ? t('send_money.amount_available_helper')
    : hasFiatPrice
      ? fCurrency(satsToFiat(amountSats, fiatPrices, state.currency), {
          currency: state.currency,
        })
      : t('send_money.no_fiat_value');
  const bolt11ExpiryLabel = bolt11ExpiresAt
    ? t(
        lightningInvoiceExpired ? 'send_money.invoice_expired_at' : 'send_money.invoice_expires_at',
        {
          time: fToNow(bolt11ExpiresAt),
          date: fDateTime(bolt11ExpiresAt),
        }
      )
    : bolt11Details.expirySeconds
      ? t('send_money.expiry_seconds', {
          count: bolt11Details.expirySeconds,
        })
      : undefined;

  useEffect(() => {
    setSelectedWalletId(walletId ?? '');
  }, [walletId]);

  useEffect(() => {
    if (open && initialInput) {
      setInput(initialInput);
    }
  }, [initialInput, open]);

  useEffect(() => {
    if (!bolt11ExpiryMs) return undefined;

    setNowMs(Date.now());
    const interval = window.setInterval(() => setNowMs(Date.now()), 30_000);

    return () => window.clearInterval(interval);
  }, [bolt11ExpiryMs]);

  const requestBody = useMemo(
    () => ({
      input,
      comment: canSendComment && comment ? comment : null,
      amount_msat: amountSats && !amountExceedsAvailable ? amountMSats : null,
      ...(isAdmin && { wallet_id: activeWalletId || null }),
    }),
    [
      activeWalletId,
      amountExceedsAvailable,
      amountMSats,
      amountSats,
      canSendComment,
      comment,
      input,
      isAdmin,
    ]
  );

  const handleClose = useCallback(() => {
    setInput('');
    setAmountValue('');
    setAmountUnit('sats');
    setComment('');
    setPayment(undefined);
    setSelectedWalletId(walletId ?? '');
    onClose();
  }, [onClose, walletId]);

  const handlePay = async () => {
    try {
      setIsSubmitting(true);
      const { data } = isAdmin
        ? await pay({ body: requestBody })
        : await walletPay({ body: requestBody });
      setPayment(data);
      onSuccess?.();
    } catch (error) {
      handleActionError(error);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleAmountUnitSwap = () => {
    const nextUnit: AmountUnit = amountUnit === 'fiat' ? 'sats' : 'fiat';
    const currentAmountSats = amountValueToSats(
      amountValue,
      amountUnit,
      fiatPrices,
      state.currency
    );

    setAmountUnit(nextUnit);
    setAmountValue(satsToAmountValue(currentAmountSats, nextUnit, fiatPrices, state.currency));
  };

  const handleEstimateFee = () => {
    toast.info(t('send_money.fee_estimate_unavailable'));
  };

  const handleClearInput = () => {
    setInput('');
    setAmountValue('');
    setComment('');
  };

  return (
    <>
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
              {isAdmin &&
                (walletId ? (
                  <TextField
                    label={t('admin_wallet_picker.label')}
                    value={walletId}
                    slotProps={{ input: { readOnly: true } }}
                    helperText={t('admin_wallet_picker.fixed')}
                  />
                ) : (
                  <WalletPicker value={selectedWalletId} onChange={setSelectedWalletId} />
                ))}

              {needsWallet && (
                <Alert severity="info" variant="outlined">
                  {t('admin_wallet_picker.required')}
                </Alert>
              )}

              <Stack spacing={1}>
                <Stack
                  direction="row"
                  spacing={1}
                  sx={{ alignItems: 'center', justifyContent: 'space-between' }}
                >
                  <Typography variant="overline" color="text.secondary">
                    {t('send_money.to')}
                  </Typography>
                  <Button
                    size="small"
                    color="inherit"
                    variant="outlined"
                    onClick={scanQR.onTrue}
                    startIcon={<Iconify icon="mdi:qrcode-scan" />}
                  >
                    {t('send_money.scan_qr')}
                  </Button>
                </Stack>
                <TextField
                  multiline
                  minRows={3}
                  value={input}
                  onChange={(event) => setInput(event.target.value)}
                  placeholder={t('send_money.recipient_placeholder')}
                  slotProps={{
                    input: {
                      endAdornment: input ? (
                        <InputAdornment position="end" sx={{ alignSelf: 'flex-start', mt: 0.5 }}>
                          <IconButton
                            size="small"
                            edge="end"
                            aria-label={t('send_money.clear_recipient')}
                            onClick={handleClearInput}
                          >
                            <Iconify icon="mingcute:close-line" width={18} />
                          </IconButton>
                        </InputAdornment>
                      ) : undefined,
                    },
                  }}
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

              {canShowParsedInput && (
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
                  <Stack spacing={1.25}>
                    <Stack
                      direction="row"
                      sx={{ alignItems: 'center', justifyContent: 'space-between', gap: 1 }}
                    >
                      <Stack direction="row" spacing={1} sx={{ alignItems: 'center', minWidth: 0 }}>
                        <Iconify icon="solar:radar-2-bold-duotone" />
                        <Typography variant="subtitle2">
                          {t('send_money.payment_summary')}
                        </Typography>
                      </Stack>
                      <Label color={railColor(kind)}>{railLabel(kind, t)}</Label>
                    </Stack>

                    {(kind === 'bip21' || kind === 'bitcoin') && bitcoinRequest.address && (
                      <SendDetailRow
                        label={t('send_money.destination')}
                        value={compactBitcoinAddress(bitcoinRequest.address)}
                      />
                    )}

                    {kind === 'bip21' && bitcoinRequest.lightning && (
                      <SendDetailRow
                        label={t('send_money.lightning_fallback')}
                        value={t('send_money.included')}
                      />
                    )}

                    {bitcoinRequest.label && (
                      <SendDetailRow
                        label={t('send_money.request_label')}
                        value={bitcoinRequest.label}
                      />
                    )}

                    {bitcoinRequest.message && (
                      <SendDetailRow
                        label={t('send_money.request_message')}
                        value={bitcoinRequest.message}
                      />
                    )}

                    {hasLightningInvoice && (
                      <>
                        <SendDetailRow
                          label={t('send_money.invoice_memo')}
                          value={bolt11Details.description}
                        />
                        <SendDetailRow label={t('send_money.expiry')} value={bolt11ExpiryLabel} />
                      </>
                    )}

                    {embeddedAmountSats > 0 && (
                      <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                        <Typography variant="caption" color="text.secondary">
                          {t('send_money.amount')}
                        </Typography>
                        <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                          <SatsWithIcon amountMSats={amountSats * 1000} />
                          <Typography variant="caption" color="text.secondary">
                            {amountFiatLabel}
                          </Typography>
                        </Stack>
                      </Stack>
                    )}

                    {kind !== 'unknown' && (
                      <Stack direction="row" sx={{ justifyContent: 'space-between' }}>
                        <Typography variant="caption" color="text.secondary">
                          {t('send_money.estimated_fee')}
                        </Typography>
                        {kind === 'internal' ? (
                          <Typography variant="caption">
                            {t('send_money.no_network_fee')}
                          </Typography>
                        ) : (
                          <Button
                            size="small"
                            color="inherit"
                            variant="outlined"
                            disabled={!canEstimateFee}
                            onClick={handleEstimateFee}
                            startIcon={<Iconify icon="solar:calculator-minimalistic-bold" />}
                            sx={{
                              minHeight: 28,
                              px: 1,
                              py: 0.25,
                              typography: 'caption',
                              whiteSpace: 'nowrap',
                            }}
                          >
                            {t('send_money.estimate_fee')}
                          </Button>
                        )}
                      </Stack>
                    )}

                    {hasInvalidBitcoinAddress && (
                      <Alert severity="warning" variant="outlined">
                        {t('send_money.invalid_bitcoin_address')}
                      </Alert>
                    )}

                    {bitcoinRequest.amountInvalid && (
                      <Alert severity="warning" variant="outlined">
                        {t('send_money.invalid_bip21_amount')}
                      </Alert>
                    )}

                    {amountExceedsAvailable && (
                      <Alert severity="warning" variant="outlined">
                        {t('send_money.amount_exceeds_available')}
                      </Alert>
                    )}

                    {bitcoinRequest.requiredParams.length > 0 && (
                      <Alert severity="warning" variant="outlined">
                        {t('send_money.unsupported_required_parameters', {
                          params: bitcoinRequest.requiredParams.join(', '),
                        })}
                      </Alert>
                    )}

                    {lightningInvoiceExpired && (
                      <Alert
                        severity={expiredInvoiceBlocksSubmit ? 'warning' : 'info'}
                        variant="outlined"
                      >
                        {expiredInvoiceBlocksSubmit
                          ? t('send_money.invoice_expired_note')
                          : t('send_money.invoice_fallback_expired_note')}
                      </Alert>
                    )}

                    {kind === 'unknown' && (
                      <Alert severity="warning" variant="outlined">
                        {t('send_money.unknown_input_note')}
                      </Alert>
                    )}
                  </Stack>
                </Box>
              )}

              {embeddedAmountSats > 0 ? null : canEnterAmount ? (
                <Stack spacing={1.5}>
                  <AmountEntryField
                    label={t('send_money.amount')}
                    value={amountValue}
                    fiatLabel={amountFiatLabel}
                    amountUnit={amountUnit}
                    currency={state.currency}
                    displayUnit={state.displayUnit ?? 'bip177'}
                    hasFiatPrice={hasFiatPrice}
                    error={amountExceedsAvailable}
                    onChange={setAmountValue}
                    onSwap={handleAmountUnitSwap}
                  />
                </Stack>
              ) : null}

              {kind !== 'unknown' && canSendComment && (
                <TextField
                  label={t(
                    kind === 'internal' ? 'send_money.internal_note' : 'send_money.recipient_note'
                  )}
                  value={comment}
                  helperText={t(
                    kind === 'internal'
                      ? 'send_money.internal_note_helper'
                      : 'send_money.recipient_note_helper'
                  )}
                  onChange={(event) => setComment(event.target.value)}
                />
              )}

              {balance != null && (
                <Stack
                  direction="row"
                  sx={{ justifyContent: 'space-between', typography: 'body2' }}
                >
                  <Typography variant="body2" color="text.secondary">
                    {t('send_money.available')}
                  </Typography>
                  <SatsWithIcon amountMSats={balance} />
                </Stack>
              )}

              <Stack direction="row" spacing={1.5}>
                <Button
                  fullWidth
                  color="inherit"
                  variant="contained"
                  loading={isSubmitting}
                  disabled={!canSubmit}
                  onClick={handlePay}
                >
                  {submitLabel}
                </Button>
              </Stack>
            </>
          )}
        </Stack>
      </Drawer>

      <ScanQRDialog open={scanQR.value} onClose={scanQR.onFalse} onResult={setInput} />
    </>
  );
}

// ----------------------------------------------------------------------

type ScanQRDialogProps = DialogProps & {
  onClose: VoidFunction;
  onResult: (result: string) => void;
};

function ScanQRDialog({ open, onClose, onResult }: ScanQRDialogProps) {
  const { t } = useTranslate();

  const handleScannerResult = (detectedCodes: IDetectedBarcode[]) => {
    const text = detectedCodes[0]?.rawValue;
    if (!text) return;

    onResult(text);
    onClose();
  };

  return (
    <Dialog open={open} fullWidth maxWidth="xs" onClose={onClose}>
      <Box sx={{ overflow: 'hidden', borderRadius: 1 }}>
        <Scanner paused={!open} onScan={handleScannerResult} formats={['qr_code']} />
      </Box>

      <DialogActions>
        <Button onClick={onClose}>{t('close')}</Button>
      </DialogActions>
    </Dialog>
  );
}

// ----------------------------------------------------------------------

export function ReceiveMoneyDrawer({
  open,
  lnAddress,
  fiatPrices,
  onClose,
  onSuccess,
  isAdmin = false,
  walletId,
}: ReceiveMoneyDrawerProps) {
  const { t } = useTranslate();
  const { state } = useSettingsContext();
  const { copy } = useCopyToClipboard();
  const { wallet } = useGetUserWallet();

  const [activePayload, setActivePayload] = useState<ReceivePayload>(
    lnAddress ? 'identity' : 'unified'
  );
  const [amountValue, setAmountValue] = useState('');
  const [amountUnit, setAmountUnit] = useState<AmountUnit>('sats');
  const [description, setDescription] = useState('');
  const [invoice, setInvoice] = useState<Invoice>();
  const [btcAddress, setBtcAddress] = useState<BtcAddress>();
  const [addressType, setAddressType] = useState<BtcAddressType>(
    (state.defaultAddressType ?? BtcAddressType.P2TR) as BtcAddressType
  );
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [addressError, setAddressError] = useState<string>();
  const [nowMs, setNowMs] = useState(0);
  const [selectedWalletId, setSelectedWalletId] = useState(walletId ?? '');

  const amountSats = useMemo(
    () => amountValueToSats(amountValue, amountUnit, fiatPrices, state.currency),
    [amountUnit, amountValue, fiatPrices, state.currency]
  );
  const activeWalletId = walletId || selectedWalletId;
  const addressWalletId = isAdmin ? activeWalletId : activeWalletId || wallet?.id;
  const needsWallet = isAdmin && !activeWalletId;
  const hasFiatPrice = (fiatPrices[state.currency] ?? 0) > 0;
  const bolt11 = invoice?.ln_invoice?.bolt11;
  const invoiceExpiresAt = invoice?.ln_invoice?.expires_at ?? null;
  const invoiceExpiryMs = invoiceExpiresAt ? new Date(invoiceExpiresAt).getTime() : null;
  const identityAddress = lnAddress ? displayLnAddress(lnAddress.username) : '';
  const identityLnurl = lnAddress ? encodeLNURL(lnAddress.username) : '';
  const onchainRequest = composeBip21(btcAddress?.address, undefined, amountSats);
  const bip21 = composeBip21(btcAddress?.address, bolt11, amountSats);
  const { btcAddresses, btcAddressesMutate } = useListBtcAddresses(
    addressWalletId ? { wallet_id: addressWalletId } : undefined
  );
  const unusedAddressForType = useMemo(
    () =>
      btcAddresses?.find((address) => address.address_type === addressType && !address.used) ??
      null,
    [addressType, btcAddresses]
  );
  const selectedPayload = activePayload;
  const qrValue =
    (selectedPayload === 'identity' && (identityLnurl || identityAddress)) ||
    (selectedPayload === 'lightning' && (bolt11 || '')) ||
    (selectedPayload === 'onchain' && (onchainRequest || '')) ||
    (selectedPayload === 'unified' && bolt11 && btcAddress ? bip21 : '');
  const payloadLabel =
    (selectedPayload === 'unified' && t('receive_money.unified')) ||
    (selectedPayload === 'identity' && t('receive_money.identity_tab')) ||
    (selectedPayload === 'lightning' && t('receive_money.lightning')) ||
    t('receive_money.onchain');
  const payloadDescription =
    (selectedPayload === 'unified' && t('receive_money.unified_payload_description')) ||
    (selectedPayload === 'identity' && t('receive_money.identity_payload_description')) ||
    (selectedPayload === 'lightning' && t('receive_money.lightning_payload_description')) ||
    t('receive_money.onchain_payload_description');
  const payloadCopyLabel =
    (selectedPayload === 'unified' && t('receive_money.copy_unified')) ||
    (selectedPayload === 'identity' && t('receive_money.copy_lnurl')) ||
    (selectedPayload === 'lightning' && t('receive_money.copy_lightning')) ||
    t('receive_money.copy_onchain');
  const selectedLightningInvoice = selectedPayload === 'unified' || selectedPayload === 'lightning';
  const invoiceHasExpired =
    invoice?.status === InvoiceStatus.EXPIRED ||
    (typeof invoiceExpiryMs === 'number' &&
      Number.isFinite(invoiceExpiryMs) &&
      invoiceExpiryMs <= nowMs);
  const showInvoiceExpiry = selectedLightningInvoice && Boolean(invoice?.ln_invoice);
  const primaryPayloadDisabled = showInvoiceExpiry && invoiceHasExpired;
  const invoiceExpiryRelative = invoiceExpiresAt ? fToNow(invoiceExpiresAt) : '';
  const invoiceExpiryAbsolute = invoiceExpiresAt ? fDateTime(invoiceExpiresAt) : '';
  const requestNeedsGeneration = selectedPayload !== 'identity';
  const selectedNeedsInvoice = selectedPayload === 'unified' || selectedPayload === 'lightning';
  const selectedNeedsAddress = selectedPayload === 'unified' || selectedPayload === 'onchain';
  const requestActionLabel =
    invoice || btcAddress
      ? t('receive_money.refresh_request')
      : selectedNeedsInvoice
        ? t('receive_money.generate_invoice')
        : t('receive_money.prepare_address');

  useEffect(() => {
    setSelectedWalletId(walletId ?? '');
  }, [walletId]);

  useEffect(() => {
    if (!invoiceExpiresAt) return undefined;

    setNowMs(Date.now());
    const interval = window.setInterval(() => setNowMs(Date.now()), 30_000);

    return () => window.clearInterval(interval);
  }, [invoiceExpiresAt]);

  const handleClose = useCallback(() => {
    setActivePayload(lnAddress ? 'identity' : 'unified');
    setAmountValue('');
    setAmountUnit('sats');
    setDescription('');
    setInvoice(undefined);
    setBtcAddress(undefined);
    setAddressError(undefined);
    setSelectedWalletId(walletId ?? '');
    onClose();
  }, [lnAddress, onClose, walletId]);

  const handleGenerate = async () => {
    if (needsWallet || !requestNeedsGeneration) return;

    try {
      setIsSubmitting(true);
      setAddressError(undefined);

      const shouldGenerateInvoice = selectedNeedsInvoice;
      const shouldGenerateAddress = selectedNeedsAddress;
      let nextInvoice: Invoice | undefined;
      let nextBtcAddress: BtcAddress | undefined;

      if (shouldGenerateInvoice) setInvoice(undefined);
      if (shouldGenerateAddress) setBtcAddress(undefined);

      if (shouldGenerateInvoice) {
        const invoiceBody = {
          description,
          amount_msat: amountSats * 1000,
          ...(isAdmin && { wallet_id: activeWalletId }),
        };
        const invoiceResult = isAdmin
          ? await generateInvoice({ body: invoiceBody })
          : await newWalletInvoice({ body: invoiceBody });

        nextInvoice = invoiceResult.data;
        setInvoice(nextInvoice);
      }

      if (shouldGenerateAddress) {
        if (unusedAddressForType) {
          nextBtcAddress = unusedAddressForType;
          setBtcAddress(nextBtcAddress);
        } else {
          const addressBody = {
            type: addressType,
            ...(isAdmin && { wallet_id: activeWalletId }),
          };

          const addressResult = await (
            isAdmin
              ? generateBtcAddress({ body: addressBody })
              : newWalletBtcAddress({ body: addressBody })
          ).catch((error) => {
            setAddressError(error?.message || t('receive_money.address_unavailable'));
            return null;
          });

          if (addressResult?.data) {
            nextBtcAddress = addressResult.data;
            setBtcAddress(nextBtcAddress);
            btcAddressesMutate();
          }
        }
      }

      onSuccess?.();
    } catch (error) {
      handleActionError(error);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleAmountUnitSwap = () => {
    const nextUnit: AmountUnit = amountUnit === 'fiat' ? 'sats' : 'fiat';
    const currentAmountSats = amountValueToSats(
      amountValue,
      amountUnit,
      fiatPrices,
      state.currency
    );

    setAmountUnit(nextUnit);
    setAmountValue(satsToAmountValue(currentAmountSats, nextUnit, fiatPrices, state.currency));
  };

  const handleAddressTypeChange = (nextAddressType: BtcAddressType) => {
    setAddressType(nextAddressType);

    const nextUnusedAddress =
      btcAddresses?.find((address) => address.address_type === nextAddressType && !address.used) ??
      null;

    if (nextUnusedAddress) {
      setBtcAddress(nextUnusedAddress);
      return;
    }

    setBtcAddress(undefined);
  };

  const handleSharePayload = async () => {
    if (!qrValue) return;

    if (typeof navigator !== 'undefined' && navigator.share) {
      try {
        await navigator.share({ text: qrValue });
        return;
      } catch {
        // Fall back to copying when native share is unavailable or cancelled.
      }
    }

    copy(qrValue);
    toast.success(t('copied_to_clipboard'));
  };

  const handleCopyPayload = (value: string) => {
    copy(value);
    toast.success(t('copied_to_clipboard'));
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
        {isAdmin &&
          (walletId ? (
            <TextField
              label={t('admin_wallet_picker.label')}
              value={walletId}
              slotProps={{ input: { readOnly: true } }}
              helperText={t('admin_wallet_picker.fixed')}
            />
          ) : (
            <WalletPicker value={selectedWalletId} onChange={setSelectedWalletId} />
          ))}

        {needsWallet && (
          <Alert severity="info" variant="outlined">
            {t('admin_wallet_picker.required')}
          </Alert>
        )}

        <Stack spacing={1}>
          <Typography variant="overline" color="text.secondary">
            {t('receive_money.display_mode')}
          </Typography>
          <ToggleButtonGroup
            exclusive
            fullWidth
            size="small"
            value={selectedPayload}
            onChange={(_, value: ReceivePayload | null) => value && setActivePayload(value)}
          >
            <ToggleButton value="unified">{t('receive_money.unified')}</ToggleButton>
            <ToggleButton value="lightning">{t('receive_money.lightning')}</ToggleButton>
            <ToggleButton value="onchain">{t('receive_money.bitcoin')}</ToggleButton>
            <ToggleButton value="identity" disabled={!identityAddress}>
              {t('receive_money.paycode')}
            </ToggleButton>
          </ToggleButtonGroup>
        </Stack>

        <Stack spacing={1.5}>
          <AmountEntryField
            label={t('receive_money.amount')}
            value={amountValue}
            fiatLabel={
              hasFiatPrice
                ? fCurrency(satsToFiat(amountSats, fiatPrices, state.currency), {
                    currency: state.currency,
                  })
                : t('send_money.no_fiat_value')
            }
            amountUnit={amountUnit}
            currency={state.currency}
            displayUnit={state.displayUnit ?? 'bip177'}
            hasFiatPrice={hasFiatPrice}
            onChange={setAmountValue}
            onSwap={handleAmountUnitSwap}
          />
        </Stack>

        <TextField
          label={t('receive_money.memo')}
          value={description}
          onChange={(event) => setDescription(event.target.value)}
        />

        <Accordion
          disableGutters
          variant="outlined"
          sx={{ borderRadius: 1, '&:before': { display: 'none' } }}
        >
          <AccordionSummary expandIcon={<Iconify icon="eva:arrow-ios-downward-fill" />}>
            <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
              <Iconify icon="solar:tuning-square-bold-duotone" />
              <Typography variant="subtitle2">{t('receive_money.advanced')}</Typography>
            </Stack>
          </AccordionSummary>
          <AccordionDetails>
            <Stack spacing={2}>
              {selectedNeedsAddress && (
                <Stack spacing={1}>
                  <Typography variant="caption" color="text.secondary">
                    {t('receive_money.address_type')}
                  </Typography>
                  <ToggleButtonGroup
                    exclusive
                    fullWidth
                    size="small"
                    value={addressType}
                    onChange={(_, value: BtcAddressType | null) =>
                      value && handleAddressTypeChange(value)
                    }
                  >
                    {addressTypeOptions.map((option) => (
                      <ToggleButton key={option.value} value={option.value}>
                        {t(option.labelKey)}
                      </ToggleButton>
                    ))}
                  </ToggleButtonGroup>
                  <Typography variant="caption" color="text.secondary">
                    {t(
                      addressTypeOptions.find((option) => option.value === addressType)
                        ?.helperKey ?? 'bitcoin_address_type.taproot_helper'
                    )}
                  </Typography>
                </Stack>
              )}

              {selectedNeedsInvoice && (
                <TextField
                  disabled
                  size="small"
                  label={t('receive_money.expiry')}
                  value={t('receive_money.default_expiry')}
                  helperText={t('receive_money.expiry_backend_needed')}
                />
              )}
            </Stack>
          </AccordionDetails>
        </Accordion>

        {requestNeedsGeneration && (
          <Button
            color="inherit"
            variant="contained"
            loading={isSubmitting}
            disabled={needsWallet}
            onClick={handleGenerate}
          >
            {requestActionLabel}
          </Button>
        )}

        {selectedNeedsAddress && unusedAddressForType && !btcAddress && (
          <Alert severity="info" variant="outlined">
            {t('receive_money.unused_address_available')}
          </Alert>
        )}

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

            <Stack spacing={1.5}>
              <Stack
                direction="row"
                spacing={1}
                sx={{ alignItems: 'center', justifyContent: 'space-between' }}
              >
                <Label color={selectedPayload === 'unified' ? 'success' : 'info'}>
                  {payloadLabel}
                </Label>
                {selectedLightningInvoice && !invoiceHasExpired && (
                  <Label color="success">{t('receive_money.waiting_for_payment')}</Label>
                )}
              </Stack>
              <Typography variant="body2" color="text.secondary">
                {payloadDescription}
              </Typography>

              <Stack direction="row" spacing={1} useFlexGap sx={{ flexWrap: 'wrap' }}>
                <Button
                  color="inherit"
                  variant="contained"
                  disabled={primaryPayloadDisabled}
                  onClick={() => handleCopyPayload(qrValue)}
                  startIcon={<Iconify icon="solar:copy-bold" />}
                  sx={{ flex: 1, minWidth: 140 }}
                >
                  {payloadCopyLabel}
                </Button>
                <Button
                  color="inherit"
                  variant="outlined"
                  disabled={primaryPayloadDisabled}
                  onClick={handleSharePayload}
                  startIcon={<Iconify icon="solar:share-bold" />}
                  sx={{ flex: 1, minWidth: 140 }}
                >
                  {t('share')}
                </Button>
              </Stack>

              {showInvoiceExpiry && (
                <Typography
                  variant="caption"
                  color={invoiceHasExpired ? 'warning.main' : 'text.secondary'}
                >
                  {invoiceHasExpired
                    ? t('receive_money.invoice_expired_description')
                    : t('receive_money.invoice_expires_short', {
                        time: invoiceExpiryRelative,
                        date: invoiceExpiryAbsolute,
                      })}
                </Typography>
              )}

              {(selectedPayload === 'onchain' || selectedPayload === 'unified') && btcAddress && (
                <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
                  <Label color={btcAddress.used ? 'warning' : 'success'}>
                    {btcAddress.used ? t('receive_money.used') : t('receive_money.unused')}
                  </Label>
                  <Typography variant="caption" color="text.secondary">
                    {t('receive_money.address_reuse_note')}
                  </Typography>
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

        {lnAddress && selectedPayload !== 'identity' && (
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
            <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
              <Iconify icon="solar:bolt-bold-duotone" sx={{ color: 'warning.main' }} />
              <Stack sx={{ flex: 1, minWidth: 0 }}>
                <Typography variant="caption" color="text.secondary">
                  {t('receive_money.identity')}
                </Typography>
                <Typography variant="subtitle2" noWrap>
                  {displayLnAddress(lnAddress.username)}
                </Typography>
              </Stack>
              <CopyButton value={displayLnAddress(lnAddress.username)} title={t('copy')} />
            </Stack>
          </Box>
        )}
      </Stack>
    </Drawer>
  );
}
