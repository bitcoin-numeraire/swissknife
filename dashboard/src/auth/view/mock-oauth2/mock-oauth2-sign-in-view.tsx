'use client';

import type { Permission as PermissionType } from 'src/lib/swissknife';

import { useState } from 'react';

import Box from '@mui/material/Box';
import Stack from '@mui/material/Stack';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';

import { useRouter, useSearchParams } from 'src/routes/hooks';

import { handleActionError } from 'src/utils/errors';

import { CONFIG } from 'src/global-config';
import { Permission } from 'src/lib/swissknife';

import { Iconify } from 'src/components/iconify';

import { useAuthContext } from 'src/auth/hooks';
import { JWT_STORAGE_KEY } from 'src/auth/context/jwt';
import { getSafeReturnTo } from 'src/auth/guard/setup-route-utils';

import { FormHead } from '../../components/form-head';

// ----------------------------------------------------------------------

type MockOAuth2TokenResponse = {
  access_token?: string;
};

type MockOAuth2Persona = {
  clientId: string;
  label: string;
  description: string;
  icon: string;
  permissions: PermissionType[];
};

const PERSONAS: MockOAuth2Persona[] = [
  {
    clientId: 'dev-admin',
    label: 'Admin',
    description: 'All dashboard permissions',
    icon: 'solar:shield-keyhole-bold-duotone',
    permissions: [
      Permission.READ_WALLET,
      Permission.WRITE_WALLET,
      Permission.READ_LN_ADDRESS,
      Permission.WRITE_LN_ADDRESS,
      Permission.READ_TRANSACTION,
      Permission.WRITE_TRANSACTION,
      Permission.READ_LN_NODE,
      Permission.WRITE_LN_NODE,
      Permission.READ_API_KEY,
      Permission.WRITE_API_KEY,
      Permission.READ_BTC_ADDRESS,
      Permission.WRITE_BTC_ADDRESS,
    ],
  },
  {
    clientId: 'dev-readonly',
    label: 'Readonly',
    description: 'Read-only account views',
    icon: 'solar:eye-bold-duotone',
    permissions: [
      Permission.READ_WALLET,
      Permission.READ_LN_ADDRESS,
      Permission.READ_TRANSACTION,
      Permission.READ_LN_NODE,
      Permission.READ_API_KEY,
      Permission.READ_BTC_ADDRESS,
    ],
  },
  {
    clientId: 'dev-wallet-operator',
    label: 'Wallet operator',
    description: 'Wallet and transaction actions',
    icon: 'solar:wallet-bold-duotone',
    permissions: [
      Permission.READ_WALLET,
      Permission.WRITE_WALLET,
      Permission.READ_TRANSACTION,
      Permission.WRITE_TRANSACTION,
    ],
  },
  {
    clientId: 'dev-address-operator',
    label: 'Address operator',
    description: 'Lightning and bitcoin addresses',
    icon: 'solar:link-round-angle-bold-duotone',
    permissions: [
      Permission.READ_WALLET,
      Permission.READ_LN_ADDRESS,
      Permission.WRITE_LN_ADDRESS,
      Permission.READ_BTC_ADDRESS,
      Permission.WRITE_BTC_ADDRESS,
    ],
  },
  {
    clientId: 'dev-api-key-admin',
    label: 'API key admin',
    description: 'API key management',
    icon: 'solar:code-bold-duotone',
    permissions: [Permission.READ_API_KEY, Permission.WRITE_API_KEY],
  },
  {
    clientId: 'dev-empty',
    label: 'Empty user',
    description: 'No privileged permissions',
    icon: 'solar:user-rounded-bold-duotone',
    permissions: [],
  },
];

async function fetchAccessToken(persona: MockOAuth2Persona) {
  const body = new URLSearchParams({
    grant_type: 'client_credentials',
    client_id: persona.clientId,
    client_secret: CONFIG.mockOAuth2.clientSecret,
    scope: 'openid',
  });

  const response = await fetch(CONFIG.mockOAuth2.tokenUrl, {
    body,
    method: 'POST',
    headers: {
      'content-type': 'application/x-www-form-urlencoded',
    },
  });

  if (!response.ok) {
    throw new Error(`Mock OAuth2 token request failed with ${response.status}`);
  }

  const data = (await response.json()) as MockOAuth2TokenResponse;

  if (!data.access_token) {
    throw new Error('Mock OAuth2 token response did not include an access token');
  }

  return data.access_token;
}

// ----------------------------------------------------------------------

export function MockOAuth2SignInView() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const { checkUserSession } = useAuthContext();
  const [loadingPersona, setLoadingPersona] = useState<string | null>(null);

  const returnTo = getSafeReturnTo(searchParams.get('returnTo'), CONFIG.auth.redirectPath);

  const handleSignIn = async (persona: MockOAuth2Persona) => {
    try {
      setLoadingPersona(persona.clientId);

      const accessToken = await fetchAccessToken(persona);
      sessionStorage.setItem(JWT_STORAGE_KEY, accessToken);
      await checkUserSession?.();

      router.replace(returnTo);
      router.refresh();
    } catch (error) {
      handleActionError(error);
    } finally {
      setLoadingPersona(null);
    }
  };

  return (
    <Stack spacing={3}>
      <FormHead
        title="Mock OAuth2"
        description="Choose a local persona"
        sx={{ mb: 0 }}
        icon={
          <Box
            sx={{
              width: 58,
              height: 58,
              display: 'grid',
              borderRadius: 1,
              placeItems: 'center',
              color: 'primary.main',
              bgcolor: 'background.neutral',
            }}
          >
            <Iconify icon="solar:login-3-bold-duotone" width={30} />
          </Box>
        }
      />

      <Stack spacing={1.25}>
        {PERSONAS.map((persona) => {
          const loading = loadingPersona === persona.clientId;

          return (
            <Button
              fullWidth
              size="large"
              key={persona.clientId}
              color="inherit"
              variant={persona.clientId === 'dev-admin' ? 'contained' : 'outlined'}
              loading={loading}
              disabled={loadingPersona !== null && !loading}
              data-testid={`mock-oauth2-persona-${persona.clientId}`}
              startIcon={<Iconify icon={persona.icon} width={22} />}
              onClick={() => handleSignIn(persona)}
              sx={{
                py: 1.2,
                gap: 1.5,
                borderRadius: 1,
                justifyContent: 'flex-start',
                '& .MuiButton-startIcon': { mr: 0 },
              }}
            >
              <Box sx={{ minWidth: 0, flex: 1, textAlign: 'left' }}>
                <Typography variant="subtitle2">{persona.label}</Typography>
                <Typography
                  noWrap
                  variant="caption"
                  sx={{ display: 'block', color: 'text.secondary' }}
                >
                  {persona.description} - {persona.permissions.length} permissions
                </Typography>
              </Box>
            </Button>
          );
        })}
      </Stack>
    </Stack>
  );
}
