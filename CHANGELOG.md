# Changelog

Všetky významné zmeny v projekte sú zdokumentované v tomto súbore.

Formát je založený na [Keep a Changelog](https://keepachangelog.com/sk/1.1.0/),
a projekt používa [Semantic Versioning](https://semver.org/lang/cs/).

## [Unreleased]

### Pridané
- **Rozpoznávanie iných nákladov** - skenovanie a priradenie dokladov za umytie auta, parkovanie, diaľničné poplatky, servis a pod.
  - AI automaticky rozpozná či ide o tankovanie (má litre) alebo iný náklad
  - Multi-stage matching: doklad s litrami ktorý nezodpovedá tankovaniu (napr. ostrekovač 2L/5€) sa klasifikuje ako iný náklad
  - Pri priradení k jazde sa automaticky vyplní pole "Iné náklady" s názvom predajcu a popisom
  - Filter dokladov podľa typu (⛽ Tankovanie / 📄 Iné náklady)
  - Vizuálne rozlíšenie dokladov ikonami
  - Ochrana proti kolízii - jazda môže mať len jeden doklad iných nákladov

### Opravené
- **Overovanie dokladov iných nákladov** - doklady bez litrov sa teraz správne párujú s jazdami podľa ceny (`other_costs_eur`)
  - Indikátor v navigácii zobrazuje len nepárované doklady (ADR-008: výpočet v backende)
- **Float→Double typ v Diesel schéme** - oprava chyby kompilácie kde `f64` vyžaduje `Double`, nie `Float`
- **Nekonečná rekurzia v year_start_odometer** - prepísané na iteratívny prístup
- **Chybný počiatočný stav ODO pri prechode na nový rok** - pri zobrazení roku 2026 sa používal statický `initialOdometer` vozidla namiesto posledného ODO z predchádzajúceho roku
  - Príčina: Frontend používal `vehicle.initialOdometer` (hodnota z vytvorenia vozidla) pre všetky roky
  - Oprava: Backend teraz vracia `yearStartOdometer` - posledný ODO z predchádzajúceho roku
  - Pridaná funkcia `get_year_start_odometer()` s rekurzívnym vyhľadávaním v predchádzajúcich rokoch
  - Pridané 3 testy pre prechod ODO medzi rokmi

## [0.12.0] - 2026-01-10

### Opravené
- **Výpočet priemernej spotreby v štatistikách** - priemerná spotreba sa počíta len z uzavretých období plných tankov
  - Predtým: zahŕňala aj jazdy po poslednom tankovaní (skreslený priemer)
  - Teraz: zobrazuje presný priemer zodpovedajúci hodnotám v tabuľke
  - Odchýlka sa zobrazí len ak existujú uzavreté obdobia

### Zmenené
- **Zjednodušený banner prekročenia spotreby** - odstránená funkcionalita pridania kompenzačnej jazdy
  - Zobrazuje len upozornenie a potrebné km na kompenzáciu
  - Používateľ si jazdu pridá manuálne podľa vlastného uváženia
- **Štandardizácia názvov vlastností (snake_case → camelCase)** - zjednotenie pomenovaní na Rust-TypeScript hranici
  - Rust štruktúry používajú `#[serde(rename_all = "camelCase")]` pre JSON serializáciu
  - TypeScript typy a Svelte komponenty používajú camelCase (napr. `licensePlate`, `distanceKm`)
  - Konzistentné s JavaScript/TypeScript konvenciami a existujúcim PreviewResult

## [0.11.0] - 2026-01-09

### Pridané
- **VIN a meno vodiča** - nové voliteľné polia pre vozidlá (PR #2 od @marekadvocate)
  - VIN (Vehicle Identification Number) pre jednoznačnú identifikáciu vozidla
  - Meno vodiča pre dokumentáciu
  - Zobrazenie v hlavičke exportu (HTML/PDF)
  - Inkrementálna migrácia pre existujúce databázy

### Zmenené
- **CI: Paralelné spúšťanie testov** - backend testy a integračné testy bežia súčasne
  - Odstránená závislosť `needs: backend-tests` z integration-tests jobu
  - Úspora ~3 minúty na každom CI behu (build Tauri app začína ihneď)
- **Migrácia na Diesel ORM** - kompletná výmena rusqlite za Diesel pre typovo bezpečné databázové operácie
  - Compile-time kontrola INSERT/UPDATE operácií (zachytí chýbajúce stĺpce pri kompilácii)
  - Row structs pattern pre čistý mapping medzi DB a doménovými modelmi
  - Embedded migrations pre automatickú inicializáciu DB
  - Rozdelenie db.rs a db_tests.rs podľa projektu (pattern ako calculations)
  - Všetkých 132 testov prechádza, existujúce DB súbory zostávajú kompatibilné

### Opravené
- **Strata presnosti desatinných čísel** - oprava chyby kde sa 5.1 zobrazovalo ako 5.099999904632568
  - Príčina: Diesel CLI generoval Float (32-bit) namiesto Double (64-bit) pre SQLite REAL stĺpce
  - Oprava: Zmena Float → Double v schema.rs a f32 → f64 v Row structs

## [0.10.0] - 2026-01-07

### Pridané
- **Komplexná sada integračných testov** (61 testov) - pokrýva kritické používateľské scenáre
  - Tier 1 (39 testov): Jazdy, spotreba, export, BEV/PHEV, prechod medzi rokmi
  - Tier 2 (13 testov): Zálohovanie, doklady, nastavenia, správa vozidiel
  - Tier 3 (9 testov): Kompenzačné jazdy, validácia, viac vozidiel, prázdne stavy
- **Stupňované spúšťanie testov** - Tier 1 pre PR (rýchla spätná väzba), všetky pre main
- **Seedovanie DB cez Tauri IPC** - testy seedujú dáta priamo cez invoke() bez priameho prístupu k DB
- **Filtrovanie dokladov podľa vozidla** - na stránke Doklady sa zobrazujú len nepriradené bločky + bločky aktuálneho vozidla
- Automatický výber vozidla pri štarte aplikácie (ak nie je nastavené, vyberie sa prvé)
- Databázový index na `vehicle_id` pre rýchlejšie vyhľadávanie dokladov

### Opravené
- Automatický výpočet spotreby a zostatku paliva pri použití autocomplete pre trasu (predtým sa prepočítalo len pri manuálnej zmene km)
- Vymazanie vozidla najprv odpojí všetky priradené bločky (predtým zlyhalo kvôli FK constraint)
- Indikátor bločkov v navigácii teraz počíta len bločky pre aktívne vozidlo (predtým počítal všetky)
- Výber vozidla už nezobrazuje prázdnu možnosť (ak existujú vozidlá)
- Overovanie bločkov (`verifyReceipts`) teraz počíta len bločky pre aktívne vozidlo (predtým počítalo všetky)
- Zmena typu vozidla (ICE → BEV/PHEV) teraz funguje správne ak vozidlo nemá žiadne jazdy

## [0.9.0] - 2026-01-07

### Pridané
- **Podpora elektrických vozidiel (BEV) a plug-in hybridov (PHEV)**
  - Nový typ vozidla s výberom: ICE (spaľovacie), BEV (elektrické), PHEV (hybridné)
  - Kapacita batérie, základná spotreba (kWh/100km), počiatočný stav batérie (%)
  - Výpočty energie: spotreba kWh, zostatok batérie v kWh aj percentách
  - Podpora pre nabíjanie: čiastočné/plné nabitie, manuálna korekcia stavu batérie (SoC override)
  - Podmienené stĺpce v tabuľke jázd: palivo pre ICE/PHEV, energia pre BEV/PHEV
  - Export podporuje BEV vozidlá s energetickými štítkami a súhrnmi
  - 26 nových unit testov pre výpočty energie a PHEV (vrátane calculate_energy_grid_data)
  - Integračné testy pre vytváranie BEV vozidiel cez UI
  - **PHEV integrácia výpočtov** - elektrina sa spotrebúva najprv, potom palivo; marža sa počíta len z km na palivo
  - **UI pre SoC override** - rozbaľovací vstup (⚡) v bunke batérie pri úprave existujúcej jazdy
- Testy pre biznis logiku: čiastočné tankovanie, varovania o dátume/spotrebe, zostatok paliva, prenos paliva medzi rokmi (15 nových testov)
- Claude Code hooks: automatické spustenie testov pred commitom, pripomienka na changelog
- Nový skill `/verify` pre kontrolu pred dokončením úlohy
- Nové review skills pre iteratívnu kontrolu kvality:
  - `/plan-review` - kontrola plánov pred implementáciou (úplnosť, realizovateľnosť)
  - `/code-review` - kontrola kódu s automatickým spustením testov
  - `/test-review` - kontrola pokrytia testami s konvergenciou
- Analýza best practices pre iteratívne review workflow (`_tasks/23-iterative-review-analysis/`)

### Zmenené
- Review skills prepracované na dvojfázový workflow: najprv analýza a dokumentácia zistení, potom aplikácia schválených zmien po manuálnom review používateľom

### Opravené
- Autocomplete účelu jazdy teraz funguje naprieč všetkými rokmi (predtým len v aktuálnom roku)
- Priradenie dokladu k jazde používa správne poradie parametrov (oprava chyby energyKwh)
- Priradenie dokladu k jazde štandardne nastaví plnú nádrž (predtým čiastočné tankovanie)
- **Vytvorenie BEV vozidla** - opravená chyba NOT NULL constraint na tank_size_liters (migrácia 006)
- **Predvolená hodnota plnej nádrže** - opravená regresia z false na true pri vytváraní nových jázd

## [0.8.0] - 2026-01-05

### Pridané
- E2E integračné testy pomocou tauri-driver + WebdriverIO
- Automatická CI pipeline pre testovanie (GitHub Actions)
- Podpora pre izolovanú testovaciu databázu cez premennú prostredia `KNIHA_JAZD_DATA_DIR`

### Opravené
- Zostatok paliva sa správne prenáša medzi rokmi (predtým sa každý rok začínal s plnou nádržou)

## [0.7.0] - 2026-01-01

### Pridané
- Tlačidlo na obnovenie optimálnej veľkosti okna (zobrazí sa len ak okno nemá predvolenú veľkosť)
- Živý náhľad spotreby pri úprave jázd - zostatok paliva a spotreba sa aktualizujú pri každom stlačení klávesy

### Zmenené
- Živý náhľad spotreby: percento marže sa zobrazuje vždy - zelená farba pri dodržaní limitu, červená pri prekročení 20%

### Opravené
- Živý náhľad spotreby: náhľad sa zobrazí hneď pri začatí úpravy riadku (nie len po zmene hodnoty)
- Živý náhľad spotreby: správne umiestnenie náhľadovej jazdy v chronologickom poradí pre výpočet spotreby
- Zjednotená výška vstupných polí v riadku úprav (tabuľka jázd)
- Pridané medzery medzi vstupnými poľami v riadku úprav
- Km a ODO zobrazené ako celé čísla (bez desatinných miest)

## [0.6.0] - 2025-12-30

### Pridané
- Filtrovanie dokladov podľa roku - podpora pre ročnú štruktúru priečinkov (2024/, 2025/)
- Automatická detekcia štruktúry priečinka s bločkami (plochá vs. ročná)
- Upozornenie pri neplatnej štruktúre priečinka (mix súborov a priečinkov)
- Indikátor nezhody dátumu - keď dátum z OCR nezodpovedá priečinku
- Rozdelenie synchronizácie dokladov na dve tlačidlá: "Skenovať priečinok" a "Rozpoznať dáta"
- Zobrazenie počtu čakajúcich dokladov na tlačidle OCR
- Priebežný ukazovateľ spracovania OCR (X/Y)

### Zmenené
- Zjednodušenie príkazov (commands) - príkazy teraz len odkazujú na skills

### Opravené
- Preklad tlačidla "Sync" na "Načítať" na stránke Doklady
- Aktualizácia počtu dokladov v navigácii pri zmene dát (skenovanie, OCR, úprava jázd)
- Prepínanie roku a filtrov na stránke Doklady

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
