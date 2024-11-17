import { Alert } from '@mui/material';

import { LoadingScreen } from '../loading-screen';

// ----------------------------------------------------------------------

type Props = {
  errors?: any[];
  data?: (object | null | undefined)[];
  isLoading?: boolean[];
};

export function ErrorView({ errors = [], data = [], isLoading = [] }: Props) {
  if (isLoading.some((loading) => loading)) {
    return <LoadingScreen />;
  }

  const error = errors.find((err) => err !== null);
  if (error) {
    return <Alert severity="error">Error while fetching data: {error.message}</Alert>;
  }

  if (data.some((d) => d == null)) {
    return <Alert severity="error">Failed to fetch data. Please contact your administrator</Alert>;
  }
}
