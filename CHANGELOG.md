# Changelog

Všetky významné zmeny v projekte sú zdokumentované v tomto súbore.

Formát je založený na [Keep a Changelog](https://keepachangelog.com/sk/1.1.0/),
a projekt používa [Semantic Versioning](https://semver.org/lang/cs/).

## [Unreleased]

### Zmenené
- Zjednodušenie príkazov (commands) - príkazy teraz len odkazujú na skills

### Opravené
- Preklad tlačidla "Sync" na "Načítať" na stránke Doklady

## [0.5.0] - 2025-12-30

### Pridané
- Dokumentácia nastavenia Doklady (AI OCR) v README - konfigurácia Gemini API kľúča a priečinka s účtenkami
- Zobrazenie cesty ku konfiguračnému priečinku v upozornení na stránke Doklady s tlačidlom na otvorenie priečinka
- Vzorový konfiguračný súbor `local.settings.json.sample` s príkladom Windows cesty (pozor na dvojité spätné lomky)
- Zobrazenie vzorového obsahu konfiguračného súboru priamo v upozornení na stránke Doklady

## [0.4.0] - 2025-12-30

### Pridané
- Modul Doklady - skenovanie účteniek z priečinka a automatická extrakcia údajov pomocou AI (Gemini)
- Automatické overovanie dokladov - párovanie účteniek s jazdami podľa dátumu, litrov a ceny
- Súhrnný panel overenia na stránke Doklady ("X/Y overených, Z neoverených")
- Indikátor chýbajúceho dokladu (⚠) pri jazdách s tankovaním bez spárovanej účtenky
- Legenda nad tabuľkou jázd s počtom pre každý typ indikátora (čiastočné tankovanie, bez dokladu, vysoká spotreba)
- Počet dokladov vyžadujúcich pozornosť vedľa odkazu "Doklady" v navigácii
- Manuálne pridelenie dokladov - modálne okno výberu jazdy pre neoverené doklady
- Hromadné spracovanie čakajúcich dokladov - tlačidlo "Spracovať čakajúce" na stránke Doklady
- E2E testovanie s Playwright
- Podpora lokálneho súboru nastavení (prepísanie predvolených hodnôt)
- Internacionalizácia (i18n) - podpora slovenčiny a angličtiny v celej aplikácii vrátane PDF exportu

### Opravené
- Poradie krokov v release workflow (build až po push)

## [0.3.0] - 2025-12-29

### Zmenené
- Predvolené radenie: najnovšie záznamy hore
- "Prvý záznam" sa radí spolu s ostatnými jazdami pri zoradení podľa dátumu
- Export používa aktuálne nastavenie radenia a obsahuje "Prvý záznam"

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
