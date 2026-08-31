/** Table identifiers published in Databento's reference-data enum catalog. */
export const referenceDataEnumNames = [
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
] as const;

export type ReferenceDataEnumName = (typeof referenceDataEnumNames)[number];

/** Row shape shared by the two-column reference-data tables. */
export interface ReferenceDataEnumEntry {
  value: string;
  description: string;
}

/** EVENTSUBTYPE rows are scoped by their parent EVENT value. */
export interface ReferenceDataEventSubtypeEntry extends ReferenceDataEnumEntry {
  event: string;
}

/** EXCHANGE is the only official table with venue and MIC columns. */
export interface ReferenceDataExchangeEntry {
  name: string;
  country: string;
  exchange: string;
  mic: string;
}

export type ReferenceDataEnumEntryFor<Name extends ReferenceDataEnumName> =
  Name extends 'EVENTSUBTYPE'
    ? ReferenceDataEventSubtypeEntry
    : Name extends 'EXCHANGE'
      ? ReferenceDataExchangeEntry
      : ReferenceDataEnumEntry;

/** A typed table returned by a reference-data catalog reader. */
export interface ReferenceDataEnumTable<
  Name extends ReferenceDataEnumName = ReferenceDataEnumName,
> {
  name: Name;
  entries: readonly ReferenceDataEnumEntryFor<Name>[];
}
