import type { Options } from 'ajv';

import { fullFormats } from 'ajv-formats/dist/formats';

export const ajvOptions: Options = { allErrors: true, formats: fullFormats, strictSchema: false, coerceTypes: true };
