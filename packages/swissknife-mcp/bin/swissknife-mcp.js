#!/usr/bin/env node
import { spawn } from 'node:child_process';

const binary = process.env.SWISSKNIFE_BIN || 'swissknife';
const child = spawn(binary, ['mcp', ...process.argv.slice(2)], { stdio: 'inherit' });

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.once(signal, () => child.kill(signal));
}
child.on('error', (error) => {
  console.error(error.code === 'ENOENT'
    ? 'Install the SwissKnife binary or set SWISSKNIFE_BIN to its path.'
    : 'Unable to start the SwissKnife MCP command.');
  process.exitCode = 1;
});
child.on('exit', (code, signal) => {
  process.exitCode = code ?? (signal === 'SIGINT' ? 130 : signal === 'SIGTERM' ? 143 : 1);
});
