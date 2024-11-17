'use client';

import { createContext } from 'react';

import type { AuthContextValue } from '../types';

// ----------------------------------------------------------------------

export const AuthContext = createContext<AuthContextValue | undefined>(undefined);

export const AuthConsumer = AuthContext.Consumer;
