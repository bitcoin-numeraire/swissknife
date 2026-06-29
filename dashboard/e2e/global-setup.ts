import path from 'node:path';
import { execFileSync } from 'node:child_process';

const repoRoot = path.resolve(__dirname, '../..');

export default function globalSetup() {
  if (process.env.PLAYWRIGHT_MANAGE_MOCK_OAUTH2 === 'false') {
    return;
  }

  execFileSync('make', ['up-oauth2'], {
    cwd: repoRoot,
    stdio: 'inherit',
  });
}
