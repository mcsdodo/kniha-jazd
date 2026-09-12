[English](README.en.md) | **Slovensky**

[![Tests](https://github.com/mcsdodo/kniha-jazd/actions/workflows/test.yml/badge.svg)](https://github.com/mcsdodo/kniha-jazd/actions/workflows/test.yml)

# Kniha Jázd

Aplikácia na evidenciu jázd služobných vozidiel pre SZČO a malé firmy.
Automaticky počíta spotrebu, sleduje 20% limit nadpotreby a pomáha s daňovou evidenciou.

Beží ako Docker kontajner na vašom serveri (NAS, Raspberry Pi, homelab) a ovládate ju
z prehliadača na ktoromkoľvek zariadení v lokálnej sieti. Desktopová verzia už nie je
udržiavaná — nainštalované kópie zostávajú funkčné, ale nedostanú ďalšie aktualizácie.

![Kniha Jázd - Hlavná obrazovka](docs/screenshots/hero.png)

## Funkcie

- **Evidencia jázd** - Záznam dátumu/času, trasy, km a účelu jazdy
- **Zákonná zhoda (od 1.1.2026)** - Poradové číslo jazdy, meno vodiča, čas ukončenia, km pred jazdou, riadky konca mesiaca
- **Automatický výpočet spotreby** - l/100km sa vypočíta automaticky pri tankovaní
- **Sledovanie zostatku paliva** - Zostatok v nádrži po každej jazde
- **20% limit nadpotreby** - Upozornenie pri prekročení zákonného limitu
- **Kontrola tachometra** - Upozornenie pri zázname, ktorého stav tachometra nesedí so zapísanými kilometrami, a pri jazdách s rovnakým dátumom a časom
- **Návrhy kompenzačných jázd** - Ako sa dostať späť pod limit
- **Návrh tankovania** - Automatický výpočet litrov pre dosiahnutie optimálnej spotreby
- **Pamätanie trás** - Časté trasy sa automaticky dopĺňajú
- **Miesta** - Zoznam všetkých miest z vašich jázd; každému určíte bod na mape a ponuka "Odkiaľ"/"Kam" je spoločná pre všetky vozidlá. Detaily nájdete v [docs/features/place-book.md](docs/features/place-book.md).
- **Mapy trás** - Ku každej jazde vygenerujete trasu po cestách z miesta odchodu do miesta príchodu (alebo okružnú, ak sú rovnaké), vyberiete si z alternatív, doladíte ju potiahnutím čiary a vzdialenosť zapíšete do jazdy. Uložené mapy sa pripoja do tlačového exportu. Detaily nájdete v [docs/features/route-maps.md](docs/features/route-maps.md).
- **Ročné prehľady** - Každý rok = samostatná kniha jázd
- **Skrývateľné stĺpce** - Prispôsobenie tabuľky jázd podľa potreby
- **Zálohovanie a obnova** - Automatická záloha pred migráciou databázy, správa záloh
- **Export** - HTML náhľad s tlačou do PDF (Ctrl+P), rešpektuje skryté stĺpce
- **Doklady (Paperless-ngx)** - Doklady sa preberajú z vášho Paperless-ngx a priraďujú sa k jazdám; Paperless-ngx je jediný zdroj dokladov
- **Home Assistant integrácia** - Zobrazenie ODO a hladiny paliva z HA, odosielanie návrhu tankovania do HA senzora
- **Prístup z prehliadača** - Telefón, tablet aj počítač pristupujú k tej istej inštancii v lokálnej sieti
- **Docker nasadenie** - Jeden kontajner, jeden `/data` zväzok, pre vždy-zapnuté zariadenia (NAS, Raspberry Pi). Detaily nájdete v [docs/features/server-mode.md](docs/features/server-mode.md).

## Inštalácia

Aplikácia sa distribuuje ako Docker image. Žiadne inštalátory sa už nezverejňujú.

```bash
mkdir -p data
docker run -d --name kniha-jazd \
  -p 3456:3456 \
  -v "$PWD/data:/data" \
  --restart unless-stopped \
  ghcr.io/mcsdodo/kniha-jazd-web:latest
```

Aplikácia beží na `http://<ip-servera>:3456`.

Ak si chcete image zostaviť sami zo zdrojov, [docker-compose.web.yml](docker-compose.web.yml)
robi build z [Dockerfile.web](Dockerfile.web):

```bash
docker compose -f docker-compose.web.yml up -d
```

### Verzie image-u

| Tag | Čo obsahuje |
|-----|-------------|
| `:latest` | Posledná vydaná verzia — pre bežné používanie |
| `:vX.Y.Z` | Konkrétne vydanie, nikdy sa nemení |
| `:main` | Aktuálny stav vetvy `main`, automaticky po každom úspešnom teste |
| `:main-<sha>` | Konkrétny commit z vetvy `main`, nikdy sa nemení |

Tag `:main` slúži na vyskúšanie noviniek pred vydaním — obsahuje len zmeny, ktoré prešli
celou testovacou sadou, ale ešte neboli vydané. Ak niečo nefunguje, vráťte sa na
`:latest` alebo na konkrétny `:main-<sha>`.

Aktualizácia = stiahnutie nového tagu a reštart kontajnera. Databáza v `/data` zostáva,
migrácie sa spustia automaticky pri štarte.

## Použitie

### 1. Pridanie vozidla

V nastaveniach pridajte vozidlo so zadaním:
- Názov a ŠPZ
- Objem nádrže (litre)
- Spotreba podľa TP (l/100km)
- Počiatočný stav tachometra

### 2. Záznam jazdy

Pre každú jazdu zadajte:
- Dátum a čas začiatku/konca
- Odkiaľ - Kam
- Počet km (alebo sa vypočíta z ODO)
- Účel jazdy

### 3. Tankovanie

Pri tankovaní zadajte:
- Počet natankovaných litrov
- Cenu (voliteľné)
- Či išlo o plnú nádrž

Aplikácia vypočíta spotrebu automaticky.

### 4. Sledovanie limitu

- Margin pod 20% = v poriadku
- Margin nad 20% = upozornenie + návrhy kompenzačných jázd

### 5. Doklady (Paperless-ngx)

Doklady sa do knihy jázd preberajú z [Paperless-ngx](https://docs.paperless-ngx.com/).
Paperless-ngx je jediný zdroj dokladov: rozpozná text na svojom serveri a aplikácia
si od neho doklady načíta, zobrazí a priradí ich k jazdám.

#### Nastavenie

1. V Paperless-ngx označte doklady značkami `fuel` (tankovanie) a `car` (ostatné
   náklady) a vytvorte doplnkové polia `total_amount` (suma v EUR), `litres` (litre,
   len pri tankovaní) a `receipt_datetime` (dátum a čas dokladu, formát ISO-8601).
2. V aplikácii v časti Nastavenia -> Paperless-ngx zadajte adresu inštancie
   a API token.

   > **Alternatíva:** premenné prostredia `PAPERLESS_URL`, `PAPERLESS_API_TOKEN`
   > a `PAPERLESS_ENABLED` na kontajneri majú prednosť pred uloženým nastavením.

3. V časti Doklady sa načítajú doklady pre zvolené vozidlo a rok.
4. Tlačidlom "Priradiť k jazde" priradíte doklad k jazde.

Tlačidlo "Otvoriť v Paperless" otvorí doklad v novej karte prehliadača.

> **Dôležité pred aktualizáciou:** táto verzia odstraňuje miestne skenovanie dokladov
> a pri aktualizácii zruší tabuľku `receipts`. Miestne doklady sa tým **nenávratne
> zahodia**. Doklady priradené k jazde ostávajú ako odkazy na Paperless, ostatné
> riadky zmiznú.
>
> Ak ich ešte potrebujete, vyexportujte si ich **pred** aktualizáciou:
>
> ```bash
> sqlite3 -header -csv data/kniha-jazd.db "SELECT * FROM receipts;" > receipts.csv
> ```
>
> Aplikácia pred migráciou sama uloží zálohu do
> `<DATA_DIR>/backups/kniha-jazd-backup-*-pre-migration-*.db`. Túto zálohu nikdy
> nemaže, takže sa z nej dá tabuľka prečítať aj neskôr cez `sqlite3`. Obnova zálohy
> ale doklady **nevráti**: nová verzia pri štarte znova spustí migrácie a tabuľku
> znova zruší.

## Často kladené otázky (FAQ)

**Kde sú uložené moje dáta?**
V SQLite databáze na `/data` zväzku kontajnera — pri bežnom nasadení je to priečinok
`./data` na hostiteľovi:
- Databáza: `/data/kniha-jazd.db`
- Zálohy: `/data/backups/`
- Nastavenia: `/data/local.settings.json`

**Zostatok paliva ukazuje zápornú hodnotu?**
Zostatok sa počíta z natankovaných litrov mínus spotreba. Ak je záporný, skontrolujte:
- Či ste zadali správny počet km
- Či ste zaznamenali všetky tankovania

**Doklady z Paperlessu sa nezobrazujú?**
1. Skontrolujte adresu a API token Paperlessu (premenné `PAPERLESS_URL`,
   `PAPERLESS_API_TOKEN`, `PAPERLESS_ENABLED` alebo `local.settings.json`)
2. Overte, že doklady majú značky `fuel` alebo `car` a doplnkové polia
   `total_amount`, `litres`, `receipt_datetime`
3. Skontrolujte stav pripojenia v Nastaveniach -> Paperless-ngx

**Ako preniesť dáta na iný server?**

*Cez priečinok:* zastavte kontajner a skopírujte celý `./data` priečinok. Obsahuje
databázu, zálohy aj nastavenia.

*Cez zálohu:*
1. V nastaveniach vytvorte zálohu
2. Skopírujte súbor `.backup` do `data/backups/` na novom serveri
3. V nastaveniach obnovte zo zálohy

Databázu otvára práve jedna inštancia — nespravujte ten istý `/data` priečinok
z dvoch kontajnerov naraz.

## Súkromie

Všetky dáta zostávajú na vašom serveri. Server nemá autentifikáciu a je určený výlučne pre dôveryhodnú lokálnu sieť (CORS povoľuje len privátne IP rozsahy). Nevystavujte ho do internetu. Aplikácia komunikuje už len s vaším Paperless-ngx a Home Assistantom, teda so službami, ktoré si sami prevádzkujete.

## Pre vývojárov

Pozrite [README.en.md](README.en.md) pre dokumentáciu v angličtine.

### Technológie

- **Frontend:** SvelteKit + TypeScript (statická SPA)
- **Backend:** Rust — `kniha-jazd-core` (logika) + `kniha-jazd-web` (Axum HTTP server)
- **Databáza:** SQLite
- **Nasadenie:** Docker image `ghcr.io/mcsdodo/kniha-jazd-web`

Pre detailnú architektúru pozrite [ARCHITECTURE.md](ARCHITECTURE.md) (v angličtine).

Pre dokumentáciu jednotlivých funkcií pozrite [docs/features/](docs/features/) (v angličtine).

### Lokálne spustenie

#### macOS: Inštalácia Rust

Pred lokálnym spustením na macOS je potrebné nainštalovať Rust:

```bash
# Inštalácia Rust (oficiálna metóda pre macOS)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Po inštalácii reštartujte terminál alebo spustite:
source "$HOME/.cargo/env"

# Overenie:
cargo --version
```

#### Spustenie aplikácie

Dva procesy v dvoch termináloch:

```bash
npm install

# 1) backend na porte 3456 (STATIC_DIR nechajte nenastavený — SPA servuje vite)
cargo run --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web

# 2) frontend na porte 5173, /api proxuje na localhost:3456
npm run dev
```

### Testy

```bash
npm run test:backend      # Rust testy (celý workspace)

# Integračné testy potrebujú zostavenú SPA a debug binárku
npm run build
cargo build --manifest-path src-tauri/Cargo.toml -p kniha-jazd-web
npm run test:integration
```

### Zostavenie

```bash
docker build -f Dockerfile.web -t kniha-jazd-web:local .
```

## Licencia

[GPL-3.0](LICENSE)

## Prispievanie

Pozrite [CONTRIBUTING.md](CONTRIBUTING.md) (v angličtine).
