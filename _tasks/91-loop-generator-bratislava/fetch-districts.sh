#!/usr/bin/env bash
# How src-tauri/core/assets/bratislava.json was made (2026-10-08).
# Overpass returned 504 for relation queries on that day, so the points come
# from Nominatim: the boundary=administrative result of each district search.
# Rača and Lamač first matched a railway station; the second query
# ("<name>, okres Bratislava III/IV", featureType=settlement) gave the boundary.
# Nominatim allows 1 request per second and needs a User-Agent.
set -euo pipefail
UA="kniha-jazd (+https://github.com/mcsdodo/kniha-jazd)"
for d in "Staré Mesto" "Ružinov" "Vrakuňa" "Podunajské Biskupice" "Nové Mesto" \
         "Rača, okres Bratislava III" "Vajnory" "Karlova Ves" "Dúbravka" \
         "Lamač, okres Bratislava IV" "Devín" "Devínska Nová Ves" "Záhorská Bystrica" \
         "Petržalka" "Jarovce" "Rusovce" "Čunovo"; do
  case "$d" in *okres*) q="$d";; *) q="Bratislava-$d";; esac
  curl -s -G -A "$UA" https://nominatim.openstreetmap.org/search \
    --data-urlencode "q=$q" --data-urlencode format=jsonv2 --data-urlencode limit=3 \
    --data-urlencode countrycodes=sk \
    | jq -c --arg d "$d" '[.[] | select(.category=="boundary")][0] | {district: $d, lat, lon}'
  sleep 1.2
done
