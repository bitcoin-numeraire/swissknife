import path from 'node:path';
import { execFileSync } from 'node:child_process';

const repoRoot = path.resolve(__dirname, '../..');

export default function globalTeardown() {
  if (process.env.PLAYWRIGHT_MANAGE_MOCK_OAUTH2 === 'false') {
    return;
  }

  execFileSync('make', ['down-oauth2'], {
    cwd: repoRoot,
    stdio: 'inherit',
  });
}
