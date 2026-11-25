'use client';

import { useState, useEffect } from 'react';

import { Alert, Stack, Button } from '@mui/material';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { useAccountContext } from 'src/contexts/account';

import { RoleBasedGuard } from 'src/auth/guard';

import { WalletCreditDialog } from './wallet-credit-dialog';

export function OpeningCredit() {
  const { account } = useAccountContext();
  return (
    account && (
      <RoleBasedGuard permissions={[Permission.CREDIT_WALLET]}>
        <OpeningCreditPrompt key={account.id} accountId={account.id} />
      </RoleBasedGuard>
    )
  );
}

function OpeningCreditPrompt({ accountId }: { accountId: string }) {
  const { t } = useTranslate();
  const { wallets, walletsLoading } = useAccountContext();
  const key = 'swissknife.opening-credit.dismissed.' + accountId;
  const [visible, setVisible] = useState(false);
  const [open, setOpen] = useState(false);
  useEffect(() => {
    setVisible(localStorage.getItem(key) !== 'true');
  }, [key]);
  const dismiss = () => {
    localStorage.setItem(key, 'true');
    setVisible(false);
  };
  if (
    !visible ||
    walletsLoading ||
    wallets.length === 0 ||
    (wallets.some((w) => w.invoices?.some((i) => i.credit_purpose === 'opening')) &&
      !wallets.some((w) =>
        sessionStorage.getItem(['swissknife.wallet-credit', accountId, w.id, 'opening'].join('.'))
      ))
  )
    return null;
  return (
    <>
      <Alert
        severity="info"
        sx={{ mb: 2 }}
        action={
          <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1}>
            <Button size="small" color="inherit" onClick={() => setOpen(true)}>
              {t('wallet_credit.opening')}
            </Button>
            <Button size="small" color="inherit" onClick={dismiss}>
              {t('wallet_credit.skip')}
            </Button>
          </Stack>
        }
      >
        {t('wallet_credit.invitation')}
      </Alert>
      {open && (
        <WalletCreditDialog
          wallets={wallets}
          purpose="opening"
          onClose={() => setOpen(false)}
          onCredited={dismiss}
        />
      )}
    </>
  );
}
