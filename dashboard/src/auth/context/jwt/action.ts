'use client';

import { JWT_STORAGE_KEY } from './constant';

// ----------------------------------------------------------------------

/** **************************************
 * Sign out
 *************************************** */
export const signOut = async (): Promise<void> => {
  sessionStorage.removeItem(JWT_STORAGE_KEY);
};
