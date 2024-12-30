'use client';

import { useEffect } from 'react';

import { handleActionError } from 'src/utils/errors';

import { View500 } from 'src/sections/error';

export default function Error({ error, reset }: { error: Error; reset: () => void }) {
  useEffect(() => {
    handleActionError(error);
  }, [error]);

  return <View500 />;
}
