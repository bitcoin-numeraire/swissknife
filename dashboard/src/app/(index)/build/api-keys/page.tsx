import { redirect } from 'next/navigation';

import { paths } from 'src/routes/paths';

export default function Page() {
  redirect(paths.build.apiKeys);
}
