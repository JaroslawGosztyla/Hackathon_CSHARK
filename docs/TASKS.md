# FocusMode — Taski i podział pracy

> Wersja 3 — speedrun (2026-10-12, [ADR-013](DECISIONS.md#adr-013-speedrun-pivot-produktu)); wersja 2 po cross-review (2026-09-27).
> Decyzje: ADR-008…013 w [DECISIONS.md](DECISIONS.md).

**Legenda statusu:** `[ ]` do zrobienia · `[~]` w toku · `[x]` zrobione · `[-]` porzucone
Przy braniu taska wpisz obok statusu kto: `[~] (A)`. Edytuj **tylko swoje linie** (mniej konfliktów).

## Speedrun (ADR-013) — obowiązuje teraz

Termin: **2026-10-13**. Produkt: czasy ustawiane przez użytkownika (domyślnie 45/15 min) w pętli Focus ⇄ Przerwa aż do Stop,
muzyka w WebView (stream / linki YouTube / folder / szum), dzwonek i toast przy każdej zmianie fazy, osłony skupienia
(wyciszenie powiadomień Windows, Teams DND, blokada stron, nieusypianie ekranu) zdejmowane w przerwie, wykres produktywności według procesu.
Całość powstaje w jednej gałęzi `b/speedrun-mvp` (równoległe agenty AI pod nadzorem B). Sekcje M0–M4 niżej są sprzed pivotu:
zadania z oknami przerwy, ćwiczeniami, ostrzeżeniem, trybem spotkania, tint i gammą są odłożone. Ich statusy zostawiamy do decyzji obu osób.

- [~] **SR-01** (B) Kontrakt IPC od nowa (`crates/focus-core/src/api.rs`): DTO configu (czasy, muzyka, osłony), snapshot sesji, `ShieldStatus`,
  użycie aplikacji i `SessionSummary`, Teams, most dla rozszerzenia (`BRIDGE_PORT = 47615`), stałe komend i eventów. Bindingi w `src/lib/bindings/`
  napisane ręcznie w formacie ts-rs. DoD: `cargo test -p focus-core` daje pusty `git diff src/lib/bindings`.
- [~] **SR-02** (B) `focus-core`: `timing.rs` (45/15, demo 40/20 s), `session.rs` (pętla faz, pauza, pomiń, idempotentny Stop, luka > 90 s → auto-pauza,
  kolejność efektów), `config.rs` (domyślne wartości + walidacja zakresów), `blocklist.rs` (normalizacja domen, procesy przeglądarek, dopasowanie tytułów),
  `stats.rs` (sumy per proces, podsumowanie). DoD: testy jednostkowe z ADR-013 zielone.
- [~] **SR-03** (B) `focus-audio`: `ChimePlayer`, syntetyczne dzwonki BreakStart/FocusStart na wątku rodio, leniwe otwarcie urządzenia, błędy bez paniki.
- [~] **SR-04** (B) `focus-platform`: aktywne okno + bezczynność, wyciszenie powiadomień Windows (COM `IQuietHoursSettings`, stan sprzed focusa awaryjnie z WNF;
  DND włączamy tylko, gdy było wyłączone, i potem wyłączamy z powrotem),
  keep-awake (`PowerSetRequest`), Teams DND przez Graph (device code, token DPAPI), implementacje `Noop` poza Windows.
- [~] **SR-05** (B) Powłoka `src-tauri/`: aktor sesji, efekty, osłony (wątek z generacją, `shields-restore.json` + RunOnce `--restore-shields`), tracker (próbkowanie + awaryjny nudge),
  most HTTP `127.0.0.1:47615`, persystencja (`config.json`, `history.jsonl`), tray, Teams, skan folderu muzyki, NSIS per-user z rozszerzeniem jako zasobem.
- [~] **SR-06** (B) Frontend `src/`: Skupienie (pierścień czasu, sterowanie, edytor czasów z presetami, wybór muzyki), odtwarzacz (YouTube/stream/linki/folder/szum),
  Podsumowanie (wykres według procesu + historia), Ustawienia (osłony, Teams, blokada stron, dźwięk, autostart), motyw lofi, teksty w `strings.ts`.
- [~] **SR-07** (B) Rozszerzenie MV3 `browser-extension/`: odpytywanie mostu, reguły `declarativeNetRequest`, strona „Wracaj do pracy”, popup, README.
- [~] **SR-08** (B) Dokumentacja: ADR-013 (+ statusy ADR-003, 009–012), README (funkcje, instalacja, rozszerzenie, prywatność), CLAUDE.md, ten plik.
- [ ] **SR-09** (A+B, po merge'u) Zaktualizować DESIGN.md i ARCHITECTURE.md do ADR-013 (tabela faz, wątki, kontrakt, okna). ARCHITECTURE §6 (kontrakt IPC) zaktualizowany 2026-10-13.
- [~] **SR-10** (B) Okno YouTube jako fallback (ADR-013 pkt 2): gdy YouTube odmawia osadzania (błędy 101/150/153; m.in. live Lofi Girl i część YouTube Music)
  albo linku nie da się osadzić (`/@kanał/live`, `/channel/<id>/live`, `/@kanał`), link gra w ukrytym oknie Tauri `media` ze stroną YouTube (odtwarzacz w tle, bez uprawnień IPC,
  sterowany przez powłokę; widoczne tylko do pierwszego wyboru ciasteczek YouTube). Domyślny stream: `@LofiGirl/live`. E2E release 2026-10-13: 12/12 + zgoda 5/5. Kontrakt: `open_media_window` / `control_media_window` / `get_media_window` + `media://window`; dokumentacja: ADR-013, ARCHITECTURE §6, README.
  DoD: domyślny stream Lofi Girl gra z zainstalowanego buildu; Pauza / Wznów / Stop / głośność działają na okno.

**Bramka speedrunu** (na zainstalowanym buildzie, fizyczny Windows): ✅/❌
- [ ] `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace` zielone; bindingi zregenerowane bez różnic; `npm run check` zielone
- [ ] `--demo`: Start → 40 s → dzwonek + toast, osłony w dół, muzyka gra dalej → 20 s → dzwonek, osłony w górę → Stop → podsumowanie z wykresem
- [ ] Pauza / Wznów / Pomiń fazę / Stop z UI i z traya; dwukrotny Stop = jedno podsumowanie; zamknięcie okna nie przerywa muzyki
- [ ] muzyka: stream Lofi Girl (w oknie YouTube), link YouTube Music, playlista, folder z mp3, szum; link YouTube bez zgody na osadzanie → okno YouTube; link nieodtwarzalny → następny + komunikat
- [ ] rozszerzenie w Chrome i Edge: `reddit.com` w Focus → „Wracaj do pracy”, w przerwie otwiera się normalnie; bez rozszerzenia (brak odpytania mostu przez 90 s) → nudge po tytule okna
- [ ] powiadomienia Windows: wyciszone w Focus, włączone z powrotem w przerwie, po Stop, po „Zakończ”, po kill + ponownym starcie i po kill + ponownym logowaniu (RunOnce); DND włączone przed focusem zostaje nietknięte
- [ ] Teams (jeśli tenant pozwala): logowanie kodem, DND w Focus, wyczyszczone w przerwie i po Stop
- [ ] uśpienie laptopa w Focus i w przerwie → auto-pauza, czas luki nie liczy się do skupienia
- [ ] instalator NSIS: instalacja per-user poza repo, skrót, start bez sieci (szum/folder), folder rozszerzenia otwiera się z Ustawień, autostart startuje zminimalizowany do zasobnika

## Role

| | Osoba A — „Silnik” | Osoba B — „Aplikacja i UX” |
|---|---|---|
| Właściciel | `crates/focus-core`, `crates/focus-audio`, `crates/focus-platform`; w `src-tauri/src/`: `session_actor.rs`, `effects.rs`, `persistence.rs`, `tint_service.rs`; `assets/music/` | `src/` (frontend); w `src-tauri/`: `lib.rs`, `commands.rs` (cienkie adaptery), `windows.rs` (+ protokół gotowości), `tray.rs`, `tauri.conf.json`, `capabilities/`, `build.rs`; `assets/exercises.json` (treść); instalator |
| Wspólne | `crates/focus-core/src/api.rs` (kontrakt, zmiany przez `/sync-contract`), `docs/` | |

Zasada: jeśli musisz dotknąć pliku drugiej osoby, zrób minimalną zmianę i zostaw notkę w sekcji **Notatki między nami**.
Obciążenie oceniamy w godzinach, nie w liczbie checkboxów. Po T0-1 wpiszcie dostępne godziny każdej osoby.

| Osoba | Dostępne godziny (łącznie) | Uwagi |
|---|---|---|
| A | ? | |
| B | ? | |

## Harmonogram (2 tygodnie, dozwolone wszelkie narzędzia AI)

Dni są orientacyjne. Liczy się kolejność bramek, nie daty.

| Milestone | Dni | Bramka (musi działać na koniec) |
|---|---|---|
| M0 Setup, kontrakt, spike'i | 1–2 | obie osoby odpalają `npm run tauri dev`; kontrakt MVP zmergowany; CI zielone; spike okien i audio rozstrzygnięte |
| M1 MVP | 3–5 | **zainstalowany** build Windows: pełny cykl demo na 2 monitorach, Stop w każdej fazie, tryb spotkania |
| M2 Wow | 6–9 | tint overlay, fade/gong, podsumowanie sesji, statystyka dzienna, ustawienia, styl |
| M3 Stretch | 10–11 | Teams DND / gamma / inne *Could* — tylko jeśli M2 zielone |
| M4 Polish + demo | 12–14 | **feature freeze na początku dnia 12**; release, próby demo, wideo, slajdy |

Synchronizacja codziennie (15 min): merge do `main`, wspólny przebieg cyklu demo, aktualizacja tego pliku.
Checkpoint na koniec dnia 7: czy M2 zdąży? Jeśli nie, tniemy zakres, a nie freeze.

---

## M0 — Setup, kontrakt, spike'i

- [~] **T0-1** (A+B) Repo na GitHubie ✅ (https://github.com/JaroslawGosztyla/Hackathon_CSHARK), brakuje jeszcze push dla drugiej osoby. Wpisać godziny do tabeli wyżej; sprzęt demo (monitory, DPI, HDR) → DESIGN §13.
- [x] **T0-2** (B → zrobione przez A) Scaffold: `npm create tauri-app@latest` (Svelte + TypeScript), identyfikator `com.focusmode.app`, nazwa `FocusMode`. DoD: `npm run tauri dev` działa u A i B. → u A działa; B: sprawdź `npm install` + `npm run tauri dev` po pullu.
- [x] **T0-3** (A) Cargo workspace (`members = ["crates/*", "src-tauri"]`), puste crate'y `focus-*`; `rust-toolchain.toml` (1.98.1), `.nvmrc`; `.cargo/config.toml` z `TS_RS_EXPORT_DIR` (relative); `justfile` (dev, check, test, fmt, bindings). Zależy od: T0-2. → `just` instaluje setup-windows.ps1.
- [ ] **T0-4** (A+B, razem) **Kontrakt MVP** w `api.rs` wg ARCHITECTURE §6: DTO + przykłady JSON, enumy, `AppError`, ID (`SessionId`, `BreakId`) i `revision`, komendy i eventy MVP jako stałe, interfejs okien (`show_break/hide_break/show_warning/window_ready`), `TimingProfile`. Zatwierdzić ADR-001…007, 011, 012. **Bez przyszłych komend Teams/stats.** Merge ASAP.
- [ ] **T0-5** (B) Stuby komend + **fałszywy aktor** odgrywający oś czasu demo (0/35/40/60 s) na tych samych DTO → UI da się robić bez core.
- [ ] **T0-6** (A) CI (GitHub Actions): Windows obowiązkowo + ubuntu/macos best-effort: `npm ci`, `cargo fmt --check`, `cargo clippy --locked -- -D warnings`, `cargo test --locked --workspace`, **brak dryfu bindings** (`git diff --exit-code src/lib/bindings`), `npm run check`.
- [ ] **T0-7** (A+B) Każdy tworzy lokalny `CLAUDE.local.md` z linią „Pracuję jako osoba A/B”.
- [ ] **T0-8** (B, ≤ 1 h, informacyjne) Test tenantu M365: czy da się zarejestrować aplikację (public client, device code) i zgodzić na `Presence.ReadWrite`. Wynik zapisz w notatkach, bo decyduje o S-01.
- [ ] **T0-9** (B) **Spike okien** na sprzęcie demo: okno tworzone ukryte → pokazane po `window_ready`; fullscreen na 2 monitorach (różne DPI, monitor po lewej); sztuczne opóźnienie WebView o 2 s; okienko ostrzeżenia bez kradzieży fokusu. Wynik: co działa, a co nie → notka + ewentualna korekta ARCHITECTURE §10.
- [x] **T0-10** (A) **Spike audio**: aktualne rodio, brown noise przez 40 min bez zacięć, odłączenie słuchawek, brak urządzenia. Przypiąć wersję; poprawić ARCHITECTURE §8 i nazwy API. → rodio `=0.22.2`, wyniki w ARCHITECTURE §8; pomiar czasu niemożliwy na VM → T0-11.
- [ ] **T0-11** (A, 30 min + 40 min przebiegu) Powtórka pomiaru audio **na fizycznym Windows** (sprzęt demo): `spikes/audio-spike` → `rawcpal 60`, `diag 2400 60`, ewentualnie `--features boost`. Jeśli są pauzy > 100 ms, które boost usuwa, przenieść MMCSS/power throttling do A-05.

## M1 — MVP (zainstalowany build, Windows)

### Osoba A
- [ ] **A-01** `timing.rs` + `clock.rs` + `session.rs`: maszyna stanów dokładnie wg tabeli ARCHITECTURE §5.4 (ostrzeżenie w focusie, Postpone raz, tryb spotkania, Stop wszędzie, SkipBreak z `break_id`, heurystyka przerwania). DoD: wszystkie testy z §5.5 zielone. **+ review 2026-10-06:** reguła spotkania (R01), pauza zamraża `remaining` od ostatniego zaufanego ticka (R05), efekty Pause/Resume; testy z ARCHITECTURE §5.5.
- [ ] **A-02** `break_plan.rs`: planer (katalog + profil + seed) → `BreakPlan`, suma = `break_secs`, `stand-walk` na końcu; demo = stały plan. DoD: testy deterministyczne. **+ R09:** maks. 2 ćwiczenia mieszczące się z `min_walk_secs`; testy: przerwa krótsza/równa/dłuższa od sumy, bez ujemnej reszty.
- [ ] **A-03** `config.rs` (typy, `Default`, walidacja, TOML string) + `persistence.rs` w powłoce (config atomowo, `.bak` przy uszkodzeniu, katalog ćwiczeń z zasobów, `sessions.jsonl` append). DoD: core testowany bez systemu plików. **+ R09:** walidacja zakresów (ARCHITECTURE §5.1) → `AppError` z polem. **+ R02:** PersistenceWorker = jedna kolejka, zapisy po kolei; `RecordSession` trwałe, deduplikacja po `(session_id, cycle)`.
- [ ] **A-04** Generatory szumu (brown/pink/white). DoD: próbki w [-1, 1], brak NaN.
- [ ] **A-05** `focus-audio`: wątek, `AudioCmd`/`AudioEvent`, model głośności × fade, Stop przerywa fade, błędy urządzenia bez crasha; `genres.toml` z brown noise + **1 utwór CC0** + `assets/music/CREDITS.md`; `bundle.resources`. **+ R04:** docelowy stan odtwarzania (Stopped/Playing/Paused + głośność + fade + generacja), reconnect odtwarza stan zamiast `Play`; test: odłączenie → Pause/Stop/przerwa → podłączenie, audio nie rusza ani nie zmienia wyciszenia.
- [ ] **A-06** `session_actor.rs` + `effects.rs`: aktor (cmd / tick 1 s / wyniki efektów), dispatch bez await na wolnych operacjach, wyniki oznaczone `session_id`, spóźnione ignorowane; podmiana fałszywego aktora z T0-5. Zależy od: A-01, T0-5. **+ R02:** spóźniony wynik ignorowany tylko w stanie aktora; efekty przez wykonawców (seryjnie per zasób, docelowy stan z generacją dla DND); test: odwrotna kolejność zakończeń Start/Stop i dwóch zapisów configu → sprawdzamy pliki i stan zewnętrzny.

### Osoba B
- [ ] **B-01** Okno główne: `TimerRing`, Start/Pauza/Stop, `GenrePicker`, głośność, now playing, „Jestem na spotkaniu ▾” z czasem następnej próby; store z rewizjami; wszystko przez `api.ts`.
- [ ] **B-02** `windows.rs`: `break-{n}` per monitor wg protokołu gotowości (ADR-012), timeout 5 s → zamknięcie + powiadomienie; `hide_break` idempotentne. Na bazie T0-9. **+ R03:** `window_heartbeat(break_id)` co 1 s + watchdog w powłoce (5 s bez sygnału → zamknięcie okna i degradacja); bootstrap okien wg tabeli ARCHITECTURE §6.3.
- [ ] **B-03** Widok `#/break`: karta ćwiczenia z pełną treścią ze snapshotu, ilustracja po `id` z fallbackiem, postęp całej przerwy, `HoldToSkip` (3 s) + przytrzymanie Esc; monitory drugorzędne: spokojne tło + odliczanie. Treść 3 ćwiczeń MVP w `assets/exercises.json` (DESIGN §8).
- [ ] **B-04** Okienko `#/warning`: róg ekranu, always-on-top, bez fokusu; [Idę] [Jeszcze 2 min] [Jestem na spotkaniu ▾]. Toast `tauri-plugin-notification` tylko z tekstem.
- [ ] **B-05** `tray.rs`: menu (Start/Pauza/Stop/Pokaż/Zakończ, pozycja z czasem), tooltip na Windows; zamknięcie okna = hide; `single-instance`. **+ R03:** w trakcie przerwy pozycje „Pomiń przerwę” i „Stop” zawsze aktywne (ścieżka niezależna od WebView).
- [ ] **B-06** Tryb demo w zainstalowanej aplikacji: argument `--demo` + oznaczenie „skrócona prezentacja” w UI.
- [ ] **B-07** **Pierwszy instalator Windows** (NSIS/MSI): instalacja, uruchomienie ze skrótu poza repo, bez sieci, z odrzuconymi powiadomieniami; muzyka i katalog ćwiczeń się ładują.

**Bramka M1:** zainstalowany build, `--demo`, 2 monitory: 0 start → 35 ostrzeżenie → 40 przerwa → 60 koniec; drugi przebieg z Postpone i ze spotkaniem; Stop w każdej fazie; obie osoby potrafią to uruchomić. ✅/❌

## M2 — „Wow”

### Osoba A
- [ ] **A-07** `tint.rs`: kelvin → RGBA overlay, harmonogram (od–do, przejście 30 min, siła), testy + `tint_service.rs` (co 60 s, płynnie, `preview_tint`).
- [ ] **A-08** `stats.rs`: agregaty dzienne (aktywny czas, cykle, wyniki przerw), **podsumowanie jednym zdaniem**, rekordy demo pomijane; komenda `get_stats` (przez `/sync-contract`).
- [ ] **A-09** Audio polish: fade-in/out, gong przy zmianie ćwiczenia, 2–3 gatunki (CC0, CREDITS.md).

### Osoba B
- [ ] **B-08** Okna `tint-{n}`: przezroczyste, klikalne na wylot, bez fokusu, per monitor; komunikat „niedostępne” tam, gdzie nie działa.
- [ ] **B-09** Ustawienia `#/settings`: długości faz, auto-start, domyślny gatunek, głośność, tint (godziny, siła, podgląd na żywo), tryb demo.
- [ ] **B-10** Widok podsumowania/statystyki dnia (zdanie podsumowania + kafelki).
- [ ] **B-11** Styl lofi: ciemny ciepły motyw, animowany deszcz, ikona (`npm run tauri icon`); pozostałe ćwiczenia z DESIGN §8 + ilustracje. **Bez grafik Lofi Girl.**
- [ ] **B-12** Izolacja komend per okno: manifest komend w `build.rs` + capabilities wg tabeli ARCHITECTURE §6.3 (`tint-*`: tylko `get_tint_state` + `window_ready`; `warning`/`break-*`: minimum). DoD: `stop` z `tint-*` odrzucone, `get_tint_state` z `tint-*` działa, nowe/odświeżone okno tint od razu ma poprawny kolor (R06).

**Bramka M2:** tint overlay działa na Windows i znika po kill; przerwa z gongiem; podsumowanie i statystyka dnia się liczą. ✅/❌

## M3 — Stretch (tylko gdy M2 zielone; bierze ten, kto pierwszy wolny)

- [ ] **S-01** (B) Teams DND (ADR-010): device code, `keyring`, `setUserPreferredPresence` z wygaśnięciem, wygaśnięcie 10 min odnawiane w trakcie Focus, **bez automatycznego clear** (R07), przycisk „Wyczyść status Teams teraz”. Warunek: T0-8 pozytywny. Testy: Pause/Resume/Postpone/Stop, ręczna zmiana statusu w Teams, brak sieci.
- [ ] **S-02** (A) **Rekomendacja review R08: nie wdrażać.** Eksperymentalna gamma (ADR-009): Windows SDR, domyślnie off, kopia bazowej rampy per monitor (`GetDeviceGammaRamp`) zamiast resetu do liniowej. Testy: HDR on/off, Night Light on/off, podłączenie monitora, zwykłe wyjście, panika, kill, z weryfikacją widocznego efektu.
- [ ] **S-03** (A) Wykrywanie bezczynności → auto-pauza.
- [ ] **S-04** (A) Gatunek `stream` (radio z URL).
- [ ] **S-05** (A) Harmonogram tint wg zachodu słońca.
- [ ] **S-06** (B) Globalny skrót + autostart.
- [ ] **S-07** (B) Wykres 7 dni i seria dni.
- [ ] **S-08** (A+B) macOS/Linux: przejść cykl, zweryfikować tabelę ograniczeń w DESIGN §4.

## M4 — Polish i demo (feature freeze!)

- [ ] **P-1** (A) Release przez `tauri-action` (Windows obowiązkowo) → GitHub Release.
- [ ] **P-2** (B) Scenariusz demo (DESIGN §10) przećwiczony 2× na zainstalowanym buildzie; **nagranie wideo jako plan B**.
- [ ] **P-3** (A+B) README ze zrzutami/GIF-em; 3–5 slajdów: problem → demo → architektura → co dalej.
- [ ] **P-4** (A+B) Bug bash: każdy testuje funkcje drugiej osoby. Checklista (też przed bramkami M1/M2):
  - [ ] Stop w każdej fazie; dwukrotny Stop; Stop z tray przy przerwie
  - [ ] Esc przytrzymany 3 s pomija przerwę; zwykłe kliknięcie nie
  - [ ] WebView opóźniony o 2 s → brak pustego overlay; brak `window_ready` → powłoka zamyka overlay po 5 s
  - [ ] zawieszenie JS okna przerwy **po** `window_ready` (np. `while(true)` z devtools) → watchdog zamyka okno po 5 s; tray „Pomiń przerwę” działa
  - [ ] odświeżenie widoku przerwy w połowie → ten sam stan
  - [ ] „Pomiń” z dwóch monitorów naraz → jeden rekord w historii
  - [ ] uśpienie laptopa w Focus i w Break; zmiana zegara; przejście przez północ
  - [ ] odłączenie słuchawek / brak urządzenia audio → brak crasha, komunikat
  - [ ] wyciszenie → Postpone → głośność wraca
  - [ ] ukryte okno główne w czasie ostrzeżenia → okienko ostrzeżenia działa
  - [ ] instalator: poza repo, offline, odrzucone powiadomienia
  - [ ] kill procesu z włączonym tint → overlay znika
  - [ ] spotkanie włączone w trakcie ostrzeżenia (demo 30 s na 15 s) → brak przerwy do końca spotkania, potem pełne ostrzeżenie
  - [ ] odłączenie audio → Pause/Stop → podłączenie → muzyka nie wraca sama
  - [ ] 2 monitory, różne DPI, monitor po lewej, odłączenie monitora w trakcie przerwy

---

## Jak pracujemy (flow)

1. **Start dnia**: `git switch main && git pull --ff-only`.
2. **Task**: w Claude Code `/next-task A` (lub `B`, opcjonalnie z ID). Claude tworzy gałąź `a/<ID>-<slug>`, pokazuje plan,
   implementuje, uruchamia `/check`, aktualizuje ten plik i proponuje commit.
3. **Review i PR**: przejrzyj diff, poproś Claude o commit + push + PR (albo zrób to sam). Merge do `main` po zielonym CI
   i przeglądzie drugiej osoby lub `/code-review`; squash merge.
4. **Kontrakt IPC**: zmiany przez `/sync-contract`, osobny PR, mergowany pierwszy; druga osoba robi `git pull` od razu.
5. **Sync codzienny (15 min)**: przegląd otwartych PR-ów, merge, wspólny przebieg demo, aktualizacja statusów w tym pliku.
6. **Przed bramką milestone'u**: checklista P-4 na `main` (najlepiej na zainstalowanym buildzie).

Narzędzia: skille projektu `/next-task`, `/check`, `/sync-contract`; wbudowane `/code-review`, `/simplify`, `/security-review`.
Nowa maszyna: `scripts\setup-windows.ps1`.

## Punkty zapalne konfliktów (merge)

- `src-tauri/src/lib.rs`: właścicielem jest B. A prosi o rejestrację komend/serwisów albo dodaje jedną linię i zostawia notkę.
- **Lockfile'e**: procedura z ARCHITECTURE §14 (najpierw manifesty, potem regeneracja właściwym narzędziem, przegląd diffu).
- `src/lib/bindings/`: pliki generowane; po merge'u uruchom `cargo test -p focus-core`, nie rozwiązuj konfliktów ręcznie.
- Ten plik: każdy edytuje tylko swoje linie.

## Notatki między nami

<!-- Format: - [data godz] (A→B) treść -->
- [2026-09-27] Środowisko Rust na maszynie Jarka gotowe: rustc 1.98.1 (MSVC), clippy, rustfmt; VS 2022 Pro + MSVC 14.33 + Windows SDK 10.0.22621; Node.js 24.19.0 LTS + npm 11.17.0 (→ wpisać `24.19.0` do `.nvmrc` w T0-3).
- [2026-10-05] (Jarek→druga osoba) Repo: https://github.com/JaroslawGosztyla/Hackathon_CSHARK. Środowisko: `powershell -ExecutionPolicy Bypass -File scripts\setup-windows.ps1`. Flow i gałęzie: sekcja „Jak pracujemy” wyżej. Utwórz `CLAUDE.local.md` z linią „Pracuję jako osoba A/B”.
- [2026-10-05] Rekomendacja: włączyć na GitHubie ochronę `main` (Settings → Branches: require PR + status checks) po T0-6 (CI).
- [2026-10-05] (A→B) **Maszyna Jarka to VirtualBox VM** (emulowana karta HDA, wirtualna grafika). Zegar audio pływa 0,75–1,3× (T0-10). **T0-9 (okna, monitory, DPI) i wszystkie pomiary czasu/wydajności rób na fizycznym sprzęcie**, a demo też na fizycznym Windows. Funkcjonalnie VM jest OK do developmentu.
- [2026-10-05] (A→B) **Scaffold T0-2 zrobiłem za ciebie** (uzgodnione), razem z T0-3. Od teraz `src/`, `src-tauri/` (poza plikami A z tabeli Role), `tauri.conf.json`, `capabilities/` są twoje. Co zastaniesz:
  - szablon `create-tauri-app` **svelte-ts = SvelteKit w trybie SPA** (`adapter-static`, `ssr = false`), a nie goły Svelte z `App.svelte`, jak zakładała ARCHITECTURE §2. Okna mają wejść na `index.html#/break` itd., więc do decyzji: router SvelteKit w trybie hash (`kit.router.type = "hash"`) albo zostawienie samego Svelte. Po decyzji popraw ARCHITECTURE §2/§10.
  - w `src-tauri/src/lib.rs` zostało demo z szablonu (`greet`, plugin `opener`) — usuń przy T0-5.
  - `productName`/tytuł okna = `FocusMode`, identyfikator `com.focusmode.app`; profil `release` przeniesiony do root `Cargo.toml` (workspace).
  - `npm audit`: 3 × low (cookie w SvelteKit, dotyczy serwera; my jesteśmy statycznym SPA) — świadomie bez `audit fix --force` (SvelteKit 3 = breaking).
- [2026-10-05] Na VM (VirtualBox) `npm install` potrafi paść na `Assertion failed: new_time >= loop->time` (libuv + niestabilny zegar). Objaw: brak `package-lock.json` albo uszkodzony plik `.node` („not a valid Win32 application”). Lekarstwo: `rm -rf node_modules` i ponowny `npm install`.
- [2026-10-12] (B→A) **Pivot produktu: speedrun, termin jutro** ([ADR-013](DECISIONS.md#adr-013-speedrun-pivot-produktu), sekcja „Speedrun” na górze tego pliku). Co się zmieniło:
  - Nowy zakres od właściciela produktu: czasy ustawia użytkownik (domyślnie 45/15) w pętli Focus ⇄ Przerwa do Stop; muzyka ze streamu (Lofi Girl na YouTube), linków YouTube/YouTube Music, folderu albo szumu; na czas focusa osłony (wyciszenie powiadomień Windows, Teams DND, blokada stron przez rozszerzenie przeglądarki, nieusypianie), zdejmowane w przerwie; muzyka w przerwie gra dalej; wykres produktywności według procesu po Stop.
  - Odpadają (odłożone): pełnoekranowe okna przerwy, ćwiczenia i planer, ostrzeżenie, tryb spotkania, tint/gamma, gatunki w rodio. Muzyka gra w WebView, rodio zostaje tylko do dzwonków.
  - **Crate'y `focus-core`, `focus-audio`, `focus-platform` i twoje moduły w `src-tauri/src/` (`session_actor.rs`, `effects.rs`, `persistence.rs`) zostały zaimplementowane w gałęzi `b/speedrun-mvp`**, razem z nowym `api.rs`. Kontrakt nie poszedł osobnym PR-em (wyjątek opisany w ADR-013). Proszę o review, zwłaszcza `session.rs` (tabela faz i kolejność efektów), DND Windows (COM `IQuietHoursSettings` + odczyt WNF) w `focus-platform/src/notifications.rs` i Teams.
  - Nowe zależności (już w manifestach i lockfile'ach): `windows` 0.62 + `reqwest` 0.13 w `focus-platform`; pluginy Tauri dialog/notification/single-instance/autostart/log, `tokio`, `tiny_http`, `anyhow`, `log` w `src-tauri`; pluginy JS dialog/notification/autostart/log.
  - **Kompilacja nie jest zweryfikowana**: na maszynie B moduł antywirusa ESET (`ebehmoni.dll`) wywala procesy przy wyjściu, w tym build scripty cargo. Po pullu uruchom `/check` (fmt, clippy, testy); `cargo test -p focus-core` zregeneruje `src/lib/bindings/` (na razie pisane ręcznie w formacie ts-rs), więc `git diff src/lib/bindings` powinien być pusty.
  - Do wspólnej decyzji: statusy starych tasków M0–M4 (porzucić albo przenieść po demo) i aktualizacja DESIGN/ARCHITECTURE (SR-09).
- [2026-10-13] (B→A) Kontrakt: okno YouTube — komendy open_media_window / control_media_window / get_media_window, event media://window (MediaWindowStatus z rewizją); powód: Lofi Girl i część YouTube Music odmawia osadzania (błąd 150).
