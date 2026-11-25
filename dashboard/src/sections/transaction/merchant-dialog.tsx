'use client';

import { useTabs } from 'minimal-shared/hooks';

import {
  Tab,
  Box,
  Tabs,
  Button,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
} from '@mui/material';

import { useTranslate } from 'src/locales';

import { MerchantRoutes } from './merchant-routes';
import { MerchantRevenue } from './merchant-revenue';

export function MerchantDialog({ walletId, onClose }: { walletId: string; onClose: () => void }) {
  const { t } = useTranslate();
  const tabs = useTabs('revenue');
  return (
    <Dialog open onClose={onClose} fullWidth maxWidth="md" aria-labelledby="merchant-title">
      <DialogTitle id="merchant-title">{t('merchant_tools.title')}</DialogTitle>
      <DialogContent>
        <Tabs
          value={tabs.value}
          onChange={tabs.onChange}
          aria-label={t('merchant_tools.title')}
          sx={{ mb: 3 }}
        >
          <Tab
            id="revenue-tab"
            aria-controls="merchant-panel"
            value="revenue"
            label={t('merchant_revenue.title')}
          />
          <Tab
            id="routes-tab"
            aria-controls="merchant-panel"
            value="routes"
            label={t('merchant_tools.routes')}
          />
        </Tabs>
        <Box role="tabpanel" id="merchant-panel" aria-labelledby={tabs.value + '-tab'}>
          {tabs.value === 'routes' ? (
            <MerchantRoutes walletId={walletId} />
          ) : (
            <MerchantRevenue walletId={walletId} />
          )}
        </Box>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>{t('close')}</Button>
      </DialogActions>
    </Dialog>
  );
}
