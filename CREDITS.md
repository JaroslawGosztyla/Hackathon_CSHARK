# Credits

Zasoby dołączone do FocusMode (CLAUDE.md, zasada 10). Każdy nowy plik muzyki, grafiki lub dźwięku dopisz tutaj.

## Muzyka

### Open Lo-Fi — wybór 12 utworów (`src-tauri/resources/music/open-lofi/`)

- **Autor:** btahir — <https://github.com/btahir/open-lofi>, wydanie `v1.0.0` (`openlofi.zip`, 2026-03-14).
- **Licencja:** [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/) — domena publiczna, bez wymogu podawania autora.
- **Pochodzenie:** według autora utwory wygenerował w Suno v5 na płatnym koncie (z prawem własności), a potem przekazał je do domeny publicznej.
- **Utwory** (lista i kategorie w `library.json`): 2 AM Debug Loop, Chapter By Lamplight, Coffee Ring Notebook, Cursor After Midnight,
  Dust on the Morning Keys, Hour Between Clicks, Porchlight Golden Hour, Rain on the Boulevard, Stacks of Quiet Books,
  Stacks of Quiet Hours, Terminal Rain, Window Seat Daydream.
- Pliki są niezmienione (oryginalne MP3 z archiwum wydania).

## Grafika

### Maskotka — aksolotl w słuchawkach (`static/mascot/`, źródła w `art/mascot/`)

- **Autor:** Bartłomiej Szolc (zespół FocusMode), grafika wygenerowana przez AI na potrzeby projektu.
- **Prawa:** własność projektu FocusMode.
- **Pliki:** arkusze źródłowe autora `art/mascot/axolotl-sheet.webp` i `art/mascot/axolotl-peek-sheet.webp`; w aplikacji atlasy klatek
  `static/mascot/axo-atlas.webp` (24 klatki) i `static/mascot/peek-atlas.webp` (14 klatek) z manifestami `axo-manifest.json` i `peek-manifest.json`.
- **Pozy (funkcje beta, ADR-022):** arkusz autora `art/mascot/axolotl-poses-sheet.webp` (11 póz, wycięte ×4 do `art/mascot/poses/`); w aplikacji
  atlas `static/mascot/poses-atlas.webp` (11 klatek) z manifestem `poses-manifest.json`. Atlas tylko **skaluje** wycięte pozy (jedna wspólna skala
  dla wszystkich, wspólna linia stóp; `tools/mascot/poses-atlas.mjs`, opis w `tools/mascot/README.md`), bez przerysowania, przekolorowania,
  obrysu ani odbicia; kanał alfa jak w plikach PNG.
- **Obróbka:** arkusze zostały **powiększone ×4 narzędziem [Real-ESRGAN](https://github.com/xinntao/Real-ESRGAN)** (`realesrgan-ncnn-vulkan` v0.2.5.0,
  licencja BSD-3, model `realesrgan-x4plus-anime`), a potem wycięte na klatki i wyrównane (wspólna skala i kotwica, usunięta miękka poświata krawędzi;
  `tools/mascot/`, opis w `art/mascot/README.md`). Grafika **nie jest wektoryzowana, przerysowywana ani przekolorowana**: klatki to oryginalne rysunki autora.
  Real-ESRGAN służył tylko do przygotowania plików i nie trafia do aplikacji. Pikselowy wygląd w motywie 8-bit to efekt wyświetlania, pliki się nie zmieniają (ADR-018).
- **Rozszerzenie przeglądarki:** pasek `browser-extension/images/axo-angry.webp` (3 klatki „angry” po 296 × 300 px, ten sam autor i prawa) na stronie
  „Wracaj do pracy”; strona pokazuje jedną nieruchomą klatkę, bez zmian w pliku.

## Dźwięki

- Dzwonki (wszystkie pakiety) są syntezowane w kodzie (`crates/focus-audio`) — bez plików z zewnątrz.
- Własne dźwięki użytkownika są kopiowane do jego folderu danych aplikacji i nie trafiają do repozytorium.
- **Głos Axi** w kreatorze pierwszego uruchomienia (`static/voice/pl/axi-setup-01…05.mp3`, tylko po polsku): nagrania przygotowane
  przez właściciela projektu (Bartłomiej Szolc) generatorem głosu AI, własność projektu. Przed publikacją poza hackathonem
  sprawdzić, czy warunki użytego narzędzia pozwalają dystrybuować nagrania w aplikacji.

## Teksty

### Porady na przerwę (`src/lib/tips.ts`)

- **Autor:** zespół FocusMode. Teksty są napisane od nowa (PL i EN), a nie skopiowane ze źródeł.
- **Źródła:** przy każdej poradzie jest link do źródła, na którym się opiera (m.in. HSE, OSHA, CIOP-PIB, AAO, WHO, CCOHS, NHS
  i publikacje naukowe o przerwach w pracy; ADR-015 w `docs/DECISIONS.md`). Porady to propozycje czynności, bez obietnic zdrowotnych.
