$ErrorActionPreference = "Stop"
$inv = [System.Globalization.CultureInfo]::InvariantCulture

# Coordinates in OSRM order: lon,lat; Index 0 = Home; 1..22 = cities/towns; 23..66 = villages (pop >= 500).
$coords = @(
    "20.5533207,48.9350604",  # 0  Home (Kamenny obrazok 26)
    "21.1972706,49.0406385",  # 1  Velky Saris
    "20.4308701,49.1362088",  # 2  Kezmarok
    "20.5887098,49.0250870",  # 3  Levoca
    "20.8754977,48.9144118",  # 4  Krompachy
    "20.1965291,49.0582357",  # 5  Svit
    "21.0973337,49.1026740",  # 6  Sabinov
    "20.5328607,48.6620675",  # 7  Roznava
    "20.5629640,48.9435344",  # 8  Spisska Nova Ves
    "20.1166667,48.6833333",  # 9  Revuca
    "20.6889231,49.3001533",  # 10 Stara Lubovna
    "20.7974076,48.9466482",  # 11 Spisske Vlachy
    "20.3649119,48.8211362",  # 12 Dobsina
    "20.4544719,49.1893417",  # 13 Spisska Bela
    "20.5337909,49.2571147",  # 14 Podolinec
    "20.2410264,48.6274840",  # 15 Jelsava
    "20.7510716,49.0000350",  # 16 Spisske Podhradie
    "20.9377900,48.8534399",  # 17 Gelnica
    "20.2170090,49.1365266",  # 18 Vysoke Tatry
    "20.2976401,49.0541521",  # 19 Poprad
    "20.9973314,48.6141424",  # 20 Moldava nad Bodvou
    "20.8922660,48.7003257",  # 21 Medzev
    "20.9640764,49.1525429",  # 22 Lipany
    "20.4935541,48.9773801",  # 23 Arnutovce
    "20.4853242,49.0191918",  # 24 Dravce
    "20.7241002,48.8840406",  # 25 Porac
    "20.5738559,48.8571549",  # 26 Hnilcik
    "20.6207124,48.9487029",  # 27 Danisovce
    "20.6631127,48.9434040",  # 28 Jamnik
    "20.7533778,48.9286964",  # 29 Olcnava
    "20.3252438,49.0297241",  # 30 Ganovce
    "20.5213996,49.0260088",  # 31 Dlhe Straze
    "20.6770082,48.8826960",  # 32 Rudnany
    "20.3448470,49.0326605",  # 33 Hozelec
    "20.7204029,48.9284196",  # 34 Vitkovce
    "20.6233841,48.9154341",  # 35 Markusovce
    "20.4672229,49.0015088",  # 36 Spissky Stvrtok
    "20.4299761,49.0592490",  # 37 Vlkova
    "20.7920261,48.9804224",  # 38 Zehra
    "20.3583507,48.9948021",  # 39 Spissky Stiavnik
    "20.3093442,48.9877666",  # 40 Hranovnica
    "20.4065865,49.0017374",  # 41 Vydrnik
    "20.7597948,48.9492458",  # 42 Bystrany
    "20.6346169,48.9341578",  # 43 Odorin
    "20.3924449,49.0198449",  # 44 Horka
    "20.7060513,48.9460642",  # 45 Spissky Hrusov
    "20.6983146,48.9268512",  # 46 Chrast nad Hornadom
    "20.6407345,49.0014666",  # 47 Spissky Hrhov
    "20.4766018,49.0856854",  # 48 Tvarozna
    "20.5832356,48.9615532",  # 49 Harichovce
    "20.4662654,48.9784055",  # 50 Letanovce
    "20.6171867,48.8648005",  # 51 Zavadka
    "20.7229784,49.0161278",  # 52 Jablonov
    "20.3589232,49.0263575",  # 53 Svabovce
    "20.7120678,48.8129487",  # 54 Svedlar
    "20.6690863,49.0024511",  # 55 Klcov
    "20.4727242,48.9610375",  # 56 Spisske Tomasovce
    "20.5895391,48.9086082",  # 57 Teplicka
    "20.4289357,48.8518348",  # 58 Mlynky
    "20.5220340,48.9571845",  # 59 Smizany
    "20.4253620,49.0878588",  # 60 Vrbov
    "20.5265321,48.9895217",  # 61 Iliasovce
    "20.4321750,49.0171366",  # 62 Janovce
    "20.3972802,49.0817207",  # 63 Zakovce
    "20.3922150,48.9796309",  # 64 Betlanovce
    "20.6185639,48.8423713",  # 65 Nalepkovo
    "20.4103349,48.9756056"   # 66 Hrabusice
) -join ";"

$url = "https://router.project-osrm.org/table/v1/driving/$coords" + "?annotations=distance"
Write-Host "Fetching matrix for 67 nodes..."
$resp = Invoke-RestMethod -Uri $url -TimeoutSec 60

if ($resp.code -ne "Ok") {
    throw "OSRM error: $($resp.code)"
}

Write-Host "Got $($resp.distances.Count) x $($resp.distances[0].Count) matrix"

$rows = @()
foreach ($row in $resp.distances) {
    $vals = ($row | ForEach-Object {
        $km = [Math]::Round($_ / 1000, 1)
        $km.ToString("0.0", $inv)
    }) -join ", "
    $rows += "    [$vals]"
}

$body = $rows -join ",`r`n"
$json = @"
{
  "unit": "km",
  "source": "OSRM public demo /table/v1/driving (router.project-osrm.org)",
  "generatedAt": "2026-05-03",
  "note": "Driving distances. Asymmetric (A->B != B->A) preserved. Index 0 = Home, 1..22 = cities/towns within 50km aerial, 23..66 = villages (pop >= 500) within 20km aerial.",
  "distances": [
$body
  ]
}
"@

$out = "C:\_dev\kniha-jazd\_tasks\61-route-map-poc\matrix.json"
Set-Content -Path $out -Value $json -Encoding utf8
Write-Host "Wrote $out"
