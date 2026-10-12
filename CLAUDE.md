# FocusMode — instrukcje dla Claude

Aplikacja desktopowa do sesji skupienia w stylu „lofi”: użytkownik ustawia czasy (domyślnie 45 min focus / 15 min przerwy)
i muzykę (stream, np. Lofi Girl na YouTube, linki YouTube/YouTube Music, folder, szum) → Start → pętla Focus ⇄ Przerwa aż do Stop,
z dzwonkiem i toastem przy każdej zmianie fazy. Na czas focusa „osłony”: wyciszenie powiadomień Windows, Teams „Nie przeszkadzać” (Graph),
blokada rozpraszających stron (rozszerzenie przeglądarki + lokalny most), nieusypianie ekranu. W przerwie osłony schodzą, a muzyka gra dalej.
Po Stop wykres produktywności według aktywnego procesu. **Aktualny zakres: ADR-013 (speedrun)**; wygrywa z DESIGN/ARCHITECTURE tam, gdzie się różnią.
Architektura multiplatformowa, **odbiór i gwarancje UX tylko na Windows** (ADR-008).
Projekt hackathonowy, 2 osoby (**A** = silnik Rust, **B** = aplikacja i UX), prowadzony z pomocą AI.

## Zanim zaczniesz pracę

1. Przeczytaj [docs/TASKS.md](docs/TASKS.md) — źródło prawdy: co robimy teraz, kto co robi, statusy.
2. Ustal, dla kogo pracujesz (A czy B): sprawdź `CLAUDE.local.md`; jeśli go nie ma — zapytaj.
3. Przy pracy nad architekturą/IPC czytaj [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md);
   przy decyzjach produktowych [docs/DESIGN.md](docs/DESIGN.md); ADR-y w [docs/DECISIONS.md](docs/DECISIONS.md).
   Oba dokumenty są sprzed pivotu: najpierw ADR-013 i `crates/focus-core/src/api.rs`.

## Stos

Tauri 2 (powłoka, okno, tray, toasty, instalator NSIS) · Rust workspace (`focus-core`, `focus-audio`, `focus-platform`) ·
rodio (tylko dzwonki) · Svelte 5 + TypeScript + Vite (UI i odtwarzacz muzyki w WebView z CSP:
iframe YouTube sterowany przez `postMessage`, `<audio>`, WebAudio) ·
ts-rs (typy TS z DTO w Ruście) · rozszerzenie MV3 dla przeglądarek Chromium.

## Struktura

- `crates/focus-core/` — czysta logika: `timing.rs` (profil czasów), `session.rs` (maszyna stanów Focus ⇄ Przerwa), `config.rs` (domyślne + walidacja),
  `blocklist.rs` (domeny, procesy przeglądarek, dopasowanie tytułów), `stats.rs` (użycie aplikacji, podsumowanie). `src/api.rs` = kontrakt IPC.
- `crates/focus-audio/` — dedykowany wątek rodio, syntetyczne dzwonki (`ChimePlayer`). Muzyka nie gra w Ruście.
- `crates/focus-platform/` — integracje OS za traitami z `Noop`: `foreground` (aktywne okno + bezczynność),
  `notifications` (DND Windows przez nieudokumentowany COM `IQuietHoursSettings`, stan sprzed focusa awaryjnie z WNF),
  `power` (keep-awake), `teams` (Graph, device code, token DPAPI).
- `src-tauri/src/` — powłoka: `commands.rs` (cienkie adaptery), `session_actor.rs` (aktor + tick 1 s), `effects.rs` (dispatch efektów),
  `shields.rs` (wątek osłon z generacją, `shields-restore.json` + wpis RunOnce `FocusModeRestore` → `--restore-shields`),
  `tracker.rs` (próbkowanie aktywnej aplikacji + awaryjny nudge po tytule okna, gdy rozszerzenie nie odpytało mostu przez 90 s),
  `bridge.rs` (HTTP `127.0.0.1:47615` dla rozszerzenia), `persistence.rs` (`config.json`, `history.jsonl`),
  `music.rs` (skan folderu; asset scope tylko dla znalezionych plików audio),
  `teams.rs`, `tray.rs`, `texts.rs` (teksty toastów).
- `src/` — **frontend Svelte/TS** (nie Rust!). `src/lib/api.ts` = jedyne miejsce z `invoke`/`listen`. `src/lib/player/` = odtwarzacz muzyki.
- `src/lib/bindings/` — **generowane** przez ts-rs, nie edytuj ręcznie.
- `browser-extension/` — rozszerzenie MV3 (Chrome/Edge/Brave/Opera/Vivaldi): odpytuje most i przekierowuje rozpraszające strony na `blocked.html`.
  Trafia do instalatora jako zasób (`bundle.resources`).

## Komendy

```bash
npm install                          # raz, po sklonowaniu
npm run tauri dev                    # uruchom aplikację (dev)
FOCUSMODE_DEMO=1 npm run tauri dev   # tryb demo (dev): 40 s focus, przerwa 20 s  (PowerShell: $env:FOCUSMODE_DEMO=1; npm run tauri dev)
                                     # zainstalowana aplikacja: focusmode.exe --demo
cargo test --workspace --locked      # testy Rust (generuje też bindingi TS)
cargo test -p focus-core             # tylko core + regeneracja src/lib/bindings/
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
npm run check                        # svelte-check / TypeScript
npm run tauri build                  # instalator NSIS → target/release/bundle/nsis/
```

Rozszerzenie w dev: `chrome://extensions` (albo `edge://extensions`) → Tryb dewelopera → Załaduj rozpakowane → `browser-extension/`.

Skróty w `justfile`: `just dev`, `just demo`, `just check`, `just test`, `just bindings`, `just build`.
Pełny zestaw kontroli przed zakończeniem taska: skill `/check`.
Frontend to **SvelteKit w trybie SPA** (`adapter-static`, `ssr = false`) z szablonu Tauri.

## Twarde zasady (nie łam bez nowego ADR)

1. **`focus-core` nie zależy od Tauri, rodio, OS, sieci ani systemu plików.** API: `Session::handle(now, Input) -> Vec<Effect>`.
   Nie czytaj zegara ani plików w core. `now` (monotoniczny + kalendarzowy) i config przychodzą z zewnątrz.
   Pliki czyta i zapisuje `src-tauri/src/persistence.rs`.
2. **Wszystkie czasy pochodzą z `TimingProfile`** (`normal()`/`demo()`/`from_config()`), bez magicznych liczb w logice ani w UI.
   Fazy liczy zegar monotoniczny, statystyki kalendarzowy (ADR-011). Zachowanie faz i kolejność efektów są dokładnie takie jak w tabeli ADR-013.
3. **Efekty uboczne wykonuje tylko powłoka** (`effects.rs`). Komendy Tauri są cienkie. **Aktor nigdy nie czeka na wolny efekt**:
   efekty idą do kolejek wykonawców (jeden na zasób, operacje seryjnie; docelowy stan z generacją dla efektów zewnętrznych).
   Ignorowanie spóźnionego wyniku chroni tylko stan aktora i **nie anuluje efektu** (ADR-012, R02).
4. **Kod per OS tylko w `focus-platform`**, za traitem, z `#[cfg(target_os = "...")]` i implementacją `Noop`.
   Żadnego UI na surowym Win32/HWND — okna robi Tauri.
5. **Kontrakt IPC zmieniasz tylko przez `/sync-contract`**. Każdy stan widoku (snapshot sesji, `ShieldStatus`, `TeamsStatus`) ma getter + event
   z rewizją (ADR-012): UI robi listen → getter → stosuje tylko nowszą `revision`.
6. **Bez pełnoekranowych okien przerwy, tint, gammy i trybu spotkania** — odłożone (ADR-013). Jeśli tint wróci, to przez overlay (ADR-009).
7. **Sesja nie może uwięzić użytkownika**: Stop działa w każdej fazie i jest idempotentny; Pauza/Wznów/Pomiń fazę z UI i z traya
   (tray idzie kanałem aktora, niezależnie od WebView). **Osłony zawsze wracają**: schodzą w przerwie, po Stop i przy wyjściu,
   a token wyciszenia powiadomień jest zapisywany zaraz po zmianie (`shields-restore.json` + wpis RunOnce `FocusModeRestore`
   z `--restore-shields`), więc po crashu powiadomienia wracają przy następnym starcie albo najpóźniej przy następnym logowaniu.
8. Błąd integracji (audio/osłony/Teams/most/zapis) nie przerywa sesji: log + event `app://error` + degradacja (status `Error` osłony).
   Dzwonki grają na dedykowanym wątku rodio z komunikacją przez kanały. Muzyka gra w WebView okna głównego,
   więc zamknięcie okna je tylko ukrywa (ADR-013).
9. Brak `unwrap()`/`expect()` poza testami i setupem. Biblioteki: `thiserror`; powłoka: `anyhow`;
   granica IPC: `AppError { code, message, field }`.
10. Dołączane do aplikacji muzyka i grafiki tylko z licencją pozwalającą na użycie (CC0/CC-BY), z wpisem w `CREDITS.md`.
    Żadnych grafik ani marki Lofi Girl; link do ich publicznego streamu, odtwarzany oficjalnym playerem YouTube, jest OK.
    Żadnych obietnic zdrowotnych w UI (przerwa to propozycje ruchu, nie terapia).
11. **Prywatność**: nic nie opuszcza komputera poza ruchem, który wybrał użytkownik (muzyka, opcjonalnie Graph dla Teams).
    Most słucha tylko na `127.0.0.1` i tylko udostępnia stan; rozszerzenie nie wysyła adresów stron i nie ładuje zdalnego kodu.
    Tracker zapisuje tylko nazwy procesów i sumy czasu; tytułów okien nie zapisujemy ani nie logujemy. Tokenów nie logujemy.

## Konwencje

- Kod, identyfikatory, komentarze i commity **po angielsku**; dokumentacja w `docs/` i teksty UI **po polsku**
  (teksty UI w `src/lib/strings.ts`).
- Commity: Conventional Commits z obszarem: `feat(core): …`, `fix(audio): …`, `feat(ui): …`,
  obszary: `core`, `audio`, `platform`, `shell`, `ui`, `docs`, `ci`. Małe i częste.
- Serde na DTO: `#[serde(rename_all = "camelCase")]`. Eventy nazywamy `obszar://nazwa`.
- Zależności dodawaj przez `cargo add` / `npm install` (aktualne wersje), nie z pamięci; nową zależność
  wymień w opisie commita.
- Ścieżki przez `std::path` / API Tauri (`app.path()`), nigdy sklejane z `\` — pracujemy na różnych OS.
- Hackathon: działające MVP > kompletność. Świadome skróty oznaczaj `// TODO(hackathon): …`.

## Git i gałęzie

- **Nie commitujemy prosto do `main`.** Jeden task = jedna krótka gałąź: `a/<ID>-<slug>` albo `b/<ID>-<slug>`
  (np. `a/T0-3-workspace`, `b/B-02-break-windows`). Gałąź żyje najwyżej 1–2 dni.
- Start taska: `git switch main && git pull --ff-only && git switch -c <gałąź>`. Brudne drzewo robocze → najpierw zapytaj.
- Koniec: `/check` zielony → commit(y) → `git push -u origin <gałąź>` → **PR do `main`**. Merge po zielonym CI
  i przeglądzie (druga osoba albo `/code-review`), **squash merge**, gałąź usuwana po merge'u.
- **Kontrakt IPC** (`api.rs`, bindings) idzie osobnym, małym PR-em i jest mergowany jako pierwszy.
- Przed PR-em: `git pull --rebase origin main` (albo merge `main` do gałęzi) i ponowny `/check`.
- Nigdy: `push --force` na `main`, `reset --hard` cudzych zmian, commit wygenerowanych plików z ręcznymi poprawkami.
- Commit, push i PR robisz tylko na prośbę użytkownika. PR przez `gh pr create`, a gdy `gh` nie ma — podaj link
  `https://github.com/JaroslawGosztyla/Hackathon_CSHARK/compare/main...<gałąź>`.

## Workflow z TASKS.md

- Bierzesz task → ustaw `[~] (A|B)`; skończony i sprawdzony (`/check`) → `[x]`. Skill `/next-task` robi to za ciebie.
- Edytuj tylko linie swojej osoby. Coś dla drugiej osoby → wpis w „Notatki między nami”.
- Dotykasz pliku, którego właścicielem jest druga osoba (tabela „Role” w TASKS) → minimalna zmiana + notka.
- Task okazuje się ponad 2× większy niż zakładano → zatrzymaj się i zaproponuj podział.
- Nie wychodź poza bieżący milestone bez pytania (feature freeze w M4 obowiązuje także AI).

## Środowisko

Co najmniej jedna maszyna deweloperska to Windows 10 (PowerShell). Wymagania narzędziowe: ARCHITECTURE §14.
Nowa maszyna Windows: `powershell -ExecutionPolicy Bypass -File scripts\setup-windows.ps1` (`-CheckOnly` tylko sprawdza).
Końce linii normalizuje `.gitattributes` (LF w repo; `.ps1` w CRLF).
