# Changelog

Všetky významné zmeny v projekte sú zdokumentované v tomto súbore.

Formát je založený na [Keep a Changelog](https://keepachangelog.com/sk/1.1.0/),
a projekt používa [Semantic Versioning](https://semver.org/lang/cs/).

## [Unreleased]

### Pridané
- **Zákonná zhoda pre knihu jázd (od 1.1.2026)** - nové stĺpce podľa zákona:
  - **Poradové číslo (§4a)** - automatické číslovanie jázd (1, 2, 3...) v chronologickom poradí
  - **Čas ukončenia (§4c)** - nový vstup pre čas skončenia jazdy (vedľa času začiatku)
  - **Meno vodiča (§4b)** - zobrazuje sa z nastavení vozidla v každom riadku
  - **Km pred jazdou (§4f)** - automaticky odvodený z predchádzajúcej jazdy
  - Všetky nové stĺpce sú voliteľne skrývateľné cez ikonu oka
  - Zahrnuté v HTML exporte

### Opravené
- **Export "Prvý záznam"** - zostatok paliva zobrazuje správnu hodnotu (prenášanú z predchádzajúceho roka alebo plnú nádrž) namiesto 0

## [0.27.2] - 2026-01-28

### Zmenené
- **HTML export** - exportuje len viditeľné stĺpce (skryté stĺpce cez ikonu oka sa nezobrazia v exporte)

## [0.27.1] - 2026-01-28

### Opravené
- **HTML export** - natankované palivo sa zobrazuje s 2 desatinnými miestami (rovnako ako v aplikácii)

## [0.27.0] - 2026-01-28

### Pridané
- **Agregovaný changelog pri aktualizácii** - pri aktualizácii cez viacero verzií sa zobrazí kompletný zoznam zmien
  - Automatické stiahnutie CHANGELOG.md z GitHub
  - Zobrazenie všetkých verzií medzi aktuálnou a cieľovou
  - Formátovaný markdown (nadpisy, zoznamy, tučné písmo)
  - Kliknuteľné verzie odkazujú na GitHub releases (otvára sa v prehliadači)

## [0.26.1] - 2026-01-27

### Zmenené
- **Prepínač dátumu** - zjednotený vzhľad s tlačidlom "Nový záznam" (rovnaké farby a štýl)

## [0.26.0] - 2026-01-27

### Pridané
- **Home Assistant integrácia** - zobrazenie reálneho stavu ODO z Home Assistant v hlavičke
  - Nastavenie HA servera a API tokenu v Nastaveniach → Home Assistant
  - Indikátor stavu pripojenia (Pripojené/Nepripojené)
  - Pre každé vozidlo možnosť nastaviť entity ID senzora odometra
  - Zobrazenie aktuálneho ODO z HA v štatistikách vozidla
  - Delta od poslednej zaznamenanej jazdy (+X km)
  - Varovanie ak delta >= 50 km (potenciálne zabudnuté jazdy)
  - Zobrazenie reálneho ODO aj v zozname vozidiel v Nastaveniach
  - Cache s automatickou obnovou každých 5 minút

### Zmenené
- **Vylepšený vzhľad hlavičky vozidla** - jednotný štýl "názov hore, hodnota dole"
  - Statické info vozidla (názov, ŠPZ, nádrž) ako prvý riadok
  - Dynamické štatistiky (km, PHM, spotreba) ako druhý riadok
  - Reálne ODO z Home Assistant zahrnuté v štatistikách

### Opravené
- **Priraďovanie dokladov** - tankovací doklad priradený k jazde bez PHM teraz správne vyplní polia paliva (nie "iné náklady")

## [0.25.0] - 2026-01-27

### Pridané
- **Čas odchodu v záznamoch jázd** - nový stĺpec "Čas" vedľa dátumu
  - Voliteľný čas odchodu vo formáte HH:MM
  - Predvolená hodnota 00:00 ak nie je zadaný
  - Zahrnutý v HTML exporte
- **Skrývateľné stĺpce v tabuľke jázd** - možnosť skryť nepotrebné stĺpce
  - Ikona oka v hlavičke tabuľky (vedľa prepínača dátumu)
  - Skrytie/zobrazenie: Čas, Spotrebované (l), Zostatok (l), Iné (€), Iné poznámka
  - Nastavenie sa ukladá a zachováva po reštarte
  - Export vždy obsahuje všetky stĺpce bez ohľadu na viditeľnosť

### Opravené
- **Šírky stĺpcov pri skrytí** - stĺpce si zachovávajú správne šírky aj keď sú niektoré skryté

## [0.24.0] - 2026-01-26

### Pridané
- **Prepínač predvyplnenia dátumu pre nové záznamy** - vedľa tlačidla "Nový záznam" v tabuľke jázd
  - "+1 deň" mód: predvyplní dátum poslednej jazdy + 1 deň (pre dávkové zadávanie)
  - "Dnes" mód: predvyplní dnešný dátum (pre denné zadávanie)
  - Nastavenie sa ukladá a zachováva aj po reštarte aplikácie

## [0.23.0] - 2026-01-26

### Pridané
- **Návrh tankovania v legende tabuľky jázd** - zobrazuje odporúčané tankovanie keď existuje otvorené obdobie
  - Zobrazuje litre a výslednú spotrebu: "Návrh tankovania: 38 L → 5.78 l/100km"
  - Zelená farba signalizuje pozitívnu akciu
  - Magic fill tlačidlo používa predpočítané hodnoty (bez volania backendu)
  - Hodnoty sa prepočítavajú pri každom načítaní dát

## [0.22.0] - 2026-01-26

### Pridané
- **Nový stĺpec "Spotr. (L)" v tabuľke jázd** - zobrazuje spotrebu paliva pre každú jazdu v litroch
  - Výpočet: km × spotreba / 100
  - Používa spotrebu z uzavretého obdobia tankovania alebo TP hodnotu ak obdobie nie je ešte uzavreté
  - Živý náhľad pri editácii jazdy (s ~ prefixom)
  - Zahrnutý aj v HTML exporte

## [0.21.1] - 2026-01-25

### Opravené
- **Magic fill pri editácii jazdy v strede obdobia** - oprava výpočtu litrov keď sa edituje jazda, ktorá nie je posledná v otvorenom období tankovania

## [0.21.0] - 2026-01-25

### Pridané
- **Tlačidlo "Magic fill" pre automatické doplnenie PHM** - nové tlačidlo s ikonou čarovnej paličky pri editácii jazdy
  - Automaticky vypočíta litre paliva pre dosiahnutie 105-120% spotreby podľa TP
  - Zohľadňuje celé obdobie od posledného plného tankovania
  - Nastaví príznak "Plná" pre správny výpočet spotreby
- **Ikony namiesto textových tlačidiel** - modernejší vzhľad pri editácii jazdy
  - Uložiť: ikona fajky (zelená pri hover)
  - Zrušiť: ikona X (oranžová pri hover)

## [0.20.0] - 2026-01-24

### Pridané
- **Automatická záloha pred aktualizáciou** - databáza sa automaticky zálohuje pred stiahnutím novej verzie
  - Záloha je vytvorená s označením verzie (napr. "pred v0.20.0")
  - Pri zlyhaní zálohy sa zobrazí dialóg s možnosťou pokračovať alebo zrušiť
  - V zozname záloh sa zobrazuje štítok "pred vX.X.X" pre automatické zálohy
- **Nastavenia čistenia záloh** - automatické mazanie starých záloh pred aktualizáciou
  - Nové nastavenie v časti Zálohy: "Ponechať iba posledných X automatických záloh"
  - Náhľad koľko záloh sa vymaže a koľko miesta sa uvoľní
  - Tlačidlo "Vyčistiť teraz" pre okamžité vymazanie
  - Manuálne zálohy nie sú nikdy automaticky vymazané

### Opravené
- **Priradenie dokladu s inou cenou k jazde** - oprava chyby kedy priradenie dokladu (napr. diaľničná známka) prepísalo uložené other_costs späť na null
- **Zálohy pri vlastnom umiestnení databázy** - zálohy sa teraz správne ukladajú vedľa databázy aj pri použití vlastnej cesty (napr. Google Drive)

## [0.19.1] - 2026-01-22

### Opravené
- **Zobrazenie istoty OCR pre staršie doklady** - oprava "Neznáma istota" pre doklady z roku 2025 a skôr
  - Migrácia existujúcich dokladov na správny formát JSON
  - Doklady teraz správne zobrazujú pôvodnú istotu (Vysoká/Stredná/Nízka)

## [0.19.0] - 2026-01-21

### Pridané
- **Podpora viacerých mien pre doklady** - rozpoznávanie dokladov v EUR, CZK, HUF, PLN
  - Automatické rozpoznanie meny cez Gemini OCR (€, Kč, Ft, zł)
  - EUR doklady: automaticky vyplnená suma v EUR
  - Cudzie meny: doklad vyžaduje manuálnu konverziu na EUR (stav "Na kontrolu")
  - Zobrazenie pôvodnej sumy s konverziou: "100 CZK → 3,95 €"
- **Editovanie dokladov** - nový modál pre úpravu údajov dokladu
  - Úprava dátumu, litrov, pôvodnej sumy a meny
  - Zadanie sumy v EUR pre cudzie meny
  - Úprava názvu čerpacej stanice / predajcu

## [0.18.0] - 2026-01-21

### Pridané
- **Priradenie dokladu k jazde s existujúcimi údajmi o tankovaní** - doklad je možné priradiť ako dokumentáciu
  - Ak doklad obsahuje zhodné údaje (dátum, litre ±0.01, cena ±0.01), je pridelenie povolené
  - Zhodné jazdy sú zvýraznené zelenou farbou s ikonou ✓ "zodpovedá dokladu"
  - Jazdy s rozdielnymi údajmi zostávajú zablokované
- **Zobrazenie dôvodu neoverenia dokladu** - pri neoverených dokladoch sa zobrazuje konkrétny dôvod
  - "Dátum 20.1. – jazda je 19.1." pri nezhode dátumu
  - "63.68 L – jazda má 50.0 L" pri nezhode litrov
  - "91.32 € – jazda má 85.0 €" pri nezhode ceny
  - "Chýbajú údaje na doklade" ak OCR nerozpoznalo údaje
  - "Žiadna jazda s tankovaním" ak neexistuje zodpovedajúca jazda

### Zmenené
- **Zobrazenie dôvodu nezhody pri priraďovaní dokladu** - prehľadnejšia informácia prečo nie je možné priradiť
  - Namiesto "už má: X L" sa zobrazuje konkrétny rozdiel: "iný dátum", "iné litre", "iná cena"
  - Kombinované dôvody pre viacero rozdielov (napr. "iný dátum a cena")

### Opravené
- **Prepočet kilometrov pri vkladaní jazdy medzi existujúce** - oprava nesprávneho výpočtu ODO
  - Pri vložení jazdy "medzi" existujúce sa ODO prepočíta správne
  - Opravené aj pri presúvaní jázd (šípky hore/dole)
- **Kontrola prekročenia limitu podľa jednotlivých období** - oprava logiky varovania
  - Varovanie sa zobrazí ak KTORÉKOĽVEK obdobie tankovania prekročí 120% TP
  - Predtým sa kontroloval len celkový priemer (mohol byť OK aj keď jedno obdobie bolo prekročené)
  - Zobrazená odchýlka teraz ukazuje najhoršie obdobie, nie priemer

## [0.17.3] - 2026-01-17

### Pridané
- **Skript pre testovacie releasy** (`scripts/test-release.ps1`) - automatizácia lokálneho testovania aktualizácií
  - Dočasne zvýši verziu (napr. 0.17.2 → 0.18.0), zostaví release, skopíruje do `_test-releases/`
  - Po zostavení automaticky vráti verziu späť
  - Aktualizuje `latest.json` pre mock update server
  - Podpora parametra `-BumpType` (minor/patch)

### Zmenené
- **Zjednodušené UI pre aktualizácie** - prehľadnejšie ovládanie v Nastaveniach
  - Kliknuteľný prechod verzií (0.17.2 → 0.18.0) otvorí modál aktualizácie
  - Tlačidlo mení text: "Skontrolovať aktualizácie" / "Aktualizovať" podľa stavu
  - Odstránené duplicitné tlačidlo "Aktualizovať" z riadku verzie

## [0.17.2] - 2026-01-17

### Zmenené
- **Automatické ukladanie nastavení** - všetky nastavenia sa ukladajú automaticky pri zmene
  - Odstránené tlačidlá "Uložiť" zo sekcií Firemné údaje a Skenovanie dokladov
  - Ukladanie s oneskorením 800ms počas písania (debounce)
  - Okamžité uloženie pri opustení poľa (blur)
  - Toast notifikácia po úspešnom uložení
- **Vylepšený modál pre presun databázy** - dizajn zodpovedajúci modálu aktualizácií
  - Žltý varovný box s ikonou pre upozornenie na reštart
  - Indikátor priebehu počas presunu
  - Cesta v monospace fonte v štylizovanom boxe
- **Prepracované zobrazenie verzie** - jednotný dizajn s ostatnými nastaveniami
  - Stavové ikony: ✓ (aktuálna), ! (chyba), ⟳ (kontrolujem)
  - Pri dostupnej aktualizácii zobrazenie prechodu verzií (0.17.2 → 0.17.3)
  - Odkaz "Zobraziť zmeny" otvára CHANGELOG na GitHub
  - Tlačidlo "Aktualizovať" priamo v riadku keď je dostupná aktualizácia
  - Tooltip texty pre prístupnosť

## [0.17.1] - 2026-01-17

### Pridané
- **Editovateľné nastavenia dokladov** - konfigurácia priamo v UI namiesto manuálnej editácie JSON
  - Nová sekcia "Skenovanie dokladov" v Nastaveniach
  - Vstup pre Gemini API kľúč s možnosťou zobraziť/skryť (password toggle)
  - Výber priečinka s dokladmi cez systémový dialóg
  - Tlačidlo uložiť s toast notifikáciou
  - Podpora URL kotvy pre priame navigovanie (#receipt-scanning)
- **Zjednodušené upozornenie na Dokladoch** - prehľadnejšie keď nie je nakonfigurované
  - Varovná ikona s titulkom
  - Zoznam požiadaviek (API kľúč, priečinok)
  - Tlačidlo "Prejsť do nastavení" naviguje priamo na správnu sekciu
- **Sekcia "Umiestnenie databázy" v Nastaveniach** - zobrazenie a správa cesty k databáze
  - Zobrazenie aktuálnej cesty s označením "Vlastná"/"Predvolená"
  - Tlačidlo pre otvorenie priečinka v systémovom správcovi súborov
  - Informácia o možnosti zdieľania cez Google Drive/NAS
- **Presun databázy na vlastnú cestu** - kompletná funkcionalita pre multi-PC použitie
  - Tlačidlo "Zmeniť umiestnenie..." s výberom priečinka
  - Potvrdzovacie okno pred presunom s upozornením na reštart
  - Presun databázy aj priečinka so zálohami
  - Tlačidlo "Obnoviť predvolené" pre návrat do štandardného umiestnenia
  - Automatický reštart aplikácie po presune
- **Banner pre režim len na čítanie** - upozornenie keď databáza obsahuje novšie migrácie
  - Žltý banner pod hlavičkou s ikonou a textom
  - Tlačidlo "Skontrolovať aktualizácie" pre rýchly prístup k aktualizácii

### Zmenené
- **Zjednotený vzhľad nastavení priečinkov a API kľúča** - konzistentný dizajn v sekcii Skenovanie dokladov
  - Priečinok s dokladmi používa rovnaký štýl ako umiestnenie databázy (monospace font)
  - Nahradenie tlačidiel "Vybrať"/"Predvolená cesta" klikateľnými odkazmi "Zmeniť"
  - Ikona oka priamo vo vstupe pre API kľúč (namiesto tlačidla Zobraziť/Skryť)
  - Monospace font pre API kľúč
  - Nové tlačidlo "Zobraziť v Prieskumníkovi" v hlavičke nastavení
  - Odstránené jednotlivé tlačidlá "Otvoriť priečinok" zo sekcií

### Opravené
- **Oprava type mismatch v API typoch** - frontend typy teraz správne používajú camelCase
  - `AppModeInfo.isReadOnly` namiesto `is_read_only`
  - `DbLocationInfo.dbPath` namiesto `db_path`
- **Aktivácia read-only ochrany** - makro `check_read_only!` teraz skutočne použité
  - Pridané do 19 zápisových príkazov (vehicles, trips, settings, backups, receipts)

### Interné
- **Vlastná cesta k databáze (Phase 1)** - backend základ pre multi-PC podporu
  - Nový modul `db_location.rs` s `DbPaths` a lock file mechanizmom
  - `LocalSettings` rozšírený o `custom_db_path` a `save()` metódu
  - Migračná kompatibilita - detekcia neznámych migrácií z novších verzií
  - Závislosť `hostname` pre identifikáciu PC v lock súboroch
- **Správa stavu aplikácie (Phase 2)** - infraštruktúra pre read-only režim
  - Nový modul `app_state.rs` s `AppMode` a `AppState`
  - Makro `check_read_only!` pre ochranu zápisových operácií
- **Príkazy pre databázu (Phase 3)** - nové Tauri commands
  - `get_db_location` - informácie o umiestnení databázy
  - `get_app_mode` - informácie o režime aplikácie
  - `check_target_has_db` - kontrola či cieľový priečinok obsahuje databázu
  - `move_database` - presun databázy na novú cestu
  - `reset_database_location` - návrat do predvoleného umiestnenia
- **Startup flow s podporou vlastnej cesty (Phase 4)** - integrácia do štartu aplikácie
  - Načítanie `LocalSettings` pre zistenie vlastnej cesty
  - Kontrola lock súboru pri štarte (varovanie ak je zamknutá inde)
  - Kontrola migračnej kompatibility (read-only ak neznáme migrácie)
  - Uvoľnenie zámku pri ukončení aplikácie
  - Background heartbeat vlákno - `refresh_lock` každých 30 sekúnd
- **Frontend pre vlastnú cestu (Phase 5)** - UI komponenty a state management
  - Store `appModeStore` pre sledovanie read-only stavu
  - API funkcie `getDbLocation`, `getAppMode`, `checkTargetHasDb`, `moveDatabase`, `resetDatabaseLocation`
  - i18n preklady (SK + EN) pre všetky nové texty
- **Dialog plugin** - pridaný `tauri-plugin-dialog` pre výber priečinkov
- **Integračné testy** - nové testy pre nastavenia dokladov a umiestnenie databázy
  - `receipt-settings.spec.ts` - testy UI a IPC príkazov
- **Dokumentácia (Phase 6)** - aktualizácia CLAUDE.md a tech debt
  - Sekcia "Database Migration Best Practices" s príkladmi
  - Tech debt item pre verziovanie záloh

## [0.17.0] - 2026-01-16

### Pridané
- **Zobraziť zálohu v prieskumníkovi** - nové tlačidlo pri každej zálohe
  - Otvorí priečinok so zálohou a zvýrazní súbor
  - Text tlačidla sa prispôsobí operačnému systému (Windows/macOS/Linux)

## [0.16.1] - 2026-01-15

### Opravené
- **Enter klávesa v editácii jazdy** - opravená race condition pri odoslaní formulára
  - Dropdown autocomplete sa zatváral s 200ms oneskorením po strate fokusu
  - Enter bol ignorovaný ak dropdown ešte existoval v DOM
  - Teraz sa kontroluje aj či má autocomplete input focus

### Interné
- **CI integračné testy** - opravené zlyhávanie buildu kvôli chýbajúcemu podpisovaciemu kľúču
- **Date input v integračných testoch** - opravené nastavovanie dátumu cez WebDriverIO

## [0.16.0] - 2026-01-15

### Pridané
- **Normalizácia lokácií** - automatické čistenie medzier pri ukladaní
  - Odstránenie úvodných a koncových medzier
  - Nahradenie viacerých medzier jednou medzerou
  - Prevencia duplicít ako "Bratislava" vs "Bratislava " (koncová medzera)
  - Aplikované na trasy aj jazdy pri vytváraní a úprave
- **Automatické aktualizácie** - aplikácia kontroluje dostupnosť novej verzie
  - Kontrola pri štarte aplikácie (na pozadí, neblokujúce)
  - Manuálne tlačidlo "Skontrolovať aktualizácie" v Nastaveniach
  - Modálne okno s verziou a poznámkami k vydaniu
  - Tlačidlo "Aktualizovať" stiahne, nainštaluje a reštartuje aplikáciu
  - Tlačidlo "Neskôr" odloží pripomienku do ďalšieho štartu
  - Modrá bodka indikátora pri Nastaveniach ak je aktualizácia dostupná
  - Podpísané aktualizácie pre bezpečnosť (Ed25519)
  - GitHub Releases ako distribučný kanál
  - Checkbox "Automaticky kontrolovať pri štarte" v Nastaveniach
  - Odloženie ("Neskôr") prežije reštart aplikácie
  - Manuálna kontrola vždy zobrazí modál aj po odložení
- **Zobrazenie verzie v Nastaveniach** - sekcia "Aktualizácie" zobrazuje aktuálnu verziu
- **Automatická extrakcia poznámok k vydaniu** - CI workflow extrahuje poznámky z CHANGELOG.md
- **Lokálny testovací server** - `_test-releases/` pre testovanie aktualizácií bez GitHub Releases

### Opravené
- **Klávesové skratky v editácii jazdy** - Enter/Escape teraz fungujú správne vo všetkých prípadoch
  - Enter odošle formulár aj keď je otvorený autocomplete dropdown
  - Enter funguje aj bez focusu na žiadnom poli
  - Zjednodušený handler - jeden globálny listener namiesto viacerých
  - Autocomplete dropdown už neblokuje odoslanie formulára

### Testy
- **Integračné testy pre autocomplete trás** - nový súbor `route-autocomplete.spec.ts`
  - Test automatického vyplnenia KM z naučených trás
  - Test zachovania užívateľom zadanej vzdialenosti
  - Test Enter pre odoslanie (bez focusu)
  - Test Enter s otvoreným autocomplete dropdownom
  - Test Escape pre zrušenie editácie

### Zmenené
- **Synchronizácia verzií** - Cargo.toml aktualizovaný na 0.15.0 (zosúladenie s package.json a tauri.conf.json)

## [0.15.0] - 2026-01-13

### Pridané
- **Oddelenie dev a prod databázy** - vývojová verzia používa samostatnú databázu
  - Nový konfiguračný súbor `tauri.conf.dev.json` s identifikátorom `com.notavailable.kniha-jazd.dev`
  - Príkaz `npm run tauri:dev` spúšťa aplikáciu s odlišným dátovým priečinkom
  - Ochrana produkčných dát pred poškodenením počas vývoja
  - Názov okna "[DEV]" pre jednoznačné rozlíšenie verzií
- **Klávesové skratky pre formulár jázd** - rýchlejšia práca s formulárom
  - ESC zruší úpravu/pridávanie jazdy
  - Enter uloží formulár
  - Globálny handler funguje bez ohľadu na pozíciu kurzora
- **Obojsmerný prepočet KM ↔ ODO** - úprava jedného poľa automaticky aktualizuje druhé
  - Zmena KM prepočíta ODO (existujúce správanie)
  - Zmena ODO teraz prepočíta KM (nové)
  - Delta prístup: zmena ODO o X = zmena KM o X

### Opravené
- **Navigácia Tab v autocomplete** - jeden Tab presunie na ďalšie pole
  - Dropdown návrhy už nezachytávajú focus pri tabovaní
  - ESC zatvorí dropdown a zároveň zruší úpravu (jeden stisk)
- **Chyba pri prvej úprave ODO** - prvá úprava ODO nesprávne prepočítala KM
  - Opravené použitím delta prístupu namiesto absolútneho výpočtu

### Testy
- **Integračné testy pre KM ↔ ODO** - nový súbor `km-odo-bidirectional.spec.ts`
  - Test prepočtu KM pri zmene ODO
  - Test viacnásobných úprav ODO
  - Test prepočtu ODO pri zmene KM

- **Oprava BEV/PHEV integračných testov** - testy teraz používajú správne konvencie
  - Použitie camelCase názvov vlastností (`energyRates` namiesto `energy_rates`) podľa task 30
  - Oprava očakávanej spotreby BEV (12 namiesto 18 kWh/100km - vzdialenosť 150km, nie 100km)
  - Oprava PHEV null asercií (`toBeNull` namiesto `toBeUndefined` - Rust Option::None)
  - Oprava PHEV margin testu (10L paliva pre dosiahnutie spotreby >1.92 l/100km)

## [0.14.0] - 2026-01-13

### Pridané
- **Podpora EV v exporte** - HTML export podporuje všetky typy vozidiel
  - ICE: palivo (litre, cena, zostatok, spotreba l/100km)
  - BEV: energia (kWh, cena, zostatok batérie, spotreba kWh/100km)
  - PHEV: kombinované stĺpce pre palivo aj energiu
  - Hlavička exportu zobrazuje správne parametre vozidla podľa typu
- **Prechod batérie medzi rokmi (BEV/PHEV)** - stav batérie sa prenáša medzi rokmi
  - Nová funkcia `get_year_start_battery_remaining()` analogická k `get_year_start_fuel_remaining()`
  - Rekurzívny výpočet stavu batérie z predchádzajúceho roka
  - Ak neexistujú dáta z minulého roka, použije sa `initial_battery_percent × capacity`

### Opravené
- **Varovania kompilátora EV kódu** - zníženie z 8 na 1 varovanie
  - Pridané `#[allow(dead_code)]` pre pomocné funkcie určené na budúce použitie
  - Odstránené varovania pre `uses_fuel()` a `uses_electricity()` (teraz použité v exporte)
  - Zostáva: `calculate_buffer_km` (out of scope, riešené v task 37)

### Testy
- **Aktivované BEV/PHEV integračné testy** - odstránené `.skip` z testov v `bev-trips.spec.ts` a `phev-trips.spec.ts`
  - Backend bol opravený v predchádzajúcej verzii (db.rs obsahuje energy polia)
  - Odstránené zastaralé TODO komentáre

### Odstránené
- **Čistenie mŕtveho kódu** - odstránenie nepoužívaného kódu a oprava varovaní kompilátora (17→1 varovanie)
  - Odstránený kód funkcie "auto-suggest compensation trip" (zjednodušená vo v0.12.0)
    - `CompensationSuggestion` struct, `generate_target_margin()`, `find_matching_route()`, `build_compensation_suggestion()`
    - Tauri príkaz `get_compensation_suggestion`
    - Frontend funkcia `getCompensationSuggestion()` a TypeScript typ
  - Odstránené nepoužívané Route CRUD operácie z `db.rs`
    - `create_route()`, `get_route()`, `update_route()`, `delete_route()`, `populate_routes_from_trips()`
    - Ponechané: `get_routes_for_vehicle()`, `find_or_create_route()` (aktívne používané)
  - Odstránený súbor `error.rs` (AppError enum nikdy nepoužitý)
  - Odstránená funkcia `is_dummy_trip()` z `export.rs`
  - Odstránená metóda `Receipt::is_assigned()` z `models.rs`

## [0.13.1] - 2026-01-13

### Opravené
- **Tmavý režim - kompletné tlmené štýlovanie** - oprava všetkých svetlých prvkov v tmavom režime
  - Odznaky typu vozidla (ICE/BEV/PHEV) - tlmené pozadia namiesto svetlých
  - Všetky tlačidlá (Skenovať, Rozpoznať, Export, Uložiť, Odstrániť, Pridať vozidlo)
  - Aktívne stavy filtrov a prepínačov
  - Pravidlo: tmavé tlmené pozadie + jasný farebný text (nie biely text na jasnom pozadí)
- **Syntax a štýl** - opravy varovaní kompilátora
  - Explicitná životnosť v `db.rs` (`MutexGuard<'_, SqliteConnection>`)
  - Klávesová navigácia pre modálne okno vozidla (Escape na zatvorenie)
  - Odstránený prázdny CSS `.trip-section {}`
  - Prístupnosť prepínača témy - použitý `<fieldset>` + `<legend>`

### Dokumentácia
- **Analýza mŕtveho kódu** - zdokumentované 17 varovaní kompilátora v `_tasks/_TECH_DEBT/03-dead-code-and-warnings.md`
  - EV scaffolding (ponechať pre task 19)
  - Route CRUD pre plánovanú funkciu BIZ-005
  - Skutočne mŕtvy kód na odstránenie (AppError, buffer_km, atď.)

## [0.13.0] - 2026-01-12

### Pridané
- **Tmavý režim (Dark Theme)** - podpora svetlej a tmavej témy s automatickou detekciou systémových preferencií
  - Prepínač v Nastavenia → Vzhľad (Podľa systému / Svetlá / Tmavá)
  - Automatické prepínanie pri zmene systémových preferencií
  - Trvalé uloženie preferencie v `local.settings.json`
  - CSS premenné pre konzistentné farby vo všetkých komponentoch
  - Kompletná migrácia všetkých stránok a komponentov na CSS premenné
- **Rozpoznávanie iných nákladov** - skenovanie a priradenie dokladov za umytie auta, parkovanie, diaľničné poplatky, servis a pod.
  - AI automaticky rozpozná či ide o tankovanie (má litre) alebo iný náklad
  - Multi-stage matching: doklad s litrami ktorý nezodpovedá tankovaniu (napr. ostrekovač 2L/5€) sa klasifikuje ako iný náklad
  - Pri priradení k jazde sa automaticky vyplní pole "Iné náklady" s názvom predajcu a popisom
  - Filter dokladov podľa typu (⛽ Tankovanie / 📄 Iné náklady)
  - Vizuálne rozlíšenie dokladov ikonami
  - Ochrana proti kolízii - jazda môže mať len jeden doklad iných nákladov

### Opravené
- **Tmavý režim - čierny text** - oprava čierneho textu v tmavom režime pre tabuľky, formulárové prvky a tlačidlá filtrov
- **Triedenie v stĺpci Akcie** - odstránené nechcené triedenie pri kliknutí na hlavičku stĺpca Akcie
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
