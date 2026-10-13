# FocusMode — instrukcje dla Claude

Aplikacja desktopowa do sesji skupienia w stylu „lofi”: użytkownik ustawia czasy (domyślnie 45 min focus / 15 min przerwy)
i muzykę (domyślnie dołączone Open Lo-Fi, CC0; stream, np. Lofi Girl na YouTube, linki YouTube/YouTube Music, folder, szum) → Start → pętla Focus ⇄ Przerwa aż do Stop,
z dzwonkiem i toastem (albo okienkiem FocusMode w rogu ekranu) przy każdej zmianie fazy. Na czas focusa „osłony”: wyciszenie powiadomień Windows,
Teams „Nie przeszkadzać” (Graph; nie rusza „offline” ani własnego DND użytkownika), blokada rozpraszających stron (rozszerzenie przeglądarki + lokalny most),
nieusypianie ekranu, zamknięcie launcherów gier. W przerwie osłony schodzą, a muzyka gra dalej; przerwa pokazuje poradę i przypomnienie użytkownika.
Opcjonalnie ciepłe kolory ekranu według pory dnia (Oświetlenie nocne Windows albo gamma). Motywy i pakiety dzwonków (także własny plik).
Maskotka: aksolotl w słuchawkach z arkuszy klatek właściciela (mruga, reaguje na sesję, „mówi” porady ruchem ust, wygląda zza krawędzi okienek; ADR-018).
Po Stop wykres produktywności według aktywnego procesu (z tytułami okien w podpowiedzi, jeśli włączone). **Aktualny zakres: ADR-013 (speedrun) + ADR-014…018**;
wygrywa z DESIGN/ARCHITECTURE tam, gdzie się różnią.
Architektura multiplatformowa, **odbiór i gwarancje UX tylko na Windows** (ADR-008).
Projekt hackathonowy, 2 osoby (**A** = silnik Rust, **B** = aplikacja i UX), prowadzony z pomocą AI.

## Zanim zaczniesz pracę

1. Przeczytaj [docs/TASKS.md](docs/TASKS.md) — źródło prawdy: co robimy teraz, kto co robi, statusy.
2. Ustal, dla kogo pracujesz (A czy B): sprawdź `CLAUDE.local.md`; jeśli go nie ma — zapytaj.
3. Przy pracy nad architekturą/IPC czytaj [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md);
   przy decyzjach produktowych [docs/DESIGN.md](docs/DESIGN.md); ADR-y w [docs/DECISIONS.md](docs/DECISIONS.md).
   Oba dokumenty są sprzed pivotu: najpierw ADR-013…018 i `crates/focus-core/src/api.rs`.

## Stos

Tauri 2 (powłoka, okna, tray, toasty, instalator NSIS) · Rust workspace (`focus-core`, `focus-audio`, `focus-platform`) ·
rodio (tylko dzwonki) · Svelte 5 + TypeScript + Vite (UI i odtwarzacz muzyki w WebView z CSP:
iframe YouTube sterowany przez `postMessage`, `<audio>`, WebAudio) ·
ts-rs (typy TS z DTO w Ruście) · rozszerzenie MV3 dla przeglądarek Chromium.

## Struktura

- `crates/focus-core/` — czysta logika: `timing.rs` (profil czasów), `session.rs` (maszyna stanów Focus ⇄ Przerwa),
  `config.rs` (domyślne + walidacja, przypomnienia na przerwy `reminder_for_break`, nazwy plików dzwonków),
  `blocklist.rs` (parser `websiteblacklist.txt`: domeny + regexy RE2, procesy przeglądarek, dopasowanie tytułów; błędy jako `BlocklistIssueCode`),
  `stats.rs` (użycie aplikacji, tytuły okien, pomijanie powłoki Windows `is_shell_surface`, podsumowanie),
  `tint.rs` (harmonogram odcienia: poziom 0–100 z minuty doby; temperatura `kelvin_for_level` i mnożniki gammy `rgb_gain_for_kelvin`),
  `launchers.rs` (launchery gier i ich dokładne nazwy procesów), `language.rs` (obsługiwane języki, tag systemu → `Language`),
  `calendar.rs` (planowane przerwy, kolizje ze spotkaniami, pory przypomnień 30/20/10; w `session.rs` blokady spotkań `MeetingHold`),
  `work.rs` (kolejność „Co dalej?”: przypisane → czeka na ciebie → powiadomienia, wynik, powody; cel sesji `Input::SetGoal`).
  `src/api.rs` = kontrakt IPC. `assets/websiteblacklist.txt` = domyślna lista stron wbudowana w aplikację (ADR-013 pkt 4).
- `crates/focus-audio/` — dedykowany wątek rodio, dzwonki (`ChimePlayer`): syntetyczne pakiety `ChimeSound` i własny plik użytkownika. Muzyka nie gra w Ruście.
- `crates/focus-platform/` — integracje OS za traitami z `Noop`: `foreground` (aktywne okno z klasą okna + bezczynność),
  `notifications` (DND Windows przez nieudokumentowany COM `IQuietHoursSettings`, stan sprzed focusa awaryjnie z WNF),
  `power` (keep-awake), `teams` (Graph, device code, token DPAPI, odczyt statusu przed DND),
  `processes` (zamykanie procesów po dokładnej nazwie exe w bieżącej sesji: Toolhelp + `TerminateProcess`),
  `locale` (język wyświetlania systemu), `clock` (minuta doby czasu lokalnego dla odcienia, `GetLocalTime`),
  `night_light` (Oświetlenie nocne bieżącego użytkownika: nieudokumentowany zapis w rejestrze), `gamma` (rampa gamma każdego monitora z kopią bazowej rampy),
  `calendar/` (`google.rs`: OAuth loopback + PKCE i Calendar API; `ics.rs` + `rrule.rs`: iCalendar ze spotkaniami cyklicznymi), `secrets.rs` (pliki DPAPI),
  `teams` także z kalendarzem Microsoft (`Calendars.Read`, tylko gdy kalendarz włączony), `work/` (klienci GitHub, GitLab, Jira, Azure DevOps, Linear; tylko odczyt),
  `notification_state` (czy użytkownik prezentuje albo ma aplikację na pełnym ekranie: `SHQueryUserNotificationState`, tylko odczyt).
- `src-tauri/src/` — powłoka: `commands.rs` (cienkie adaptery), `session_actor.rs` (aktor + tick 1 s), `effects.rs` (dispatch efektów),
  `shields.rs` (wątek osłon z generacją, `shields-restore.json` + wpis RunOnce `FocusModeRestore` → `--restore-shields`; także zamykanie launcherów),
  `tracker.rs` (próbkowanie aktywnej aplikacji + awaryjny nudge po tytule okna, gdy rozszerzenie nie odpytało mostu przez 90 s),
  `bridge.rs` (HTTP `127.0.0.1:47615` dla rozszerzenia: `GET /v1/state` i jedyny zapis, zgłoszenie `POST /v1/report`; ADR-018),
  `persistence.rs` (`config.json`, `history.jsonl`, `keepHistory`, `clear_history`),
  `blocklist.rs` (`websiteblacklist.txt` w katalogu konfiguracji: utworzenie z domyślnej listy, odczyt, auto-przeładowanie po zapisie, `blocklist://changed`;
  wzorce, których przeglądarka nie przyjęła jako reguł, ze zgłoszenia rozszerzenia jako `extensionSkipped`),
  `music.rs` (skan folderu i muzyka w zestawie; asset scope tylko dla znalezionych plików audio), `media_window.rs` (ukryte okno YouTube, ADR-013 pkt 2),
  `popup.rs` (okno `popup`: okienka faz w rogu ekranu, `popup://show`; czekają, gdy użytkownik prezentuje albo gra na pełnym ekranie), `tint.rs` (ciepło ekranu przez Oświetlenie nocne albo gammę, harmonogram, podgląd,
  `tint-restore.json`, `tint://changed`; ADR-015), `safety_net.rs` (jeden wpis RunOnce `FocusModeRestore` dla osłon i kolorów ekranu),
  `close.rs` (X na oknie głównym: zasobnik, wyjście albo pytanie `app://close-requested`), `window_util.rs` (opcje WebView2 okna głównego dla
  pozostałych okien, pozycja w rogu ekranu), `teams.rs`, `tray.rs`, `texts.rs` (wszystkie teksty powłoki: toasty, tray, okienka),
  `calendar.rs` + `calendar/` (synchronizacja kalendarzy, wybory w `meeting-choices.json`, okienka `meetingReminder`, `Input::SetMeetingHolds`, `calendar://changed`),
  `work.rs` + `work/` (synchronizacja co 5 min, `work-seen.json`, wstrzymywanie nowych pozycji w focusie, okienko `workDigest`, `work://changed`);
  `persistence.rs` także `mark_goal_done`. `lib.rs`: argumenty startu (`--demo`, `--autostart`, `--restore-shields`); drugie uruchomienie z `--demo`
  przełącza aplikację w Idle na profil demo; przy starcie odświeża wpis autostartu (ADR-018).
  `src-tauri/capabilities/`: `default.json` (okno `main`), `overlays.json` (okno `popup`: tylko eventy).
  `src-tauri/resources/music/open-lofi/` = muzyka w zestawie (Open Lo-Fi, CC0, opis w `library.json`, licencja w `CREDITS.md`).
- `src/` — **frontend Svelte/TS** (nie Rust!). `src/lib/api.ts` = jedyne miejsce z `invoke`/`listen`. `src/lib/player/` = odtwarzacz muzyki.
  `src/lib/themes/` = motywy (zmienne CSS i tła), `src/lib/tips.ts` = porady na przerwę (PL/EN, źródło przy każdej).
  `src/lib/mascot/` = maskotka (ADR-018): manifesty klatek, komponent rysujący jedną klatkę z atlasu, sterownik stanów (mruganie, reakcje na sesję,
  ruch ust według samogłosek, pikselizacja w motywie 8-bit); wyłącza ją `Config.showMascot`.
  Kalendarze: `MeetingConflicts.svelte` (Skupienie), `MeetingHoldBanner.svelte`, `CalendarSettings.svelte`; „Co dalej?”: `NextUp.svelte`, `SessionGoalLine.svelte`, `WorkSettings.svelte`.
  Trasy: `src/routes/(app)/` = okno główne (Skupienie / Podsumowanie / Ustawienia, w layoucie odtwarzacz i animowana nawigacja),
  `src/routes/(overlay)/` = lekka strona okna `popup`, **bez odtwarzacza** (inaczej muzyka grałaby w kilku oknach).
- `src/lib/bindings/` — **generowane** przez ts-rs, nie edytuj ręcznie.
- `static/mascot/` — atlasy WebP i manifesty klatek maskotki (generowane z `art/mascot/` narzędziem `tools/mascot/`, nie edytuj ręcznie).
  `art/mascot/` — arkusze źródłowe właściciela + README (jak powstały klatki). `tools/mascot/` — powtarzalne cięcie arkuszy
  (Node + Edge headless; Real-ESRGAN tylko do przygotowania grafiki); nic z `art/` ani `tools/` nie trafia do aplikacji.
- `browser-extension/` — rozszerzenie MV3 (Chrome/Edge/Brave/Opera/Vivaldi): odpytuje most i przekierowuje rozpraszające strony na `blocked.html`.
  Trafia do instalatora jako zasób (`bundle.resources`).

## Komendy

```bash
npm install                          # raz, po sklonowaniu
npm run tauri dev                    # uruchom aplikację (dev)
FOCUSMODE_DEMO=1 npm run tauri dev   # tryb demo (dev): 40 s focus, przerwa 20 s  (PowerShell: $env:FOCUSMODE_DEMO=1; npm run tauri dev)
                                     # zainstalowana aplikacja: focusmode.exe --demo (przy działającej w Idle: przełącza ją w demo)
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
   Pliki czyta i zapisuje powłoka: `src-tauri/src/persistence.rs` (a `websiteblacklist.txt` — `src-tauri/src/blocklist.rs`).
2. **Wszystkie czasy pochodzą z `TimingProfile`** (`normal()`/`demo()`/`from_config()`), bez magicznych liczb w logice ani w UI.
   Fazy liczy zegar monotoniczny, statystyki kalendarzowy (ADR-011). Zachowanie faz i kolejność efektów są dokładnie takie jak w tabeli ADR-013.
   Wyjątek: pory przypomnień o spotkaniach (`MEETING_REMINDER_LEAD_MINUTES` w `timing.rs`, 30/20/10 min) i przedłużenie focusa do końca spotkania (ADR-016).
3. **Efekty uboczne wykonuje tylko powłoka** (`effects.rs`). Komendy Tauri są cienkie. **Aktor nigdy nie czeka na wolny efekt**:
   efekty idą do kolejek wykonawców (jeden na zasób, operacje seryjnie; docelowy stan z generacją dla efektów zewnętrznych).
   Ignorowanie spóźnionego wyniku chroni tylko stan aktora i **nie anuluje efektu** (ADR-012, R02).
4. **Kod per OS tylko w `focus-platform`**, za traitem, z `#[cfg(target_os = "...")]` i implementacją `Noop`.
   Żadnego UI na surowym Win32/HWND — okna robi Tauri.
5. **Kontrakt IPC zmieniasz tylko przez `/sync-contract`**. Każdy stan widoku (snapshot sesji, `ShieldStatus`, `TeamsStatus`, `TintStatus`, `PopupInfo`…)
   ma getter + event z rewizją (ADR-012): UI robi listen → getter → stosuje tylko nowszą `revision`.
6. **Bez pełnoekranowych okien przerwy i ręcznego trybu spotkania** („Jestem na spotkaniu”) — odłożone (ADR-013). Spotkania z kalendarza
   zmieniają sesję tylko według wyboru użytkownika (`extend`, `pauseAtBreak`), tylko gdy focus dochodzi do terminu, i nigdy nie blokują Pomiń fazę ani Stop (ADR-016). **Odcień ekranu tylko przez Oświetlenie nocne Windows
   albo gamma ramp, nigdy przez overlay** (warstwę koloru), **domyślnie wyłączony**. Stan sprzed zmiany (ustawienia Oświetlenia nocnego, bazowe rampy monitorów)
   zapisujemy zaraz po zmianie i przywracamy jak osłony: przy wyjściu, przy następnym starcie, najpóźniej przy logowaniu (zasada 7; ADR-015).
7. **Sesja nie może uwięzić użytkownika**: Stop działa w każdej fazie i jest idempotentny; Pauza/Wznów/Pomiń fazę z UI i z traya
   (tray idzie kanałem aktora, niezależnie od WebView). **Osłony zawsze wracają**: schodzą w przerwie, po Stop i przy wyjściu,
   a token wyciszenia powiadomień jest zapisywany zaraz po zmianie (`shields-restore.json` + wpis RunOnce `FocusModeRestore`
   z `--restore-shields`), więc po crashu powiadomienia wracają przy następnym starcie albo najpóźniej przy następnym logowaniu.
8. Błąd integracji (audio/osłony/Teams/most/zapis) nie przerywa sesji: log + event `app://error` + degradacja (status `Error` osłony).
   Dzwonki grają na dedykowanym wątku rodio z komunikacją przez kanały. Muzyka gra w WebView okna głównego,
   więc tego okna nie niszczymy: X chowa je do zasobnika albo kończy całą aplikację, zależnie od `closeToTray`
   (dopóki wyboru nie ma, pierwsze X pyta; ADR-013, ADR-014).
9. Brak `unwrap()`/`expect()` poza testami i setupem. Biblioteki: `thiserror`; powłoka: `anyhow`;
   granica IPC: `AppError { code, message, field }`.
10. Dołączane do aplikacji muzyka i grafiki tylko z licencją pozwalającą na użycie (CC0/CC-BY) albo własne projektu, z wpisem w `CREDITS.md`.
    **Maskotki (arkusze właściciela w `art/mascot/`) nie przerysowujemy, nie przekolorowujemy, nie obrysowujemy ani nie wektoryzujemy**;
    aplikacja tylko wyświetla jej klatki (wolno je skalować i pikselizować przy wyświetlaniu), a nowa mina to nowy arkusz od właściciela (ADR-018).
    Żadnych grafik ani marki Lofi Girl; link do ich publicznego streamu, odtwarzany oficjalnym playerem YouTube, jest OK.
    Żadnych obietnic zdrowotnych w UI (przerwa to propozycje ruchu, nie terapia).
11. **Prywatność**: nic nie opuszcza komputera poza ruchem, który wybrał użytkownik (muzyka, opcjonalnie Graph dla Teams i kalendarza Microsoft,
    Google Calendar, linki ICS, narzędzia pracy: GitHub, GitLab, Jira, Azure DevOps, Linear). Kalendarze i narzędzia pracy tylko do odczytu;
    token Google, linki ICS i tokeny narzędzi tylko w plikach szyfrowanych DPAPI (`focus-platform/src/secrets.rs`), nigdy w `config.json` ani w logach;
    tematy spotkań i tytuły zadań tylko lokalnie, nigdy w logach, historii, moście ani rozszerzeniu (toast podaje tylko liczbę).
    Kolejność „Co dalej?” liczy `focus-core` lokalnie; nic nie trafia do modeli AI (ADR-016, ADR-017).
    Most słucha tylko na `127.0.0.1` i tylko udostępnia stan, z jednym wyjątkiem: `POST /v1/report`, w którym rozszerzenie zgłasza wzorce z listy aplikacji,
    których przeglądarka nie przyjęła jako reguł (tylko origin rozszerzenia + nagłówek `X-FocusMode-Extension: 1`, sprawdzany `Host`, limity rozmiaru;
    aplikacja zostawia tylko wzorce z bieżącej listy; ADR-018). Innych zapisów do mostu nie dodajemy bez nowego ADR.
    Rozszerzenie nie wysyła adresów stron ani tytułów i nie ładuje zdalnego kodu.
    Tracker zapisuje nazwy procesów i sumy czasu, a tytuły okien **tylko przy włączonym `trackTitles`**: przycięte, tylko lokalnie w `history.jsonl`
    (przy wyłączonym `keepHistory` nic), **nigdy w logach** (także nie w `Debug`), moście, rozszerzeniu ani w sieci (ADR-015).
    Powierzchnie powłoki Windows (pasek zadań, pulpit, menu Start, `ShellExperienceHost`…) pomijamy. Tokenów nie logujemy.
    Launchery gier zamykamy tylko po dokładnych nazwach exe z `focus-core/src/launchers.rs`, tylko w bieżącej sesji,
    nigdy usług, procesów gier ani niczego, co wymaga praw administratora (ADR-014).

## Konwencje

- Kod, identyfikatory, komentarze i commity **po angielsku**; dokumentacja w `docs/` **po polsku**.
- Teksty dla użytkownika są na razie po polsku, docelowo w 10 językach (ADR-014). Każdy tekst UI trafia do `src/lib/strings.ts`,
  każdy tekst powłoki (toasty, tray, okienka) do `src-tauri/src/texts.rs`. `focus-core` nie zwraca tekstów: tylko kody
  (`AppError.code`, `BlocklistIssue.code`) i ewentualny szczegół po angielsku.
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
