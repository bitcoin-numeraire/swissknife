import { appTitle } from 'src/utils/format-string';

import { NodeHealthView } from 'src/sections/node/view/node-health-view';

export const metadata = { title: appTitle('Node health') };

export default function NodePage() {
  return <NodeHealthView />;
}
