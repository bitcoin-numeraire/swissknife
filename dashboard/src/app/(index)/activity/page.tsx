import { appTitle } from 'src/utils/format-string';

import { ActivityView } from 'src/sections/activity/view';

// ----------------------------------------------------------------------

export const metadata = {
  title: appTitle('Activity'),
};

export default function ActivityPage() {
  return <ActivityView />;
}
