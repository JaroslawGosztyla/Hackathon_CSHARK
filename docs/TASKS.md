# FocusMode — Taski i podział pracy

> Wersja 2 — po cross-review (2026-09-27). Decyzje: ADR-008…012 w [DECISIONS.md](DECISIONS.md).

**Legenda statusu:** `[ ]` do zrobienia · `[~]` w toku · `[x]` zrobione · `[-]` porzucone
Przy braniu taska wpisz obok statusu kto: `[~] (A)`. Edytuj **tylko swoje linie** (mniej konfliktów).

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

- [ ] **T0-1** (A+B) Repo na GitHubie, obaj mają push. Wpisać godziny do tabeli wyżej; sprzęt demo (monitory, DPI, HDR) → DESIGN §13.
- [ ] **T0-2** (B) Scaffold: `npm create tauri-app@latest` (Svelte + TypeScript), identyfikator `com.focusmode.app`, nazwa `FocusMode`. DoD: `npm run tauri dev` działa u A i B.
- [ ] **T0-3** (A) Cargo workspace (`members = ["crates/*", "src-tauri"]`), puste crate'y `focus-*`; `rust-toolchain.toml` (1.98.1), `.nvmrc`; `.cargo/config.toml` z `TS_RS_EXPORT_DIR` (relative); `justfile` (dev, check, test, fmt, bindings). Zależy od: T0-2.
- [ ] **T0-4** (A+B, razem) **Kontrakt MVP** w `api.rs` wg ARCHITECTURE §6: DTO + przykłady JSON, enumy, `AppError`, ID (`SessionId`, `BreakId`) i `revision`, komendy i eventy MVP jako stałe, interfejs okien (`show_break/hide_break/show_warning/window_ready`), `TimingProfile`. Zatwierdzić ADR-001…007, 011, 012. **Bez przyszłych komend Teams/stats.** Merge ASAP.
- [ ] **T0-5** (B) Stuby komend + **fałszywy aktor** odgrywający oś czasu demo (0/35/40/60 s) na tych samych DTO → UI da się robić bez core.
- [ ] **T0-6** (A) CI (GitHub Actions): Windows obowiązkowo + ubuntu/macos best-effort: `npm ci`, `cargo fmt --check`, `cargo clippy --locked -- -D warnings`, `cargo test --locked --workspace`, **brak dryfu bindings** (`git diff --exit-code src/lib/bindings`), `npm run check`.
- [ ] **T0-7** (A+B) Każdy tworzy lokalny `CLAUDE.local.md` z linią „Pracuję jako osoba A/B”.
- [ ] **T0-8** (B, ≤ 1 h, informacyjne) Test tenantu M365: czy da się zarejestrować aplikację (public client, device code) i zgodzić na `Presence.ReadWrite`. Wynik zapisz w notatkach, bo decyduje o S-01.
- [ ] **T0-9** (B) **Spike okien** na sprzęcie demo: okno tworzone ukryte → pokazane po `window_ready`; fullscreen na 2 monitorach (różne DPI, monitor po lewej); sztuczne opóźnienie WebView o 2 s; okienko ostrzeżenia bez kradzieży fokusu. Wynik: co działa, a co nie → notka + ewentualna korekta ARCHITECTURE §10.
- [ ] **T0-10** (A) **Spike audio**: aktualne rodio, brown noise przez 40 min bez zacięć, odłączenie słuchawek, brak urządzenia. Przypiąć wersję; poprawić ARCHITECTURE §8 i nazwy API.

## M1 — MVP (zainstalowany build, Windows)

### Osoba A
- [ ] **A-01** `timing.rs` + `clock.rs` + `session.rs`: maszyna stanów dokładnie wg tabeli ARCHITECTURE §5.4 (ostrzeżenie w focusie, Postpone raz, tryb spotkania, Stop wszędzie, SkipBreak z `break_id`, heurystyka przerwania). DoD: wszystkie testy z §5.5 zielone.
- [ ] **A-02** `break_plan.rs`: planer (katalog + profil + seed) → `BreakPlan`, suma = `break_secs`, `stand-walk` na końcu; demo = stały plan. DoD: testy deterministyczne.
- [ ] **A-03** `config.rs` (typy, `Default`, walidacja, TOML string) + `persistence.rs` w powłoce (config atomowo, `.bak` przy uszkodzeniu, katalog ćwiczeń z zasobów, `sessions.jsonl` append). DoD: core testowany bez systemu plików.
- [ ] **A-04** Generatory szumu (brown/pink/white). DoD: próbki w [-1, 1], brak NaN.
- [ ] **A-05** `focus-audio`: wątek, `AudioCmd`/`AudioEvent`, model głośności × fade, Stop przerywa fade, błędy urządzenia bez crasha; `genres.toml` z brown noise + **1 utwór CC0** + `assets/music/CREDITS.md`; `bundle.resources`.
- [ ] **A-06** `session_actor.rs` + `effects.rs`: aktor (cmd / tick 1 s / wyniki efektów), dispatch bez await na wolnych operacjach, wyniki oznaczone `session_id`, spóźnione ignorowane; podmiana fałszywego aktora z T0-5. Zależy od: A-01, T0-5.

### Osoba B
- [ ] **B-01** Okno główne: `TimerRing`, Start/Pauza/Stop, `GenrePicker`, głośność, now playing, „Jestem na spotkaniu ▾” z czasem następnej próby; store z rewizjami; wszystko przez `api.ts`.
- [ ] **B-02** `windows.rs`: `break-{n}` per monitor wg protokołu gotowości (ADR-012), timeout 5 s → zamknięcie + powiadomienie; `hide_break` idempotentne. Na bazie T0-9.
- [ ] **B-03** Widok `#/break`: karta ćwiczenia z pełną treścią ze snapshotu, ilustracja po `id` z fallbackiem, postęp całej przerwy, `HoldToSkip` (3 s) + przytrzymanie Esc; monitory drugorzędne: spokojne tło + odliczanie. Treść 3 ćwiczeń MVP w `assets/exercises.json` (DESIGN §8).
- [ ] **B-04** Okienko `#/warning`: róg ekranu, always-on-top, bez fokusu; [Idę] [Jeszcze 2 min] [Jestem na spotkaniu ▾]. Toast `tauri-plugin-notification` tylko z tekstem.
- [ ] **B-05** `tray.rs`: menu (Start/Pauza/Stop/Pokaż/Zakończ, pozycja z czasem), tooltip na Windows; zamknięcie okna = hide; `single-instance`.
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
- [ ] **B-12** Izolacja komend per okno: manifest komend w `build.rs` + capabilities (`tint-*`: nic; `warning`/`break-*`: minimum). DoD: wywołanie z `tint-*` odrzucone.

**Bramka M2:** tint overlay działa na Windows i znika po kill; przerwa z gongiem; podsumowanie i statystyka dnia się liczą. ✅/❌

## M3 — Stretch (tylko gdy M2 zielone; bierze ten, kto pierwszy wolny)

- [ ] **S-01** (B) Teams DND (ADR-010): device code, `keyring`, `setUserPreferredPresence` z wygaśnięciem, czyszczenie tylko własnego DND, ostrzeżenie w ustawieniach. Warunek: T0-8 pozytywny. Testy: Pause/Resume/Postpone/Stop, ręczna zmiana statusu w Teams, brak sieci.
- [ ] **S-02** (A) Eksperymentalna gamma (ADR-009): Windows SDR, domyślnie off, znacznik `gamma.active` i naprawa przy starcie. Testy: HDR on/off, Night Light on/off, podłączenie monitora, zwykłe wyjście, panika, kill, z weryfikacją widocznego efektu.
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
  - [ ] WebView opóźniony o 2 s → brak pustego overlay; niedziałający frontend → powłoka zamyka overlay po 5 s
  - [ ] odświeżenie widoku przerwy w połowie → ten sam stan
  - [ ] „Pomiń” z dwóch monitorów naraz → jeden rekord w historii
  - [ ] uśpienie laptopa w Focus i w Break; zmiana zegara; przejście przez północ
  - [ ] odłączenie słuchawek / brak urządzenia audio → brak crasha, komunikat
  - [ ] wyciszenie → Postpone → głośność wraca
  - [ ] ukryte okno główne w czasie ostrzeżenia → okienko ostrzeżenia działa
  - [ ] instalator: poza repo, offline, odrzucone powiadomienia
  - [ ] kill procesu z włączonym tint → overlay znika
  - [ ] 2 monitory, różne DPI, monitor po lewej, odłączenie monitora w trakcie przerwy

---

## Punkty zapalne konfliktów (merge)

- `src-tauri/src/lib.rs`: właścicielem jest B. A prosi o rejestrację komend/serwisów albo dodaje jedną linię i zostawia notkę.
- **Lockfile'e**: procedura z ARCHITECTURE §14 (najpierw manifesty, potem regeneracja właściwym narzędziem, przegląd diffu).
- `src/lib/bindings/`: pliki generowane; po merge'u uruchom `cargo test -p focus-core`, nie rozwiązuj konfliktów ręcznie.
- Ten plik: każdy edytuje tylko swoje linie.

## Notatki między nami

<!-- Format: - [data godz] (A→B) treść -->
- [2026-09-27] Środowisko Rust na maszynie Jarka gotowe: rustc 1.98.1 (MSVC), clippy, rustfmt; VS 2022 Pro + MSVC 14.33 + Windows SDK 10.0.22621; Node.js 24.19.0 LTS + npm 11.17.0 (→ wpisać `24.19.0` do `.nvmrc` w T0-3).
