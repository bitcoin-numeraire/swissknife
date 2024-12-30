import type { ErrorResponse } from 'src/lib/swissknife';

import { toast } from 'sonner';

export const shouldFail = (
  errors?: (any | null | undefined)[],
  data?: (object | null | undefined)[],
  isLoading?: boolean[]
): boolean => {
  if (isLoading?.some((loading) => loading)) {
    return true;
  }

  if (errors?.some((err) => err != null)) {
    return true;
  }

  if (data?.some((d) => d == null)) {
    return true;
  }

  return false;
};

function isErrorResponse(error: unknown): error is ErrorResponse {
  return typeof error === 'object' && error !== null && 'reason' in error;
}

export function handleActionError(error: unknown) {
  console.error('Error occurred:', error);

  if (isErrorResponse(error)) {
    toast.error(error.reason);
  } else if (error instanceof Error) {
    toast.error(error.message || 'An error occurred');
  } else {
    toast.error('An unknown error occurred');
  }
}
