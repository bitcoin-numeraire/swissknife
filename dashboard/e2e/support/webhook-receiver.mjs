// Test-only HTTPS tunnel target. Captures synthetic/regtest events after verifying their signatures.
import { createServer } from 'node:http';
import { createHmac, timingSafeEqual } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
const file = process.env.SWISSKNIFE_E2E_RECEIVER_FILE;
if (!file) throw new Error('Set SWISSKNIFE_E2E_RECEIVER_FILE to a local capture file');
writeFileSync(file, '[]', { mode: 0o600 });
createServer((req, res) => {
  if (req.method !== 'POST') {
    res.writeHead(200);
    res.end('SwissKnife integration test receiver');
    return;
  }
  const chunks = [];
  let size = 0;
  req.on('data', (chunk) => {
    size += chunk.length;
    if (size > 1024 * 1024) req.destroy();
    else chunks.push(chunk);
  });
  req.on('end', () => {
    try {
      const body = Buffer.concat(chunks);
      const secret = JSON.parse(readFileSync(file + '.secret', 'utf8'))[req.url];
      const timestamp = req.headers['x-swissknife-timestamp'];
      const expected = Buffer.from(
        'v1=' +
          createHmac('sha256', Buffer.from(secret, 'base64url'))
            .update(timestamp + '.')
            .update(body)
            .digest('hex')
      );
      const received = Buffer.from(req.headers['x-swissknife-signature'] ?? '');
      if (
        expected.length !== received.length ||
        !timingSafeEqual(expected, received) ||
        Math.abs(Date.now() / 1000 - Number(timestamp)) > 300
      ) {
        res.writeHead(401);
        res.end();
        return;
      }
      const records = JSON.parse(readFileSync(file, 'utf8'));
      records.push({ path: req.url, headers: req.headers, body: body.toString('utf8') });
      writeFileSync(file, JSON.stringify(records), { mode: 0o600 });
      res.writeHead(204);
      res.end();
    } catch {
      res.writeHead(503);
      res.end();
    }
  });
}).listen(Number(process.env.PORT ?? 8555), '127.0.0.1');
