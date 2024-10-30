import { CONFIG } from 'src/config-global';

export function appTitle(str: string) {
  return `${str} - ${CONFIG.site.name}`;
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
