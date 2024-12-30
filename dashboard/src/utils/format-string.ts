import { CONFIG } from 'src/global-config';

export function appTitle(str: string) {
  return `${str} - ${CONFIG.appName}`;
}

export function truncateText(text?: string | null, maxLength: number = 30) {
  if (!text) {
    return '';
  }

  if (text.length <= maxLength) {
    return text;
  }
  return `${text.substring(0, maxLength)}...`;
}
