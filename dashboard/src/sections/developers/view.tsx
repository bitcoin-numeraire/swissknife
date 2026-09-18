'use client';

import {
  Box,
  Tab,
  Card,
  Grid,
  Tabs,
  Stack,
  Button,
  Typography,
  ToggleButton,
  ToggleButtonGroup,
} from '@mui/material';

import { useRouter, useSearchParams } from 'src/routes/hooks';

import { useTranslate } from 'src/locales';
import { Permission } from 'src/lib/swissknife';
import { useAccountContext } from 'src/contexts/account';
import { DashboardContent } from 'src/layouts/dashboard';

import { Iconify } from 'src/components/iconify';
import { ErrorView } from 'src/components/error';
import { CustomBreadcrumbs } from 'src/components/custom-breadcrumbs';

import { WebhooksPanel } from 'src/sections/webhook/webhooks-panel';
import { ApiKeysPanel } from 'src/sections/api-key/view/api-key-list-view';

import { useAuthContext } from 'src/auth/hooks';

export function DevelopersView() {
  const { t } = useTranslate();
  const { user } = useAuthContext();
  const { account, accountLoading, accountError, refreshAccount } = useAccountContext();
  const router = useRouter();
  const params = useSearchParams();
  const tab = params.get('tab') === 'webhooks' ? 'webhooks' : 'api-keys';
  const permissions: string[] = user?.permissions ?? [];
  const canRead = permissions.includes(
    tab === 'webhooks' ? Permission.READ_WEBHOOK : Permission.READ_API_KEY
  );
  const canWrite = permissions.includes(
    tab === 'webhooks' ? Permission.WRITE_WEBHOOK : Permission.WRITE_API_KEY
  );
  const isAdmin = params.get('scope') === 'admin' && (canRead || canWrite);
  const navigate = (nextTab: string, admin: boolean) => {
    const next = new URLSearchParams({ tab: nextTab });
    if (admin) next.set('scope', 'admin');
    router.push(`/developers?${next}`);
  };
  if (accountLoading || accountError || !account) {
    return (
      <DashboardContent>
        <ErrorView errors={[accountError]} isLoading={[accountLoading]} data={[account]} />
        {!accountLoading && <Button onClick={() => refreshAccount()}>{t('refresh')}</Button>}
      </DashboardContent>
    );
  }
  return (
    <DashboardContent>
      <CustomBreadcrumbs
        heading={t('developers.title')}
        links={[{ name: t('build') }, { name: t('developers.title') }]}
        sx={{ mb: { xs: 3, md: 5 } }}
      />
      <Grid container spacing={3}>
        <Grid size={{ xs: 12, md: 3 }}>
          <Card sx={{ p: 1, borderRadius: 1 }}>
            <Tabs
              orientation="vertical"
              value={tab}
              onChange={(_, value) => navigate(value, false)}
              aria-label={t('developers.title')}
              sx={{ '& .MuiTabs-indicator': { display: 'none' } }}
            >
              {(['api-keys', 'webhooks'] as const).map((value) => (
                <Tab
                  key={value}
                  value={value}
                  aria-controls={`developers-${value}`}
                  id={`developers-tab-${value}`}
                  iconPosition="start"
                  icon={
                    <Iconify
                      icon={
                        value === 'api-keys'
                          ? 'solar:code-bold-duotone'
                          : 'solar:programming-bold-duotone'
                      }
                    />
                  }
                  label={t(value === 'api-keys' ? 'api_keys' : 'webhooks')}
                  sx={{
                    px: 2,
                    py: 1.5,
                    borderRadius: 1,
                    justifyContent: 'flex-start',
                    gap: 1.5,
                    bgcolor: tab === value ? 'action.selected' : 'transparent',
                    color: tab === value ? 'text.primary' : 'text.secondary',
                  }}
                />
              ))}
            </Tabs>
          </Card>
        </Grid>
        <Grid size={{ xs: 12, md: 9 }}>
          <Stack spacing={3}>
            <Stack spacing={1.5}>
              <Typography color="text.secondary">{t('developers.description')}</Typography>
              {(canRead || canWrite) && (
                <ToggleButtonGroup
                  exclusive
                  value={isAdmin ? 'admin' : 'me'}
                  onChange={(_, value) => value && navigate(tab, value === 'admin')}
                  aria-label={t('developers.scope')}
                  size="small"
                  sx={{ alignSelf: 'flex-start' }}
                >
                  <ToggleButton value="me">{t('developers.mine')}</ToggleButton>
                  <ToggleButton value="admin">{t('developers.instance')}</ToggleButton>
                </ToggleButtonGroup>
              )}
            </Stack>
            <Box
              role="tabpanel"
              id={`developers-${tab}`}
              aria-labelledby={`developers-tab-${tab}`}
              key={`${user?.sub}-${tab}-${isAdmin}`}
            >
              {tab === 'webhooks' ? (
                <WebhooksPanel
                  key={params.get('id') ?? 'list'}
                  isAdmin={isAdmin}
                  canRead={!isAdmin || canRead}
                  canWrite={!isAdmin || canWrite}
                />
              ) : (
                <ApiKeysPanel
                  isAdmin={isAdmin}
                  canRead={!isAdmin || canRead}
                  canWrite={!isAdmin || canWrite}
                />
              )}
            </Box>
          </Stack>
        </Grid>
      </Grid>
    </DashboardContent>
  );
}
