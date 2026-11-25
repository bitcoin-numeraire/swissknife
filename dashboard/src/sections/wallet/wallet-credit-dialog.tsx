'use client';

import type { Wallet, CreditPurpose, CreateWalletCredit } from 'src/lib/swissknife';

import { useSWRConfig } from 'swr';
import { useRef, useState } from 'react';

import {
  Box,
  Alert,
  Stack,
  Button,
  Dialog,
  MenuItem,
  TextField,
  Typography,
  DialogTitle,
  DialogActions,
  DialogContent,
} from '@mui/material';

import {
  availableUnits,
  parseCreditAmount,
  creditDenomination,
  displayCreditUnits,
  projectedCreditBalance,
} from 'src/utils/credit-amount';

import { useTranslate } from 'src/locales';
import { endpointKeys } from 'src/actions/keys';
import { creditWallet } from 'src/lib/swissknife';
import { useAccountContext } from 'src/contexts/account';
import { zCreateWalletCredit } from 'src/lib/swissknife/zod.gen';

import { toast } from 'src/components/snackbar';

type Props = {
  wallets: Wallet[];
  purpose: CreditPurpose;
  onClose: () => void;
  onCredited?: () => void;
};

export function WalletCreditDialog({ wallets, purpose, onClose, onCredited }: Props) {
  const { t } = useTranslate();
  const { account } = useAccountContext();
  const [walletId, setWalletId] = useState(
    purpose === 'top_up' && wallets.length === 1 ? wallets[0].id : ''
  );
  const wallet = wallets.find((row) => row.id === walletId);
  const [busy, setBusy] = useState(false);
  return (
    <Dialog open fullWidth maxWidth="sm" onClose={busy ? undefined : onClose}>
      <DialogTitle>
        {t(purpose === 'opening' ? 'wallet_credit.opening' : 'wallet_credit.add')}
      </DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ pt: 1 }}>
          <Typography variant="body2">{t('wallet_credit.explanation')}</Typography>
          <TextField
            select
            label={t('wallet_credit.wallet')}
            value={walletId}
            disabled={busy || purpose === 'top_up'}
            onChange={(event) => setWalletId(event.target.value)}
          >
            {wallets
              .filter((row) => row.asset)
              .map((row) => (
                <MenuItem key={row.id} value={row.id}>
                  {row.label || row.asset?.display_ticker} · {row.asset?.network} · {row.id}
                </MenuItem>
              ))}
          </TextField>
          {wallet?.asset && account && (
            <CreditForm
              key={wallet.id}
              actorId={account.id}
              wallet={wallet}
              purpose={purpose}
              onBusy={setBusy}
              onClose={onClose}
              onCredited={onCredited}
            />
          )}
        </Stack>
      </DialogContent>
      {!wallet && (
        <DialogActions>
          <Button onClick={onClose}>{t('wallet_credit.cancel')}</Button>
        </DialogActions>
      )}
    </Dialog>
  );
}

type FormProps = {
  wallet: Wallet;
  purpose: CreditPurpose;
  actorId: string;
  onBusy: (value: boolean) => void;
  onClose: () => void;
  onCredited?: () => void;
};

function CreditForm({ wallet, purpose, actorId, onBusy, onClose, onCredited }: FormProps) {
  const { t, currentLang } = useTranslate();
  const { refreshWallets, refreshActiveWallet } = useAccountContext();
  const { mutate } = useSWRConfig();
  const storageKey = ['swissknife.wallet-credit', actorId, wallet.id, purpose].join('.');
  const [pending, setPending] = useState<CreateWalletCredit | null>(() => {
    const value = sessionStorage.getItem(storageKey);
    if (!value) return null;
    try {
      const parsed = zCreateWalletCredit.safeParse(JSON.parse(value));
      return parsed.success &&
        /^[1-9]\d{0,18}$/.test(parsed.data.amount_units) &&
        parsed.data.account_id === wallet.account_id &&
        parsed.data.asset_id === wallet.asset_id &&
        parsed.data.purpose === purpose
        ? parsed.data
        : null;
    } catch {
      return null;
    }
  });
  const [amount, setAmount] = useState('');
  const [reason, setReason] = useState(pending?.reason ?? '');
  const [message, setMessage] = useState('');
  const [busy, setBusy] = useState(false);
  const submitting = useRef(false);
  const { symbol, decimals } = creditDenomination(wallet.asset!);
  const units = pending ? BigInt(pending.amount_units) : parseCreditAmount(amount, decimals);
  const current = availableUnits(wallet.balance);
  const projected = projectedCreditBalance(current, units);
  const fmt = (value: bigint) =>
    displayCreditUnits(value, decimals, currentLang.numberFormat.code) + ' ' + symbol;

  const submit = async () => {
    if (submitting.current || (!pending && (units == null || projected == null || !reason.trim())))
      return;
    const request = pending ?? {
      request_id: crypto.randomUUID(),
      account_id: wallet.account_id,
      asset_id: wallet.asset_id,
      amount_units: units!.toString(),
      purpose,
      reason: reason.trim(),
    };
    submitting.current = true;
    setBusy(true);
    onBusy(true);
    setMessage('');
    try {
      sessionStorage.setItem(storageKey, JSON.stringify(request));
      setPending(request);
      const { data, error, response } = await creditWallet({
        path: { id: wallet.id },
        body: request,
        throwOnError: false,
      });
      if (!data) {
        if (
          response &&
          response.status >= 400 &&
          response.status < 500 &&
          ![408, 429].includes(response.status)
        ) {
          sessionStorage.removeItem(storageKey);
          setPending(null);
          setMessage(error?.reason ?? t('wallet_credit.rejected'));
        } else setMessage(t('wallet_credit.uncertain'));
        return;
      }
      sessionStorage.removeItem(storageKey);
      setPending(null);
      toast.success(t('wallet_credit.recorded'));
      onCredited?.();
      onClose();
      await Promise.allSettled([
        mutate(endpointKeys.wallets.get(wallet.id)),
        mutate(endpointKeys.wallets.listOverviews),
        refreshWallets(),
        refreshActiveWallet(),
      ]);
    } catch {
      setMessage(t('wallet_credit.uncertain'));
    } finally {
      submitting.current = false;
      setBusy(false);
      onBusy(false);
    }
  };
  return (
    <>
      <Typography variant="body2">
        {t('wallet_credit.recipient', {
          id: wallet.account_id,
          asset: wallet.asset!.display_ticker,
          network: wallet.asset!.network,
        })}
      </Typography>
      <TextField
        label={t('wallet_credit.amount', { unit: symbol })}
        value={pending ? fmt(BigInt(pending.amount_units)) : amount}
        onChange={(event) => setAmount(event.target.value)}
        disabled={busy || !!pending}
        slotProps={{ htmlInput: { inputMode: 'decimal', maxLength: 70 } }}
      />
      <TextField
        label={t('wallet_credit.reason')}
        value={reason}
        onChange={(e) => setReason(e.target.value)}
        disabled={busy || !!pending}
        slotProps={{ htmlInput: { maxLength: 240 } }}
      />
      {units != null && (
        <Box>
          <Typography variant="body2">
            {t('wallet_credit.credit_value', { amount: fmt(units) })}
          </Typography>
          {!pending && projected != null && (
            <Typography variant="subtitle2">
              {t('wallet_credit.after', { balance: fmt(projected) })}
            </Typography>
          )}
        </Box>
      )}
      {!pending && amount && projected == null && (
        <Alert severity="warning">{t('wallet_credit.invalid')}</Alert>
      )}
      {(message || pending) && (
        <Alert severity="warning">{message || t('wallet_credit.pending')}</Alert>
      )}
      <Stack direction="row" spacing={1} sx={{ justifyContent: 'flex-end' }}>
        <Button disabled={busy} onClick={onClose}>
          {t('wallet_credit.cancel')}
        </Button>
        <Button
          variant="contained"
          disabled={busy || (!pending && (!reason.trim() || projected == null))}
          onClick={() => void submit()}
        >
          {t(pending ? 'wallet_credit.retry' : 'wallet_credit.confirm')}
        </Button>
      </Stack>
    </>
  );
}
