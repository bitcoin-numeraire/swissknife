'use client';

import { clearSession } from './utils';

// ----------------------------------------------------------------------

/** **************************************
 * Sign out
 *************************************** */
export const signOut = async (): Promise<void> => {
  clearSession();
};
