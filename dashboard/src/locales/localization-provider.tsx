'use client';

import 'dayjs/locale/en';
import 'dayjs/locale/fr';
import 'dayjs/locale/de';
import 'dayjs/locale/es';
import 'dayjs/locale/it';
import 'dayjs/locale/pt';

import dayjs from 'dayjs';
import { useEffect } from 'react';

import { AdapterDayjs } from '@mui/x-date-pickers/AdapterDayjs';
import { LocalizationProvider as Provider } from '@mui/x-date-pickers/LocalizationProvider';

import { useTranslate } from './use-locales';

// ----------------------------------------------------------------------

type Props = {
  children: React.ReactNode;
};

export function LocalizationProvider({ children }: Props) {
  const { currentLang } = useTranslate();

  useEffect(() => {
    dayjs.locale(currentLang.adapterLocale);
    document.documentElement.lang = currentLang.value;
  }, [currentLang]);

  return (
    <Provider dateAdapter={AdapterDayjs} adapterLocale={currentLang.adapterLocale}>
      {children}
    </Provider>
  );
}
