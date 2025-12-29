# Changelog

Všetky významné zmeny v projekte sú zdokumentované v tomto súbore.

Formát je založený na [Keep a Changelog](https://keepachangelog.com/sk/1.1.0/),
a projekt používa [Semantic Versioning](https://semver.org/lang/cs/).

## [Unreleased]

## [0.2.0] - 2025-12-29

### Pridané
- Možnosť vymazať zálohy
- Výber roku v hlavičke aplikácie

### Opravené
- Oprava reaktivity dropdown-u pre výber roku
- Export: dummy riadky (0 km) sa nezapočítavajú do súčtov
- Autocomplete pre odkiaľ/kam: oprava generovania trás pri úprave jázd

## [0.1.0] - 2024-12-28

### Pridané
- Evidencia jázd s automatickým výpočtom spotreby
- Sledovanie tankovania a zostatku paliva (zostatok)
- Upozornenie pri prekročení 20% limitu nadpotreby
- Návrhy kompenzačných jázd pre dodržanie limitu
- Automatický výpočet ODO z predchádzajúcej jazdy
- Pamätanie trás s automatickým dopĺňaním vzdialenosti
- Zálohovanie a obnova databázy
- Export (HTML náhľad s tlačou do PDF)
- Podpora pre čiastočné tankovanie
- Ročné prehľady (každý rok = samostatná kniha jázd)
