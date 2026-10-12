# FocusMode — instrukcje dla Claude

Aplikacja desktopowa do sesji skupienia w stylu „lofi”: wybór dźwięku → 40 min focus → aplikacja wygania
na 5 min przerwy z prowadzonym ruchem. Do tego ocieplanie ekranu przez overlay i ręczny tryb „Jestem na spotkaniu”
(Teams DND to tylko stretch). Architektura multiplatformowa, **odbiór i gwarancje UX tylko na Windows** (ADR-008).
Projekt hackathonowy, 2 osoby (**A** = silnik Rust, **B** = aplikacja i UX), prowadzony z pomocą AI.

## Zanim zaczniesz pracę

1. Przeczytaj [docs/TASKS.md](docs/TASKS.md) — źródło prawdy: co robimy teraz, kto co robi, statusy.
2. Ustal, dla kogo pracujesz (A czy B): sprawdź `CLAUDE.local.md`; jeśli go nie ma — zapytaj.
3. Przy pracy nad architekturą/IPC czytaj [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md);
   przy decyzjach produktowych [docs/DESIGN.md](docs/DESIGN.md); ADR-y w [docs/DECISIONS.md](docs/DECISIONS.md).

## Stos

Tauri 2 (powłoka, okna, tray) · Rust workspace (`focus-core`, `focus-audio`, `focus-platform`) ·
rodio (audio) · Svelte 5 + TypeScript + Vite (UI w WebView) · ts-rs (typy TS z DTO w Ruście).

## Struktura

- `crates/focus-core/` — czysta logika: profil czasów, maszyna stanów, planer przerwy, walidacja configu, statystyki, matematyka tint. `src/api.rs` = kontrakt IPC.
- `crates/focus-audio/` — dedykowany wątek rodio, playlisty, generatory szumu, model fade.
- `crates/focus-platform/` — (stretch) Teams DND przez Graph, eksperymentalna gamma; za traitami z `Noop`.
- `src-tauri/` — powłoka: komendy, aktor sesji, efekty, persystencja, okna (main, warning, break, tint), tray.
- `assets/exercises.json` — jedyny katalog ćwiczeń.
- `src/` — **frontend Svelte/TS** (nie Rust!). `src/lib/api.ts` = jedyne miejsce z `invoke`/`listen`.
- `src/lib/bindings/` — **generowane** przez ts-rs, nie edytuj ręcznie.
- `assets/music/` — gatunki (`genres.toml`), pliki audio, `CREDITS.md`.

## Komendy

```bash
npm install                          # raz, po sklonowaniu
npm run tauri dev                    # uruchom aplikację (dev)
FOCUSMODE_DEMO=1 npm run tauri dev   # tryb demo (dev): 40 s focus, ostrzeżenie w 35 s, przerwa 20 s  (PowerShell: $env:FOCUSMODE_DEMO=1; npm run tauri dev)
                                     # zainstalowana aplikacja: FocusMode.exe --demo
cargo test --workspace --locked      # testy Rust (generuje też bindingi TS)
cargo test -p focus-core             # tylko core + regeneracja src/lib/bindings/
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
npm run check                        # svelte-check / TypeScript
npm run tauri build                  # instalator release
```

Skróty w `justfile`: `just dev`, `just demo`, `just check`, `just test`, `just bindings`, `just build`.
Pełny zestaw kontroli przed zakończeniem taska: skill `/check`.
Frontend to **SvelteKit w trybie SPA** (`adapter-static`, `ssr = false`) z szablonu Tauri.

## Twarde zasady (nie łam bez nowego ADR)

1. **`focus-core` nie zależy od Tauri, rodio, OS, sieci ani systemu plików.** API: `Session::handle(now, Input) -> Vec<Effect>`.
   Nie czytaj zegara ani plików w core. `now` (monotoniczny + kalendarzowy), katalog ćwiczeń i seed przychodzą z zewnątrz.
   Pliki czyta i zapisuje `src-tauri/src/persistence.rs`.
2. **Wszystkie czasy pochodzą z `TimingProfile`** (`normal()`/`demo()`), bez magicznych liczb w logice ani w UI.
   Fazy liczy zegar monotoniczny, statystyki kalendarzowy (ADR-011). Zachowanie faz jest dokładnie takie jak w tabeli ARCHITECTURE §5.4.
3. **Efekty uboczne wykonuje tylko powłoka** (`effects.rs`). Komendy Tauri są cienkie. **Aktor nigdy nie czeka na wolny efekt**:
   efekty idą do kolejek wykonawców (jeden na zasób, operacje seryjnie; docelowy stan z generacją dla efektów zewnętrznych).
   Ignorowanie spóźnionego wyniku chroni tylko stan aktora i **nie anuluje efektu** (ADR-012, R02).
4. **Kod per OS tylko w `focus-platform`**, za traitem, z `#[cfg(target_os = "...")]` i implementacją `Noop`.
   Żadnego UI na surowym Win32/HWND — okna robi Tauri.
5. **Kontrakt IPC zmieniasz tylko przez `/sync-contract`**. Każdy stan okna ma getter + event z rewizją (ADR-012):
   okno robi listen → getter → stosuje tylko nowszą `revision` → `window_ready`. Okna `tint-*` mają tylko odczyt.
6. **Tint robimy przez overlay** (ADR-009). Gamma ramp jest tylko eksperymentem (S-02), domyślnie wyłączonym, poza demo.
7. **Przerwa nie może uwięzić użytkownika**: Stop działa w każdej fazie i jest idempotentny, pominięcie przez przytrzymanie przycisku lub Esc,
   tray „Pomiń przerwę”/„Stop” niezależne od WebView, a okno przerwy bez `window_ready` albo bez heartbeatu przez 5 s powłoka zamyka.
8. Błąd integracji (audio/tint/Teams/zapis) nie przerywa sesji: log + event `app://error` + degradacja.
   Audio działa na dedykowanym wątku z komunikacją przez kanały. Głośność = głośność użytkownika × fade.
   Wątek audio trzyma docelowy stan odtwarzania; reconnect go odtwarza, nie jest nowym `Play` (R04).
9. Brak `unwrap()`/`expect()` poza testami i setupem. Biblioteki: `thiserror`; powłoka: `anyhow`;
   granica IPC: `AppError { code, message }`.
10. Muzyka i grafiki tylko z licencją pozwalającą na użycie (CC0/CC-BY), wpis w `assets/music/CREDITS.md`.
    Żadnych grafik ani marki Lofi Girl. Żadnych obietnic zdrowotnych w UI (tint to preferencja wizualna).

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
