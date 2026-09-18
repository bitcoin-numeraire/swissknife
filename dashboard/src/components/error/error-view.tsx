import { Alert } from '@mui/material';

import { isErrorResponse } from 'src/utils/errors';

import { LoadingScreen } from '../loading-screen';

// ----------------------------------------------------------------------

type Props = {
  errors?: unknown[];
  data?: (object | null | undefined)[];
  isLoading?: boolean[];
};

export function ErrorView({ errors = [], data = [], isLoading = [] }: Props) {
  if (isLoading.some((loading) => loading)) {
    return <LoadingScreen />;
  }

  const error = errors.find((err) => err != null);
  if (error) {
    const message = isErrorResponse(error)
      ? error.reason
      : error instanceof Error
        ? error.message
        : '';
    return <Alert severity="error">Error while fetching data: {message}</Alert>;
  }

  if (data.some((d) => d == null)) {
    return <Alert severity="error">Failed to fetch data. Please contact your administrator</Alert>;
  }
}
