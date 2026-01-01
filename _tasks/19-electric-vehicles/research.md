# Electric Vehicle Support - Legislation & Accounting Research

> **Focus:** Slovak legislation and accounting practices for EV/PHEV trip logs.
> For app implementation implications, see [technical-analysis.md](./technical-analysis.md).

---

## 1. Core Problem: No TP Consumption for EVs

### The Fundamental Difference

| Aspect | ICE Vehicle | Electric Vehicle |
|--------|-------------|------------------|
| **TP/OE shows** | Consumption: `l/100km` | Only power: `kW` |
| **Consumption source** | Official (legal baseline) | Must establish yourself |
| **20% margin rule** | ✅ Applies (§19 ods. 2 písm. l) | ❌ **Cannot apply** - no baseline |
| **Tracking unit** | Liters | kWh |

**Official statement from Slovak Financial Administration:**
> "V prípade elektromobilov je pohonnou látkou elektrická energia a v technickom preukaze alebo osvedčení o evidencii elektromobilov sa neuvádza spotreba, ale len výkon."

**Source:** [Finančná správa - Preukazovanie spotreby elektrickej energie](https://podpora.financnasprava.sk/690680-Preukazovanie-spotreby-elektrickej-energie-elektromobilov)

---

## 2. Consumption Tracking Methods for EVs

### Method 1: Own Measurement (Vlastné meranie)

**Requirements:**
1. **Logbook (kniha jázd)** - mandatory
2. **Internal directive (interná smernica)** - documenting:
   - How consumption is measured
   - What methodology is used
   - How kWh/100km is calculated
3. **Separate consumption records** - tracking `kWh/100km`

**Formula:**
```
consumption = (kWh_charged / km_driven) × 100
```

**Source:** [Finančná správa - Spôsob preukazovania spotreby](https://podpora.financnasprava.sk/562298-Sp%C3%B4sob-preukazovania-spotreby-elektrickej-energie-elektromobilov)

### Method 2: Built-in Vehicle Meter

Use the vehicle's onboard computer to read precise kWh consumption.

**Advantage:** No need for internal directive if vehicle meter is used.

### Method 3: Charging Station Receipts Only

For public charging - receipts serve as proof of purchase.
Does not require calculating per-100km consumption.

---

## 3. The 20% Margin Rule - Does It Apply to EVs?

### For ICE Vehicles

§19 ods. 2 písm. l) zákona o dani z príjmov allows:
- Use TP consumption as baseline
- **Increase by 20%** automatically (since July 2020)
- If actual > TP × 1.2, the excess is non-deductible

### For EVs: **The Rule Cannot Apply**

**Reason:** There is no TP consumption value to:
1. Use as baseline
2. Increase by 20%
3. Compare actual consumption against

**NÚR Interpretation (SKDP) states:**
> "Obmedzenia pri elektrických vozidlách súvisia hlavne s uvádzaním priemernej spotreby elektriny v osvedčení o evidencii alebo v technickom preukaze. Z praxe vieme, že sa spotreba v takýchto dokladoch väčšinou neuvádza."

**Source:** [SKDP - NÚR interpretácia 2: Elektromobilita](https://www.skdp.sk/clanky/nur-interpretacia-2-elektromobilita-v-danovych-uctovnych-suvislostiach)

### Alternative Baseline Sources for EVs

If consumption data is needed (e.g., for internal directive):

| Source | Reliability | Notes |
|--------|-------------|-------|
| **WLTP value** | Optimistic (real-world 20-40% higher) | From manufacturer/importer |
| **Manufacturer certificate** | Accurate but hard to obtain | Official for specific VIN |
| **User's own measurement** | Most realistic | First few months of driving |
| **Importer website** | Uncertain legal status | May not qualify as official data |

**Finančná správa guidance:**
> "Daňovník môže preukazovať spotrebu elektrickej energie vlastným meraním (pri ktorom si daňovník musí viesť knihu jázd) – spotreba preukázaná na základe vlastnej internej smernice."

---

## 4. Zostatok Equivalent for EVs

### ICE: Zostatok in Liters

Standard fuel remaining calculation, capped at tank size.

### EV: State of Charge (SoC)

**Electronic logbook systems track:**
> "úroveň nabitia batérie EV v kWh na začiatku a na konci zvoleného obdobia"

Two tracking approaches in practice:
- **Absolute kWh** - energy remaining in battery
- **Percentage SoC** - what users see in the vehicle

**Source:** [Autoreport - Elektronická kniha jázd](https://www.autoreport.sk/resources/EKJ_2026_novela_zakona_DPH.pdf)

---

## 5. Plug-in Hybrid (PHEV) Specifics

### Critical Constraint: One Method Per Vehicle

> "V prípade spotreby dvoch rôznych PHL – elektriny a benzínu, t.j. v prípade plug-in hybridov, napriek dvom rôznym druhom PHL musí počas jedného zdaňovacieho obdobia uplatňovať len jeden spôsob preukazovania daňových výdavkov na spotrebu PHL, nakoľko výber sa vzťahuje na vozidlo, nie na druh PHL."

**Translation:** Despite two fuel types, only ONE documentation method can be used per year. The method applies to the **vehicle**, not the fuel type.

**Source:** [Accace - Výdavky na spotrebu PHM elektromobilov a plug-in hybridov](https://accace.sk/vydavky-na-spotrebu-pohonnych-latok-v-pripade-elektromobilov-a-plug-in-hybridov/)

### PHEV TP Consumption Complexity

PHEV technical documents may show combined consumption but:
> "Combined fuel consumption in registration certificates may not distinguish between: engine-only operation, combined operation, or battery-charging modes."

The NÚR interpretation identifies these distinct modes:
1. Engine consumption (combined mode with electric motor, no charging)
2. Engine consumption (with battery charging via engine)
3. Electric-only consumption

### PHEV Tracking Requirements

| Fuel Type | Must Track | Unit | Has TP Value? | 20% Rule? |
|-----------|------------|------|---------------|-----------|
| Gasoline | Yes | Liters | Usually yes | Yes |
| Electricity | Yes | kWh | Usually no | No |

---

## 6. Charging Session Documentation

### Charging Sources

| Source | Receipt Available | Documentation Method |
|--------|-------------------|---------------------|
| **Public station** | Yes - charging receipt | Direct proof |
| **Home wallbox** | No - electricity bill only | Separate metering required |
| **Workplace** | Depends | Company policy |
| **Free public** | No cost | Still track kWh |

### Home Charging Requirements

Per Slovak guidance:
- Separate electricity meter or dedicated circuit (recommended)
- Authentication controls (chip cards) for multi-vehicle scenarios
- Software logs showing date, time, kWh per session
- Internal policy for business/personal allocation

**Accounting treatment:**
- Invoice to company → Account 502 (Spotreba energie)
- Invoice to employee → Account 548 + reimbursement agreement

**Source:** [DuoFin - Elektromobil vo firme](https://www.duofin.sk/elektromobil-vo-firme-moznosti-uplatnenia-danovych-vydavkov/)

---

## 7. Consumption Calculation

### Universal Formula (same for ICE and EV)

```
consumption_rate = (fuel_or_energy / distance_km) × 100
```

### Typical EV Consumption Ranges

| Driving Condition | Small EV | Large EV |
|-------------------|----------|----------|
| City | 14-16 kWh/100km | 19-21 kWh/100km |
| Highway (90 km/h) | 16-18 kWh/100km | 21-23 kWh/100km |
| Highway (130 km/h) | 22-25 kWh/100km | 27-30 kWh/100km |

**Note:** WLTP values are typically 20-40% lower than real-world consumption.

**Source:** [Portál řidiče - Spotřeba elektromobilu](https://www.portalridice.cz/clanek/spotreba-elektromobilu-na-100-km)

---

## 8. Key Legislative Points Summary

| Aspect | ICE | BEV | PHEV |
|--------|-----|-----|------|
| **TP shows consumption** | ✅ l/100km | ❌ Only kW | ⚠️ Combined only |
| **20% increase allowed** | ✅ Yes | ❌ No baseline | ⚠️ Gasoline only |
| **Logbook required** | If using Method 1 | If using own measurement | Yes (for both fuels) |
| **Internal directive** | Optional | Required for own measurement | Required |
| **Documentation method** | 1 per vehicle/year | 1 per vehicle/year | 1 per vehicle/year (both fuels) |

---

## 9. Sources

### Official Slovak Financial Administration
- [Spôsob preukazovania spotreby elektrickej energie elektromobilov](https://podpora.financnasprava.sk/562298-Sp%C3%B4sob-preukazovania-spotreby-elektrickej-energie-elektromobilov)
- [Preukazovanie spotreby elektrickej energie elektromobilov](https://podpora.financnasprava.sk/690680-Preukazovanie-spotreby-elektrickej-energie-elektromobilov)
- [Použitie údaja o spotrebe PHL podľa cyklu WLTP](https://podpora.financnasprava.sk/675496-Použitie-údaja-o-spotrebe-PHL-podľa-cyklu-WLTP)
- [Spotreba PHL (general)](https://podpora.financnasprava.sk/495981-Spotreba-PHL)

### Slovak Accounting/Tax Interpretation
- [SKDP - NÚR interpretácia 2: Elektromobilita v daňových a účtovných súvislostiach](https://www.skdp.sk/clanky/nur-interpretacia-2-elektromobilita-v-danovych-uctovnych-suvislostiach)
- [Accace - Výdavky na spotrebu PHM elektromobilov a plug-in hybridov](https://accace.sk/vydavky-na-spotrebu-pohonnych-latok-v-pripade-elektromobilov-a-plug-in-hybridov/)
- [DuoFin - Elektromobil vo firme – možnosti uplatnenia daňových výdavkov](https://www.duofin.sk/elektromobil-vo-firme-moznosti-uplatnenia-danovych-vydavkov/)
- [APZD - Ako má podnikateľ preukazovať spotrebu elektrickej energie](https://www.apzd.sk/ako-ma-podnikatel-preukazovat-spotrebu-elektrickej-energie-elektromobilov/)

### Practical Guidance
- [Alpha Finance - Kniha jázd – návod, povinné údaje a bezplatný vzor](https://www.alphafinance.sk/kniha-jazd/)
- [Nabito - Elektromobil a klasika - meranie spotreby](https://nabito.sk/elektromobil-a-klasika-meranie-spotreby/)
- [Portál řidiče - Spotřeba elektromobilu na 100 km](https://www.portalridice.cz/clanek/spotreba-elektromobilu-na-100-km)

### Electronic Logbook Systems
- [Autoreport - Elektronická kniha jázd (PDF)](https://www.autoreport.sk/resources/EKJ_2026_novela_zakona_DPH.pdf)

---

*Research conducted: 2026-01-01*
*Focus: Trip log and consumption tracking legislation for EVs/PHEVs*
