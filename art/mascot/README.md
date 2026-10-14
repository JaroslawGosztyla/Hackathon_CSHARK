# Maskotka — aksolotl w słuchawkach

Grafika wygenerowana przez AI przez Bartłomieja Szolca dla FocusMode (własność projektu, wpis w `CREDITS.md`).
**Projektu nie przerysowujemy** — klatki to oryginalne rysunki, tylko powiększone i wycięte.

| Plik źródłowy | Zawartość | Klatki w aplikacji |
|---|---|---|
| `axolotl-sheet.webp` (1536×745, przezroczyste tło) | usta O A E I U Y; mrugnięcie, oczko, uśmiech, smutek, złość, radość ×3 | `static/mascot/axo-atlas.webp` + `axo-manifest.json` (24 klatki 415×420) |
| `axolotl-peek-sheet.webp` (1536×1024) | „wyglądanie” zza krawędzi: spoczynek, mrugnięcie ×4, A E I U Y, uśmiech, smutek, złość, radość | `static/mascot/peek-atlas.webp` + `peek-manifest.json` (14 klatek 495×300) |
| `axolotl-poses-sheet.webp` (1536×1024, przezroczyste tło) | 11 póz (siatka 4×3, w 2. wierszu bez 1. kolumny): wskazywanie z „!” (oczy otwarte / zamknięte), łapki pod brodą z serduszkiem, zamyślenie z „?”, leżenie (czuwanie, sen z „zzZ”, z uniesioną głową), patrzenie w górę, zachwyt z iskierkami, widok od tyłu, serduszko w łapkach | `art/mascot/poses/*.png` + `poses.json` (×4); w aplikacji: `static/mascot/poses-atlas.webp` + `poses-manifest.json` (nagrody beta, ADR-022; opis w `tools/mascot/README.md`) |
| `axolotl-poses-2-sheet.webp` (1536×1024, **białe tło**) | 18 póz w 4 wierszach (5, 5, 4, 4; w 3. wierszu bez 1. kolumny, w 4. bez 4.): machanie, chichot z serduszkiem, wskazywanie, „?”, leżenie z uśmiechem, wiwat, zerkanie w bok, zachwyt z iskierkami, sen z „zzZ”, widok od tyłu, oczko, serduszko w łapkach, foch ze znakiem złości, wskazywanie z żółtymi kreskami, podskok z kreskami ruchu, leżenie z oczkiem, zamyślenie z bąbelkiem, radość z gwiazdkami | `art/mascot/poses-2/*.png` + `poses.json` (×4, przezroczyste tło); jeszcze nie w aplikacji |

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

## Pozy (`poses/`; atlas dla aplikacji: `tools/mascot/README.md`)

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

## Pozy 2 (`poses-2/`)

Arkusz `axolotl-poses-2-sheet.webp` przyszedł na **białym tle** (RGB bez kanału alfa). Kolejność: Real-ESRGAN ×4 na arkuszu
z białym tłem → `tools/mascot/white-to-alpha.mjs` (przezroczyste tło) → `tools/mascot/sheet-cut.mjs` (cięcie).
Tło usuwamy dopiero na obrazie ×4, bo krawędzie rysunku są tam dokładniejsze.

- **Tło**: białe piksele (każdy kanał ≥ 240) połączone z krawędzią arkusza → α = 0. Biel otoczona rysunkiem zostaje
  (błyski oczu, różowawy odblask na głowie, wąskie białe kliny pod nausznikiem): na przezroczystych arkuszach właściciela
  to miejsce jest narysowane, a nie jest dziurą.
- **Krawędź**: wygładzone piksele między tłem a rysunkiem, które są mieszanką bieli i najbliższego pełnego koloru (zwykle
  ciemnego obrysu), dostają α równe udziałowi tego koloru i kolor bez domieszki bieli, więc na ciemnym motywie nie ma
  jasnej obwódki. Przejście zatrzymuje się na obrysie (≥ 85% koloru obrysu), więc obrysy i wnętrze rysunku zostają
  bez zmian; zmienia się ok. 62 tys. pikseli krawędzi na 25 mln. Kolorów rysunku nie zmieniamy.
- **Cięcie**: maska α > 40, rozszerzenie o 8 px, etykietowanie; regiony powyżej 200 tys. pikseli to pozy, mniejsze
  („!”, „?”, „zzZ”, serduszka, iskierki, gwiazdki, kreski ruchu, bąbelek) dołączane do pozy o najbliższym pudełku.
  Wiersze z położenia środków póz, kolumna z siatki 5 kolumn. Piksele wycinane maską regionów (nie prostokątem), margines
  24 px, kolor i α dokładnie jak po kroku tła. `poses.json`: pole wycięcia w arkuszu ×4 (`x`, `y`, `width`, `height`),
  `row`, `col` i `baselineY` (najniższy wiersz ciała z α ≥ 128 od górnej krawędzi PNG).

Uruchomienie (arkusz ×4 z Real-ESRGAN jak wyżej; pełna lista nazw w kolejności czytania jest w `poses-2/poses.json`):

```bash
node tools/mascot/white-to-alpha.mjs sheet-x4.png sheet-x4-rgba.png
node tools/mascot/sheet-cut.mjs sheet-x4-rgba.png art/mascot/poses-2 axolotl-poses-2-sheet.webp 01-wave-hello,02-giggle-heart,...
```
