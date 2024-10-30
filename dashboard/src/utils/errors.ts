export const shouldFail = (errors?: (any | null | undefined)[], data?: (object | null | undefined)[], isLoading?: boolean[]): boolean => {
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
