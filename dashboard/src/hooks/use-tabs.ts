import { useCallback } from 'react';
import { useRouter, usePathname, useSearchParams } from 'next/navigation';

export type UseTabsReturn = {
  value: string;
  setValue: (newValue: string) => void;
  onChange: (event: React.SyntheticEvent, newValue: string) => void;
};

export function useTabs(defaultValue: string): UseTabsReturn {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  // Get the 'tab' parameter from the URL
  const tab = searchParams.get('tab');

  // Determine the current tab value
  const value = tab || defaultValue;

  // Function to set a new tab value
  const setValue = useCallback(
    (newValue: string) => {
      // Create a new URLSearchParams object to manipulate query parameters
      const params = new URLSearchParams(searchParams.toString());
      params.set('tab', newValue);

      // Update the URL without reloading the page
      router.push(`${pathname}?${params.toString()}`);
    },
    [router, pathname, searchParams]
  );

  // Handle tab change event
  const onChange = useCallback(
    (event: React.SyntheticEvent, newValue: string) => {
      setValue(newValue);
    },
    [setValue]
  );

  return {
    value,
    setValue,
    onChange,
  };
}
