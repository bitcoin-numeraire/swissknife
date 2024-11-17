import type { JwtPayload } from 'jwt-decode';
import type { Permission } from 'src/lib/swissknife';

export type UserType = Record<string, any> | null;

export type AuthState = {
  user: UserType;
  loading: boolean;
};

export type AuthContextValue = {
  user: UserType;
  loading: boolean;
  authenticated: boolean;
  unauthenticated: boolean;
  checkUserSession?: () => Promise<void>;
};

export type DecodedToken = JwtPayload & {
  permissions: Permission[];
};
