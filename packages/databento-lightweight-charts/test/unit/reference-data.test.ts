import { describe, expect, it } from 'vitest';
import { referenceDataEnumNames, type ReferenceDataEnumTable } from '../../src/index.js';

describe('Databento reference data models', () => {
  it('covers every official reference-data table', () => {
    expect(referenceDataEnumNames).toEqual([
      'ACTION',
      'CNTRY',
      'CUREN',
      'DRTYPE',
      'EVENT',
      'EVENTSUBTYPE',
      'EXCHANGE',
      'FRACTIONS',
      'FREQ',
      'GLOBSTATUS',
      'LISTSOURCE',
      'LISTSTAT',
      'MANDVOLU',
      'MARKER',
      'MICSEG',
      'MKTSG',
      'MRGRSTAT',
      'ONOFFMKT',
      'OUTTURNSTYLE',
      'PARTFINAL',
      'PAYTYPE',
      'REGS144A',
      'SECTYPE',
      'TKOVRSTAT',
      'UTTYP',
      'VOTING',
    ]);
  });

  it('specializes multi-column table rows', () => {
    const exchange = {
      name: 'EXCHANGE',
      entries: [
        {
          name: 'New York Stock Exchange',
          country: 'United States',
          exchange: 'USNYSE',
          mic: 'XNYS',
        },
      ],
    } satisfies ReferenceDataEnumTable<'EXCHANGE'>;
    const subtype = {
      name: 'EVENTSUBTYPE',
      entries: [{ event: 'AGM', value: 'EGM', description: 'Extraordinary General Meeting' }],
    } satisfies ReferenceDataEnumTable<'EVENTSUBTYPE'>;

    expect(exchange.entries[0]?.mic).toBe('XNYS');
    expect(subtype.entries[0]?.event).toBe('AGM');
  });
});
