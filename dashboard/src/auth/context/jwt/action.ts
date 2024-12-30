'use client';

import { endpointKeys } from 'src/actions/keys';

import { JWT_STORAGE_KEY } from './constant';

// ----------------------------------------------------------------------

export type SignUpParams = {
  email: string;
  password: string;
  firstName: string;
  lastName: string;
};

/** **************************************
 * Sign up
 *************************************** */
export const signUp = async ({
  email,
  password,
  firstName,
  lastName,
}: SignUpParams): Promise<void> => {
  const res = await fetch(endpointKeys.auth.signUp, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password, firstName, lastName }),
  });

  const { accessToken } = await res.json();

  if (!accessToken) {
    throw new Error('Access token not found in response');
  }

  sessionStorage.setItem(JWT_STORAGE_KEY, accessToken);
};

/** **************************************
 * Sign out
 *************************************** */
export const signOut = async (): Promise<void> => {
  sessionStorage.removeItem(JWT_STORAGE_KEY);
};
