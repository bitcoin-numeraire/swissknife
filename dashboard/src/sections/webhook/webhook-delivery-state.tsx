import { useTranslate } from 'src/locales';

import { Label } from 'src/components/label';

export function DeliveryState({ status }: { status: string }) {
  const { t } = useTranslate();
  return (
    <Label
      color={status === 'Delivered' ? 'success' : status === 'Exhausted' ? 'error' : 'warning'}
    >
      {t(`webhook.status_${status}`)}
    </Label>
  );
}
