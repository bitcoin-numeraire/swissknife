import { it, expect, describe } from 'vitest';

import { bitcoinUriWithoutLightning } from './bitcoin-request';

describe('bitcoinUriWithoutLightning', () => {
  it('removes only optional Lightning alternatives without rewriting the remaining request', () => {
    const input =
      'bitcoin:1BoatSLRHtKNngkdXEeobR76b53LETtpyT?amount=0.0005&lightning=invoice&message=Order%20%2342&req-feature=1&label=Alice+Bob&extra=%2B&LIGHTNING=duplicate';

    expect(bitcoinUriWithoutLightning(input)).toBe(
      'bitcoin:1BoatSLRHtKNngkdXEeobR76b53LETtpyT?amount=0.0005&message=Order%20%2342&req-feature=1&label=Alice+Bob&extra=%2B'
    );
  });

  it('recognizes encoded Lightning keys while retaining required parameters', () => {
    expect(
      bitcoinUriWithoutLightning(
        'bitcoin:address?%6cightning=one&req-lightning=two&LIGHTNING=three'
      )
    ).toBe('bitcoin:address?req-lightning=two');
  });

  it('removes the query delimiter if Lightning was the only parameter', () => {
    expect(bitcoinUriWithoutLightning('bitcoin:address?lightning=one')).toBe('bitcoin:address');
  });

  it.each(['bitcoin:address', 'lightning:invoice', 'invoice', 'https://example.com?lightning=one'])(
    'leaves non-Unified input unchanged: %s',
    (input) => expect(bitcoinUriWithoutLightning(input)).toBe(input)
  );
});
