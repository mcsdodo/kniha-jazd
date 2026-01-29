**Date:** 2026-01-29
**Subject:** Legal Requirements Updates for Slovak Logbook (from 1.1.2026)
**Status:** Planning

## Original Requirements (Slovak)

Je potrebné zabezpečiť, aby od 1.1.2026 tabuľka obsahovala nižšie uvedené (pridávam Ti k tomu aj poznámky):

stav počítadla kilometrov osobného motorového vozidla v deň začatia vedenia záznamov, na konci každého zdaňovacieho obdobia (t.j. ku koncu každého mesiaca) a v deň ukončenia vedenia záznamov (v zákone je explicitne napísané "na konci každého zdaňovacieho obdobia" (zdaňovacím obdobím pre Teba je mesiac), neviem, čo tým mysleli, ale môžeme to skúsiť vyriešiť tak, že posledný deň v mesiaci bude riadok farebne odlíšený...alebo teda robiť tabuľku za každý mesiac)

evidenciu o každom použití osobného motorového vozidla, ktorá obsahuje najmä tieto údaje:

4a) poradové číslo záznamu o jazde (doplniť stĺpec a začať od 1 do nekonečna)

4b) meno a priezvisko osoby, ktorá viedla osobné motorové vozidlo počas jazdy (doplniť stĺpec a uvádzať Tvoje meno)

4c) dátum, čas začatia jazdy a skončenia jazdy (treba doplniť aj čas; v rámci jedného dňa môže byť aj viac jázd a každú treba vykázať samostatne)

4f) počet najazdených kilometrov za každú jazdu, stav počítadla kilometrov pred každou jazdou a po každej jazde - toto je opäť vyslovene napísané v zákone, takže, aj keď to postráda logiku, treba pridať stĺpec km na začiatku jazdy (kam sa prepíše stav z riadku nad) a ponechať stĺpec km po skončení jazdy

## Design Decisions (from brainstorming)

| Item | Decision | Rationale |
|------|----------|-----------|
| Driver name | Vehicle-level, not per-trip | Simpler, covers typical single-driver case |
| Taxable period | Monthly (fixed, not configurable) | Law specifies monthly; no need for flexibility |
| Trip numbering | Calculated per-year, not stored | Updates automatically if trips reordered |
| Odometer before | Derived from previous trip | No extra data entry, mathematically correct |

## Out of Scope (Deferred)

- Configurable taxable period (not needed per current law)
- Per-trip driver override (may add later if requested)
- Configurable artificial rows in the table (mandatory in export)

## See Also

- [02-design.md](02-design.md) - Full technical design
