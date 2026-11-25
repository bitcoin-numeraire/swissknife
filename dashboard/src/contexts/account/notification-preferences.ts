import { dashboardSettings } from './account-selection';

export function notificationPreferences(settings: unknown) {
  const saved = dashboardSettings(dashboardSettings(settings).notifications);
  return {
    incoming: saved.incoming !== false,
    outgoing: saved.outgoing !== false,
    sound: saved.sound === true,
  };
}

export type NotificationPreference = keyof ReturnType<typeof notificationPreferences>;
