[English](README.en.md) | **Slovensky**

# Kniha Jázd

Desktopová aplikácia na evidenciu jázd služobných vozidiel pre SZČO a malé firmy.
Automaticky počíta spotrebu, sleduje 20% limit nadpotreby a pomáha s daňovou evidenciou.

![Kniha Jázd - Hlavná obrazovka](docs/screenshots/hero.png)

## Funkcie

- **Evidencia jázd** - Záznam dátumu, trasy, km a účelu jazdy
- **Automatický výpočet spotreby** - l/100km sa vypočíta automaticky pri tankovaní
- **Sledovanie zostatku paliva** - Zostatok v nádrži po každej jazde
- **20% limit nadpotreby** - Upozornenie pri prekročení zákonného limitu
- **Návrhy kompenzačných jázd** - Ako sa dostať späť pod limit
- **Pamätanie trás** - Časté trasy sa automaticky dopĺňajú
- **Ročné prehľady** - Každý rok = samostatná kniha jázd
- **Zálohovanie a obnova** - Jednoduchá správa databázy
- **Export** - HTML náhľad s tlačou do PDF (Ctrl+P)
- **Doklady (AI OCR)** - Automatické rozpoznávanie blokov z čerpacích staníc

## Inštalácia

Stiahnite si najnovšiu verziu pre váš systém z [Releases](../../releases):

| Systém | Súbor |
|--------|-------|
| Windows | `Kniha-Jazd_x.x.x_x64-setup.msi` |
| macOS (Apple Silicon) | `Kniha-Jazd_x.x.x_aarch64.dmg` |
| macOS (Intel) | `Kniha-Jazd_x.x.x_x64.dmg` |

## Použitie

### 1. Pridanie vozidla

V nastaveniach pridajte vozidlo so zadaním:
- Názov a ŠPZ
- Objem nádrže (litre)
- Spotreba podľa TP (l/100km)
- Počiatočný stav tachometra

### 2. Záznam jazdy

Pre každú jazdu zadajte:
- Dátum
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

### 5. Doklady (AI rozpoznávanie blokov)

Aplikácia podporuje automatické rozpoznávanie blokov z čerpacích staníc pomocou AI (Gemini).

#### Nastavenie

1. **Získajte Gemini API kľúč:**
   - Navštívte [Google AI Studio](https://aistudio.google.com/apikey)
   - Vytvorte nový API kľúč (bezplatný tier stačí pre bežné použitie)

2. **Vytvorte konfiguračný súbor** `local.settings.json`:

   - Windows: `%APPDATA%\com.notavailable.kniha-jazd\local.settings.json`
   - macOS: `~/Library/Application Support/com.notavailable.kniha-jazd/local.settings.json`

   ```json
   {
     "gemini_api_key": "AIza...",
     "receipts_folder_path": "C:\\Cesta\\K\\Blokom"
   }
   ```

   > **Tip:** Na Windows môžete otvoriť priečinok príkazom `Win+R` → `%APPDATA%\com.notavailable.kniha-jazd`

#### Použitie

1. Uložte fotky blokov do nastaveného priečinka
2. Otvorte sekciu "Doklady" a kliknite na "Sync"
3. AI rozpozná dátum, litre a sumu
4. Priraďte bloky k jazdám

## Pre vývojárov

Pozrite [README.en.md](README.en.md) pre dokumentáciu v angličtine.

### Technológie

- **Frontend:** SvelteKit + TypeScript
- **Backend:** Tauri (Rust)
- **Databáza:** SQLite

### Lokálne spustenie

```bash
npm install
npm run tauri dev
```

### Testy

```bash
cd src-tauri && cargo test
```

## Licencia

[GPL-3.0](LICENSE)

## Prispievanie

Pozrite [CONTRIBUTING.md](CONTRIBUTING.md) (v angličtine).
