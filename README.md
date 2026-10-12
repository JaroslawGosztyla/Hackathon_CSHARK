# FocusMode 🎧

Sesje skupienia w klimacie lofi: ustawiasz czasy pracy i przerwy (domyślnie 45 / 15 minut), wybierasz muzykę
i klikasz Start. Na czas skupienia FocusMode wycisza to, co rozprasza: powiadomienia Windows, status w Teams
i strony z memami. W przerwie wszystko wraca, a muzyka gra dalej. Po zakończeniu sesji widzisz, w których
aplikacjach naprawdę spędziłeś czas.

Rust + Tauri 2 + Svelte 5 · Windows 10/11 (pełne wsparcie), macOS / Linux (best-effort) · projekt hackathonowy.
Zakres i decyzje: [ADR-013](docs/DECISIONS.md#adr-013-speedrun-pivot-produktu).

## Co potrafi

- **Pętla skupienie ⇄ przerwa** aż do Stop. Czasy ustawiasz sam (1–240 / 1–120 min) albo wybierasz preset: 25/5, 45/15, 50/10, 90/20.
- **Dzwonek i powiadomienie** przy każdej zmianie fazy. W przerwie aplikacja proponuje, jak się poruszać.
- **Pauza, Wznów, Pomiń fazę, Stop** w dowolnej chwili, w oknie albo z ikony w zasobniku (działa też przy ukrytym oknie).
  Uśpienie komputera w trakcie sesji automatycznie ją wstrzymuje.
- **Muzyka** (gra też w przerwie, chyba że wyłączysz to w ustawieniach):
  - **stream**: domyślnie publiczny live „lofi hip hop radio” kanału Lofi Girl na YouTube albo dowolny adres radia internetowego,
  - **linki**: filmy i playlisty YouTube, YouTube Music, `youtu.be`, bezpośrednie adresy plików audio (z mieszaniem),
  - **folder**: pliki mp3, m4a, aac, flac, wav, ogg, opus i webm z wybranego katalogu,
  - **szum**: brązowy, różowy lub biały, generowany lokalnie (działa offline),
  - albo cisza.

  YouTube gra w odtwarzaczu wbudowanym w okno FocusMode. Filmy, których właściciel nie pozwala osadzać w innych stronach
  (m.in. live Lofi Girl i część utworów YouTube Music), oraz linki do kanałów (`/@kanał`, `/@kanał/live`) otwierają się
  w **ukrytym oknie YouTube grającym w tle**, którym FocusMode steruje jak odtwarzaczem (odtwarzanie, pauza, głośność).
  Domyślny stream to `https://www.youtube.com/@LofiGirl/live` (zawsze bieżąca transmisja kanału). Przy pierwszym uruchomieniu YouTube pyta
  o zgodę na ciasteczka: okno pojawi się wtedy na środku ekranu, a po wyborze opcji schowa się samo (wybór zostaje zapamiętany).
  Przycisk „Pokaż okno” w odtwarzaczu pokazuje je na żądanie; zamknięcie (X) tylko je chowa, muzyka gra dalej.
- **Osłony skupienia** (każdą włączasz osobno w Ustawieniach; schodzą w przerwie, po Stop i przy wyjściu z aplikacji):
  - wyciszenie powiadomień Windows („Nie przeszkadzać”); w przerwie, po Stop i przy wyjściu powiadomienia są włączane z powrotem
    (jeśli „Nie przeszkadzać” było włączone już przed focusem, FocusMode go nie rusza),
  - status **„Nie przeszkadzać” w Microsoft Teams** (przez Microsoft Graph, wymaga jednorazowej konfiguracji),
  - **blokada rozpraszających stron** (reddit, 9gag, kwejk, jbzd, tiktok… lista do edycji) przez rozszerzenie przeglądarki,
    które przekierowuje na stronę „Wracaj do pracy”; gdy rozszerzenie nie odezwało się w ostatnich 90 s (np. go nie ma),
    FocusMode rozpoznaje stronę po tytule okna przeglądarki i przywołuje cię powiadomieniem,
  - ekran i komputer nie usypiają się w trakcie skupienia.
- **Wykres produktywności**: w trakcie skupienia FocusMode co sekundę sprawdza, która aplikacja jest na pierwszym planie.
  Po Stop pokazuje czas według procesu (np. `Code.exe`, `chrome.exe`), osobno bezczynność, a historię sesji zapisuje lokalnie.
- Okno działa w tle (zamknięcie = schowanie do zasobnika), jedna instancja, opcjonalny start przy logowaniu
  (wtedy FocusMode startuje zminimalizowany do zasobnika).

## Instalacja (Windows)

1. Uruchom instalator `FocusMode_0.1.0_x64-setup.exe` (jak go zbudować: [Rozwój](#rozwój)).
   Instaluje się tylko dla bieżącego użytkownika, bez praw administratora, do `%LOCALAPPDATA%\FocusMode`, ze skrótem w menu Start.
2. Instalator nie jest podpisany, więc SmartScreen może pokazać „System Windows ochronił ten komputer”. Wybierz **Więcej informacji → Uruchom mimo to**.
3. Wymagany jest Microsoft Edge WebView2 (jest w Windows 11 i aktualnym Windows 10). Jeśli go brakuje, instalator go pobierze.
4. Uruchom FocusMode, wybierz czasy i muzykę, kliknij **Start**.

Odinstalowanie: Ustawienia Windows → Aplikacje → Zainstalowane aplikacje → FocusMode.
Twoje dane (`%APPDATA%\com.focusmode.app`) możesz potem usunąć ręcznie.

## Rozszerzenie przeglądarki (blokada stron)

Działa w Chrome, Edge, Brave, Operze i Vivaldi (Chromium). Firefox nie jest na razie wspierany.

1. W FocusMode: **Ustawienia → Blokuj rozpraszające strony → Otwórz folder rozszerzenia**
   (po instalacji to `%LOCALAPPDATA%\FocusMode\browser-extension`, w repo: `browser-extension/`).
2. W przeglądarce otwórz stronę rozszerzeń: `chrome://extensions`, `edge://extensions`, `brave://extensions`, `opera://extensions` albo `vivaldi://extensions`.
3. Włącz **Tryb dewelopera** i kliknij **Załaduj rozpakowane**, a potem wskaż folder z punktu 1.
4. W Ustawieniach FocusMode status zmieni się na „Rozszerzenie połączone” (najpóźniej po 30 s).

Rozszerzenie co 30 s (i przy zmianie karty) pyta aplikację o stan pod `http://127.0.0.1:47615/v1/state`. W trakcie skupienia przekierowuje
domeny z listy na lokalną stronę „Wracaj do pracy”, a w przerwie wszystko otwiera się normalnie. Gdy aplikacja nie działa, rozszerzenie niczego nie blokuje.
Szczegóły: [browser-extension/README.md](browser-extension/README.md).

## Teams „Nie przeszkadzać” (opcjonalnie)

Microsoft Graph pozwala ustawić status tylko aplikacji zarejestrowanej w twoim tenancie, więc potrzebny jest jednorazowy setup
(czasem tylko administrator może go zrobić albo zatwierdzić):

1. [Entra ID](https://entra.microsoft.com) → **Rejestracje aplikacji → Nowa rejestracja**, typ kont: konta w katalogu organizacji.
2. **Uwierzytelnianie** → „Zezwalaj na przepływy klienta publicznego” = **Tak**.
3. **Uprawnienia interfejsu API** → Microsoft Graph → delegowane: `Presence.ReadWrite` i `User.Read` (jeśli tenant tego wymaga, ze zgodą administratora).
4. Skopiuj **Identyfikator aplikacji (klienta)** do FocusMode: **Ustawienia → Teams**, tenant zostaw `organizations` albo wpisz ID swojego tenanta.
5. Kliknij **Połącz**, otwórz pokazaną stronę Microsoft i wpisz kod.

W trakcie skupienia status to „Nie przeszkadzać” z wygaśnięciem (długość skupienia + 5 min), a na przerwę i po Stop FocusMode go czyści.
Uwaga: czyszczenie usuwa też status ustawiony w Teams ręcznie.

## Tryb demo

Skrócone czasy do prezentacji: **40 s skupienia, 20 s przerwy**, a w UI oznaczenie „skrócona prezentacja”.

```powershell
# zainstalowana aplikacja
& "$env:LOCALAPPDATA\FocusMode\focusmode.exe" --demo

# dev (PowerShell)
$env:FOCUSMODE_DEMO=1; npm run tauri dev
```

W bashu: `FOCUSMODE_DEMO=1 npm run tauri dev`. Z `just`: `just demo`.

## Prywatność

- **Nic nie opuszcza komputera.** Nie ma kont, telemetrii ani analityki.
- **Śledzenie aplikacji** zapisuje tylko nazwę procesu na pierwszym planie i czas bezczynności, raz na sekundę i tylko w trakcie
  biegnącego skupienia (nie w pauzie i nie w przerwie). Tytułów okien nie zapisujemy: służą wyłącznie w pamięci do awaryjnego rozpoznania
  rozpraszającej strony. Śledzenie wyłączysz w Ustawieniach.
- **Rozszerzenie nie wysyła historii przeglądania** ani do aplikacji, ani nigdzie indziej. Samo pobiera z aplikacji stan i listę domen,
  a adres karty porównuje lokalnie w przeglądarce. Nie ładuje zdalnego kodu. Jedynym adresem, z którym się łączy, jest `127.0.0.1:47615`.
- **Lokalny most** nasłuchuje tylko na `127.0.0.1` (nie jest widoczny w sieci) i udostępnia wyłącznie odczyt stanu.
- **Ruch sieciowy** jest tylko taki, jaki sam wybierzesz: odtwarzacz łączy się z YouTube / adresem streamu / linkami jak zwykła przeglądarka,
  a opcjonalna integracja z Teams łączy się z Microsoft Graph. YouTube gra w osadzonym iframe, którym aplikacja steruje przez `postMessage`.
  Skrypty Google nie są ładowane do samej aplikacji, a okno ma ustawione CSP. Player YouTube w iframe może zapisać swoje ciasteczka w danych WebView.
  Linki, których nie da się osadzić (np. live Lofi Girl), grają w małym osobnym oknie ze zwykłą stroną YouTube, jak karta przeglądarki:
  działają w nim skrypty i ciasteczka YouTube/Google (zapisane w profilu WebView2 aplikacji, także zgoda na ciasteczka). To okno nie ma dostępu
  do komend aplikacji, przechodzi tylko po domenach YouTube i Google, a inne linki otwiera w twojej przeglądarce.
  Token Teams jest zaszyfrowany DPAPI dla bieżącego użytkownika Windows.
- **Dostęp do plików**: odtwarzacz może czytać z dysku tylko pliki audio znalezione w wybranym folderze muzyki.
- **Wyciszenie powiadomień** zmienia tylko ustawienie „Nie przeszkadzać” bieżącego użytkownika (bez praw administratora) i tylko wtedy,
  gdy było wyłączone. Zaraz po włączeniu FocusMode zapisuje, co trzeba cofnąć (`shields-restore.json`), i dodaje wpis `FocusModeRestore`
  w `HKCU\Software\Microsoft\Windows\CurrentVersion\RunOnce`, który przy następnym logowaniu uruchamia FocusMode z `--restore-shields`.
  Powiadomienia wracają w przerwie, po Stop i przy wyjściu, a po awarii przy następnym uruchomieniu aplikacji albo najpóźniej przy następnym logowaniu.
- Dane leżą lokalnie w `%APPDATA%\com.focusmode.app`: `config.json` (ustawienia), `history.jsonl` (podsumowania sesji),
  `teams-token.bin` (jeśli połączysz Teams), `shields-restore.json` (tylko w trakcie wyciszenia). Logi: `%LOCALAPPDATA%\com.focusmode.app\logs`,
  dane WebView2 (w tym ciasteczka YouTube z okna YouTube i z osadzonego playera): `%LOCALAPPDATA%\com.focusmode.app\EBWebView`.

## Znane ograniczenia

- Gwarantujemy tylko Windows 10/11. Na macOS i Linuksie osłony i śledzenie aplikacji są niedostępne.
- Wyciszenie powiadomień korzysta z nieudokumentowanego interfejsu COM Windows (`IQuietHoursSettings`, ten sam, którego używa centrum powiadomień)
  i może przestać działać po aktualizacji systemu. Wtedy osłona będzie oznaczona jako niedostępna albo z błędem, a sesja trwa dalej.
  Automatyczne reguły Windows (gry, pełny ekran, godziny) zostają pod kontrolą systemu.
- Część filmów YouTube (m.in. live Lofi Girl i część utworów YouTube Music) nie pozwala na odtwarzanie poza youtube.com.
  Takie linki grają w ukrytym oknie YouTube. W tym trybie FocusMode nie przechodzi sam do następnego linku po końcu utworu.
  Link, którego nie da się odtworzyć wcale, jest pomijany z komunikatem.
- Rozszerzenie trzeba raz załadować ręcznie. Zmiana fazy dociera do niego najpóźniej po 30 s (przy otwieraniu stron szybciej).
- Przerwa to dzwonek, powiadomienie i propozycje ruchu w oknie. Nie ma pełnoekranowej blokady ekranu.
- W trybie dev powiadomienia Windows mogą mieć nazwę i ikonę PowerShell. Po instalacji mają nazwę FocusMode.

## Rozwój

Wymagania: Rust stable (`rustup`), Node.js LTS, zależności systemowe Tauri. Lista:
[docs/ARCHITECTURE.md §14](docs/ARCHITECTURE.md#14-narzędzia-i-środowisko).
Na Windows wszystko instaluje skrypt (sprawdza, co już jest, i doinstalowuje resztę):

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-windows.ps1
```

Potem:

```bash
npm install                          # na VM: przy „Assertion failed … loop->time” usuń node_modules i powtórz
npm run tauri dev                    # aplikacja w trybie dev
FOCUSMODE_DEMO=1 npm run tauri dev   # tryb demo (PowerShell: $env:FOCUSMODE_DEMO=1; npm run tauri dev)
cargo test --workspace --locked      # testy Rust; regeneruje też bindingi TS w src/lib/bindings/
cargo clippy --workspace --all-targets -- -D warnings
npm run check                        # svelte-check / TypeScript
npm run tauri build                  # instalator NSIS → target/release/bundle/nsis/FocusMode_0.1.0_x64-setup.exe
```

Z [`just`](https://github.com/casey/just): `just dev`, `just demo`, `just check`, `just build` (lista: `just`).
W trybie dev rozszerzenie ładujesz z `browser-extension/` w repo; po zmianach w jego kodzie kliknij „Odśwież” na stronie rozszerzeń.

| Katalog | Zawartość |
|---|---|
| `crates/focus-core` | czysta logika: profil czasów, maszyna stanów, config, blocklista, statystyki; kontrakt IPC w `api.rs` |
| `crates/focus-audio` | dzwonki (rodio) na dedykowanym wątku |
| `crates/focus-platform` | integracje Windows: aktywne okno, wyciszenie powiadomień, keep-awake, Teams (Graph) |
| `src-tauri/` | powłoka Tauri: komendy, aktor sesji, osłony, tracker, most dla rozszerzenia, persystencja, tray |
| `src/` | frontend SvelteKit (SPA): ekrany, odtwarzacz muzyki, teksty UI |
| `browser-extension/` | rozszerzenie MV3 do blokady stron |
| `spikes/` | wyrzucalne eksperymenty z M0 |

## Dokumentacja

| Plik | Zawartość |
|---|---|
| [docs/DECISIONS.md](docs/DECISIONS.md) | decyzje architektoniczne (ADR); **ADR-013 opisuje aktualny zakres** |
| [docs/TASKS.md](docs/TASKS.md) | taski (sekcja „Speedrun”), milestone'y i podział pracy A/B |
| [docs/DESIGN.md](docs/DESIGN.md) | produkt, UX, ryzyka (sprzed pivotu, w części nieaktualne) |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | stos, crate'y, wątki, narzędzia (sprzed pivotu, w części nieaktualne); kontrakt IPC w §6 aktualny |
| [docs/CROSS_AI_REVIEW.md](docs/CROSS_AI_REVIEW.md) | jak zrobić review planu drugim modelem AI |
| [CLAUDE.md](CLAUDE.md) | instrukcje dla Claude Code |

## Praca z Claude Code

- `CLAUDE.md` ładuje się automatycznie i zawiera zasady projektu.
- Stwórz lokalny `CLAUDE.local.md` (nie trafia do gita) z jedną linią: `Pracuję jako osoba A` albo `… B`.
- Skille: `/next-task A` (bierze kolejny task, robi go na własnej gałęzi), `/check` (fmt, clippy, testy),
  `/sync-contract <opis>` (spójna zmiana kontraktu UI↔Rust).
- Flow: gałąź na task → PR do `main` → review → squash merge. Szczegóły: [docs/TASKS.md — „Jak pracujemy”](docs/TASKS.md).
