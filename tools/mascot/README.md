# Narzędzia maskotki (`tools/mascot/`)

Narzędzia do przygotowania klatek aksolotla z arkuszy właściciela (`art/mascot/`, opis arkuszy i cięcia klatek
w `art/mascot/README.md`). Nic stąd nie trafia do aplikacji; aplikacja dostaje tylko gotowe atlasy w `static/mascot/`.
Grafiki **nie przerysowujemy, nie przekolorowujemy, nie obrysowujemy ani nie wektoryzujemy** (CLAUDE.md, zasada 10; ADR-018).

| Plik | Do czego |
|---|---|
| `tool.html`, `server.mjs`, `cdp.mjs`, `fast.js`, `cut3.js` | cięcie klatek głównych i „peek” (atlasy `axo-atlas.webp`, `peek-atlas.webp`) |
| `png.mjs`, `poses-analyze.mjs`, `poses-cut.mjs` | kodek PNG w czystym Node i wycięcie 11 póz do `art/mascot/poses/` |
| `white-to-alpha.mjs` | białe tło arkusza ×4 → przezroczyste (z wygładzoną krawędzią bez jasnej obwódki), opis w `art/mascot/README.md` (Pozy 2) |
| `sheet-cut.mjs` | cięcie dowolnego arkusza póz w wierszach (nazwy w kolejności czytania), np. 18 póz do `art/mascot/poses-2/` |
| `poses-atlas.mjs`, `poses-atlas.html` | atlas póz dla aplikacji (funkcje beta, ADR-022), opis niżej |

## Atlas póz (`static/mascot/poses-atlas.webp` + `poses-manifest.json`)

Wejście: `art/mascot/poses/*.png` (×4, RGBA) i `poses.json` (`baselineY` = linia stóp). Nic w `art/` się nie zmienia.

```bash
node tools/mascot/poses-atlas.mjs                       # buduje atlas i manifest
node tools/mascot/poses-atlas.mjs --preview qa.png      # + arkusz kontrolny (blink-1 i 11 póz, linia stóp), poza repo
node tools/mascot/poses-atlas.mjs --scale 0.36          # inna skala po kontroli wizualnej (domyślnie zmierzona)
```

Wymaga Node 22+ (wbudowane `fetch` i `WebSocket`) i Microsoft Edge (`--edge <ścieżka>` albo `EDGE_PATH`, gdy nie ma go
w standardowym miejscu). Skrypt sam uruchamia lokalny serwer tylko do odczytu na `127.0.0.1` (losowy port) i Edge
w trybie headless z tymczasowym profilem, a po pracy wszystko zamyka i usuwa.

Co robi:

1. **Pomiar póz w Node** (`png.mjs`): pierwszy wiersz z α ≥ 128, środek ciała `cx` (średnie x pikseli z α ≥ 128 w dolnej
   połowie postaci, wiersze `[baselineY − 0,5·(baselineY − góra), baselineY]`) i zasięg wszystkich pikseli z α > 0.
2. **Pomiar klatki głównej w headless Edge** (`blink-1` z `axo-atlas.webp`): wysokość ciała (linia stóp `anchorY` minus
   pierwszy wiersz z α ≥ 128) i środek ciała liczony tak samo jak dla póz. Odczyt pikseli tylko w headless Edge (panel
   przeglądarki aplikacji dodaje szum do `getImageData`).
3. **Jedna wspólna skala** dla wszystkich póz: wysokość ciała `blink-1` / wysokość ciała `05-look-up`, zaokrąglona do
   4 miejsc (bez skalowania pojedynczych póz, więc proporcje między pozami zostają jak na arkuszu).
4. **Komórka**: każda poza ma środek ciała na środku komórki i linię stóp na wspólnym wierszu; komórka mieści wszystkie
   piksele z α > 0 każdej pozy plus 4 px marginesu z każdej strony. Kotwica w manifeście (`anchorX`, `anchorY`) jest
   przesunięta o różnicę między kotwicą klatki głównej (środek głowy) a środkiem jej ciała, więc poza narysowana kotwicą
   na kotwicy klatki głównej (`Axolotl.svelte`) stoi tam, gdzie ciało klatki głównej.
5. **Rysowanie i kodowanie w Edge**: canvas `drawImage` z `imageSmoothingQuality = "high"`, każda poza przycięta do swojej
   komórki, siatka 4 kolumny × 3 wiersze (ostatnia komórka pusta), `toBlob("image/webp", 0.9)`. Budżet **≤ 600 KB**:
   powyżej najpierw jakość 0,85, nigdy mniejsza skala. Kontrola po zakodowaniu: zewnętrzny pierścień każdej komórki
   ma α = 0 (żadna poza nie jest ucięta i nic nie przecieka do sąsiadki przy skalowaniu w aplikacji).
6. **Manifest** w tym samym schemacie co `axo-manifest.json` (`frameWidth`, `frameHeight`, `columns`, `anchorX`,
   `anchorY`, `frames` z kluczami = nazwy póz) plus pola informacyjne (`poseScale`, `webpQuality`, `source`, `reference`),
   które `parseManifest` pomija.

Wynik (skala 0,3491): komórka 558×466, kotwica 296,460, atlas 2232×1398, ok. 320 KB. Te same pliki wejściowe dają
ten sam manifest (i ten sam plik WebP) przy każdym uruchomieniu. Po przebudowie z inną geometrią zaktualizuj
`FRAME_GEOMETRY.poses` w `src/lib/mascot/assets.svelte.ts` (lustro manifestu: rozmiar ramki znany przed wczytaniem).

Atlas jest w Git LFS (reguła `*.webp` w `.gitattributes`).

## Jak aplikacja pokazuje pozy

- Klatka `poses:<nazwa>` (np. `poses:05-look-up`) w `Axolotl.svelte` wariantu `main` pochodzi z atlasu póz i jest rysowana
  wokół tego samego pudełka co klatki główne: kotwica pozy na kotwicy klatki głównej, w skali klatki głównej (wysokość
  pudełka / `frameHeight` klatki głównej). Pudełko i układ strony się nie zmieniają; poza może wystawać poza pudełko
  (np. leżące pozy), nie przyjmuje kliknięć. W motywie 8-bit poza przechodzi przez tę samą pikselizację co klatki główne.
- Atlas póz jest pobierany leniwie, przy pierwszej klatce pozy, i tylko gdy pozy są dozwolone (`posesAllowed`: funkcje
  beta włączone z iskierkami albo małymi krokami i maskotka widoczna). Do tego czasu (i gdy atlas się nie wczyta)
  widać klatkę zastępczą z atlasu głównego (`POSE_FALLBACK` w `frames.ts`). W okienku (`peek`) póz nie ma.
- Dwa wyjątki od `posesAllowed`, oba na wyraźne życzenie użytkownika: album naklejek na stronie Podsumowanie pokazuje
  pozy odblokowanych naklejek także przy ukrytej maskotce, ale atlas pobiera dopiero przy pierwszym rozwinięciu albumu
  (zwinięty album niczego nie pobiera); próbki póz i nastrojów `thinking` / `working` ze strony Developer pokazują pozę
  także przy wyłączonych funkcjach beta, tylko na czas próbki.
- Strona Developer → Maskotka → „Pose gallery (beta)”: wszystkie pozy obok `blink-1` w pudełkach tego samego rozmiaru,
  na wspólnej linii stóp (140 / 96 / 66 px, opcjonalnie z `blink-1` nałożonym na każdą pozę). Do kontroli atlasu
  w motywach Midnight, Light i 8-bit.
