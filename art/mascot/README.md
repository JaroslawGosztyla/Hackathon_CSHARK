# Maskotka — aksolotl w słuchawkach

Grafika wygenerowana przez AI przez Bartłomieja Szolca dla FocusMode (własność projektu, wpis w `CREDITS.md`).
**Projektu nie przerysowujemy** — klatki to oryginalne rysunki, tylko powiększone i wycięte.

| Plik źródłowy | Zawartość | Klatki w aplikacji |
|---|---|---|
| `axolotl-sheet.webp` (1536×745, przezroczyste tło) | usta O A E I U Y; mrugnięcie, oczko, uśmiech, smutek, złość, radość ×3 | `static/mascot/axo-atlas.webp` + `axo-manifest.json` (24 klatki 415×420) |
| `axolotl-peek-sheet.webp` (1536×1024) | „wyglądanie” zza krawędzi: spoczynek, mrugnięcie ×4, A E I U Y, uśmiech, smutek, złość, radość | `static/mascot/peek-atlas.webp` + `peek-manifest.json` (14 klatek 495×300) |
| `axolotl-poses-sheet.webp` (1536×1024, przezroczyste tło) | 11 póz (siatka 4×3, w 2. wierszu bez 1. kolumny): wskazywanie z „!” (oczy otwarte / zamknięte), łapki pod brodą z serduszkiem, zamyślenie z „?”, leżenie (czuwanie, sen z „zzZ”, z uniesioną głową), patrzenie w górę, zachwyt z iskierkami, widok od tyłu, serduszko w łapkach | `art/mascot/poses/*.png` + `poses.json` (×4, nie trafia jeszcze do aplikacji — do wykorzystania później) |

## Jak powstały klatki (powtarzalnie, `tools/mascot/`)

1. Arkusz → PNG z kanałem alfa (canvas w przeglądarce).
2. Powiększenie ×4: [Real-ESRGAN](https://github.com/xinntao/Real-ESRGAN) `realesrgan-ncnn-vulkan` v0.2.5.0 (BSD-3), model
   `realesrgan-x4plus-anime`. Narzędzie służy tylko do przygotowania grafiki — nie trafia do aplikacji.
3. Wykrycie klatek (`fast.js`: maska nieprzezroczystych pikseli → rozszerzenie o 2 px → etykietowanie; etykiety-pigułki
   są pomijane, drobne dodatki, np. znak złości i iskierki, dołączane do najbliższej klatki).
4. Wycięcie z obrazu ×4 (`cut3.js`), wyrównanie skali (wiersze rysowane mniejszymi postaciami powiększone ×1,30 do wielkości
   wiersza z ustami), kotwica: linia stóp + środek głowy (bez „skakania” przy zmianie klatki), oczyszczenie miękkiej poświaty
   alfa (α ≤ 40 → 0, α ≥ 215 → 255).
5. Eksport: atlas WebP (6 kolumn) + manifest (`frameWidth`, `frameHeight`, `anchorX/Y`, pozycje klatek).

Uruchomienie: `node tools/mascot/server.mjs` (port 8777, folder z `tool.html` i arkuszem), Edge headless
`--remote-debugging-port=9333 http://127.0.0.1:8777/tool.html?sheet=<arkusz>.png`, potem `node cdp.mjs "<wyrażenie>"`,
np. `window.analyze2(2).then(() => window.cutSheet({...}))` (parametry jak w historii commita).
Odczyt pikseli rób w headless Edge — panel przeglądarki aplikacji dodaje szum do `getImageData`.

## Pozy (`poses/`, jeszcze nie w aplikacji)

Ten sam Real-ESRGAN ×4 → maska α > 40, rozszerzenie o 8 px (przy ×4) i etykietowanie → dodatki („!”, „?”, serduszko,
iskierki, „zzZ”, czerwone kreski ruchu) dołączone do pozy z tej samej komórki siatki → wycięcie maską etykiet (nie prostokątem)
z marginesem 24 px → PNG RGBA. Oczyszczenie alfy: α ≤ 40 → 0, α ≥ 215 → 255, **wartości 41–214 bez zmian** (inaczej niż
w kroku 4, gdzie wartości pośrednie są rozciągane liniowo); kolory i pozostała alfa są dokładnie takie jak w arkuszu ×4.
Jedna wspólna skala ×4, bez wyrównywania wielkości między pozami. `poses.json` podaje pole wycięcia (`x`, `y`, `width`,
`height`) we współrzędnych arkusza ×4 oraz `baselineY` — najniższy wiersz ciała (bez dodatków) z α ≥ 128, liczony
od górnej krawędzi PNG pozy.

Pozy wycięto w czystym Node (tylko `node:zlib`, bez Edge i bez zależności): `tools/mascot/png.mjs` (kodek PNG),
`tools/mascot/poses-analyze.mjs` (podgląd regionów) i `tools/mascot/poses-cut.mjs`. Uruchomienie:
`node tools/mascot/poses-cut.mjs <arkusz-x4>.png <katalog-wyjściowy>`.
