# FocusMode — Architektura

> Status: **wersja 3** — po dwóch cross-review ([2026-09-27](reviews/codex-crosscheck-2026-09-27.md), [2026-10-06](reviews/codex-review-2026-10-06.md); poprawki R01–R09).
> Decyzje: [DECISIONS.md](DECISIONS.md). Produkt i profil czasów: [DESIGN.md](DESIGN.md).
>
> **Uwaga (2026-10-13):** aktualny zakres to [ADR-013](DECISIONS.md#adr-013-speedrun-pivot-produktu) (speedrun)
> + [ADR-014](DECISIONS.md#adr-014-zasobnik-i-okienka-faz-odcień-ekranu-overlay-zamykanie-launcherów-gier-języki) (zasobnik i okienka faz, odcień ekranu jako overlay, launchery gier, języki).
> Aktualny jest §6 (kontrakt IPC, zgodny z `api.rs`). Pozostałe sekcje są sprzed pivotu (okna przerwy, ostrzeżenie, tryb spotkania, tint, muzyka w rodio).
> Tam, gdzie różnią się od ADR-013 i ADR-014, wygrywają ADR-y (task SR-09).

## 1. Stos technologiczny w skrócie

| Warstwa | Technologia | Po co |
|---|---|---|
| Powłoka / okna / tray | **Tauri 2** (Rust) | okna (także overlaye per monitor), tray, powiadomienia, instalatory — bez ręcznego HWND |
| Logika domenowa | **Rust**, crate `focus-core` | profil czasów, maszyna stanów, planer przerwy, walidacja configu, agregaty statystyk — czysta, bez I/O |
| Audio | **rodio** w `focus-audio` | odtwarzanie plików, generowany szum, fade |
| Integracje OS | `focus-platform` | (Could) Teams DND przez Graph, eksperymentalna gamma ramp |
| UI | **Svelte 5 + TypeScript + Vite** w WebView | szybki, ładny interfejs; animacje CSS |
| Typy UI↔Rust | **ts-rs** (`#[ts(export)]`) | typy TS generowane z DTO w Ruście |
| Build | `tauri build`, GitHub Actions | instalator Windows (NSIS/MSI) od M1; macOS/Linux w CI best-effort |

Platforma odbioru to Windows, pozostałe best-effort z tabelą ograniczeń (DESIGN §4, ADR-008).
Tauri nie gwarantuje identycznego UX na każdym OS; to, co obiecujemy, sprawdzamy testami na sprzęcie.

## 2. Układ repozytorium

```
.
├── Cargo.toml                 # workspace: crates/* + src-tauri
├── Cargo.lock                 # commitowany; CI buduje z --locked
├── rust-toolchain.toml        # przypięta wersja Rusta
├── .nvmrc                     # przypięta wersja Node
├── .cargo/config.toml         # [env] TS_RS_EXPORT_DIR = src/lib/bindings (relative = true)
├── assets/
│   ├── exercises.json         # JEDYNY katalog ćwiczeń (czyta powłoka → planer w core)
│   └── music/                 # genres.toml, pliki audio, CREDITS.md
├── crates/
│   ├── focus-core/            # ZERO zależności od Tauri/rodio/OS/sieci/systemu plików
│   │   └── src/
│   │       ├── api.rs         # DTO współdzielone z UI (#[ts(export)]) — KONTRAKT
│   │       ├── timing.rs      # TimingProfile { normal(), demo() } — jedyne źródło czasów
│   │       ├── clock.rs       # Now { mono, wall } — typy czasu (bez czytania zegara)
│   │       ├── session.rs     # maszyna stanów: Session::handle(now, Input) -> Vec<Effect>
│   │       ├── break_plan.rs  # planer przerwy: (katalog, profil, seed) -> BreakPlan
│   │       ├── config.rs      # Config + Default + walidacja + from/to TOML string (bez plików)
│   │       ├── stats.rs       # SessionRecord, agregaty dzienne, podsumowanie jednym zdaniem
│   │       └── tint.rs        # kelvin -> kolor/alfa overlay, harmonogram
│   ├── focus-audio/           # wątek audio (rodio), playlisty, generatory szumu, model fade
│   └── focus-platform/        # (Could) DndProvider/Teams, eksperymentalna gamma — za traitami
├── src-tauri/                 # powłoka (imperative shell)
│   ├── tauri.conf.json
│   ├── build.rs               # tauri_build + (Should) manifest komend aplikacji dla ACL
│   ├── capabilities/          # uprawnienia per typ okna
│   └── src/
│       ├── main.rs            # tylko focusmode_lib::run()
│       ├── lib.rs             # Builder, pluginy, setup, rejestracja komend       [B]
│       ├── commands.rs        # cienkie adaptery #[tauri::command] → SessionCmd   [B]
│       ├── session_actor.rs   # aktor: Session + tick + dispatch efektów          [A]
│       ├── effects.rs         # Effect → audio / okna / persystencja / eventy     [A]
│       ├── persistence.rs     # odczyt/zapis config.toml, sessions.jsonl, katalog [A]
│       ├── windows.rs         # main, warning, break-{n}, tint-{n} + bootstrap    [B]
│       ├── tray.rs                                                                [B]
│       └── tint_service.rs    # harmonogram → okna tint                           [A]
├── src/                       # FRONTEND: SvelteKit w trybie SPA (adapter-static, ssr=false) — nie Rust!
│   ├── app.html               # szablon SvelteKit; routing okien (#/break itd.): decyzja B — router hash SvelteKit lub własny
│   ├── lib/
│   │   ├── api.ts             # JEDYNE miejsce z invoke()/listen()
│   │   ├── bindings/          # GENEROWANE przez ts-rs — nie edytować
│   │   ├── stores/            # session store: snapshot + rewizje
│   │   ├── components/        # TimerRing, GenrePicker, ExerciseCard, HoldToSkip, ...
│   │   ├── illustrations/     # SVG/komponenty po id ćwiczenia + fallback
│   │   └── strings.ts         # teksty UI
│   └── routes/                # SvelteKit: +page.svelte (main), warning/, break/, tint/, settings/, stats/
└── docs/
```

## 3. Diagram komponentów

```mermaid
flowchart LR
  subgraph UI["Frontend: Svelte w WebView"]
    Main["Okno główne"]
    Warn["Okienko ostrzeżenia"]
    Break["Overlay przerwy per monitor"]
    TintOv["Overlay tint per monitor"]
  end
  subgraph Shell["src-tauri: powłoka"]
    Cmd["commands.rs"]
    Actor["SessionActor"]
    Eff["effects.rs"]
    Pers["persistence.rs"]
    Win["windows.rs"]
    Tray["tray.rs"]
    TintSvc["tint_service.rs"]
  end
  Core[("focus-core")]
  Audio["focus-audio: wątek audio"]
  Plat["focus-platform: Teams, gamma (Could)"]

  Main -- "invoke" --> Cmd
  Warn -- "invoke" --> Cmd
  Break -- "invoke" --> Cmd
  Cmd -- "SessionCmd" --> Actor
  Tray -- "SessionCmd" --> Actor
  Actor -- "handle(now, input)" --> Core
  Core -- "Vec of Effect" --> Actor
  Actor --> Eff
  Eff -- "AudioCmd" --> Audio
  Eff --> Win
  Eff --> Pers
  Eff -. "Could" .-> Plat
  Eff -- "session://snapshot" --> UI
  TintSvc --> Core
  TintSvc --> Win
```

## 4. Wzorzec: functional core, imperative shell (ADR-002)

```rust
// crates/focus-core (szkic — nazwy do ustalenia w T0-4)
pub struct Now { pub mono: Duration /* od startu aplikacji, monotoniczny */, pub wall: OffsetDateTime /* lokalny kalendarz */ }

pub enum Input {
    Start { genre: GenreId },
    Pause, Resume, Stop,
    Tick,
    AcceptBreak,                          // „Idę” (tylko w ostrzeżeniu)
    Postpone,                             // „Jeszcze 2 min” (tylko w ostrzeżeniu, raz na cykl)
    SetMeetingMode { minutes: Option<u32> }, // None = wyłącz
    SkipBreak { break_id: BreakId },      // idempotentne, z ID konkretnej przerwy
    ConfigChanged(Config),
}

pub enum Effect {
    Audio(AudioCmd),                      // Play{genre}, Pause, Resume, FadeTo{factor, ms}, Stop, Gong
    ShowBreak { break_id: BreakId },      // plan jest w snapshocie, nie w efekcie
    HideBreak { break_id: BreakId },
    ShowWarning, HideWarning,
    Notify { title: String, body: String },   // tylko tekst
    RecordSession(SessionRecord),         // dokładnie raz na cykl; klucz (session_id, cycle)
    SetDnd { on: bool },                  // Could; to *docelowy stan*, uzgadnia go DndWorker (§7)
}

impl Session {
    pub fn new(profile: TimingProfile, catalog: ExerciseCatalog, seed: u64) -> Self;
    pub fn handle(&mut self, now: Now, input: Input) -> Vec<Effect>;
    pub fn snapshot(&self, now: Now) -> SessionSnapshot; // odtwarza CAŁY widok (§6.3)
}
```

Zasady:
- `focus-core` **nie czyta zegara, plików ani sieci**. `now`, katalog ćwiczeń i seed przychodzą z zewnątrz,
  więc testy są deterministyczne.
- Config w core to typy, `Default`, walidacja oraz `from_toml_str` / `to_toml_string`. **Pliki czyta i zapisuje powłoka** (`persistence.rs`).
- Wszystkie czasy pochodzą z `TimingProfile` (żadnych „magicznych” liczb w logice ani w UI; UI dostaje czasy w snapshocie).

## 5. Model czasu i maszyna stanów (ADR-011)

### 5.1 Profil czasów

| Parametr | `normal()` | `demo()` |
|---|---:|---:|
| `focus_secs` (ostrzeżenie się w nim zawiera) | 2400 | 40 |
| `warning_secs` | 30 | 5 |
| `break_secs` | 300 | 20 |
| `postpone_secs` (raz na cykl) | 120 | 10 |
| `meeting_options` | 15/30/60 min | 15 s |
| `interruption_gap_secs` | 60 | 60 |
| `min_walk_secs` (minimalny `stand-walk` na końcu przerwy) | 30 | 10 |

Długości faz z configu użytkownika nadpisują `normal()`. Demo ignoruje config i używa stałego planu przerwy.

**Walidacja configu** (core, `Config::validate`, R09): `focus_secs` ∈ [300, 7200], `break_secs` ∈ [60, 1800],
`warning_secs` < `focus_secs`, `postpone_secs` ∈ [60, 600]. Wartości spoza zakresu dają `AppError` z polem, a nie cichą korektę.
Arytmetyka czasu tylko `checked_*`/`saturating_*` (bez ujemnych reszt na `u32`).

**Planer przerwy** (`break_plan.rs`, R09): z przetasowanego (seed) katalogu bierze **co najwyżej 2** ćwiczenia, ale tylko takie,
dla których `suma + min_walk_secs ≤ break_secs`. Resztę wypełnia `stand-walk`. Przy krótkiej przerwie wychodzi 0–1 ćwiczenie + spacer.
Czasów ćwiczeń nie skracamy w trybie normalnym. Suma planu zawsze = `break_secs`.

### 5.2 Zegary
- **Fazy i deadline'y**: `now.mono` (monotoniczny). **Statystyki dzienne i harmonogram tint**: `now.wall`.
- **Heurystyka przerwania**: jeśli między tickami minęło > `interruption_gap_secs` w zegarze monotonicznym **albo**
  zegar kalendarzowy przeskoczył o tyle więcej niż monotoniczny, to sesja jest przerwana. Tak łapiemy uśpienie niezależnie od tego,
  czy dany OS wlicza suspend do `Instant`; fałszywym alarmem będzie np. ręczna zmiana zegara. Skutki:
  - w Focus → `Paused { reason: Interrupted }`, czeka na Resume; luka **nie** liczy się jako czas focusa,
  - w Break → Idle, wynik przerwy `interrupted` (nie zaliczamy ćwiczeń automatycznie).
- **Pauza zamraża pozostały czas, a nie przesuwa deadline'u** (R05). `Paused { remaining_secs }`; Resume ustawia `deadline = now + remaining`.
  - Pause użytkownika: `remaining = deadline − now`.
  - Przerwanie (luka): `remaining = deadline − last_trusted_tick`, czyli czas od ostatniego zaufanego ticka do wykrycia luki **nie** jest zużywany.
  - Przykład: deadline 40 s, ostatni zaufany tick 10 s, wykrycie 130 s, Resume 135 s → nowy deadline 165 s (30 s focusa zostaje, aktywny czas = 10 s).
- Czas aktywnej sesji = suma odcinków, w których Focus faktycznie biegł (do ostatniego zaufanego ticka; bez pauz i luk).
- Tryb spotkania: `meeting_until` (zegar monotoniczny). `meeting_active = meeting_until > now`.

### 5.3 Stany

```mermaid
stateDiagram-v2
  [*] --> Idle
  Idle --> Focus: Start
  Focus --> Paused: Pause lub luka czasu
  Paused --> Focus: Resume
  Focus --> Break: koniec focusa, AcceptBreak
  Break --> Focus: koniec przerwy i autoStart
  Break --> Idle: koniec przerwy bez autoStart
  Break --> Idle: SkipBreak lub luka czasu
  Focus --> Idle: Stop
  Paused --> Idle: Stop
  Break --> Idle: Stop
```

Ostrzeżenie **nie jest osobną fazą**, tylko flagą `warning` w Focus (ostatnie `warning_secs`).
Tryb spotkania to pole `meeting_until` w Focus/Paused.

**Reguła spotkania (R01), wspólna dla wszystkich przejść:** dopóki `meeting_active`, **nie zaczyna się ani ostrzeżenie, ani przerwa**.
Włączenie spotkania w trakcie ostrzeżenia zeruje `warning`. Gdy spotkanie się kończy (wygaśnięcie albo `SetMeetingMode(None)`)
i do deadline'u zostało mniej niż `warning_secs` (albo deadline już minął), ustawiamy `deadline = now + warning_secs`
i od razu startuje **pełne** ostrzeżenie. „Idę” (AcceptBreak) działa tylko w ostrzeżeniu, więc w trakcie spotkania przerwy nie da się wymusić:
najpierw trzeba zakończyć spotkanie.

### 5.4 Pełna tabela: faza × wejście

`–` = ignorowane (no-op, bez błędu). Każde wejście jest idempotentne. „spotk.” = `meeting_active`.

| Faza \ wejście | Start | Pause | Resume | Stop | AcceptBreak | Postpone | SetMeetingMode | SkipBreak{id} | Tick |
|---|---|---|---|---|---|---|---|---|---|
| **Idle** | → Focus | – | – | – | – | – | – | – | – |
| **Focus** | – | → Paused(User, remaining) | – | → Idle, record(`stopped`) | – | – | ustaw / wyłącz (przy wyłączeniu: reguła spotkania) | – | spotk. → nic (przerwa czeka); koniec spotk. → reguła spotkania; bez spotk.: T−warn → `warning=true`, T → Break; luka → Paused(Interrupted) |
| **Focus + warning** | – | → Paused(User, remaining), `warning=false` | – | → Idle, record(`stopped`) | → Break | `deadline += postpone_secs`, `warning=false`, raz na cykl | ustaw → `warning=false` | – | T → Break (spotk. tu nie występuje); luka → Paused(Interrupted) |
| **Paused** | – | – | → Focus, `deadline = now + remaining`; jeśli bez spotk. i `remaining ≤ warning_secs` → `warning=true` | → Idle, record(`stopped`) | – | – | ustaw / wyłącz | – | nic (czas stoi) |
| **Break** | – | – | – | → Idle, record(`stopped`) | – | – | – | id zgodne → Idle, record(`skipped`); inne → – | koniec: autoStart ? → Focus : → Idle, record(`completed`); luka → Idle, record(`interrupted`) |

Efekty przejść:

| Przejście | Efekty |
|---|---|
| Idle → Focus | `Audio(Play)` z fade-in; (Could) `SetDnd{on}` |
| wejście w ostrzeżenie (także po końcu spotkania i po Resume) | `ShowWarning`, `Notify` (tekst), `Audio(FadeTo{0.3})` |
| wyjście z ostrzeżenia bez przerwy (Postpone / włączenie spotkania) | `HideWarning`, `Audio(FadeTo{1.0})` |
| Focus → Paused (User lub Interrupted) | `Audio(Pause)`, `HideWarning` (jeśli było) |
| Paused → Focus | `Audio(Resume)` (fade 1.0), a jeśli od razu ostrzeżenie: efekty wejścia w ostrzeżenie |
| Focus → Break | `HideWarning`, `ShowBreak{id}`, `Audio(FadeTo{0})`, `Audio(Gong)`; (Could) `SetDnd{off}` |
| zmiana ćwiczenia w Break | `Audio(Gong)` |
| Break → Focus | `HideBreak{id}`, `RecordSession`, `Audio(Play)`; (Could) `SetDnd{on}` |
| * → Idle | `Audio(Stop)`, `HideWarning`, `HideBreak`, `RecordSession` (jeśli był cykl), (Could) `SetDnd{off}` |

`RecordSession` zapisuje **dokładnie jeden** rekord na cykl (`session_id` + `cycle`), z wynikiem:
`completed | skipped | stopped | interrupted`.

### 5.5 Testy obowiązkowe (A-01)
Granice demo 34/35/39/40/60 s; Postpone raz (drugi ignorowany) i po nim ponowne ostrzeżenie; Stop w każdej fazie i Stop dwukrotny;
SkipBreak z cudzym `break_id`; luka w Focus i w Break; dokładnie jeden `RecordSession` na cykl; aktywny czas bez pauz.
**Spotkanie (R01):** włączone przed ostrzeżeniem i w trakcie ostrzeżenia (demo: spotkanie w 30 s na 15 s → brak przerwy w 40 s,
pełne ostrzeżenie 45–50 s, przerwa w 50 s); wygaśnięcie i ręczne wyłączenie po deadline; AcceptBreak w trakcie spotkania ignorowany.
**Pauza (R05):** oś z przykładu w §5.2 daje 30 s do przerwy i 10 s aktywnego focusa; Pause w ostrzeżeniu chowa okienko ostrzeżenia,
a Resume z `remaining ≤ warning_secs` pokazuje je ponownie.
**Planer i config (R09):** przerwa krótsza od, równa i dłuższa od sumy ćwiczeń + `min_walk_secs`; suma planu = `break_secs`, bez ujemnej reszty;
config spoza zakresów → błąd walidacji.

## 6. Kontrakt IPC (UI ↔ Rust) (ADR-006, ADR-012, ADR-013)

> Stan na 2026-10-13 (z ADR-020, panel prywatności, i ADR-021, logowanie do YouTube, oba wdrożone; ADR-022, funkcje beta: kontrakt z R-00,
> rdzeń i usługa nagród w powłoce z R-01…R-05; ADR-023, wyjątki odcienia, inteligentne przerwy, statystyki tygodnia i kreator: wszystkie cztery
> części wdrożone, §6.15),
> zgodny z `crates/focus-core/src/api.rs`. Przy rozbieżności wygrywa `api.rs`.

DTO w `crates/focus-core/src/api.rs`: `#[derive(Serialize, Deserialize, TS)]`, **`#[ts(export)]`**,
`#[serde(rename_all = "camelCase")]`. Liczby jako `u32` (unikamy `bigint` w TS); wyjątek to znaczniki czasu w ms od epoki Unix
(`i64` eksportowane jako `number` przez `#[ts(type = "number")]`): `startedAtMs` / `endedAtMs` w `SessionSummary` oraz czasy spotkań i przerw z ADR-016
(`Meeting.startMs` / `endMs`, `MeetingConflict.breakStartMs` / `breakEndMs` / `nextReminderMs`, `CalendarStatus.lastSyncMs`, `MeetingHoldView.untilMs`),
z ADR-017 (`WorkItem.dueMs` / `updatedMs`, `WorkStatus.lastSyncMs`), z ADR-022 (`RewardMoment.atMs`) i z ADR-023 (`SmartBreak.meetingStartMs` /
`meetingEndMs` / `breakStartMs` / `breakEndMs`, `TimeSpan.startMs` / `endMs`). `SurpriseOdds` ma pola `u16` (promile). ADR-023 dokłada jedyną liczbę
ze znakiem, `SmartBreak.focusDeltaSecs: i32` (skrócenie < 0), i współrzędne `f64` (`TintLocation`, `SunCity`), przez które `TintConfig` nie ma już `Eq`.
Eksport: `cargo test -p focus-core` zapisuje do `TS_RS_EXPORT_DIR` (ustawione w `.cargo/config.toml` względem workspace).
**CI sprawdza, czy po regeneracji `src/lib/bindings/` nie ma zmian** (`git diff --exit-code`).
Nazwy komend i eventów są stałymi w `api.rs` (`commands::*`, `events::*`), które test `export_names` zapisuje do `src/lib/bindings/names.ts`
(`COMMANDS`, `EVENTS`, `BRIDGE_PORT`), więc `api.ts` nie wpisuje ich ręcznie. Zmiany kontraktu przechodzą przez skill `/sync-contract`.

### 6.1 Komendy

Cienkie adaptery w `src-tauri/src/commands.rs`; logika w aktorze sesji, w wykonawcach efektów i w core.
Każda komenda zwraca `Result<T, AppError>`, gdzie `AppError { code, message, field }`: `code` to stabilny kod (`config.invalid`, `audio.device`,
`shields.notifications`, `teams.auth`, `media.url`…), `message` to szczegół po angielsku do logów (UI mapuje `code` na polski tekst),
a `field` to pole configu, które nie przeszło walidacji (`focusMinutes`, `music.volume`…). Argumenty `invoke` są w camelCase.

| Komenda | Argumenty | Zwraca | Uwagi |
|---|---|---|---|
| `get_snapshot` | – | `SessionSnapshot` | |
| `start_session` | – | `()` | sesja z bieżącym configiem (albo z profilem demo) |
| `pause_session` | – | `()` | zamraża pozostały czas bieżącej fazy (Focus albo Przerwa) |
| `resume_session` | – | `()` | |
| `stop_session` | – | `()` | idempotentna, działa w każdej fazie |
| `skip_phase` | – | `()` | Focus → Przerwa od razu, Przerwa → następny Focus od razu |
| `get_config` | – | `Config` | |
| `update_config` | `config: Config` | `Config` | znormalizowany, zwalidowany i zapisany |
| `list_folder_tracks` | `path: string` | `Track[]` | skan folderu; znalezione pliki audio trafiają do scope asset protocol |
| `get_shield_status` | – | `ShieldStatus` | |
| `get_current_usage` | – | `UsageReport` | użycie aplikacji w trwającej sesji (puste w Idle) |
| `get_last_summary` | – | `SessionSummary \| null` | |
| `list_history` | `limit: number` | `SessionSummary[]` | od najnowszej |
| `get_teams_status` | – | `TeamsStatus` | |
| `teams_connect` | – | `DeviceCodePrompt` | start logowania device code; postęp przychodzi jako `teams://status`. O `Calendars.Read` prosi tylko przy włączonym `calendar.microsoft` (w zapisanym configu); przy już połączonym koncie to „Połącz ponownie” (stare logowanie działa, dopóki nowe się nie uda) |
| `teams_disconnect` | – | `()` | zapomina zapisany token |
| `get_bridge_info` | – | `BridgeInfo` | port mostu, folder rozszerzenia, kiedy rozszerzenie ostatnio pytało |
| `open_extension_dir` | – | `()` | otwiera folder dołączonego rozszerzenia w menedżerze plików |
| `test_chime` | `chime: ChimeKind` | `()` | dzwonek z głośnością dzwonków z configu |
| `open_media_window` | `url: string, play: boolean` | `()` | otwiera (albo przekierowuje) osobne okno YouTube, które gra link jako stronę najwyższego poziomu. Dla linków odmawiających osadzania (błędy YouTube 101/150/153) albo takich, których nie da się osadzić (ADR-013 pkt 2). Tylko https YouTube / YouTube Music, inaczej `media.url` |
| `control_media_window` | `action: MediaAction, volume: number \| null` | `()` | `MediaAction`: `play` / `pause` / `setVolume` / `show` (pokazuje, przywraca i aktywuje okno, np. do akceptacji strony zgody YouTube) / `close`. `volume` (0..1) tylko dla `setVolume`. Przy zamkniętym oknie no-op |
| `get_media_window` | – | `MediaWindowStatus` | |
| `get_blocklist` | – | `BlocklistInfo` | wczytana lista `websiteblacklist.txt` (przy pierwszym starcie plik powstaje z domyślnej listy wbudowanej w aplikację) |
| `open_blocklist_file` | – | `()` | otwiera `websiteblacklist.txt` w domyślnym edytorze tekstu systemu |
| `reload_blocklist` | – | `BlocklistInfo` | wczytuje plik od razu (po zapisaniu i tak wczytuje się sam) |
| `resolve_close` | `toTray: boolean` | `Config` | odpowiedź na `app://close-requested` (pierwsze X na oknie głównym): zapisuje `closeToTray` (przy `toTray` włącza też `phasePopups`), potem chowa okno albo kończy aplikację (ADR-014) |
| `get_system_language` | – | `Language` | obsługiwany język najbliższy językowi wyświetlania systemu (`en`, gdy żaden nie pasuje); nigdy `system` |
| `get_popup` | – | `PopupInfo` | co pokazuje teraz okienko faz (dla okna `popup`) |
| `popup_action` | `action: PopupAction` | `()` | przycisk w okienku albo jego timeout: `dismiss` (zamknij), `showMain` (pokaż i aktywuj okno główne), `skipPhase` (jak „Pomiń fazę” w zasobniku) |
| `get_tint_status` | – | `TintStatus` | |
| `preview_tint` | `strength: number \| null` | `()` | podgląd ciepła o sile 0..100 przez kilka sekund, mimo harmonogramu (Ustawienia), wybranym mechanizmem (Oświetlenie nocne albo gamma, ADR-015); `null` kończy podgląd od razu |
| `import_chime` | `path: string, chime: ChimeKind` | `Config` | kopiuje plik audio (mp3/wav/ogg/flac/m4a, najwyżej 10 MB, musi się dekodować) do folderu `sounds` w danych aplikacji jako własny dźwięk tego dzwonka i wybiera `chimeSound: custom` (ADR-015) |
| `remove_custom_chime` | `chime: ChimeKind` | `Config` | usuwa własny dźwięk tego dzwonka; potem gra plik drugiego dzwonka, a gdy nie ma żadnego, dźwięk domyślny |
| `list_library_tracks` | – | `Track[]` | muzyka dołączona do aplikacji (Open Lo-Fi, CC0); te pliki trafiają do scope asset protocol, odtwarzane przez `convertFileSrc(path)` |
| `clear_history` | – | `()` | usuwa całą historię sesji (`history.jsonl` i ostatnie podsumowanie); UI najpierw prosi o potwierdzenie |
| `get_calendar` | – | `CalendarStatus` | spotkania z najbliższych `calendar.horizonHours` godzin ze wszystkich włączonych kalendarzy i ich kolizje z planowanymi przerwami: trwającej sesji albo sesji, która zaczęłaby się teraz (`planned`) (ADR-016) |
| `refresh_calendar` | – | `CalendarStatus` | pobiera wszystkie włączone kalendarze od razu |
| `set_meeting_choice` | `meetingId: string, choice: MeetingChoice` | `CalendarStatus` | jak potraktować spotkanie kolidujące z przerwą: `extend` / `pauseAtBreak` trafiają do sesji jako blokady (`Input::SetMeetingHolds`; dla spotkania dłuższego niż 4 h błąd `calendar.meetingTooLong`), `remindLater` planuje okienko `meetingReminder` (30 / 20 / 10 min przed spotkaniem; gdy nie ma już etapu, błąd `calendar.remindLater`), `ignore` nic nie zmienia |
| `connect_google` | – | `()` | logowanie Google w przeglądarce systemowej (OAuth dla aplikacji instalowanych: loopback + PKCE, zakres tylko do odczytu; `calendar.googleClientSecret` wysyłany do punktu tokenów, gdy nie jest pusty); postęp w `calendar://changed` (`googlePending`, stan źródła `google`) |
| `disconnect_calendar` | `source: CalendarSource` | `CalendarStatus` | `google` / `ics`: zapomina zapisany token albo linki. `microsoft`: **nie usuwa tokenu Teams** (status w Teams działa dalej), tylko wyłącza `calendar.microsoft` (zapis jak przez `update_config`; UI ustawia to pole u siebie bez ponownego zapisu) i usuwa spotkania Microsoft |
| `set_ics_urls` | `urls: string[]` | `CalendarStatus` | zastępuje linki ICS; zapis w pliku szyfrowanym DPAPI, nigdy w `config.json` ani w logach |
| `get_ics_urls` | – | `string[]` | zapisane linki ICS (do edycji w Ustawieniach) |
| `get_work` | – | `WorkStatus` | lista „Co dalej?”: pozycje z włączonych narzędzi pracy w kolejności `focus_core::work::rank` (najwyżej 50) i stan każdego narzędzia (ADR-017) |
| `refresh_work` | – | `WorkStatus` | pobiera wszystkie włączone narzędzia od razu |
| `set_work_token` | `source: WorkSource, token: string \| null` | `WorkStatus` | zapisuje token narzędzia w pliku szyfrowanym DPAPI (`work-<źródło>-token.bin`), nigdy w `config.json` ani w logach; `null` usuwa token. Token: 1–4096 widocznych znaków ASCII (inaczej `work.token`, pole `token`); błąd zapisu → `work.tokenSave`. Tokenu nie da się odczytać z powrotem |
| `set_session_goal` | `goal: SessionGoal \| null` | `()` | cel sesji (pozycja z „Co dalej?”), w każdej fazie, bez efektów (`Input::SetGoal`); widoczny w `SessionSnapshot.goal`, na Stop trafia do `SessionSummary.goal`; `null` czyści |
| `mark_goal_done` | `sessionId: string, done: boolean` | `()` | odpowiedź na „Zrobione?” w podsumowaniu: `goalDone` w ostatnim podsumowaniu i przy tej sesji w `history.jsonl` (powłoka przepisuje plik atomowo). Sesji nie ma ani w pamięci, ani w pliku → `history.unknownSession`; błąd zapisu → `persistence.history`; przy wyłączonym `keepHistory` zmienia tylko ostatnie podsumowanie |
| `dev_trigger` | `action: DevAction` | `()` | przycisk strony Deweloper (ADR-019, §6.11): przykładowe okienko, toast fazy, skrócenie bieżącej fazy, tryb prezentacji (profil demo) albo przywrócenie ustawień fabrycznych z restartem; przy wyłączonym `developerMode` → `developerModeOff`. Nic nie wysyła; na dysku zmienia coś tylko reset fabryczny (usuwa dane aplikacji) |
| `get_privacy_report` | – | `PrivacyReport` | co aplikacja trzyma na tym komputerze, pozycja po pozycji (ADR-020, §6.12): obecność, rozmiar w KiB, liczba wpisów. Nigdy treść tokenu, linku ICS ani ciasteczka |
| `open_data_folder` | – | `()` | otwiera folder danych aplikacji (`PrivacyReport.folders[0]`) w menedżerze plików |
| `export_privacy_data` | `path: string` | `()` | jeden plik JSON pod ścieżką z okna zapisu: config bez sekretów, historia, wybory dla spotkań, widziane pozycje „Co dalej?”, księga nagród `rewards.json`, gdy istnieje (ADR-022) (format w §6.12). **Nigdy** tokeny, linki ICS, ciasteczka ani inne sekrety. Błąd → `privacy.export` |
| `delete_privacy_data` | `kinds: PrivacyItemKind[]` | `Config` | usuwa wskazane pozycje (tylko `deletable`, inaczej `privacy.notDeletable`, pole `kinds`); trwającą sesję najpierw zatrzymuje (osłony wracają, zasada 7). Zwraca zapisany config (usunięcie własnych dzwonków go zmienia); nowy raport przychodzi jako `privacy://changed`. Błąd → `privacy.delete` |
| `delete_all_data` | – | `Config` | „Usuń wszystko”: zatrzymuje sesję, usuwa wszystkie pozycje `deletable` i przywraca ustawienia domyślne (zwraca je). Nie rusza niczego poza folderami aplikacji; aplikacja zostaje zainstalowana |
| `get_youtube_account` | – | `YoutubeAccount` | stan zalogowania do YouTube we wspólnym profilu WebView2 (ADR-021, §6.13); tylko obecność ciasteczek, nigdy ich wartości. Gdy stan jest wciąż nieznany i nic nie sprawdza, uruchamia jeszcze jedno sprawdzenie (wynik jako `youtube://account`; „Sprawdź ponownie” w UI) |
| `youtube_sign_in` | – | `()` | otwiera (albo aktywuje) widoczne okno logowania `youtube-login` ze stroną Google; zamyka się samo, gdy pojawią się ciasteczka logowania youtube.com. Postęp w `youtube://account` |
| `youtube_sign_out` | – | `()` | usuwa ciasteczka google.com / youtube.com profilu WebView2 (ciasteczka zgody `SOCS` / `CONSENT` zostają) |
| `get_rewards` | – | `RewardsStatus` | iskierki, cel dnia, poziom, seria, tydzień i niepotwierdzone momenty (funkcje beta, ADR-022, §6.14); okno główne |
| `ack_reward_moments` | `upTo: number` | `()` | usuwa z `pending` momenty o `id <= upTo` (zagrane albo pokazane); tylko okno główne, okienko `popup` nigdy nie potwierdza |
| `reset_rewards` | – | `RewardsStatus` | „Zacznij od nowa”: nowa księga z nowym ziarnem, podgląd demo znika; historii sesji nie rusza. UI najpierw pyta |
| `play_reward_sound` | `sound: RewardSound` | `()` | krótki dźwięk nagrody (`pop` / `sparkle` / `fanfare`); no-op, gdy nagrody są nieaktywne, dźwięki nagród wyłączone, `celebrations: numbersOnly` albo użytkownik prezentuje; czeka, aż skończy się dzwonek fazy |
| `set_session_steps` | `steps: StepDraft[]` | `()` | zastępuje małe kroki nadchodzącej albo trwającej sesji (każda faza); `id` = istniejący krok (zachowuje `done`), `null` = nowy. Błędy: `stepsOff` (małe kroki wyłączone), `tooManySteps` (pole `steps`, więcej niż 5). Nowy plan przychodzi w `session://snapshot`. Tekstów kroków nikt nie loguje |
| `set_step_done` | `id: number, done: boolean` | `()` | odhacza / odznacza krok (Focus albo Przerwa, także w pauzie); w Idle albo dla nieznanego `id` no-op; `stepsOff` przy wyłączonych krokach |
| `list_sun_cities` | – | `SunCity[]` | wbudowana lista miast (ok. 40, w tym wszystkie miasta wojewódzkie) dla harmonogramu „zachód → wschód słońca” (ADR-023, §6.15); dane w aplikacji (`focus_core::sun::CITIES`, 43 miasta), bez usług lokalizacji i bez sieci. Wdrożone |
| `set_smart_break_opt_out` | `meetingId: string, optOut: boolean` | `CalendarStatus` | „Zapytaj mnie” dla jednego spotkania dopasowanego przez inteligentne przerwy (ADR-023): `true` cofa dopasowanie (sesja dostaje z powrotem niezmieniony koniec fazy przez aktora, spotkanie dostaje zwykłe pytanie), `false` pozwala dopasować je znowu (i zapomina wybór podany w międzyczasie, bo spotkanie z wyborem nie jest dopasowywane). Zapis w `meeting-choices.json` (`optOuts`: id i koniec spotkania). Nigdy nie blokuje Stop / Pomiń / Pauzy. Próbka ze strony Deweloper (`dev-sample-…`) to no-op. Nieznane spotkanie przy `true` → `calendar.unknownMeeting`. Wdrożone |
| `get_weekly_stats` | `weekStart: string` | `WeeklyStats` | tydzień ISO (od poniedziałku, czas lokalny) zawierający dzień `weekStart` (`RRRR-MM-DD`) z `history.jsonl` (ADR-023, §6.15); zła data → `stats.week` (pole `weekStart`), błąd odczytu → `persistence.history`. Wdrożone (`src-tauri/src/weekly.rs`) |

Most dla rozszerzenia nie jest komendą Tauri: `GET http://127.0.0.1:47615/v1/state` → `BridgeState { active, blocklist, patterns, phase, remainingSecs }`
(port to stała `BRIDGE_PORT` w `api.rs`). `blocklist` to znormalizowane domeny (każda blokuje też swoje subdomeny), a `patterns` to źródła
wyrażeń regularnych (RE2) dopasowywanych do pełnego adresu URL bez rozróżniania wielkości liter. Poza blokadą obie listy są puste.
`patterns` ma `#[serde(default)]`, więc stan bez tego pola też się wczytuje.
Jedyny zapis do mostu (ADR-018): `POST http://127.0.0.1:47615/v1/report` z treścią `{ "skipped": string[] }`, w którym rozszerzenie zgłasza wzorce
z `patterns`, których przeglądarka nie przyjęła jako reguł; aplikacja pokazuje je jako `BlocklistInfo.extensionSkipped`. Warunki i limity: §6.10.
HTTP mostu to własny minimalny serwer HTTP/1.1 (`src-tauri/src/bridge/http.rs`, bez `tiny_http`): jedno zapytanie na połączenie (`Connection: close`),
limity rozmiaru i czasu, treść tylko dla `/v1/report`; szczegóły w „Ochrona HTTP mostu” (§6.10).

### 6.2 Eventy

Emituje tylko powłoka; nazwy `obszar://nazwa`.

| Event | Payload | Kiedy |
|---|---|---|
| `session://snapshot` | `SessionSnapshot` | co tick (1 s) i po każdej zmianie |
| `session://phase-changed` | `PhaseChange { from, to, cycle }` | przy każdym przejściu fazy |
| `session://summary` | `SessionSummary` | raz, gdy sesja się kończy (Stop) |
| `shields://status` | `ShieldStatus` | gdy któraś osłona zmienia stan |
| `teams://status` | `TeamsStatus` | gdy zmienia się połączenie z Teams (także postęp logowania) |
| `focus://nudge` | `Nudge` | awaryjny nudge: tracker wykrył stronę z listy w tytule okna przeglądarki (bez rozszerzenia); `matched` to domena albo źródło regexu |
| `app://error` | `AppError` | integracja zawiodła; sesja trwa (degradacja) |
| `media://window` | `MediaWindowStatus` | okno YouTube się otwiera, zamyka, przechodzi na inny adres albo zmienia tytuł |
| `blocklist://changed` | `BlocklistInfo` | `websiteblacklist.txt` wczytany ponownie (automatycznie albo przez `reload_blocklist`) z innym wynikiem, albo zgłoszenie rozszerzenia (`POST /v1/report`) zmieniło `extensionSkipped` (ADR-018) |
| `app://close-requested` | `null` | do okna głównego, gdy użytkownik kliknął X, a `closeToTray` jest jeszcze `null`: UI pyta i odpowiada `resolve_close` |
| `popup://show` | `PopupInfo` | okienko faz ma pokazać komunikat (albo się schować: `kind: null`) |
| `tint://changed` | `TintStatus` | zmienił się poziom ciepła, podgląd, użyty mechanizm (`mode`), ograniczenie siły (`capped`) albo błąd |
| `calendar://changed` | `CalendarStatus` | po każdej synchronizacji kalendarzy, zmianie wyboru dla spotkania, zmianie połączenia (także postęp logowania Google) i zmianie kolizji |
| `work://changed` | `WorkStatus` | po każdej synchronizacji narzędzi pracy, zmianie tokenu i zmianie wstrzymanych pozycji (`heldCount`, `unseen`) |
| `privacy://changed` | `PrivacyReport` | po każdej zmianie zapisanych danych (ponowny skan z opóźnieniem, wysyłany tylko przy innym wyniku) oraz na początku i końcu usuwania albo eksportu (`busy`) (ADR-020) |
| `youtube://account` | `YoutubeAccount` | po pierwszym sprawdzeniu ciasteczek, przy otwarciu i zamknięciu okna logowania, po zalogowaniu, wylogowaniu, odmowie Google (`browserRejected`) i po usunięciu danych przeglądarki (ADR-021) |
| `rewards://changed` | `RewardsStatus` | po każdym zastosowanym fakcie sesji, potwierdzeniu, resecie, zmianie configu, zmianie dnia nagród i po wczytaniu (ADR-022); przy wyłączonych funkcjach beta nigdy z `active: true` |
| `history://changed` | `HistoryChanged { revision }` | zmieniła się zapisana historia (zapis sesji, „Zrobione?”, „Usuń historię”, usunięcie w panelu prywatności) albo `Config.appCategories` (ADR-023); UI pyta `get_weekly_stats` jeszcze raz o pokazywany tydzień |

**Każdy stan widoku ma parę getter + event z rewizją** (ADR-012, R06):

| Stan | Getter | Event |
|---|---|---|
| `SessionSnapshot` | `get_snapshot` | `session://snapshot` |
| `ShieldStatus` | `get_shield_status` | `shields://status` |
| `TeamsStatus` | `get_teams_status` | `teams://status` |
| `MediaWindowStatus` | `get_media_window` | `media://window` |
| `BlocklistInfo` | `get_blocklist` (także wynik `reload_blocklist`) | `blocklist://changed` |
| `PopupInfo` | `get_popup` | `popup://show` |
| `TintStatus` | `get_tint_status` | `tint://changed` |
| `CalendarStatus` | `get_calendar` (także wynik `refresh_calendar`, `set_meeting_choice`, `disconnect_calendar` i `set_ics_urls`) | `calendar://changed` |
| `WorkStatus` | `get_work` (także wynik `refresh_work` i `set_work_token`) | `work://changed` |
| `PrivacyReport` | `get_privacy_report` | `privacy://changed` |
| `YoutubeAccount` | `get_youtube_account` | `youtube://account` |
| `RewardsStatus` | `get_rewards` (także wynik `reset_rewards`) | `rewards://changed` |
| `WeeklyStats` | `get_weekly_stats { weekStart }` | `history://changed` (sama rewizja: UI pyta znowu o swój tydzień) |

Pole `revision` ustawia powłoka i rośnie z każdym wysłanym stanem. UI, dla każdego z tych stanów:
1. **najpierw** rejestruje `listen(...)`,
2. potem wywołuje getter,
3. stosuje dane tylko wtedy, gdy `revision` jest większa od już posiadanej (chroni przed starszą odpowiedzią gettera).

`WeeklyStats` to jedyny getter z parametrem (ADR-023): event niesie tylko rewizję historii, a `api.watchWeeklyStats` nasłuchuje go najpierw,
potem czyta wybrany tydzień i czyta go znowu po każdej zmianie. Stosuje tylko odpowiedź na najnowsze pytanie i dla tego samego tygodnia
nigdy odpowiedzi ze starszą `revision` (wolna odpowiedź o poprzedni tydzień ani starsza nie cofną widoku).

`session://phase-changed`, `session://summary`, `focus://nudge`, `app://error` i `app://close-requested` to zdarzenia jednorazowe, bez rewizji
(ostatnie podsumowanie da się też pobrać przez `get_last_summary`).

### 6.3 Snapshot sesji i okna

```rust
pub struct SessionSnapshot {
    pub revision: u32,              // ustawia powłoka; rośnie z każdym wysłanym snapshotem
    pub session_id: Option<String>,
    pub phase: Phase,               // Idle | Focus | Break
    pub paused: bool,               // pauza użytkownika albo automatyczna po luce zegara; ma sens tylko w Focus/Break
    pub cycle: u32,                 // numer bieżącego cyklu skupienia od 1 (0 w Idle)
    pub remaining_secs: u32,        // pozostało w bieżącej fazie (zamrożone w pauzie)
    pub phase_total_secs: u32,      // długość bieżącej fazy (0 w Idle); z przedłużeniem przez spotkanie (ADR-016)
    pub focus_secs: u32,            // profil w użyciu (z configu albo demo)
    pub break_secs: u32,
    pub focused_secs: u32,          // czas skupienia faktycznie spędzony w tej sesji (bez pauz i luk)
    pub demo: bool,                 // profil demo (`--demo` / `FOCUSMODE_DEMO=1`)
    #[serde(default)]
    pub music_muted: bool,          // muzyka ma milczeć: Focus (także w pauzie) w trakcie spotkania z wyborem `extend` (ADR-016)
    #[serde(default)]
    pub meeting_hold: Option<MeetingHoldView>, // spotkanie przedłuża focus albo wstrzymuje przerwę (do Wznów)
    #[serde(default)]
    pub goal: Option<SessionGoal>,  // cel sesji z „Co dalej?” (ADR-017); widoczny w Idle i w trakcie sesji, czyszczony na Stop
    #[serde(default)]
    pub steps: Vec<MicroStep>,      // małe kroki nadchodzącej (Idle) albo trwającej sesji (funkcje beta, ADR-022); puste przy wyłączonych krokach
    #[serde(default)]
    pub smart_break: Option<SmartBreak>, // przerwa po bieżącym bloku focusu (albo trwająca) przesunięta przez inteligentne przerwy (ADR-023):
                                    // linia pod zegarem i jej cofnięcie; bez tematu spotkania; `None` w Idle i przy wyłączonych `smartBreaks`
}
```

Odtwarzacz (osadzony i okno YouTube) milknie, gdy `musicMuted`, i wraca do zwykłego zachowania, gdy flaga zgaśnie. Wstrzymana przerwa
(`pauseAtBreak`) ma już `paused: true`, więc muzyka stoi jak w każdej pauzie.

**Kto włączył muzykę** (odtwarzacz, tylko frontend: `MusicIntent` w `src/lib/state/music.svelte.ts`, stosuje `MusicPlayer.svelte`;
kontrakt IPC bez zmian). Przycisk Odtwórz/Wstrzymaj w pasku odtwarzacza działa także bez sesji (każde źródło, także okno YouTube
po zalogowaniu; „Następny” i głośność też). Stan jest tylko w pamięci, więc start aplikacji nigdy sam nie włącza muzyki.

| Stan | Skąd | Start | Stop |
|---|---|---|---|
| `off` (cisza) | start aplikacji; Stop muzyki sesji | → `session`: sesja włącza muzykę (nowe tasowanie playlisty, jak dotąd) | – |
| `session` | Start z `off` | – | → `off`: muzyka staje (playlista od nowa, jak dotąd) |
| `user` | Odtwórz w pasku (w Idle albo w trakcie sesji) | gra dalej bez restartu i bez zmiany utworu; gdy nic nie gra (wszystkie pozycje zawiodły, okno YouTube zamknięte) → `session`, jak z `off` | gra dalej; jeśli sesja trzymała ją w tej chwili w ciszy (niżej) → `off`, więc Stop nigdy nie włącza muzyki |
| `userPaused` | Wstrzymaj w pasku | zostaje w ciszy | zostaje w ciszy |

Pauza w pasku zawsze wygrywa: wstrzymana przez użytkownika muzyka milczy przez zmiany faz, Start i Stop, aż użytkownik naciśnie Odtwórz.
W trakcie sesji sesja może trzymać muzykę w ciszy bez względu na to, kto ją włączył: Pauza, przerwa przy `music.pauseDuringBreak`
i wyciszenie na spotkanie (`musicMuted`); sama ją potem przywraca (Wznów, koniec przerwy, koniec spotkania). Odtwórz naciśnięte w czasie
takiej ciszy gra mimo niej (`override`), aż ta cisza się skończy; następna znów wycisza. Zmiana źródła w trakcie grania (wybór źródła
jest na stronie Idle) trafia do playlisty dopiero po 0,8 s bez kolejnych zmian, więc wpisywanie linku nie restartuje muzyki przy każdym
znaku ani nie próbuje odtworzyć połowy adresu. Okno `popup` dalej nie ma odtwarzacza.
Zasobnik nie ma pozycji muzyki (wymagałaby nowego eventu do WebView, czyli zmiany kontraktu), a `navigator.mediaSession` nie jest używane.

| Okno | Zawartość | Uprawnienia IPC |
|---|---|---|
| `main` | SPA z grupy tras `(app)` (Skupienie / Podsumowanie / Ustawienia, przy `developerMode` także Deweloper, ADR-019) + odtwarzacz muzyki w layoucie. Nigdy nie jest niszczone: X chowa je do zasobnika albo kończy aplikację, zależnie od `closeToTray`; dopóki wyboru nie ma, X wysyła `app://close-requested` (ADR-014) | wszystkie komendy (capability `default`) |
| `media` | strona YouTube / YouTube Music jako strona najwyższego poziomu (fallback, ADR-013 pkt 2; po zalogowaniu do YouTube cała muzyka z YouTube, ADR-021) | **żadnych**: powłoka steruje nim przez `eval` wstrzykniętego kontrolera, a stan wraca do UI jako `media://window` |
| `youtube-login` | widoczne, wyśrodkowane okno ze stroną logowania Google (ADR-021, §6.13), ten sam profil WebView2 co `media`; zamyka się samo po zalogowaniu | **żadnych** (strona zdalna, bez skryptu kontrolera); stan wraca do UI jako `youtube://account` |
| `popup` | okienko faz z grupy tras `(overlay)`: małe, w prawym dolnym rogu ekranu, zawsze na wierzchu, bez paska zadań i bez fokusu; tylko gdy `phasePopups` (ADR-014); także przypomnienie o spotkaniu `meetingReminder`, wstrzymana przerwa `breakHeld` (ADR-016) i paczka nowych pozycji „Co dalej?” `workDigest` (ADR-017). Gdy użytkownik prezentuje albo gra na pełnym ekranie, okienko czeka (ADR-018, §6.10); przy `showMascot` z maskotką „zza krawędzi” | potrzebuje tylko `get_popup`, `popup_action`, `get_config` (dobór treści i motywu, `showMascot`) i `popup://show`, a przy `meetingReminder` także `set_meeting_choice` |

Okien przerwy i ostrzeżenia nie ma (ADR-013), więc odpadają `window_ready`, heartbeat, watchdog i `break_id`.
Okien overlay odcienia (`tint-*`) też już nie ma: ciepło ekranu daje Oświetlenie nocne Windows albo gamma ramp (ADR-015), a `TintStatus` czyta tylko okno główne.
Strony z grupy `(overlay)` nie mają odtwarzacza muzyki ani nawigacji. Capability `overlays.json` daje oknu `popup` tylko
`core:event:default` (listen/unlisten), więc wtyczek (opener, dialog, autostart…) wywołać nie może. Komendy aplikacji Tauri przepuszcza lokalnym
oknom bez ACL, bo aplikacja nie ma manifestu komend: twarda izolacja komend per okno (manifest w `build.rs`, §10, B-12) nadal nie jest zrobiona,
więc kolumna „Uprawnienia IPC” dla `popup` opisuje, czego to okno używa, a nie egzekwowaną granicę.

### 6.4 Przykłady JSON

`SessionSnapshot` (demo, przerwa po pierwszym cyklu):

```json
{ "revision": 42, "sessionId": "1760350000000-12345-1", "phase": "break", "paused": false, "cycle": 1,
  "remainingSecs": 12, "phaseTotalSecs": 20, "focusSecs": 40, "breakSecs": 20, "focusedSecs": 40, "demo": true,
  "musicMuted": false, "meetingHold": null }
```

`SessionSnapshot` (profil 45/15, focus przedłużony o 20 min przez spotkanie z wyborem `extend`, spotkanie trwa):

```json
{ "revision": 1804, "sessionId": "1760350000000-12345-1", "phase": "focus", "paused": false, "cycle": 2,
  "remainingSecs": 900, "phaseTotalSecs": 3900, "focusSecs": 2700, "breakSecs": 900, "focusedSecs": 5700, "demo": false,
  "musicMuted": true,
  "meetingHold": { "subject": "Daily zespołu", "untilMs": 1760362200000, "choice": "extend" } }
```

`ShieldStatus` (Focus, Teams wyłączone w ustawieniach, na starcie focusa zamknięto Steam i Epic):

```json
{ "revision": 7, "active": true,
  "notifications": { "state": "on", "detail": null },
  "teams": { "state": "off", "detail": null },
  "sites": { "state": "on", "detail": null },
  "keepAwake": { "state": "on", "detail": null },
  "launchers": { "state": "on", "detail": null },
  "closedLaunchers": ["steam", "epic"],
  "extensionConnected": true }
```

`Config`, tylko pola dodane przez ADR-014 (wartości domyślne; `tint.mode` z ADR-015):

```json
{ "language": "system", "closeToTray": null, "phasePopups": true,
  "tint": { "enabled": false, "startMinutes": 1080, "endMinutes": 540, "strength": 45, "transitionMinutes": 30, "mode": "nightLight" },
  "shields": { "closeLaunchers": false, "launchers": ["steam", "epic", "gog", "ubisoft", "ea", "battleNet", "riot"] } }
```

`Config`, tylko pola dodane przez ADR-015 (wartości domyślne):

```json
{ "theme": "midnight", "chimeSound": "midnight", "customChimes": { "breakFile": null, "focusFile": null },
  "breakReminders": [], "repeatReminders": false, "trackTitles": true, "keepHistory": true,
  "music": { "source": "library" } }
```

`PopupInfo` (okienko na start drugiej przerwy, profil 45/15, z przypomnieniem użytkownika):

```json
{ "revision": 5, "kind": "breakStarted", "cycle": 2, "phaseSecs": 900, "launchers": [], "reminder": "woda i spacer z psem", "meeting": null }
```

`TintStatus` (18:15, domyślny harmonogram: połowa przejścia, więc połowa siły 45; Oświetlenie nocne):

```json
{ "revision": 12, "enabled": true, "level": 23, "preview": false, "active": true, "mode": "nightLight", "capped": false, "error": null }
```

`AppUsage` w `UsageReport.apps` z tytułami okien (`trackTitles` włączone; najwyżej 5 tytułów):

```json
{ "process": "chrome.exe", "secs": 1260,
  "titles": [ { "title": "Pull request #42 · focusmode - Google Chrome", "secs": 840 },
              { "title": "Tauri 2 docs - Google Chrome", "secs": 300 } ] }
```

Argumenty nowych komend: `resolve_close` → `{ "toTray": true }`, `popup_action` → `{ "action": "skipPhase" }`,
`preview_tint` → `{ "strength": 60 }` albo `{ "strength": null }`, `import_chime` → `{ "path": "C:\\Users\\ola\\Music\\gong.mp3", "chime": "breakStart" }`,
`remove_custom_chime` → `{ "chime": "focusStart" }`, `set_meeting_choice` → `{ "meetingId": "…", "choice": "remindLater" }`,
`disconnect_calendar` → `{ "source": "google" }`, `set_ics_urls` → `{ "urls": ["https://…/calendar.ics"] }`.

`CalendarStatus` (Idle, czyli przerwy sesji, która zaczęłaby się teraz; Microsoft połączony, ICS z dwoma linkami, Google wyłączony;
jedno spotkanie nachodzi na 2. przerwę i ma wybrane „przypomnij później”):

```json
{ "revision": 9,
  "sources": [ { "source": "microsoft", "state": "on", "detail": null, "account": "ola@firma.pl" },
               { "source": "google", "state": "off", "detail": null, "account": null },
               { "source": "ics", "state": "on", "detail": null, "account": "2" } ],
  "meetings": [ { "id": "…", "source": "microsoft", "subject": "Przegląd sprintu", "startMs": 1760356800000, "endMs": 1760360400000, "online": true } ],
  "conflicts": [ { "meeting": { "id": "…", "source": "microsoft", "subject": "Przegląd sprintu", "startMs": 1760356800000, "endMs": 1760360400000, "online": true },
                   "breakNumber": 2, "breakStartMs": 1760356300000, "breakEndMs": 1760357200000,
                   "choice": "remindLater", "nextReminderMs": 1760355000000, "remindLaterAllowed": true, "tooLong": false } ],
  "planned": true, "lastSyncMs": 1760350000000, "googlePending": false }
```

Profil 45/15 od `lastSyncMs`: 2. przerwa to minuty 105–120, spotkanie trwa od minuty 113 do 173 (nachodzi też na 3. przerwę, ale kolizja jest zgłaszana raz,
z pierwszą przerwą). „Przypomnij później” wybrane na stronie Skupienie daje okienko 30 min przed spotkaniem (`nextReminderMs`).

`PopupInfo` (ostatnie przypomnienie o tym spotkaniu, 10 min przed, w 2. cyklu sesji uruchomionej w chwili `lastSyncMs`: bez „przypomnij później”;
`phaseSecs` ma znaczenie tylko w okienkach faz):

```json
{ "revision": 14, "kind": "meetingReminder", "cycle": 2, "phaseSecs": 0, "launchers": [], "reminder": null,
  "meeting": { "meeting": { "id": "…", "source": "microsoft", "subject": "Przegląd sprintu", "startMs": 1760356800000, "endMs": 1760360400000, "online": true },
               "breakNumber": 2, "breakStartMs": 1760356300000, "breakEndMs": 1760357200000,
               "choice": "remindLater", "nextReminderMs": null, "remindLaterAllowed": false, "tooLong": false } }
```

`Config`, tylko pole dodane przez ADR-016 (wartości domyślne):

```json
{ "calendar": { "microsoft": false, "google": false, "googleClientId": "", "googleClientSecret": "", "ics": false, "horizonHours": 10 } }
```

`Config`, tylko pole dodane przez ADR-017 (wartości domyślne):

```json
{ "work": { "github": false, "gitlab": false, "gitlabUrl": "https://gitlab.com", "jira": false, "jiraUrl": "", "jiraEmail": "",
            "azureDevops": false, "azureOrg": "", "linear": false, "holdDuringFocus": true } }
```

`Config`, tylko pole dodane przez ADR-018 (wartość domyślna):

```json
{ "showMascot": true }
```

`WorkStatus` (GitHub, Jira i Linear włączone; w chwili `lastSyncMs` trwa przerwa, a jedna pozycja przyszła w trakcie focusa i czeka jako nowa):

```json
{ "revision": 23,
  "sources": [ { "source": "github", "state": "on", "detail": null, "account": "ola-dev" },
               { "source": "gitlab", "state": "off", "detail": null, "account": null },
               { "source": "jira", "state": "on", "detail": null, "account": "Ola Nowak" },
               { "source": "azureDevops", "state": "off", "detail": null, "account": null },
               { "source": "linear", "state": "on", "detail": null, "account": "Ola" } ],
  "items": [
    { "id": "jira:SHOP-128", "source": "jira", "kind": "assigned", "key": "SHOP-128", "title": "Płatność BLIK w koszyku",
      "url": "https://firma.atlassian.net/browse/SHOP-128", "project": "Sklep", "priorityLabel": "High", "priorityRank": 1,
      "dueMs": 1760392799999, "status": "In Progress", "inSprint": true, "updatedMs": 1760256400000,
      "score": 3013210026, "reasons": ["assigned", "inSprint", "highPriority", "dueToday", "inProgress"], "unseen": false },
    { "id": "github:firma/sklep#42", "source": "github", "kind": "reviewRequested", "key": "firma/sklep#42", "title": "Cache koszyka w Redisie",
      "url": "https://github.com/firma/sklep/pull/42", "project": "firma/sklep", "priorityLabel": null, "priorityRank": null,
      "dueMs": null, "status": null, "inSprint": false, "updatedMs": 1760004400000,
      "score": 2302000096, "reasons": ["blockingOthers", "waitingLong"], "unseen": false },
    { "id": "linear:ENG-7", "source": "linear", "kind": "notification", "key": "ENG-7", "title": "Eksport raportu do CSV",
      "url": "https://linear.app/firma/issue/ENG-7", "project": "Engineering", "priorityLabel": null, "priorityRank": null,
      "dueMs": null, "status": null, "inSprint": false, "updatedMs": 1760342800000,
      "score": 1002009997, "reasons": [], "unseen": true } ],
  "heldCount": 1, "lastSyncMs": 1760350000000 }
```

Wyniki z §6.9: Jira (zmiana 26 h temu; termin bez godziny 2025-10-13 = koniec tego dnia w Warszawie, 23:59:59.999 UTC+2, czyli „dziś”) = 3·10⁹ + sprint 10⁷ + priorytet 3·10⁶ + termin 2·10⁵ + „w toku” 10⁴ + 26;
review GitHuba (czeka 96 h, więc `waitingLong`) = 2·10⁹ + review 3·10⁸ + nieznany priorytet 2·10⁶ + 96; powiadomienie Linear (2 h temu) = 10⁹ + 2·10⁶ + (9999 − 2).

`PopupInfo` (paczka nowych pozycji na start przerwy, po okienku przerwy; `cycle` i `phaseSecs` mają znaczenie tylko w okienkach faz):

```json
{ "revision": 16, "kind": "workDigest", "cycle": 0, "phaseSecs": 0, "launchers": [], "reminder": null, "meeting": null, "workCount": 3 }
```

`SessionSummary`, tylko pola dodane przez ADR-017 (po odpowiedzi „Tak” na „Zrobione?”):

```json
{ "goal": { "source": "jira", "key": "SHOP-128", "title": "Płatność BLIK w koszyku", "url": "https://firma.atlassian.net/browse/SHOP-128" },
  "goalDone": true }
```

Argumenty komend „Co dalej?”: `set_work_token` → `{ "source": "github", "token": "…" }` albo `{ "source": "github", "token": null }`,
`set_session_goal` → `{ "goal": { "source": "jira", "key": "SHOP-128", "title": "Płatność BLIK w koszyku", "url": "https://firma.atlassian.net/browse/SHOP-128" } }`
albo `{ "goal": null }`, `mark_goal_done` → `{ "sessionId": "1760350000000-12345-1", "done": true }`.

`MediaWindowStatus` (okno YouTube z domyślnym streamem):

```json
{ "revision": 3, "open": true, "url": "https://www.youtube.com/@LofiGirl/live",
  "title": "lofi hip hop radio - beats to relax/study to" }
```

Argumenty komend okna YouTube: `open_media_window` → `{ "url": "https://www.youtube.com/@LofiGirl/live", "play": true }`,
`control_media_window` → `{ "action": "setVolume", "volume": 0.4 }` albo `{ "action": "pause", "volume": null }`.

`AppError` (walidacja configu):

```json
{ "code": "config.invalid", "message": "focusMinutes must be between 1 and 240, got 300", "field": "focusMinutes" }
```

`BlocklistInfo` (plik z jedną pominiętą linią; listy skrócone):

```json
{ "revision": 2, "path": "C:\\Users\\ola\\AppData\\Roaming\\com.focusmode.app\\websiteblacklist.txt",
  "domains": ["kwejk.pl", "jbzd.com.pl", "reddit.com"],
  "patterns": ["linkedin\\.com/feed", "youtube\\.com/shorts"],
  "issues": [{ "line": 42, "text": "(reddit|wykop\\.pl", "code": "invalidRegex", "message": "unclosed group" }],
  "error": null, "extensionSkipped": [] }
```

`extensionSkipped` jest pusta, dopóki rozszerzenie nie zgłosi wzorców, których przeglądarka nie przyjęła jako reguł; wtedy zawiera te źródła z `patterns`
(w kolejności z pliku). Zgłoszenie rozszerzenia (`POST /v1/report`, ADR-018) z jednym takim wzorcem (tu: ponad limitem reguł regex przeglądarki):

```json
{ "skipped": ["youtube\\.com/shorts"] }
```

`BridgeState` (Focus, blokada włączona):

```json
{ "active": true, "blocklist": ["kwejk.pl", "reddit.com"], "patterns": ["youtube\\.com/shorts"],
  "phase": "focus", "remainingSecs": 1712 }
```

`PrivacyReport` (ADR-020; Teams i GitHub połączone, historia z tytułami okien, bez kalendarza Google i ICS; `items` skrócone, w pełnym raporcie
jest po jednej pozycji na każdy `PrivacyItemKind`, w jego kolejności):

```json
{ "revision": 7,
  "folders": ["C:\\Users\\ola\\AppData\\Roaming\\com.focusmode.app", "C:\\Users\\ola\\AppData\\Local\\com.focusmode.app"],
  "items": [
    { "kind": "settings", "present": true, "sizeKib": 3, "count": null, "deletable": false, "includesTitles": false },
    { "kind": "history", "present": true, "sizeKib": 182, "count": 46, "deletable": true, "includesTitles": true },
    { "kind": "microsoftToken", "present": true, "sizeKib": 4, "count": null, "deletable": true, "includesTitles": false },
    { "kind": "googleToken", "present": false, "sizeKib": null, "count": null, "deletable": true, "includesTitles": false },
    { "kind": "githubToken", "present": true, "sizeKib": 1, "count": null, "deletable": true, "includesTitles": false },
    { "kind": "shieldsRestore", "present": false, "sizeKib": null, "count": null, "deletable": false, "includesTitles": false },
    { "kind": "browserData", "present": true, "sizeKib": 48211, "count": null, "deletable": true, "includesTitles": false }
  ],
  "busy": false }
```

`YoutubeAccount` (ADR-021; zalogowany, okno logowania już zamknięte):

```json
{ "revision": 4, "known": true, "signedIn": true, "signInOpen": false, "browserRejected": false }
```

Argumenty: `export_privacy_data` → `{ "path": "C:\\Users\\ola\\Documents\\focusmode-export.json" }`,
`delete_privacy_data` → `{ "kinds": ["history", "meetingChoices", "browserData"] }`.

`Config`, tylko pola dodane przez ADR-022 (wartości domyślne; stary config wczytuje się z wyłączonymi funkcjami beta):

```json
{ "betaFeatures": false,
  "rewards": { "enabled": true, "microSteps": true, "surprises": true, "sounds": true, "celebrations": "full",
               "focusCompanion": false, "showStreak": true, "streakPaused": false, "dailyGoal": "light", "restWeekdays": ["sat", "sun"] } }
```

`SessionSummary`, tylko pola dodane przez ADR-022 (sesja z dwoma krokami przy aktywnych nagrodach). Oba pola znikają z JSON-a, gdy są puste
(`skip_serializing_if`), więc linia `history.jsonl` bez funkcji beta wygląda jak wcześniej, a w TS są opcjonalne (`steps?`, `sparks?`):

```json
{ "steps": [ { "id": 1, "text": "Napisz tytuł", "done": true }, { "id": 2, "text": "Wypisz 3 rzeczy", "done": false } ],
  "sparks": 138 }
```

`PopupInfo` (start przerwy przy aktywnych nagrodach; pola `rewards` i `nextStep` mają `#[serde(default)]`):

```json
{ "revision": 21, "kind": "breakStarted", "cycle": 1, "phaseSecs": 900, "launchers": [], "reminder": null, "meeting": null, "workCount": 0,
  "sticky": false, "rewards": { "sparks": 72, "dailyGoal": true, "streak": 5, "levelUp": 8, "surprise": "pearl" }, "nextStep": null }
```

Argumenty komend funkcji beta: `ack_reward_moments` → `{ "upTo": 4 }`, `play_reward_sound` → `{ "sound": "sparkle" }`,
`set_session_steps` → `{ "steps": [ { "id": 1, "text": "Napisz tytuł" }, { "id": null, "text": "Otwórz plik" } ] }`, `set_step_done` → `{ "id": 1, "done": true }`.

`Config`, tylko pola dodane przez ADR-023 (wartości domyślne; `onboardingDone: true` także, gdy pola brak: kreator widzi tylko świeża instalacja,
dla której powłoka tworzy `Config::first_run()` z `false`; lista `pauseApps` skrócona, pełna to `tint::DEFAULT_PAUSE_APPS`, 17 nazw):

```json
{ "smartBreaks": false, "appCategories": {}, "onboardingDone": true,
  "tint": { "schedule": "fixed", "location": null, "pauseForApps": true,
            "pauseApps": ["Photoshop.exe", "Illustrator.exe", "Lightroom.exe", "…", "CaptureOne.exe"], "pauseForFullscreen": false } }
```

Te same pola po zmianach użytkownika (zachód → wschód słońca w Warszawie z listy, etykiety dwóch aplikacji):

```json
{ "smartBreaks": true, "appCategories": { "code.exe": "productive", "discord.exe": "distracting" }, "onboardingDone": true,
  "tint": { "schedule": "sun", "location": { "latitude": 52.2297, "longitude": 21.0122, "city": "pl-warszawa" },
            "pauseForApps": true, "pauseApps": ["Photoshop.exe", "Figma.exe"], "pauseForFullscreen": true } }
```

Argumenty komend ADR-023: `set_smart_break_opt_out` → `{ "meetingId": "…", "optOut": true }`, `get_weekly_stats` → `{ "weekStart": "2026-10-14" }`
(odpowiedź ma `"weekStart": "2026-10-12"`, poniedziałek tego tygodnia). Odpowiedzi: §6.15.

### 6.5 Lista blokowanych stron (`websiteblacklist.txt`, ADR-013 pkt 4)

Lista stron do blokowania nie jest częścią `Config`: **jedynym źródłem jest plik `websiteblacklist.txt`** w katalogu konfiguracji
(`app_config_dir()`, na Windows `%APPDATA%\com.focusmode.app\websiteblacklist.txt`). Pole `shields.blocklist` usunięto z `ShieldsConfig`.
Zostaje włącznik `shields.blockSites`. Stary `config.json` z polem `shields.blocklist` nadal się wczytuje, a to pole jest ignorowane.

- `BlocklistInfo { revision, path, domains, patterns, issues, error, extensionSkipped }`: `path` to bezwzględna ścieżka pliku, `domains` to znormalizowane domeny
  (zwykłe linie), a `patterns` to źródła regexów (linie w `/…/` albo ze znakami regexu). Obie listy zachowują kolejność z pliku, bez duplikatów.
  `issues` to pominięte linie, a `error` jest ustawiony, gdy pliku nie da się przeczytać; wtedy w użyciu zostaje poprzednia lista (albo domyślna).
  `extensionSkipped` (ADR-018, `#[serde(default)]`) to wzorce z `patterns`, których przeglądarka nie przyjęła jako reguł, według ostatniego zgłoszenia
  rozszerzenia (§6.10); bez zgłoszenia pusta.
- `BlocklistIssue { line, text, code, message }`: numer linii (od 1), linia jak w pliku (przycięta, najwyżej 200 znaków),
  kod `BlocklistIssueCode` i szczegół po angielsku (ADR-014: core nie zwraca tekstów dla użytkownika). Kody: `invalidDomain`, `invalidRegex`
  (`message` = powód z parsera regexu, np. `unclosed group`), `regexTooComplex`, `matchesEverything`, `pastedUrl` (`message` = domena do podpowiedzi),
  `lineTooLong` (limit 500 znaków), `tooManyDomains` (limit 2000), `tooManyPatterns` (limit 500). Ustawienia mapują `code` na tekst ze `strings.ts`,
  a `message` pokazują jako dodatkowy szczegół.
- Parsowanie i walidacja to czysta funkcja w `focus-core` (`blocklist.rs`, crate `regex`). Plik tworzy, czyta i przeładowuje powłoka
  (`src-tauri/src/blocklist.rs`): przy starcie, automatycznie po zapisaniu i przez `reload_blocklist`. Domyślna treść
  (`crates/focus-core/assets/websiteblacklist.txt`) jest wbudowana w aplikację, a przy braku pliku zostaje do niego skopiowana.
- Dopasowanie: domena blokuje siebie i swoje subdomeny. Regex jest dopasowywany bez rozróżniania wielkości liter: w rozszerzeniu
  do pełnego adresu URL karty, a w awaryjnym nudge'u trackera do tytułu okna przeglądarki.

### 6.6 Zasobnik, okienka faz, odcień, launchery i język (ADR-014)

Nowe pola `Config` (stary `config.json` bez nich wczytuje się z wartościami domyślnymi; częściowy obiekt `tint` jest uzupełniany domyślnymi):

| Pole | Typ | Domyślnie | Znaczenie |
|---|---|---|---|
| `language` | `Language` | `system` | język UI, zasobnika, okienek i toastów: `system` albo `en`, `pl`, `de`, `fr`, `it`, `uk`, `ru`, `es`, `zh`, `ja`; `system` = język wyświetlania systemu (`get_system_language`), a gdy go nie obsługujemy, `en`. Do przebiegu tłumaczeń teksty są po polsku |
| `closeToTray` | `boolean \| null` | `null` | X na oknie głównym: `true` chowa do zasobnika, `false` kończy aplikację, `null` = zapytaj przy pierwszym X (`app://close-requested` → `resolve_close`) |
| `phasePopups` | `boolean` | `true` | okienka FocusMode (okno `popup`) zamiast toastów Windows (nigdy oba; toast tylko przy wyłączonych okienkach albo gdy okna nie da się pokazać); włącza je też `resolve_close { toTray: true }`. Do 2026-10-13 domyślnie `false`: zapisany `config.json` ma to pole, więc wybór użytkownika zostaje; brak pola (plik sprzed ADR-014 albo edytowany ręcznie) daje nową wartość domyślną, jak każde brakujące pole |
| `tint.enabled` | `boolean` | `false` | ciepło ekranu według pory dnia (mechanizm: `tint.mode`, §6.7; overlay z ADR-014 usunięty przez ADR-015) |
| `tint.startMinutes` | `number` | `1080` (18:00) | minuta doby, od której odcień narasta; 0..1439 |
| `tint.endMinutes` | `number` | `540` (09:00) | minuta doby, w której ekran wraca do normy; 0..1439, różna od `startMinutes` (okno może przechodzić przez północ) |
| `tint.strength` | `number` | `45` | pełna siła 0..100 |
| `tint.transitionMinutes` | `number` | `30` | długość narastania po `startMinutes` i zanikania przed `endMinutes`, 0..180 (najwyżej połowa okna) |
| `shields.closeLaunchers` | `boolean` | `false` | zamykaj wybrane launchery gier na każdy start fazy Focus |
| `shields.launchers` | `GameLauncher[]` | `steam`, `epic`, `gog`, `ubisoft`, `ea`, `battleNet`, `riot` | które launchery zamykać; do wyboru także `xbox`, `amazon`, `itch`; normalizowane (bez duplikatów, stała kolejność) |

Błędy walidacji: `config.invalid` z polem `tint.startMinutes`, `tint.endMinutes` (także gdy początek = koniec), `tint.strength` albo `tint.transitionMinutes`,
również przy wyłączonym odcieniu.

Nowe DTO i pola:

| Typ | Pola / warianty | Uwagi |
|---|---|---|
| `Language` | `system`, `en`, `pl`, `de`, `fr`, `it`, `uk`, `ru`, `es`, `zh`, `ja` | `get_system_language` nigdy nie zwraca `system` |
| `TintConfig` | `enabled`, `startMinutes`, `endMinutes`, `strength`, `transitionMinutes`, `mode` (ADR-015) | poziom liczy `focus_core::tint::tint_level(cfg, minute_of_day)`; minutę doby czasu lokalnego daje `focus-platform` (`clock.rs`) |
| `TintStatus` | `revision`, `enabled`, `level` (0..100, 0 = kolory nietknięte), `preview`, `active` (ciepło jest teraz zastosowane), `mode`, `capped`, `error` (angielski szczegół, gdy mechanizm zawiódł albo jest niedostępny; harmonogram próbuje dalej) | getter `get_tint_status`, event `tint://changed`; `mode` i `capped` z ADR-015 (§6.7) |
| `GameLauncher` | `steam`, `epic`, `gog`, `ubisoft`, `ea`, `battleNet`, `riot`, `xbox`, `amazon`, `itch` | nazwy procesów i nazwy wyświetlane: `focus_core::launchers` |
| `ShieldStatus.launchers` | `IntegrationStatus` | osłona „zamknij launchery”; poza Windows `unavailable` |
| `ShieldStatus.closedLaunchers` | `GameLauncher[]` | launchery zamknięte na ostatnim starcie focusa (puste, gdy żaden nie działał) |
| `PopupKind` | `breakStarted`, `focusStarted`, `interrupted` (auto-pauza po uśpieniu), `launchersClosed` | |
| `PopupInfo` | `revision`, `kind` (`null` = schowaj), `cycle`, `phaseSecs` (długość fazy, która się zaczęła), `launchers`, `reminder` (ADR-015) | getter `get_popup`, event `popup://show` |
| `PopupAction` | `dismiss`, `showMain`, `skipPhase` | argument `popup_action` |
| `BlocklistIssueCode` | §6.5 | |

Kolejność efektów z tabeli ADR-013 się nie zmienia: core nadal wysyła `Notify` w tym samym miejscu, a powłoka wybiera kanał (okienko `popup` albo toast).
Zamykanie launcherów: na start każdej fazy Focus, tylko dokładne nazwy exe z `focus_core::launchers::process_names`, tylko procesy bieżącej sesji
(`focus-platform/src/processes.rs`: Toolhelp + `ProcessIdToSessionId` + `TerminateProcess`), nigdy usługi ani procesy gier.
Teams: przed ustawieniem DND powłoka czyta `GET /me/presence`; przy `availability` = `Offline` albo `DoNotDisturb` nie ustawia DND i potem go nie czyści.

### 6.7 Ciepło ekranu, motywy, dźwięki, muzyka w zestawie, przypomnienia i historia (ADR-015)

Nowe pola `Config` (stary `config.json` bez nich wczytuje się z wartościami domyślnymi):

| Pole | Typ | Domyślnie | Znaczenie |
|---|---|---|---|
| `theme` | `Theme` | `midnight` | wygląd: `midnight` (dotychczasowy), `vaporwave`, `lunar` (fazy księżyca), `autumn` (przytulna jesień), `dark`, `light`, `pixel` (8-bit); zmienne CSS w `src/lib/themes/` |
| `chimeSound` | `ChimeSound` | `midnight` | pakiet dzwonków: `midnight`, `vaporwave`, `lunar`, `autumn`, `classic`, `soft`, `chiptune` (syntezowane w `focus-audio`) albo `custom` (pliki z `customChimes`) |
| `customChimes` | `CustomChimes` | `{ breakFile: null, focusFile: null }` | nazwy plików w folderze `sounds` danych aplikacji; same nazwy (`is_chime_file_name`), bez ścieżek; `chimeSound: custom` bez plików jest poprawne i gra `midnight` |
| `breakReminders` | `string[]` | `[]` | przypomnienie na kolejne przerwy: indeks 0 = pierwsza przerwa sesji; `""` = bez przypomnienia dla tej przerwy; najwyżej 24 pozycje po 160 znaków |
| `repeatReminders` | `boolean` | `false` | po ostatniej pozycji `breakReminders` zacznij od pierwszej |
| `trackTitles` | `boolean` | `true` | zapamiętuj najczęstsze tytuły okien każdej aplikacji do podpowiedzi na wykresie (tylko lokalnie, nigdy w logach) |
| `keepHistory` | `boolean` | `true` | zapisuj zakończone sesje w `history.jsonl`; wyłączone = podsumowanie po Stop się pokazuje, ale nie jest zapisywane |
| `tint.mode` | `TintMode` | `nightLight` | `nightLight` (Oświetlenie nocne Windows, ustawienie użytkownika, bez praw administratora) albo `gamma` (rampa gamma każdego monitora; bez HDR, Windows może ograniczyć bardzo ciepłe wartości) |
| `music.source` | `MusicSourceKind` | `library` | nowy wariant `library` (muzyka w zestawie) jest domyślny; config z zapisanym źródłem je zachowuje |

Błędy walidacji: `config.invalid` z polem `breakReminders` (więcej niż 24 pozycje albo pozycja dłuższa niż 160 znaków, liczonych jako znaki, nie bajty),
`customChimes.breakFile` albo `customChimes.focusFile` (nazwa, która nie przechodzi `is_chime_file_name`). Przypomnienia są przycinane, a znaki sterujące
zamieniane na spacje; puste nazwy plików dzwonków stają się `null`.

Nowe DTO i pola:

| Typ | Pola / warianty | Uwagi |
|---|---|---|
| `Theme` | `midnight`, `vaporwave`, `lunar`, `autumn`, `dark`, `light`, `pixel` | tylko frontend (motyw okna głównego i `popup`) |
| `ChimeSound` | `midnight`, `vaporwave`, `lunar`, `autumn`, `classic`, `soft`, `chiptune`, `custom` | każdy pakiet ma wariant przerwy (opadający) i skupienia (rosnący) |
| `CustomChimes` | `breakFile`, `focusFile` (`string \| null`) | `CustomChimes::file_for(chime)`: własny plik dzwonka, potem plik drugiego, a `null` = dźwięk domyślny; `import_chime` zapisuje kopię pod nową nazwą `custom-<break\|focus>-<ms>.<ext>` (`src-tauri/src/chimes.rs`), a nieużywane pliki usuwa |
| `TintMode` | `nightLight`, `gamma` | |
| `TintStatus.mode` | `TintMode` | mechanizm, który faktycznie działa: wybrany albo `gamma` jako zastępstwo niedostępnego Oświetlenia nocnego (powód w `error`) |
| `TintStatus.capped` | `boolean` | Windows nie przyjął pełnej siły (limity rampy gamma) i działa słabsza |
| `PopupInfo.reminder` | `string \| null` | przypomnienie użytkownika na przerwę, która właśnie się zaczęła: `focus_core::config::reminder_for_break(cfg, snapshot.cycle)` |
| `AppUsage.titles` | `TitleUsage[]` | najwyżej 5 tytułów na aplikację, od największej liczby sekund (remis: alfabetycznie); puste, gdy `trackTitles` wyłączone; `#[serde(default)]`, więc stara historia bez tytułów się wczytuje |
| `TitleUsage` | `title` (oczyszczony, najwyżej 120 znaków), `secs` | |
| `MusicSourceKind.library` | – | muzyka z `list_library_tracks` |

Temperatura: `focus_core::tint::kelvin_for_level(level)` (0 → 6500 K, 100 → 1900 K, interpolacja w miredach) dla Oświetlenia nocnego,
`rgb_gain_for_kelvin(kelvin)` (mnożniki `[r, g, b]` w 0..1, 6500 K = `[1, 1, 1]`) dla rampy gamma: `rampa = bazowa rampa monitora × mnożnik`.
Platforma: `focus-platform/src/night_light.rs` i `gamma.rs`. Stan sprzed pierwszej zmiany powłoka (`src-tauri/src/tint.rs`) zapisuje w `tint-restore.json`
i uzbraja wpis RunOnce `FocusModeRestore` (`--restore-shields` przywraca też kolory), jak przy osłonach; jeden wpis obsługuje oba przywracania (`safety_net.rs`).
Gdy Oświetlenie nocne jest niedostępne, powłoka przechodzi na gammę (w drugą stronę zastępstwa nie ma).

Śledzenie: tracker wywołuje `UsageAccumulator::add_sample(process, title, idle, ms)`, przy czym `title` to `None`, gdy `trackTitles` jest wyłączone.
Sekundy, w których na pierwszym planie jest powierzchnia powłoki Windows (`focus_core::stats::is_shell_surface(process, class)`; klasa okna z `focus-platform`
`foreground`), są pomijane w całości. `clear_history` usuwa `history.jsonl` i ostatnie podsumowanie; przy `keepHistory: false` powłoka nie dopisuje sesji do historii.

### 6.8 Kalendarze i spotkania w przerwach (ADR-016)

Nowe pole `Config.calendar` (stary `config.json` bez niego wczytuje się z kalendarzami wyłączonymi; częściowy obiekt jest uzupełniany domyślnymi):

| Pole | Typ | Domyślnie | Znaczenie |
|---|---|---|---|
| `calendar.microsoft` | `boolean` | `false` | kalendarz Microsoft 365 / Outlook przez konto Microsoft z ustawień Teams (Graph, delegowane `Calendars.Read`; logowanie Teams prosi o ten zakres tylko wtedy, gdy to pole jest włączone) |
| `calendar.google` | `boolean` | `false` | Google Calendar (OAuth dla aplikacji instalowanych); potrzebuje `googleClientId` (włączenie bez niego jest poprawne, ale źródło nie działa) |
| `calendar.googleClientId` | `string` | `""` | identyfikator klienta OAuth typu „Aplikacja komputerowa” z projektu Google Cloud użytkownika; przycinany, a niepusty musi przejść `is_google_client_id` (`<prefiks>.apps.googleusercontent.com`, tylko `[A-Za-z0-9._-]`, najwyżej 255 znaków) |
| `calendar.googleClientSecret` | `string` | `""` | sekret tego samego klienta „Aplikacja komputerowa”; punkt tokenów Google go wymaga (wymiana kodu i odświeżanie; pusty nie jest wysyłany). Google dokumentuje go w aplikacjach instalowanych jako niepoufny, więc leży w `config.json`, nigdy w logach. Przycinany, a niepusty musi przejść `is_google_client_secret` (najwyżej 100 znaków `[A-Za-z0-9_-]`). Stary config bez pola: `""` |
| `calendar.ics` | `boolean` | `false` | linki ICS / iCal; same linki leżą osobno, zaszyfrowane (`set_ics_urls` / `get_ics_urls`) |
| `calendar.horizonHours` | `number` | `10` | ile godzin naprzód czytamy spotkania, 1..=24 |

Błędy walidacji: `config.invalid` z polem `calendar.horizonHours`, `calendar.googleClientId` albo `calendar.googleClientSecret` (komunikat nie powtarza sekretu).

Nowe DTO i pola:

| Typ | Pola / warianty | Uwagi |
|---|---|---|
| `CalendarSource` | `microsoft`, `google`, `ics` | argument `disconnect_calendar` |
| `MeetingChoice` | `extend`, `pauseAtBreak`, `ignore`, `remindLater` | argument `set_meeting_choice` |
| `Meeting` | `id` (źródło + id wydarzenia + początek), `source`, `subject`, `startMs`, `endMs`, `online` (jest link Teams / Meet / Zoom / Webex) | bez spotkań odrzuconych, odwołanych, „wolnych” i całodniowych; każde wystąpienie spotkania cyklicznego osobno |
| `MeetingConflict` | `meeting`, `breakNumber` (od 1), `breakStartMs`, `breakEndMs`, `choice` (`null` = jeszcze bez wyboru), `nextReminderMs`, `remindLaterAllowed`, `tooLong` | pierwsza przerwa, na którą spotkanie nachodzi; `remindLaterAllowed: false` w ostatnim przypomnieniu (10 min przed) i gdy do spotkania zostało 10 min lub mniej; `tooLong: true` (`#[serde(default)]`) dla spotkania dłuższego niż 4 h: sesja ignoruje dla niego `extend` / `pauseAtBreak`, Skupienie pokazuje notkę „Spotkanie dłuższe niż 4 h — nie przedłużam skupienia” i wyłącza te przyciski, okienko przypomnienia ich nie pokazuje |
| `CalendarSourceStatus` | `source`, `state` (`IntegrationState`), `detail` (angielski szczegół: błąd albo stan), `account` (konto albo liczba linków ICS) | jedno na źródło |
| `CalendarStatus` | `revision`, `sources`, `meetings` (posortowane po początku), `conflicts`, `planned` (`true` = przerwy sesji, która zaczęłaby się teraz; `false` = trwającej sesji), `lastSyncMs`, `googlePending` | getter `get_calendar`, event `calendar://changed` |
| `MeetingHoldView` | `subject`, `untilMs` (koniec spotkania), `choice` | `SessionSnapshot.meetingHold` |
| `SessionSnapshot.musicMuted` | `boolean` | Focus (także w pauzie) w trakcie spotkania z wyborem `extend`; w Idle i w przerwie zawsze `false` |
| `SessionSnapshot.meetingHold` | `MeetingHoldView \| null` | przedłużony focus (do końca fazy) albo wstrzymana przerwa (do Wznów) |
| `PopupKind.meetingReminder` | – | okienko „przypomnij później” z wyborem dla spotkania (`PopupInfo.meeting`) |
| `PopupKind.breakHeld` | – | przerwa zaczęła się wstrzymana z powodu spotkania (`PopupInfo.meeting`) |
| `PopupInfo.meeting` | `MeetingConflict \| null` | `#[serde(default)]` |

Core (`focus-core`, bez zegara i sieci; czasy kalendarzowe w ms od epoki Unix):
- `calendar::plan_breaks(now_ms, focus_remaining_ms, in_break_remaining_ms, cycle, &profile, horizon_ms) -> Vec<BreakWindow>`: przerwy zaczynające się przed
  `now + horizon` (najwyżej `MAX_PLANNED_BREAKS` = 512). Idle (oba `None`): sesja, która zaczęłaby się teraz, numeracja od 1. Focus (`Some(pozostało)`):
  pierwsza przerwa ma numer `cycle` i zaczyna się za `pozostało`. Przerwa: trwającej nie ma na liście, następna ma numer `cycle + 1`. Pauza: zamrożony czas,
  jakby faza ruszyła teraz. Wejście z powłoki: `snapshot.cycle` i `Session::remaining_ms(now)`.
- `calendar::conflicts(&breaks, &meetings) -> Vec<(indeks spotkania, BreakWindow)>`: przecięcie półotwartych przedziałów (styk się nie liczy), każde spotkanie raz.
- `calendar::reminder_schedule(meeting_start_ms, now_ms, stage) -> Option<(due_ms, remind_later_allowed)>`: etap `n` przypada
  `timing::MEETING_REMINDER_LEAD_MINUTES[n]` (30, 20, 10) minut przed spotkaniem; etap z porą ≤ `now` jest pomijany; ostatni (10 min) daje `false`;
  `None` = nie ma już etapu (nie oferować „przypomnij później”). Powłoka trzyma dla każdego spotkania etap (ile razy wybrano „przypomnij później”),
  przy każdym takim wyborze bierze z tej funkcji porę następnego okienka (`nextReminderMs`) i zwiększa etap, a `remindLaterAllowed` to „został jeszcze etap”.
- `config::CalendarConfig::normalized()` i `horizon_ms()`, `config::is_google_client_id(id)`, `config::is_google_client_secret(secret)`.
- `calendar::meeting_hold_allowed(start_ms, end_ms)`: spotkanie ma długość i trwa najwyżej `timing::MAX_MEETING_HOLD_MS` (4 h; dokładnie 4 h jeszcze tak).
  Powłoka liczy z tego `MeetingConflict.tooLong` i odrzuca `extend` / `pauseAtBreak` dla dłuższych spotkań (`calendar.meetingTooLong`).
- `session::MeetingHold { id, subject, start_mono_ms, end_mono_ms, start_wall_ms, end_wall_ms, choice }` i `MeetingHold::from_meeting(&meeting, choice, now)`
  (`mono = now.mono_ms + (wall − now.wall_ms)`, z nasyceniem; `None` dla `ignore` / `remindLater`, spotkania bez długości i spotkania dłuższego niż 4 h).
- `Input::SetMeetingHolds(Vec<MeetingHold>)`: zastępuje wszystkie blokady, bez efektów, w Idle i w trakcie sesji; zostają tylko `extend` / `pauseAtBreak`
  z `end > start` i długością najwyżej `MAX_MEETING_HOLD_MS` na obu zegarach (dłuższe spotkanie nie przedłuża focusa, nie wstrzymuje przerwy
  i nie wycisza muzyki). Powłoka wysyła **wszystkie** spotkania z takim wyborem po każdej synchronizacji, zmianie wyboru, Start i Wznów.
- Reguły (tylko gdy tick osiąga termin focusa; okno `[now, now + break)`; każda blokada najwyżej raz na sesję; tylko spotkania do 4 h): `extend` wygrywa (najpóźniejszy koniec;
  termin = koniec spotkania, bez efektów, przedłużenia się łączą); potem `pauseAtBreak` (najpóźniejszy koniec; przerwa w pauzie z pełną długością,
  efekty jak zwykle, tylko `Notify(BreakHeld { subject })` zamiast `Notify(BreakStarted)`); inaczej zwykła przerwa. Pomiń fazę nigdy nie stosuje blokad.
  Usunięcie blokady w trakcie przedłużenia go nie skraca.
- `NotifyKind::BreakHeld { subject: Option<String> }` (`None` przy pustym temacie); `NotifyKind` nie jest już `Copy`.
  Powłoka pokazuje go jako okienko `breakHeld` (albo toast) z długością przerwy z profilu.

Platforma i powłoka (ADR-016 pkt 1–2): kalendarz Microsoft przez logowanie Teams (te same rejestracja, token i device code; `TeamsClient::begin_login(calendar)`
prosi o `teams::login_scopes(calendar)`, czyli `Calendars.Read` tylko przy włączonym `calendar.microsoft`; logowanie bez tego zakresu daje źródłu błąd
„reconnect needed …”, a Ustawienia → Kalendarze pokazują „Połącz ponownie” = `teams_connect`; koniec logowania przy już połączonym koncie powłoka traktuje
jak nowe połączenie i od razu czyta kalendarz),
`focus-platform/src/calendar/google.rs` (OAuth loopback + PKCE, Calendar API tylko do odczytu, zakres `calendar.readonly`, `client_secret` w formularzu
wymiany kodu i odświeżania, gdy niepusty), `focus-platform/src/calendar/ics.rs`
(pobieranie i parser iCalendar z `RRULE` / `EXDATE` / `RECURRENCE-ID` i strefami), `focus-platform/src/secrets.rs` (pliki szyfrowane DPAPI: `google-token.bin`,
`ics-links.bin`; poza Windows niedostępne). Powłoka (`src-tauri/src/calendar/`, czysta część w `model.rs`) synchronizuje źródła poza aktorem sesji
(aktor nie czeka na sieć, zasada 3), zapisuje wybory w `meeting-choices.json` (tylko id spotkania i wybór, bez tematu; od ADR-023 także „Zapytaj mnie” inteligentnych przerw w `optOuts`), liczy kolizje, planuje okienka `meetingReminder`, wysyła
`Input::SetMeetingHolds` do aktora i emituje `calendar://changed`. Okno `popup` wysyła wybór z przypomnienia przez `set_meeting_choice`.
Tokenów, linków ICS ani tematów spotkań nie loguje: szczegół błędu zamienia każdy adres na `<link>`, a błąd linku ICS podaje jego numer, nie treść.
Czysta część (`model.rs`): łączenie źródeł (kopie spotkania z kilku źródeł: to samo id albo ten sam temat z początkiem w odstępie do minuty; wybór kopii przechodzi
na spotkanie, które zostało), wybory z etapem przypomnień (zapominane po końcu spotkania), walidacja linków ICS (`https://` albo `webcal://` z hostem, bez spacji,
najwyżej 10 po 2048 znaków; `webcal://` jest pobierany przez `https://`), najwyżej 200 spotkań.

### 6.9 „Co dalej?” — zadania z narzędzi pracy (ADR-017)

Nowe pole `Config.work` (stary `config.json` bez niego wczytuje się z wyłączonymi narzędziami; częściowy obiekt jest uzupełniany domyślnymi).
Tokenów w configu nie ma: zapisuje je `set_work_token` (plik szyfrowany DPAPI na narzędzie).

| Pole | Typ | Domyślnie | Znaczenie |
|---|---|---|---|
| `work.github` | `boolean` | `false` | GitHub (github.com) |
| `work.gitlab` | `boolean` | `false` | GitLab |
| `work.gitlabUrl` | `string` | `https://gitlab.com` | adres instancji GitLab (`normalize_work_url`) |
| `work.jira` | `boolean` | `false` | Jira Cloud albo Data Center |
| `work.jiraUrl` | `string` | `""` | witryna Jira Cloud (`https://firma.atlassian.net`) albo adres serwera Data Center (`normalize_work_url`) |
| `work.jiraEmail` | `string` | `""` | e-mail konta Atlassian do tokenu API Jira Cloud (nazwa użytkownika w Basic auth); pusty = Data Center (token jako `Bearer`) |
| `work.azureDevops` | `boolean` | `false` | Azure DevOps Services |
| `work.azureOrg` | `string` | `""` | organizacja z `https://dev.azure.com/<org>`; wklejony adres (`dev.azure.com/<org>/…`, `<org>.visualstudio.com`) jest skracany do nazwy |
| `work.linear` | `boolean` | `false` | Linear |
| `work.holdDuringFocus` | `boolean` | `true` | nowe pozycje z czasu focusa czekają do przerwy albo do Stop (`heldCount`, okienko `workDigest`); wyłączone = lista widoczna także w focusie, a nowe pozycje ogłaszane od razu (okienko `workDigest` także w focusie) |

Błędy walidacji: `config.invalid` z polem `work.gitlabUrl`, `work.jiraUrl` (oba: `https://`, host `[a-z0-9.-]` z opcjonalnym portem 1–65535, bez `@`, `?` i `#`,
ścieżka bez segmentów `.` i `..`, najwyżej 512 znaków; końcowe `/` usuwane; dla `gitlab.com` i `*.atlassian.net` ścieżka odrzucana), `work.jiraEmail`
(pusty albo `nazwa@domena` bez spacji, znaków sterujących i `:`, najwyżej 254 znaki) albo `work.azureOrg` (`[A-Za-z0-9._-]`, najwyżej 64 znaki, nie same kropki).
Komunikat nie powtarza wartości. Walidacja działa także przy wyłączonym narzędziu.

Nowe DTO i pola:

| Typ | Pola / warianty | Uwagi |
|---|---|---|
| `WorkSource` | `github`, `gitlab`, `jira`, `azureDevops`, `linear` | argument `set_work_token`; `SessionGoal.source` |
| `WorkItemKind` | `assigned` (próg 1); `reviewRequested`, `changesRequested`, `ciFailed`, `mention` (próg 2); `notification` (próg 3) | `focus_core::work::tier` |
| `WorkReason` | `assigned`, `inSprint`, `highPriority`, `dueToday`, `overdue`, `inProgress`, `blockingOthers`, `waitingLong`, `mentioned` | liczy core; UI pokazuje teksty ze `strings.ts` („Dlaczego teraz”) |
| `WorkItem` | `id` (źródło + klucz), `source`, `kind`, `key` (np. `PROJ-123`, `owner/repo#45`, `ENG-7`), `title`, `url` (pusty = bez linku), `project`, `priorityLabel` (nazwa z narzędzia), `priorityRank` (0 = najwyższy … 4 = najniższy, `null` = brak), `dueMs`, `status` (nazwa z narzędzia), `inSprint`, `updatedMs`, `score`, `reasons`, `unseen` | `score` i `reasons` zawsze liczy `rank`; wartości z platformy są ignorowane |
| `WorkSourceStatus` | `source`, `state` (`IntegrationState`), `detail` (angielski szczegół bez tokenu i adresów), `account` (nazwa konta, jeśli narzędzie ją podaje) | jedno na narzędzie: `off` (wyłączone), `unavailable` (brak adresu, tokenu albo DPAPI), `on` (także „not synced yet”), `error` |
| `WorkStatus` | `revision`, `sources`, `items` (najwyżej 50, od najlepszej), `heldCount` (nowe pozycje wstrzymane w focusie), `lastSyncMs` | getter `get_work`, event `work://changed` |
| `SessionGoal` | `source`, `key`, `title`, `url` | argument `set_session_goal` |
| `SessionSnapshot.goal` | `SessionGoal \| null` | `#[serde(default)]`; cel w Idle i w trakcie sesji |
| `SessionSummary.goal` / `goalDone` | `SessionGoal \| null` / `boolean \| null` | `#[serde(default)]`, więc stara historia się wczytuje; `goalDone` ustawia `mark_goal_done` |
| `PopupKind.workDigest` | – | nowe pozycje z czasu focusa, na start przerwy albo po Stop |
| `PopupInfo.workCount` | `number` | `#[serde(default)]`; liczba nowych pozycji dla `workDigest`, w innych okienkach 0 |

Core (`focus-core`, bez zegara, plików i sieci; czasy kalendarzowe w ms od epoki Unix):
- `work::rank(items, now_ms) -> Vec<WorkItem>`: czyści każdą pozycję, łączy kopie o tym samym `id`, liczy `score` i `reasons`, sortuje od najlepszej
  i zwraca najwyżej `MAX_ITEMS` (50). Wynik nie zależy od kolejności wejścia; arytmetyka nasyca się (żadne czasy nie powodują paniki).
- Układ `score` (cyfry dziesiętne, wyższy = wyżej):

  | Cyfry | Znaczenie |
  |---|---|
  | `10^9` | próg: 3 = przypisane, 2 = czeka na użytkownika, 1 = powiadomienie |
  | `10^8` | tylko próg 2: 3 = prośba o review, 2 = zmiany do zrobienia / nieudane CI, 1 = wzmianka |
  | `10^7` | 1 = aktywny sprint / cykl / iteracja |
  | `10^6` | priorytet: 4 − ranga (ranga 0 → 4 … ranga 4 → 0); brak = średni (ranga 2); rangi powyżej 4 przycinane do 4 |
  | `10^5` | termin: 3 = po terminie (`dueMs < now`), 2 = w ciągu 24 h, 1 = w ciągu 3 dni (`DUE_SOON_MS`), 0 = później albo brak |
  | `10^4` | 1 = status „w toku” (`is_in_progress`) |
  | `10^0..10^3` | godziny od `updatedMs`, najwyżej 9999 (w progach 1 i 2 starsze wyżej; w progu 3 9999 − godziny, więc nowsze wyżej) |

  Remis: `key`, potem `id`.
- Powody, w tej kolejności i tylko gdy dotyczą: `assigned` (próg 1), `blockingOthers` (`reviewRequested`), `mentioned` (`mention`); `inSprint`;
  `highPriority` (ranga 0 albo 1, `HIGH_PRIORITY_MAX_RANK`); `overdue` albo `dueToday`; `inProgress`; `waitingLong` (tylko próg 2, `updatedMs` starsze niż 3 dni, `WAITING_LONG_MS`).
- Łączenie kopii (`id` = źródło + klucz): wygrywa najlepszy próg, potem silniejsza grupa progu 2, potem więcej ustawionych pól, potem nowsza (dalej stała kolejność);
  zwycięzca zachowuje swoje pola i uzupełnia brakujące z pozostałych kopii; `inSprint` i `unseen` to „lub”, `updatedMs` to maksimum.
- Czyszczenie: znaki sterujące → spacje, przycięcie; tytuł do `MAX_TITLE_CHARS` (300), klucz do `MAX_KEY_CHARS` (120), projekt, etykieta priorytetu i status
  do `MAX_LABEL_CHARS` (80) z „…” na końcu; pusta etykieta → `null`; adres inny niż `http(s)://` z hostem, z białymi znakami albo dłuższy niż `MAX_URL_LEN` (2048) → `""`.
- `work::tier(kind)` (1 / 2 / 3), `work::is_in_progress(status)` (bez wielkości liter, spacji i interpunkcji; fragmenty w dowolnym miejscu: „in progress”,
  „in review”, „code review”, „in development”, „doing”, `workflow::in progress` i odpowiedniki PL / DE / FR / ES / IT / RU; tylko jako cały status: „active”,
  „started”, „review”, „wip”, „development”, „implementing”, „implementation”, „indeterminate”), `work::normalize_goal(goal)` (czyszczenie jak wyżej,
  adres spoza `http(s)://` pusty, `None` przy pustym kluczu i tytule).
- `config::WorkConfig::normalized()` (w `Config::normalized`), `config::normalize_work_url(field, raw)`; stałe `DEFAULT_GITLAB_URL`, `MAX_WORK_URL_LEN` (512),
  `MAX_JIRA_EMAIL_LEN` (254), `MAX_AZURE_ORG_LEN` (64).
- `session::Input::SetGoal(Option<SessionGoal>)`: w każdej fazie, bez efektów, przez `normalize_goal`; cel jest w każdym snapshocie i przechodzi przez Start;
  na Stop trafia do `SessionTotals.goal`, a sesja zostaje bez celu; `Session::goal()`. `stats::summarize` kopiuje go do `SessionSummary.goal` z `goalDone: None`.

Platforma (`focus-platform/src/work/`: `mod.rs` + klient na narzędzie za traitem `WorkProvider`; blokujące HTTP z limitem 10 s; tylko zapytania odczytu,
także te wysyłane jako `POST`: wyszukiwanie Jira, `workitemsbatch`, GraphQL; przekierowania tylko w obrębie tego samego adresu; 401 / 403 → błąd tokenu
`token invalid or missing scope`, 429 albo 403 z wyczerpanym limitem → błąd limitu; komunikaty bez adresów i treści odpowiedzi)
mapuje odpowiedzi narzędzi na `WorkItem`: `id` = `<źródło>:<klucz>` (np. `github:owner/repo#45`, `azure-devops:#42`),
termin bez godziny = ostatnia milisekunda tego dnia czasu lokalnego, priorytet narzędzia → `priorityRank` 0–4 (nazwa w `priorityLabel`), status jak w narzędziu
(kategoria statusu Jira `indeterminate` i typ stanu Linear `started` też liczą się jako „w toku”). API, z których korzysta (ADR-017 pkt 1 i 7):

| Narzędzie | API | Rodzaje pozycji |
|---|---|---|
| GitHub | REST `api.github.com`: `GET /user`; `GET /search/issues` (otwarte issue przypisane do użytkownika: priorytet i „w toku” z etykiet, termin z kamienia milowego; otwarte PR z prośbą o jego review; jego otwarte PR); dla własnych PR `GET …/pulls/{n}/reviews` (prośba o zmiany), `GET …/pulls/{n}` i `GET …/commits/{sha}/check-runs` (Checks API tylko z tokenem klasycznym), potem łączny status commita (nieudane CI); `GET /notifications?participating=true` (tylko token klasyczny; nieprzeczytane; `If-Modified-Since` z ostatniego `Last-Modified`, odpowiedź 304 = poprzednie pozycje; `X-Poll-Interval` idzie do powłoki; powód `mention` → wzmianka) | `assigned`, `reviewRequested`, `changesRequested`, `ciFailed`, `mention`, `notification` |
| GitLab | REST `/api/v4` (token w nagłówku `PRIVATE-TOKEN`): `GET /user`; `GET /issues?scope=assigned_to_me&state=opened` (termin `due_date`, priorytet i „w toku” z etykiet, bieżąca iteracja = sprint); `GET /merge_requests?scope=all&reviewer_id=…&state=opened` (bez MR, które użytkownik już ocenił: `…/reviewers`); jego otwarte MR (`scope=created_by_me`: `detailed_merge_status` `requested_changes` albo nieudany pipeline); `GET /todos?state=pending` (`mentioned` / `directly_addressed` → wzmianka, reszta → powiadomienie) | `assigned`, `reviewRequested`, `changesRequested`, `ciFailed`, `mention`, `notification` |
| Jira | `GET /rest/api/{3\|2}/myself`; `GET /rest/api/{3\|2}/field` (pola sprintu: `schema.custom` kończy się na `:gh-sprint`; zapamiętane w kliencie); JQL przypisanych, nierozwiązanych zgłoszeń przez `POST /rest/api/3/search/jql` (Cloud, z listą `fields`) albo `POST /rest/api/2/search` (Data Center, token jako `Bearer`); obserwowane zgłoszenia innych osób zmienione w ostatniej dobie. Token Cloud z zakresami: po odmowie witryny `cloudId` z `/_edge/tenant_info` (bez danych logowania) i zapytania przez `https://api.atlassian.com/ex/jira/<cloudId>` | `assigned`, `notification` |
| Azure DevOps | REST `dev.azure.com/<org>` (PAT jako Basic z pustą nazwą użytkownika): `GET /_apis/connectionData` (id i nazwa użytkownika; API Profiles wymaga tokenów Entra); WIQL na poziomie organizacji (przypisane, otwarte) i `POST /_apis/wit/workitemsbatch` (tytuł, stan, typ, projekt, iteracja, priorytet 1–4, termin); bieżąca iteracja domyślnego zespołu projektu (`GET /<projekt>/_apis/work/teamsettings/iterations?$timeframe=current`); `GET /_apis/projects` i PR w ostatnio zmienianych projektach: `searchCriteria.reviewerId` bez głosu użytkownika, `searchCriteria.creatorId` z głosem „czeka na autora” (−5) albo „odrzucone” (−10). Nieważny PAT bywa odpowiedzią 203 ze stroną logowania zamiast 401 | `assigned`, `reviewRequested`, `changesRequested` |
| Linear | GraphQL `https://api.linear.app/graphql` (klucz w nagłówku `Authorization` bez `Bearer`), jedno zapytanie: nazwa użytkownika, `viewer.assignedIssues` (typ stanu inny niż completed / canceled / duplicate, najwyżej 50; `priority` 1 Urgent … 4 Low → ranga 0–3, 0 = brak; `dueDate`; aktywny cykl = sprint) i ostatnie powiadomienia (nieprzeczytane, nieodłożone, niezarchiwizowane; kategoria `mentions` → wzmianka, reszta → powiadomienie; klucz = identyfikator issue, więc łączą się z przypisanym). Błędy GraphQL `AUTHENTICATION_ERROR` / `FORBIDDEN` → błąd tokenu, `RATELIMITED` → limit | `assigned`, `mention`, `notification` |

Powłoka (`src-tauri/src/work.rs`, czysta część w `work/model.rs`, pobieranie w `work/fetch.rs`): wątek `focus-work` trzyma stan, a każde pobranie idzie w osobnym
krótkim wątku z klientem narzędzia (klient i jego token zostają między pobraniami, dopóki nie zmienią się ustawienia ani token). Harmonogram: co 5 minut
(`POLL_EVERY`) i od razu po `refresh_work`, zmianie tokenu albo ustawień narzędzia oraz na starcie sesji; po błędzie ponowienie po 1, 2, 4… min (najwyżej 30,
`FIRST_BACKOFF` / `MAX_BACKOFF`) z zachowaniem poprzednich pozycji; GitHub nie częściej niż jego `X-Poll-Interval` (najwyżej 1 h, `MAX_POLL_HINT`), także przy
`refresh_work`. Pobranie bez odpowiedzi przez 3 min jest porzucane, a `refresh_work` czeka na wyniki najwyżej 60 s i oddaje bieżący stan.
`set_work_token` sprawdza token (`validate_token`: przycięty, 1–4096 widocznych znaków ASCII; błąd `work.token` z polem `token`), zapisuje go szyfrowany
(`work-<github|gitlab|jira|azure-devops|linear>-token.bin`, błąd zapisu `work.tokenSave`) albo usuwa przy `null`. Szczegół błędu narzędzia przechodzi przez
`model::redact` (token → `<token>`, każde słowo z `://` → `<link>`, najwyżej 200 znaków), więc w `WorkSourceStatus.detail` i w logach nie ma tokenów ani adresów.
`unseen` i ogłaszanie nowych pozycji: pozycje oddane przez `get_work` / `refresh_work` liczą się od następnego stanu jako widziane; ich identyfikatory
(najwyżej 2000, bez tytułów) leżą w `work-seen.json`. Pierwsza lista narzędzia po starcie albo zmianie jego ustawień jest punktem wyjścia i niczego nie ogłasza.
Nowe pozycje w Focus przy `holdDuringFocus` trafiają do `heldCount`; powłoka ogłasza je okienkiem `workDigest` (toastem bez tytułów, `texts::work_digest_toast`,
tylko gdy okienka nie da się otworzyć) na start przerwy i po Stop, nigdy w focusie. `workDigest` nie zastępuje okienka na ekranie: czeka, aż zniknie (najpierw okienko
przerwy, potem paczka), a z kilku czekających zostaje najnowsza (`popup.rs`). Okienko startu focusa zastępuje paczkę zamiast ją odkładać, a na start focusa
z wstrzymaniem `PopupHandle::withdraw_work_digest` usuwa ją z ekranu i z kolejki, więc paczka sprzed focusa nie wraca w jego trakcie. W focusie z wstrzymaniem
błąd pobierania nie wysyła `app://error` (widać go tylko w stanie narzędzia). `set_session_goal` wysyła `Input::SetGoal` do aktora; `mark_goal_done` (`Persistence::mark_goal_done`) ustawia
`goalDone` w ostatnim podsumowaniu i przepisuje atomowo `history.jsonl` (linie tej sesji), a odpowiedź udzielona, zanim podsumowanie trafiło do pliku, jest dopisywana razem z nim.
Tytułów, kluczy i linków pozycji powłoka nie loguje (tylko liczby).

UI: `src/lib/state/work.svelte.ts` (listen → `get_work` → nowsza rewizja), `NextUp.svelte` (karta „Co dalej?” na stronie Skupienie: Idle, przerwa, a w focusie
tylko przy wyłączonym `holdDuringFocus`), `SessionGoalLine.svelte` (cel pod minutnikiem, „×” czyści), `WorkSettings.svelte` (Ustawienia → Narzędzia pracy:
włącznik, adres / e-mail / organizacja, token z instrukcją najwęższych uprawnień, stan, „Odśwież teraz”, `holdDuringFocus`); podsumowanie pyta „Zrobione?”,
a historia pokazuje cel ze znacznikiem.

### 6.10 Maskotka i poprawki (ADR-018)

Nowe pole `Config` (stary `config.json` bez niego wczytuje się z `true`):

| Pole | Typ | Domyślnie | Znaczenie |
|---|---|---|---|
| `showMascot` | `boolean` | `true` | maskotka (aksolotl) w oknie głównym, na karcie przerwy i w okienkach; Ustawienia → Wygląd. Bez walidacji w core |

Nowe pole DTO:

| Typ | Pole | Uwagi |
|---|---|---|
| `BlocklistInfo` | `extensionSkipped: string[]` | `#[serde(default)]`; wzorce z `patterns`, których przeglądarka nie przyjęła jako reguł `declarativeNetRequest`, według ostatniego zgłoszenia rozszerzenia; w kolejności z pliku, bez powtórzeń; tylko w pamięci (po restarcie pusta do następnego zgłoszenia) |

Bez nowych komend i eventów: `extensionSkipped` przychodzi parą `get_blocklist` + `blocklist://changed` (nowa rewizja, gdy lista się zmieni).

**Most: `POST /v1/report`** (`src-tauri/src/bridge.rs`; jedyny zapis do mostu, poza nim tylko `GET /v1/state`):

| | |
|---|---|
| Treść | `{ "skipped": string[] }` (`Content-Type: application/json`; inne pola ignorowane): źródła wzorców z `BridgeState.patterns`, dokładnie w tej postaci |
| Warunki | `Host` loopback (`127.0.0.1`, `localhost`, `[::1]`, z portem albo bez); `Origin` rozszerzenia (`chrome-extension://`, `moz-extension://`, `extension://`); nagłówek `X-FocusMode-Extension: 1`. Wszystkie trzy naraz |
| Limity | najwyżej 500 pozycji (`REPORT_MAX_ENTRIES` = `MAX_PATTERNS`), pozycja najwyżej 500 znaków (`REPORT_MAX_ENTRY_CHARS` = `MAX_LINE_CHARS`, znaki, nie bajty); treść najwyżej ok. 1,5 MB (`REPORT_MAX_BYTES`: najdłuższa lista, jaką aplikacja może wysłać, nawet z każdym znakiem zapisanym jako `\u0001`; zadeklarowana większa długość jest odrzucana bez czytania) |
| Odpowiedzi | `204` przyjęte (także bez zmiany); `403` zły `Host`, brak originu rozszerzenia albo nagłówka; `413` treść ponad `REPORT_MAX_BYTES`; `400` nie ten JSON albo ponad limity. Zawsze `Cache-Control: no-store`; nagłówki CORS tylko dla originu rozszerzenia |
| CORS | preflight (`OPTIONS`) dla originu rozszerzenia: `Access-Control-Allow-Methods: GET, POST, OPTIONS`, `Access-Control-Allow-Headers: X-FocusMode-Extension, Content-Type`, `Access-Control-Allow-Private-Network: true`; origin strony WWW nie dostaje nagłówków CORS, więc przeglądarka nie wyśle z niej zgłoszenia (własny nagłówek wymusza preflight) |
| Skutek | `BlocklistService::set_extension_skipped` zostawia tylko źródła obecne w bieżącej liście (`kept_skipped`: dokładne porównanie, kolejność z pliku, bez powtórzeń) i przy zmianie emituje `blocklist://changed`. Przeładowanie pliku zostawia w `extensionSkipped` tylko wzorce, które w nim zostały; błąd odczytu pliku zostawia wszystko. Logi podają tylko liczby |

Rozszerzenie (`browser-extension/background.js`, `BRIDGE_REPORT_URL` w `common.js`) wysyła listę `skippedPatterns` po ustawieniu reguł w Focus: po 2 s spokoju,
jedno zgłoszenie naraz, raz na każdą nową listę w danym focusie (zapamiętaną w `chrome.storage.session`; poza focusem zapomina ją, więc następny focus zgłasza
od nowa). Gdy aplikacja nie odpowie, ponawia przy następnym odświeżeniu stanu; każda odpowiedź HTTP (także `404` ze starszej aplikacji) kończy próby dla tej listy.

**Ochrona HTTP mostu** (`src-tauri/src/bridge/http.rs`, limity w `Limits::DEFAULT`; ADR-018, „Uzupełnienie: utwardzenie mostu”). Własny minimalny serwer
HTTP/1.1 na osobnym wątku `focus-bridge` z jednowątkowym runtime Tokio (`net`, `io-util`); trasy, `Host`, `Origin`, CORS i parsowanie zgłoszenia zostają w `bridge.rs`.
Lokalny proces, który łamie protokół, nie może wyczerpać pamięci, zablokować wątku mostu ani zamknąć aplikacji:

| | |
|---|---|
| Połączenie | jedno zapytanie na połączenie; każda odpowiedź ma `Connection: close`, po niej most zamyka połączenie. Nieprzeczytanej treści nie „drenuje” (zamknięcie może wtedy zresetować połączenie) |
| Głowa zapytania | wiersz zapytania i nagłówki razem ≤ 16 KiB i ≤ 48 pól (inaczej `431`); cała głowa w 2 s od przyjęcia połączenia (`408`; termin łączny, więc wysyłanie po bajcie go nie przedłuża); tylko `HTTP/1.0` / `HTTP/1.1` (`505` dla innych wersji), poza tym poprawne tokeny i wartości bez znaków sterujących (`400`) |
| Ramy treści | `400`: kilka `Host` / `Content-Length` / `Origin` / `X-FocusMode-Extension`, `HTTP/1.1` bez `Host`, `Content-Length`, który nie jest liczbą (albo nie mieści się w `u64`), `Content-Length` razem z `Transfer-Encoding`, treść przy metodzie innej niż `POST` (np. `GET` z `Content-Length: 5`, odrzucany bez czekania na treść). `Expect` inne niż `100-continue` → `417` |
| Treść | czytana tylko dla `POST /v1/report` z originem rozszerzenia i nagłówkiem `X-FocusMode-Extension: 1`, tylko z `Content-Length` ≤ `REPORT_MAX_BYTES`. Większa zadeklarowana długość (np. 2 GB) albo `Transfer-Encoding` (np. `chunked`) → `413` od razu, bez czytania i bez bufora o zadeklarowanej wielkości; bufor rośnie dopiero z nadchodzącymi bajtami. `100 Continue` wychodzi dopiero po sprawdzeniu limitu. Cała treść w 5 s (`408`); najwyżej 2 treści czytane albo obsługiwane naraz (pozwolenie trwa do końca obsługi zgłoszenia, więc wolna obsługa nie zbiera treści w pamięci; kolejna czeka w tym terminie, potem `503`). Odpowiedzi `413` / `408` / `503` też mają `Cache-Control: no-store` i CORS dla originu rozszerzenia |
| Odpowiedź | zapis w 2 s, potem połączenie jest zamykane |
| Wiele połączeń | powyżej 64 otwartych nowe połączenie zamyka najstarsze, ale tylko te otwarte co najmniej 100 ms (czas na wysłanie zapytania) i nigdy czytające treść; bezczynne albo powolne połączenia nie odetną więc rozszerzenia, a zalew nowych połączeń nie zamknie jego połączenia, zanim zapytanie zostało przeczytane. Twardy limit 1024 otwartych (wszystkie w swoich 100 ms): kolejne nowe połączenie jest od razu zamykane bez odpowiedzi (pamięć: najwyżej ok. 20 KiB na połączenie). Jedno nowe połączenie na obrót runtime, więc przyjęte wcześniej działają pomiędzy. Każde połączenie i tak kończy się w kilka sekund. Błąd `accept` samego połączenia (reset, przerwane) → nasłuch dalej bez przerwy; inny błąd (np. brak gniazd) → 50 ms przerwy i nasłuch dalej |

Testy na prawdziwych gniazdach (`src-tauri/src/bridge/server_tests.rs`): zwykłe zapytania rozszerzenia (`200`, `204`, preflight, `Expect: 100-continue`,
najdłuższe możliwe zgłoszenie), `Content-Length` 2 GB i `u64::MAX` (`413` w < 1 s, połączenie zamknięte, żadna alokacja ≥ 256 MiB w całym procesie testów),
treść `chunked`, `GET` z treścią, powolne i za duże nagłówki, zatrzymana treść, limit czytanych treści, wolna obsługa zgłoszenia (druga treść czeka), 30 bezczynnych połączeń przy limicie 8, zalew połączeń (4 klienci otwierający je bez przerwy: każde z 50 zapytań rozszerzenia dostaje `200`), kolejka połączeń przed startem serwera (zapytanie rozszerzenia za 40 i przed 60 pustymi, także wysłane z opóźnieniem 50 ms przy limicie 16), twardy limit otwartych połączeń i 16 równoległych klientów obok
40 bezczynnych połączeń. Uwaga: oprogramowanie zabezpieczające, które sprawdza HTTP (spotkane na komputerze deweloperskim z Windows), potrafi wstrzymać niepełną
głowę `GET` / `POST` na loopbacku, aż będzie kompletna; testy niepełnych głów używają więc metody, której ono nie rozpoznaje (`FOO`), bo do końca głowy serwer traktuje wszystkie metody tak samo.

**Okienka a prezentacja** (`focus-platform/src/notification_state.rs`, `src-tauri/src/popup.rs`):
- `notification_state::user_is_presenting() -> bool`: `SHQueryUserNotificationState`. `true` dla `QUNS_BUSY` (2), `QUNS_RUNNING_D3D_FULL_SCREEN` (3),
  `QUNS_PRESENTATION_MODE` (4) i `QUNS_APP` (7); `false` dla `QUNS_NOT_PRESENT` (1), `QUNS_ACCEPTS_NOTIFICATIONS` (5), `QUNS_QUIET_TIME` (6), wartości nieznanej
  i błędu. Bez COM, z dowolnego wątku. Poza Windows zawsze `false`. Import `shell32` zadeklarowany ręcznie (`windows::core::link!`), bo `windows` nie ma
  włączonej funkcji `Win32_UI_Shell` (`TODO(hackathon)`).
- Wątek okienek: przy każdym okienku pyta `user_is_presenting()`. Gdy `true`, okienko trafia do `held` (jedno, najnowsze; kolejne spotyka je według tych samych
  reguł co okienko na ekranie, `merge_held` / `arrival`: zastępuje, odkłada przypomnienie o spotkaniu albo paczkę „Co dalej?” do `parked`, albo paczka czeka za nim).
  Co `PRESENTING_POLL` (5 s) wątek pyta ponownie; po końcu prezentacji pokazuje okienko (przypomnienie o zakończonym już spotkaniu pomija), potem kolejne z `parked`.
  Po `PRESENTING_HOLD_MAX` (10 min) od przyjścia okienko przepada bez toastu zastępczego. Odpowiedź na spotkanie (`resolve_meeting`) i start focusa
  (`withdraw_work_digest`) usuwają czekające okienko tak samo jak widoczne. Okienka już widocznego prezentacja nie chowa.

**Drugie uruchomienie z `--demo`** (`src-tauri/src/lib.rs`, `session_actor.rs`): `on_second_instance` wysyła do aktora `ActorCmd::UseDemoProfile` i pokazuje okno główne.
Aktor w Idle przełącza profil Idle na `TimingProfile::demo()` (od teraz do końca procesu; snapshot `demo: true`, następny Start w demo). W trakcie sesji (także
w pauzie) profil zostaje, a powłoka pokazuje toast `texts::demo_needs_stop_toast` („Zatrzymaj sesję, aby włączyć tryb prezentacji”); gdy profil demo już działa, nic się nie zmienia (bez toastu). Zmienne środowiska
(`FOCUSMODE_DEMO`) drugiego uruchomienia nie docierają do działającej instancji.

**Autostart** (`src-tauri/src/lib.rs`, `refresh_autostart`): przy starcie z włączonym `launchOnLogin` (poza `--restore-shields`) powłoka pyta wtyczkę
`is_enabled()`; tylko przy `true` robi `disable()` + `enable()`, więc wpis w `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` dostaje bieżącą ścieżkę
i `--autostart`. Wpisu brakującego albo wyłączonego w Menedżerze zadań (`…\Explorer\StartupApproved\Run`) nie rusza: `enable()` ustawia tam „włączony”.
Błędy tylko w logu. Zmiana `launchOnLogin` w Ustawieniach działa jak dotąd (`sync_autostart`).

**Maskotka** (frontend, `src/lib/mascot/`; grafika: `static/mascot/`, źródła i opis obróbki: `art/mascot/`, narzędzie: `tools/mascot/`):

| Plik | Rola |
|---|---|
| `assets.svelte.ts` | pobiera raz na okno manifest i atlas wariantu (`main`: `/mascot/axo-manifest.json` + `axo-atlas.webp`; `peek`: `/mascot/peek-manifest.json` + `peek-atlas.webp`); `FRAME_GEOMETRY` (rozmiar klatki i kotwica znane przed pobraniem, więc ramka ma od razu docelowy rozmiar) |
| `frames.ts` | czyste funkcje: klatka spoczynku (`blink-1` / `idle`), mrugnięcie, reakcje (`start`, `break`, `cycle`, `stopEarly`, `nudge`, `meeting`, `digest`) jako kroki z czasami, ruch ust z tekstu (`lipSyncSteps`: `CHAR_MS` = 70 ms na znak, najwyżej `TALK_MAX_MS` = 6 s) |
| `controller.svelte.ts` | `MascotController`: odgrywa kroki, losowe mrugnięcie co 3–7 s (`BLINK_DELAY_MS`), `prefers-reduced-motion`, sprzątanie timerów przy odmontowaniu |
| `cues.svelte.ts` | zdarzenia sesji → reakcje maskotek w oknie głównym (zmiana fazy `PhaseChange`: Idle → Focus = start, Przerwa → Focus = ukończony cykl, → Przerwa = przerwa; podsumowanie z `focusedSecs` mniejszym niż długość focusa = wcześniejszy Stop); maskotka, która pojawi się zaraz po zdarzeniu (karta przerwy, Podsumowanie), też reaguje, jeśli zdarzenie jest świeże (do 4 s) |
| `Axolotl.svelte` | jedna klatka jako tło elementu (`background-position` / `background-size` z manifestu); ramka ma rozmiar od pierwszego malowania; w motywie `pixel` klatka zmniejszana do małego canvasu (piksel grafiki = 2–4 px CSS; filtr przeglądarki uśrednia piksele), alfa progowana do 0 albo 255, canvas powiększany z `image-rendering: pixelated`; `aria-hidden` |
| `SpeechBubble.svelte` | dymek ze zwykłym tekstem (`aria-live="polite"` tylko dla nowych porad) |
| `FocusMascot.svelte` | maskotka obok minutnika na stronie Skupienie (140 px; w wąskim oknie 84 px w rogu pierścienia, bez dymka), pozycjonowana absolutnie, więc nie przesuwa strony; w Idle „mówi” pierwszą pozycję „Co dalej?” albo dobrą praktykę dnia; w przerwie przechodzi do karty przerwy (`BreakCard.svelte`), więc na ekranie jest jeden aksolotl |
| `theme.svelte.ts` | czy działa motyw `pixel` (atrybut `data-theme` na `<html>`, w oknie głównym i w `popup`) |

Manifest (`static/mascot/*-manifest.json`): `frameWidth`, `frameHeight`, `columns` (6), `anchorX`, `anchorY` (wspólne dla wszystkich klatek wariantu)
i `frames: { "<nazwa>": { x, y, scale } }` (lewy górny róg klatki w atlasie; `scale` to współczynnik wyrównania użyty przy cięciu, informacyjnie).
Nazwy klatek: ADR-018 pkt 1. Atlasy i manifesty powstają tylko narzędziem z `tools/mascot/`; grafiki się nie przerysowuje (CLAUDE.md, zasada 10).
CSP bez zmian: atlas i manifest są z własnego originu (`img-src 'self'`, `connect-src 'self'`).

### 6.11 Tryb dewelopera (ADR-019)

Nowe pole `Config` (stary `config.json` bez niego wczytuje się z `false`):

| Pole | Typ | Domyślnie | Znaczenie |
|---|---|---|---|
| `developerMode` | `boolean` | `false` | czwarta zakładka „Deweloper” (trasa `/dev`) z przyciskami do testów i zrzutów ekranu; Ustawienia → Deweloper. Bez walidacji w core |

Nowe pole DTO:

| Typ | Pole | Uwagi |
|---|---|---|
| `PopupInfo` | `sticky: boolean` | `#[serde(default)]`; `true` tylko dla przykładowego okienka z `dev_trigger` z `sticky`: strona okienka nie odlicza, a powłoka go nie chowa (zostaje do X). Prawdziwe okienka zawsze `false` |

Nowa komenda `dev_trigger { action: DevAction } -> ()` (§6.1), bez nowych eventów: okienko przychodzi zwykłą parą `get_popup` + `popup://show`,
skrócona faza w `session://snapshot`. `DevAction` to enum z tagiem `type` (`#[serde(tag = "type")]`, pola w camelCase):

| `type` | Pola | Skutek |
|---|---|---|
| `popup` | `kind: PopupKind`, `leadMinutes: number \| null`, `sticky: boolean` | okienko faz z przykładowym `PopupInfo` od powłoki: cykl i długość fazy z bieżącego `TimingProfile`, przykładowe przypomnienie, przykładowe spotkanie (id z prefiksem `dev-sample-`) dla `meetingReminder` / `breakHeld`, `leadMinutes` 30 / 20 / 10 (`null` = 30; przy 10 bez „przypomnij później”), Steam + Epic dla `launchersClosed`, `workCount` 3 dla `workDigest`. Pokazuje się od razu, także w czasie prezentacji (bez czekania z §6.10) |
| `toast` | `phase: Phase` | ten sam toast Windows co przy prawdziwej zmianie fazy (`focus` / `break`; `idle` to błąd) |
| `setRemaining` | `secs: number` | tylko w trwającej sesji (w Idle → `notRunning`): bieżąca faza kończy się za `secs` s, przycięte do 1..=długość fazy, przez nowy `Input` w core (`Input::DevSetRemaining`); zegar monotoniczny i profil czasów bez zmian |
| `presentationMode` | `on: boolean` | tryb prezentacji: profil `TimingProfile::demo()` (ten sam co `--demo` / `FOCUSMODE_DEMO=1`, 40 s / 20 s) albo profil z configu, do końca działania procesu (nic się nie zapisuje; restart = zwykłe czasy, chyba że z `--demo`). W Idle od razu (`SessionSnapshot.demo` w snapshocie), w trwającej albo wstrzymanej sesji dopiero od następnego Startu: bieżąca sesja zachowuje swój profil i długości faz (`ActorCmd::SetPresentationMode`, zmienia tylko profil Idle). Sesje w trybie prezentacji mają `demo: true`, jak przy `--demo` |
| `factoryReset` | — | reset fabryczny: to samo co `delete_all_data` (ta sama ścieżka kodu, `privacy::delete_all`: najpierw Stop i zejście osłon), potem `config.json` i pusty folder `sounds` (`Persistence::remove_for_first_start`), potem restart aplikacji (`request_fresh_restart` w `lib.rs`: przez zwykłe wyjście, więc osłony i kolory ekranu wracają; nowy proces startuje po zwolnieniu blokady single-instance, bez `--autostart` i `--restore-shields`, więc z oknem głównym jak pierwsze uruchomienie; `--demo` zostaje). Pliki przywracania osłon i odcienia usuwają ich wątki po przywróceniu (zasada 7); niedokończone przywrócenie kończy następny start. Nic poza folderami aplikacji (zainstalowana aplikacja, folder rozszerzenia i foldery z muzyką zostają). Błąd usuwania → `privacy.delete` i bez restartu |

Kody błędów: `developerModeOff` (tryb wyłączony, każda akcja), `notRunning` i `dev.invalidPhase` (`toast` z `idle`, `field: "phase"`). `set_meeting_choice` dla spotkania `dev-sample-…`
jest przyjmowane jako no-op: okienko próbki się zamyka, a wynik to bieżący `CalendarStatus` (nic nie trafia do `meeting-choices.json` ani do sesji); „Pomiń fazę” (`popup_action`)
na przykładowym okienku tylko je zamyka (decyduje wątek okienek: przycisk w ciągu 500 ms od pojawienia się nowego okienka należy do poprzedniego). Przykładowe okienka nigdy nie czekają w kolejce: zastępują inną próbkę, odkładają prawdziwe pytanie o spotkanie albo paczkę „Co dalej?”
jak każde okienko, a prawdziwe okienko zastępuje próbkę. Próbka `sticky` nie ma limitu czasu: powłoka co 5 s sprawdza tylko, czy jej okno istnieje (zamknięte Alt+F4 → odłożone okienka wracają), a wyłączenie `developerMode` ją zamyka. Powłoka: `src-tauri/src/dev.rs` (próbki), `popup.rs` (`PopupRequest.sample` / `sticky`), `ActorCmd::DevSetRemaining`. Przykłady argumentów: `{ "action": { "type": "popup", "kind": "meetingReminder", "leadMinutes": 10, "sticky": true } }`,
`{ "action": { "type": "toast", "phase": "break" } }`, `{ "action": { "type": "setRemaining", "secs": 5 } }`, `{ "action": { "type": "presentationMode", "on": true } }`, `{ "action": { "type": "factoryReset" } }`.

Frontend: trasa `src/routes/(app)/dev/+page.svelte`, stan `src/lib/state/dev.svelte.ts` (próbki i „Clean screenshots” tylko w pamięci; Ctrl+Shift+D przywraca
ukrytą zakładkę), próbki danych w `src/lib/state/samples.ts` (budowane w chwili kliknięcia, id `dev-sample-…`, linki `https://dev-sample.invalid/`, których
aplikacja nie otwiera; magazyny podsumowania, kalendarza, „Co dalej?” i sesji mają warstwę `sample` nad prawdziwą, a prawdziwe podsumowanie nowej sesji ją usuwa), teksty strony Deweloper tylko po angielsku w `src/lib/i18n/dev.ts` (poza typem `Strings`; przełącznik w Ustawieniach i nazwa zakładki są w 10 językach).
Przełącznik „Presentation mode” (karta Session) pokazuje stan z `SessionSnapshot.demo`, a przełączony w trakcie sesji — zapamiętaną w pamięci wartość dla następnego Startu
(z dopiskiem, że bieżąca sesja zachowuje długości). „Factory reset…” (karta Window & app, styl `btn-danger`) otwiera `ConfirmDialog` z listą tego, co znika; bez wpisywania potwierdzenia.

### 6.12 Panel prywatności (ADR-020)

Ustawienia → „Prywatność i dane”: lista tego, co aplikacja trzyma na komputerze, z akcjami. Komendy `get_privacy_report`, `open_data_folder`,
`export_privacy_data { path }`, `delete_privacy_data { kinds }`, `delete_all_data` (§6.1), stan `PrivacyReport` z rewizją (`get_privacy_report` + `privacy://changed`).

```rust
pub struct PrivacyReport { pub revision: u32, pub folders: Vec<String>, pub items: Vec<PrivacyItem>, pub busy: bool }
pub struct PrivacyItem {
    pub kind: PrivacyItemKind,
    pub present: bool,
    pub size_kib: Option<u32>,   // KiB w górę (niepusty plik ≥ 1); None: brak, nieznany albo za drogi do policzenia
    pub count: Option<u32>,      // sesje, wybory, id, linki, pliki, wpisy listy — tam, gdzie ma sens
    pub deletable: bool,         // = PrivacyItemKind::is_deletable()
    pub includes_titles: bool,   // tylko `history`: któraś zapisana sesja ma tytuły okien
}
```

`folders`: wszystkie foldery z danymi aplikacji, bez powtórzeń i bez zagnieżdżonych w innych z listy; pierwszy otwiera `open_data_folder`
(`app_data_dir`, na Windows `%APPDATA%\com.focusmode.app`: ustawienia, historia, tokeny, dźwięki), dalej `app_config_dir`, gdy jest inny (Linux),
i `app_local_data_dir` (`%LOCALAPPDATA%\com.focusmode.app`: profil WebView2 `EBWebView` i logi).

| `PrivacyItemKind` | Co to jest | `count` | Usuwanie (`delete_privacy_data`) |
|---|---|---|---|
| `settings` | `config.json` + `config.json.bak` | – | nie pojedynczo; `delete_all_data` zapisuje `Config::default()` jak `update_config` i usuwa `.bak` |
| `history` | `history.jsonl` (`includesTitles`, gdy któraś sesja ma tytuły okien) | sesje | jak `clear_history` (plik + ostatnie podsumowanie) |
| `rewards` | `rewards.json` + `rewards.json.bak` i odłożone `rewards.unreadable-*.json` (funkcje beta, ADR-022): liczby, numery dni i id sesji, bez tekstów | dni (z `rewards.json`) | jak `reset_rewards` (nowa księga w pamięci), przez wątek zapisu nagród, więc żaden zaległy zapis nie przywróci pliku; usuwa też odłożone `rewards.unreadable-*.json`. Nowego pliku nie zapisuje: pozycja wraca dopiero z następną iskierką |
| `meetingChoices` | `meeting-choices.json` (id spotkań i wybory, bez tematów) | wybory | plik + wybory w pamięci modułu kalendarzy (blokady spotkań znikają z planu) |
| `workSeen` | `work-seen.json` (id pozycji, bez tytułów) | id | plik + pamięć modułu „Co dalej?”; bieżące pozycje trafiają do znanych (`Announcer::remember`), więc nie wracają jako okienko `workDigest`, a na liście są „nowe”, dopóki użytkownik jej nie obejrzy (wtedy plik wraca) |
| `microsoftToken` | `teams-token.bin` (DPAPI): logowanie Microsoft dla Teams i kalendarza | – | jak `teams_disconnect` (status Teams przywrócony wcześniej przez Stop) |
| `googleToken` | `google-token.bin` (DPAPI) | – | jak `disconnect_calendar(google)` |
| `icsLinks` | `ics-links.bin` (DPAPI) | linki, gdy znane | jak `disconnect_calendar(ics)` |
| `githubToken` … `linearToken` | `work-<narzędzie>-token.bin` (DPAPI), po jednym na narzędzie (`PrivacyItemKind::work_source`) | – | jak `set_work_token(source, null)` |
| `customChimes` | własne dzwonki w folderze `sounds` | pliki | pliki `custom-*` + `customChimes` w configu na puste; `chimeSound: custom` → domyślny `midnight` |
| `blocklist` | `websiteblacklist.txt` | domeny + wzorce | zapis domyślnej listy wbudowanej w aplikację (→ `blocklist://changed`) |
| `shieldsRestore` | `shields-restore.json` (tylko gdy osłony są podniesione albo po awarii) | – | **nigdy** (zasada 7): znika sam po przywróceniu ustawień Windows |
| `tintRestore` | `tint-restore.json` (tylko gdy odcień działa albo po awarii) | – | **nigdy** (zasada 7) |
| `browserData` | profil WebView2 (`EBWebView`): ciasteczka (też logowanie i zgoda YouTube), cache, localStorage | – | najpierw zamyka okna `media` i `youtube-login`, potem `clear_all_browsing_data()` na oknie głównym (wspólny profil); wylogowuje z YouTube (`youtube://account`), czyści `localStorage` aplikacji (lista linków odmawiających osadzenia, stan strony Deweloper); przy następnym oknie YouTube znów pytanie o ciasteczka |
| `logs` | pliki logu (`app_log_dir`) | pliki | usuwa starsze pliki, bieżący (`<productName>.log`) skraca do zera (logger trzyma go otwartym; plik, którego nie da się usunąć, też jest skracany) |

Zasady:
- **Usuwanie zatrzymuje trwającą sesję** (`ActorCmd::Stop`, jak Stop z UI) i czeka (do 8 s) na fazę Idle i zejście osłon (`ShieldStatus.active == false`),
  potem jeszcze 0,5 s na dopisanie podsumowania do `history.jsonl` (dzieje się na puli blokującej), więc usunięcie historii obejmuje także tę sesję.
  UI w tym czasie nie przechodzi do Podsumowania (`privacy.deleting`). Usuwania i eksporty idą po jednym; na ich czas `busy: true`.
- Lista rodzajów z `delete_privacy_data` przechodzi przez `focus_core::privacy::deletion_plan` (bez powtórzeń, zawsze w kolejności `PrivacyItemKind::ALL`;
  rodzaj nieusuwalny odrzuca całe żądanie, zanim cokolwiek zniknie). Błąd jednej pozycji nie zatrzymuje pozostałych; na końcu `privacy.delete` z listą
  rodzajów, które się nie udały, a UI pobiera wtedy config na nowo (`get_config`).
- `delete_all_data` po kolei: Stop → zamknięcie okien `media` i `youtube-login` → wszystkie pozycje `deletable` → `Config::default()` zapisany jak przez
  `update_config` (odcień wyłączony: kolory wracają, `tint-restore.json` znika; `launchOnLogin` wyłączony: wpis autostartu aplikacji znika; `closeToTray`
  znów `null`, więc pierwsze X znów pyta) → `privacy://changed`. UI przejmuje cały zwrócony config. Poza folderami aplikacji zmieniają się tylko rzeczy
  samej aplikacji: wpis autostartu i ustawienia Windows przywracane jak przy Stop (zasada 7). Rozszerzenie przeglądarki zostaje (usuwa je użytkownik).
- Eksport (`export_privacy_data`) pisze atomowo (plik tymczasowy + zmiana nazwy) pod ścieżkę z okna zapisu (`pickSaveFile` w `api.ts`; domyślna nazwa
  `focusmode-export-RRRR-MM-DD.json`, brak rozszerzenia → `.json`). Ścieżka musi być pełna i leżeć poza folderami aplikacji (`folders`, porównanie bez
  wielkości liter), inaczej `privacy.export` z polem `path`: eksport nigdy nie nadpisze danych aplikacji. Dokument buduje `focus_core::privacy::Export`. Format:

```json
{ "format": "focusmode-export", "version": 1, "exportedAtMs": 1760360000000, "appVersion": "0.1.0",
  "config": { "focusMinutes": 45, "...": "...", "calendar": { "...": "...", "googleClientSecret": "" } },
  "history": [ { "sessionId": "1760350000000-12345-1", "...": "..." } ],
  "meetingChoices": "zawartość meeting-choices.json albo null",
  "workSeen": "zawartość work-seen.json albo null",
  "rewards": "zawartość rewards.json (pole tylko, gdy plik istnieje; ADR-022)" }
```

  `config` bez sekretów: `calendar.googleClientSecret` puste. Tokeny, linki ICS, ciasteczka, dźwięki i logi **nigdy** nie trafiają do eksportu
  (UI mówi to jednym zdaniem przy przycisku). Historia jest taka, jak zapisana (z tytułami okien, jeśli były).
- Raport liczy powłoka (`src-tauri/src/privacy.rs`, wątek „focus-privacy”; getter skanuje od razu na puli blokującej): rozmiary z metadanych plików;
  `history.jsonl`, `meeting-choices.json` i `work-seen.json` są parsowane tylko po zmianie rozmiaru albo czasu modyfikacji (same liczby); liczba linków ICS
  pochodzi ze statusu kalendarzy, liczba wpisów listy stron z `BlocklistInfo` (pliku DPAPI nikt tu nie otwiera); `browserData` przez przejście folderu
  `EBWebView` (bez dowiązań) z limitem 20 000 wpisów albo 2 s (potem `sizeKib: null`), najwyżej co 30 s, od razu po wyczyszczeniu.
  Odświeżanie: co 3 s (same metadane, więc panel widzi też pliki usunięte ręcznie w folderze) i ~0,4 s po eventach modułów, które coś zapisują
  (`calendar://changed`, `work://changed`, `teams://status`, `blocklist://changed`, `tint://changed`, `shields://status`, `session://summary`,
  `youtube://account`) oraz po każdym usuwaniu i eksporcie. Event tylko przy innym wyniku; skan trzyma swój zamek do publikacji, więc starszy wynik
  nigdy nie nadpisze nowszego.
- Kody błędów: `privacy.export` (pole `path` przy złej ścieżce), `privacy.delete`, `privacy.notDeletable` (pole `kinds`), `privacy.openFolder`,
  `privacy.report` (skan się nie udał).
- UI: `src/lib/components/PrivacySettings.svelte` (sekcja Ustawień przed „Deweloper”, grupy: ustawienia i historia, połączenia, przeglądarka i logi,
  pliki przywracania), stan `src/lib/state/privacy.svelte.ts` (subskrypcja tylko, gdy panel jest widoczny). Każde usuwanie za `ConfirmDialog`
  (skutek pozycji + „sesja zostanie zatrzymana”, gdy trwa); „Usuń wszystko” wymienia obecne pozycje i ustawienia (`ConfirmDialog.list`).
  Pozycja `browserData` ma znaczek „Zalogowano do YouTube” (`strings.youtube.signedIn`), gdy `YoutubeAccount` mówi, że w tych ciasteczkach
  jest logowanie do YouTube (ADR-021, §6.13); stan bierze z `state/youtubeAccount.svelte.ts`, raport prywatności go nie powiela.
  Pozycja `rewards` (ADR-022) jest na liście, gdy `rewards.json` istnieje albo funkcje beta są włączone; bez tego panel wygląda jak przed ADR-022.

### 6.13 Logowanie do YouTube dla Premium (ADR-021)

Komendy `get_youtube_account`, `youtube_sign_in`, `youtube_sign_out` (§6.1), stan `YoutubeAccount` z rewizją (`get_youtube_account` + `youtube://account`).

```rust
pub struct YoutubeAccount {
    pub revision: u32,
    pub known: bool,            // ciasteczka sprawdzone co najmniej raz od startu
    pub signed_in: bool,        // jest któreś z: LOGIN_INFO, SAPISID, __Secure-3PAPISID na https://www.youtube.com/
    pub sign_in_open: bool,     // okno `youtube-login` jest otwarte
    pub browser_rejected: bool, // Google pokazał „Ta przeglądarka lub aplikacja może nie być bezpieczna” (URL z `/signin/rejected`)
}
```

Moduł powłoki: `src-tauri/src/youtube_account.rs` (`YoutubeAccountHandle` w `AppState.youtube`). UI: `src/lib/state/youtubeAccount.svelte.ts`
(subskrypcja w `state/app.ts`), wiersz „Konto YouTube” w Ustawieniach → Dźwięk (`YoutubeAccountSettings.svelte`), linijka w wyborze muzyki
(`MusicSourcePicker.svelte`, gdy stream albo linki to YouTube) i podpowiedź w pasku odtwarzacza. Teksty: `strings.youtube` (10 języków), tytuł okna
`texts.youtube_sign_in_title`; kody błędów `youtube.signIn`, `youtube.signOut`.

- **Okno `youtube-login`**: widoczne, wyśrodkowane, 480×640 (min. 380×480, logiczne px), tytuł z `texts` (10 języków, zmienia się z językiem),
  `with_main_webview_options`, czyli ten sam profil WebView2 co `main` i `media` (wspólne ciasteczka). Startuje na
  `https://accounts.google.com/ServiceLogin?service=youtube&continue=https%3A%2F%2Fwww.youtube.com%2F` (Google przekierowuje na bieżący formularz).
  Bez skryptu inicjalizacyjnego i bez `eval`: aplikacja nie widzi tego, co użytkownik wpisuje. Nawigacja tylko https na `accounts.google.com`,
  `consent.google.com`, `myaccount.google.com`, `gds.google.com` (krok „zadbaj o możliwość logowania”), `www.google.com` (sprawdzenie „nietypowy ruch”),
  `youtube.com` z subdomenami (też `accounts.youtube.com` i `consent.youtube.com`) i `accounts.google.<kraj>` (`google.pl`, `google.co.uk`,
  `google.com.au`; ta sama reguła doszła do okna `media`, bo zalogowana strona może odświeżać logowanie przez te hosty); reszta i każde nowe okno →
  przeglądarka systemowa (`window_util::open_link_externally`, najwyżej raz na 2 s na okno, w logu tylko host). Okno ma zwykłe X (bez chowania).
  Ponowne `youtube_sign_in` aktywuje otwarte okno, a po odmowie Google wraca na stronę logowania i zeruje `browserRejected`
  (przycisk w wierszu nazywa się wtedy „Zacznij logowanie od nowa”, a nie „Pokaż okno logowania”).
- **Wykrycie zalogowania**: co 1,5 s zadanie async czyta na wątku blokującym (odczyt ciasteczek WebView2 na głównym wątku się zakleszcza) adres strony
  okna (`url()`) i nazwy ciasteczek dla `https://www.youtube.com/` (`cookies_for_url`); wartości są odrzucane od razu, porównywane są **tylko nazwy**.
  Któraś z `LOGIN_INFO`, `SAPISID`, `__Secure-3PAPISID` oznacza zalogowanie; gdy do tego przepływ wrócił na
  stronę YouTube (`www`/`m`/`music.youtube.com`, poza `/signin…`) albo YouTube zapisał `LOGIN_INFO` (najpóźniej 8 s po pojawieniu się ciasteczek),
  1 s później `signedIn: true` (dopiero wtedy, żeby przeładowanie okna `media` dostało komplet ciasteczek) i okno się zamyka (`destroy`).
  Sprawdzanie kończy się z oknem (albo po 30 min; okno wtedy zostaje). Zamknięcie okna → `signInOpen: false`; gdy zamknął je użytkownik (X), jeszcze
  jedno sprawdzenie ciasteczek (mógł zamknąć okno zaraz po zalogowaniu). Okno zamknięte przez samą aplikację (po zalogowaniu, przy wylogowaniu, przy
  usuwaniu danych przeglądarki) tego sprawdzenia nie robi: odczyt sprzed zastosowania usunięcia mógłby opublikować „zalogowano” już po wyniku
  sprawdzenia wylogowania. Strona odmowy Google (`/signin/rejected`, też `/v3/…`, na `accounts.google.*`;
  wykrywana w `on_navigation` i w odpytywaniu `url()`, bo formularz Google bywa SPA) → `browserRejected: true`; okno zostaje, żeby było widać wyjaśnienie
  Google, a UI pokazuje jedno zdanie do następnej próby.
- **Start aplikacji** (poza `--restore-shields`, `YoutubeAccountHandle::start`): sprawdzenie nazw ciasteczek przez okno główne na wątku blokującym (do 5 prób co 3 s,
  gdy WebView2 jeszcze nie odpowiada, każda najwyżej 10 s) → `known: true`. Do tego czasu wiersz w Ustawieniach pokazuje „Sprawdzam…” (logowanie już działa), a
  odtwarzacz zachowuje się jak bez logowania. Gdy po 20 s od startu stan dalej jest nieznany (wszystkie próby zawiodły), wiersz mówi jednym zdaniem
  „Stan logowania do YouTube nieznany”, zostawia „Zaloguj się do YouTube” i daje cichy przycisk „Sprawdź ponownie”: woła `get_youtube_account`,
  które przy nieznanym stanie uruchamia jeszcze jedno sprawdzenie (jedno naraz); bez wyniku w 12 s wiersz wraca do „nieznany”.
- **Odtwarzanie po zalogowaniu** (okno `media` nie zgłasza zdarzeń odtwarzania, więc postęp załatwia adres strony): ciąg pojedynczych filmów z listy
  (od bieżącego, w kolejności po tasowaniu, najwyżej 50) okno otwiera jako `https://www.youtube.com/watch_videos?video_ids=A,B,…` (YouTube odpowiada 303
  na tymczasową playlistę `/watch?v=A&list=TLGG…` dokładnie tych filmów i sam po niej przechodzi), linki do playlist zostają przy `list=`, a kontroler
  w `media_window.rs` zapętla stronę `/watch` (po ostatnim filmie zatrzymuje się na początku, na 1,5 s ustawia tytuł-znacznik `END_TITLE`, który
  powłoka zamienia na chwilowy brak tytułu, i dopiero potem `playVideoAt(0)` / `playVideo()`; gdy autoodtwarzanie YouTube zdążyło uciec,
  raz na dokument `location.replace` na adres otwarcia). `engine.ts` śledzi postęp po tytule strony z `media://window` („n z m”, „Następny” od
  właściwej pozycji); nowy tytuł pierwszego filmu albo chwila bez tytułu to „pełny obieg”, po którym lista z innymi pozycjami przechodzi dalej
  (także pojedynczy film, którego tytuł się nie zmienia). „Następny” z ostatniego filmu kolejki wracający na jej początek ładuje ją od nowa
  (ten sam adres z fragmentem `#n`, którego przeglądarka nie wysyła do YouTube), a Stop zamyka okno z kolejką.
- **Wylogowanie**: zamyka okno logowania, potem `cookies()` okna głównego (cały profil) i `delete_cookie` dla domen `youtube.com`, `google.com`,
  `google.<kraj>` i ich subdomen, **poza** `SOCS` i `CONSENT` (wybór ciasteczek zostaje, więc muzyka nie trafia znów na okno zgody). WebView2 usuwa po
  nazwie, ścieżce i *dokładnej* domenie (`.youtube.com` ≠ `youtube.com`), a `cookie::Cookie::domain()` (0.18), którą Tauri podaje do WebView2, ucina
  wiodącą kropkę; dlatego każde usunięcie idzie dwa razy: w formie z odczytu i z domeną zapisaną jako `..domena` (z niej `domain()` daje `.domena`;
  zachowanie przypięte testem), poza `__Host-*` (zawsze host-only). Potem do 15 sprawdzeń co 400 ms, aż ciasteczek logowania nie ma (WebView2 usuwa
  asynchronicznie); jeśli zostały → błąd `youtube.signOut`.
- **Dla panelu prywatności** (ADR-020, `browserData`): `state.youtube.close_sign_in_window(&app)` przed `clear_all_browsing_data()`, potem
  `state.youtube.refresh(&app, true)` (ponowne sprawdzenia jak przy wylogowaniu, wynik w `youtube://account`).
- **Zmiana konta a okno `media`**: każda zmiana `signedIn` po pierwszym sprawdzeniu przeładowuje otwarte okno `media` (`MediaWindow::reload`; hak
  `on_page_load` znów stosuje żądany stan grania i głośność), żeby odtwarzacz YouTube na stronie użył nowych ciasteczek (Premium od razu po zalogowaniu).
- **Odtwarzacz** (`src/lib/player/engine.ts`, `setSignedIn` + `windowPageOf`): przy `known && signedIn` każda pozycja YouTube (link, playlista, stream)
  gra w oknie `media`, bo tylko tam ciasteczka youtube.com są własne (strona najwyższego poziomu na youtube.com), więc działa Premium (bez reklam).
  Osadzony iframe w `main` (origin `tauri.localhost`) dostaje je jako ciasteczka stron trzecich, na których nie polegamy. Link do playlisty
  (`/playlist?list=X` na YouTube albo YouTube Music) okno otwiera wtedy jako `https://www.youtube.com/watch?list=X` (`windowUrlOf`): strona playlisty
  tylko wylicza filmy i niczego nie odtwarza, a strona odtwarzania z samym `list` gra playlistę od pierwszego filmu. Zalogowanie przenosi pozycję
  YouTube, którą trzyma iframe, do okna od razu (w pauzie: iframe się zwalnia, wznowienie ładuje okno). Bez zalogowania wszystko jak dotąd (najpierw
  iframe, okno dla odmów osadzania); wylogowanie nie przerywa bieżącej pozycji. Efekt `setSignedIn` w `MusicPlayer.svelte` stoi przed efektem playlisty,
  więc znany stan konta kieruje już pierwszą pozycję nowej playlisty.
- Logi: tylko hosty i zdarzenia („signed in”, „signed out”, „rejected”), nigdy nazwy kont, adresy z parametrami ani wartości ciasteczek.

### 6.14 Funkcje beta: iskierki, seria i małe kroki (ADR-022)

Kontrakt z R-00, małe kroki w sesji (R-02, `session.rs`), reguły w `focus-core/src/rewards.rs` (R-03), dźwięki i data lokalna (R-04) oraz usługa nagród
w powłoce (R-05, `src-tauri/src/rewards.rs`). Wszystko działa tylko przy `Config.betaFeatures` (domyślnie wyłączone); bez tego aplikacja zachowuje się jak
przed ADR-022: fakty sesji są odrzucane, `rewards://changed` nigdy nie ma `active: true`, okienka nie mają linii nagród, linie `history.jsonl` nie mają
`sparks`, a `set_session_steps` / `set_step_done` odpowiadają `stepsOff`.

```rust
pub struct RewardsStatus {
    pub revision: u32, pub active: bool /* betaFeatures && rewards.enabled */, pub preview: bool /* księga podglądu demo */,
    pub total_sparks: u32, pub level: LevelView /* { level, into, span } */,
    pub today: TodayView /* { date: "RRRR-MM-DD", sparks, goal, goalMet, pondFull } */,
    pub streak: StreakView, pub week: Vec<DayCell> /* 7, pon..niedz */, pub session: Option<SessionSparks>,
    pub collection: Collection /* { bubbles, pearls, golden } */, pub odds: SurpriseOdds, pub stickers: Vec<StickerId> /* P1 */,
    pub pending: Vec<RewardMoment> /* najstarsze pierwsze, najwyżej 8 */,
}
pub struct StreakView { pub state: StreakState /* first | alive | waiting | paused */, pub days: u32 /* 0 dla first/waiting: UI go nie pokazuje */,
    pub best: u32, pub active_days: u32, pub today_counted: bool, pub rest_bank: u32, pub rest_bank_max: u32, pub rest_reserved: u32,
    pub next_rest_in: u32 /* 1..=5 */ }
pub struct DayCell { pub date: String, pub mark: DayMark /* goal | rest | neutral | quiet | today | future */ }
pub struct SessionSparks { pub session_id: String, pub sparks: u32, pub lines: Vec<RewardLine> }
pub struct RewardLine { pub kind: RewardKind, pub sparks: u32, pub count: u32 }
// RewardKind: start | firstOfDay | plan | focusTime | fullBlock | breakTaken | backFromBreak | step | allSteps | goalDone | dailyGoal | comeback | surprise
pub struct SurpriseOdds { pub none_permille: u16, pub bubble_permille: u16, pub pearl_permille: u16, pub golden_permille: u16, pub pity_blocks: u32 }
// Rarity: bubble | pearl | goldenPearl
// StickerId (P1): firstStep | dailyGoal | planner | goodBreak | comeback | amazed | restDay | together | threeDays | goldenPearl | level5
pub struct RewardMoment { pub id: u32, pub kind: MomentKind, pub at_ms: i64, pub sparks: u32, pub session_total: u32, pub lines: Vec<RewardLine>,
    pub surprise: Option<Rarity>, pub daily_goal: bool, pub comeback: bool, pub streak: Option<u32>, pub new_streak: bool, pub milestone: Option<u32>,
    pub rest_days_used: u32, pub rest_day_earned: bool, pub level_up: Option<u32>, pub levels_gained: u32, pub pond_full: bool,
    pub stickers: Vec<StickerId>, pub first_step: bool, pub quiet: bool, pub preview: bool, #[serde(default)] pub sample: bool }
// MomentKind: start | step | allSteps | backFromBreak | breakStart | sessionEnd | goalDone | update;  RewardSound: pop | sparkle | fanfare
pub struct MicroStep { pub id: u32, pub text: String, pub done: bool }   // Debug drukuje tylko długość tekstu
pub struct StepDraft { pub id: Option<u32>, pub text: String }           // jw.
pub struct PopupRewards { pub sparks: u32, pub daily_goal: bool, pub streak: Option<u32>, pub level_up: Option<u32>, pub surprise: Option<Rarity> }
// Config: beta_features: bool, rewards: RewardsConfig { enabled, micro_steps, surprises, sounds, celebrations: CelebrationLevel (full | calm | numbersOnly),
//         focus_companion, show_streak, streak_paused, daily_goal: DailyGoal (light | regular | strong), rest_weekdays: Vec<Weekday> (mon..sun) }
```

`RewardsStatus` (seria 6 dni, dziś cel zrobiony, przerwa po pierwszym bloku sesji; jeden moment czeka na okno główne):

```json
{ "revision": 31, "active": true, "preview": false, "totalSparks": 1834,
  "level": { "level": 12, "into": 74, "span": 280 },
  "today": { "date": "2026-10-13", "sparks": 62, "goal": 30, "goalMet": true, "pondFull": false },
  "streak": { "state": "alive", "days": 6, "best": 14, "activeDays": 41, "todayCounted": true,
              "restBank": 1, "restBankMax": 3, "restReserved": 0, "nextRestIn": 2 },
  "week": [ { "date": "2026-10-12", "mark": "goal" }, { "date": "2026-10-13", "mark": "goal" }, { "date": "2026-10-14", "mark": "future" },
            { "date": "2026-10-15", "mark": "future" }, { "date": "2026-10-16", "mark": "future" }, { "date": "2026-10-17", "mark": "future" },
            { "date": "2026-10-18", "mark": "future" } ],
  "session": { "sessionId": "1760342400000-4242-3", "sparks": 72,
               "lines": [ { "kind": "start", "sparks": 5, "count": 1 }, { "kind": "firstOfDay", "sparks": 5, "count": 1 },
                          { "kind": "plan", "sparks": 2, "count": 1 }, { "kind": "focusTime", "sparks": 45, "count": 45 },
                          { "kind": "fullBlock", "sparks": 10, "count": 1 }, { "kind": "surprise", "sparks": 5, "count": 1 } ] },
  "collection": { "bubbles": 12, "pearls": 3, "golden": 0 },
  "odds": { "nonePermille": 500, "bubblePermille": 320, "pearlPermille": 150, "goldenPermille": 30, "pityBlocks": 4 },
  "stickers": [],
  "pending": [ { "id": 3, "kind": "breakStart", "atMs": 1760345100000, "sparks": 60, "sessionTotal": 72,
                 "lines": [ { "kind": "focusTime", "sparks": 45, "count": 45 }, { "kind": "fullBlock", "sparks": 10, "count": 1 },
                            { "kind": "surprise", "sparks": 5, "count": 1 } ],
                 "surprise": "bubble", "dailyGoal": true, "comeback": false, "streak": 6, "newStreak": false, "milestone": null,
                 "restDaysUsed": 0, "restDayEarned": false, "levelUp": null, "levelsGained": 0, "pondFull": false, "stickers": [],
                 "firstStep": false, "quiet": false, "preview": false, "sample": false } ] }
```

Kody błędów: `stepsOff`, `tooManySteps` (pole `steps`), `rewards.load` i `rewards.save` (oba jako `app://error`; `rewards.load` tylko przy włączonych
nagrodach, `rewards.save` najwyżej raz na uruchomienie).
Bez zmian w `capabilities/*.json`: okienko `popup` czyta nagrody tylko z `PopupInfo` i nigdy nie potwierdza momentów. W TS: `api.ts` ma `getRewards`,
`ackRewardMoments`, `resetRewards`, `playRewardSound`, `setSessionSteps`, `setStepDone`, `subscribeRewards` (ADR-012) i `onRewardsChanged`.

**Fakty sesji (`Effect::Progress(ProgressFact)`, `session.rs`).** Wstawione w wektory efektów ADR-013; pozostałe efekty zachowują kolejność
(test `old_effects_keep_their_order`, a każdy wiersz ma test dokładnej kolejności z `Progress`). Fakty niosą tylko liczby i id, nigdy tekstów;
`ranSecs` to czas biegu tylko tej fazy (bez pauz i luk snu, z przedłużeniem przez spotkanie).

| Przejście | Efekty |
|---|---|
| Start | PhaseChanged, Shields(true), Tracking(true), Chime(FocusStart), **Progress(Started { steps })** |
| Focus → Przerwa (termin / Pomiń) | PhaseChanged, Tracking(false), Shields(false), Chime(BreakStart), **Progress(FocusEnded { cycle, ranSecs, deadline \| skipped, held: false })**, Notify(BreakStarted) |
| Focus → wstrzymana przerwa | jw. z `held: true` i Notify(BreakHeld) |
| Przerwa → Focus (termin / Pomiń) | PhaseChanged, Chime(FocusStart), **Progress(BreakEnded { cycle, ranSecs })**, Notify(FocusStarted), Shields(true), Tracking(true) |
| Stop w Focus (także w pauzie) | Tracking(false), Shields(false), PhaseChanged(→Idle), **Progress(FocusEnded { stopped })**, SessionEnded |
| Stop w Przerwie | Tracking(false), Shields(false), PhaseChanged(→Idle), **Progress(BreakEnded { stopped })**, SessionEnded |
| Odhaczenie kroku (false → true) w trakcie sesji | **Progress(StepDone { stepId, firstTime, done, total, phase })** |

Pauza, Wznów, auto-pauza po luce, `SetGoal`, `SetMeetingHolds`, `DevSetRemaining` i `SetSteps` nie dają faktów; podwójny Stop nie daje nic.
Małe kroki (`Input::SetSteps` po `normalize_steps`: przycięte, znaki sterujące → spacje, puste odrzucone, najwyżej 80 znaków z `…`, najwyżej 5)
przeżywają Start; przy Stop wszystkie trafiają do `SessionTotals.steps` (`SessionSummary.steps`), a otwarte zostają jako następny plan (te same id).
Nowe id są zawsze wyższe od każdego wydanego. Wyłączenie małych kroków czyści plan (aktor).

**Reguły (`focus-core/src/rewards.rs`, czyste: bez zegara, plików, losowości i sieci).** Dzień nagród (`reward_day`) zaczyna się o 04:00 czasu
lokalnego; zegar cofnięty o najwyżej 2 dni (`CLOCK_BACK_TOLERANCE_DAYS`) jest przycinany do `lastDay`, a cofnięty dalej to poprawka zegara, który
się spieszył: dzień jest przyjmowany, a dzień serii policzony „w przyszłości” wraca na ostatni dzień z celem. Iskierki: start 5 (3 pierwsze starty dnia) + pierwszy start dnia 5 + plan 2 (start
z ≥ 1 krokiem, 3 dziennie); 1 za każdą `sparkUnitSecs` biegu focusu (limit 300 dziennie, „staw pełny”); pełny blok 10 (≥ 80 % długości i ≥
`rewardMinBlockSecs`); przerwa 5 (≥ 50 %); powrót 5 (po prawdziwym bloku); krok 3 (raz na id w sesji, limit 45 dziennie); wszystkie kroki 5 (≥ 2, raz; płaci odhaczenie, które domyka plan, także ponowne);
„Zrobione? → Tak” 10 (raz na id sesji; dla sesji demo do podglądu); cel dnia 30 / 60 / 120 z premią 10 (premia nie liczy się do celu); powrót po przerwie ≥ 2 dni roboczych 10.
Poziom z sumy (`min(60 + 20·(L−1), 500)`). Seria: dzień z celem +1; dni wolne, zakresy wstrzymania i dni oddechu (bank 2, max 3, +1 co 5 dni z celem)
chronią serię, za długa luka zaczyna nową (bank zostaje). Niespodzianka tylko za pełny blok: `splitmix64(seed ^ fnv1a64(sessionId) ^ cycle) % 1000`
(złota < 30, perła < 180, bąbelek < 500), perła najpóźniej po 4 blokach bez niej, pierwsza w ogóle co najmniej bąbelkiem. „Skupienie jest święte”:
moment zwrócony, gdy po fakcie trwa Focus, ma tylko własne linie (start, plan, powrót, krok, wszystkie kroki); osiągnięcia i linie, które by je zdradziły
(premia celu dnia, powrót po przerwie), czekają w `SessionTally` na najbliższy `BreakStart` albo `SessionEnd`, a `view` pokazuje do tego czasu
księgę sprzed nich plus linie pokazane w focusie (`SessionTally::shown`; poziom osiągnięty w focusie czeka z pełnym paskiem). Naklejki (P1)
odblokowują się deterministycznie i też czekają. Fakt sesji, której Startu księga nie widziała (nagrody włączone w trakcie, „Zacznij od nowa”,
usunięcie w panelu prywatności), zaczyna rachunek „od teraz” (`SessionTally::joined`): trwająca faza płaci tylko czas od dołączenia, bez pełnego
bloku, niespodzianki i premii za powrót. Wstrzymanie (seria wstrzymana albo nagrody wyłączone) zapisuje `pausedSince` tylko przy istniejącej serii; ponowne
włączenie zamienia wstrzymane dni w zakres neutralny. Widok w czasie wstrzymania pokazuje serię z chwili wstrzymania (seria, która już
czekała, czeka dalej).

**Usługa nagród (`src-tauri/src/rewards.rs`, `RewardsHandle`).** Fakt liczony synchronicznie pod mutexem (mikrosekundy), zapis pliku w kolejce wątku
`focus-rewards` (zapisuje tylko najnowszą księgę; plik tymczasowy + rename; poprzedni plik do `rewards.json.bak` najwyżej raz na minutę; przy wyjściu
`flush_blocking`). Dzień nagród z `focus_platform::clock::local_date_time` (`GetLocalTime`; poza Windows data UTC). Sesja demo trafia do podglądu
(kopia księgi z demo-Startu, `preview: true`, nigdy nie zapisywana; znika przy następnym prawdziwym Starcie, resecie i wyjściu). Momenty dostają id
z licznika usługi i czekają w `pending` (najwyżej 8). `Effects` kopiuje linię ostatniego faktu (`popup_rewards`) do okienka `BreakStarted` /
`BreakHeld` / `FocusStarted` tej samej zmiany fazy; okienko `FocusStarted` po przerwie dostaje też `nextStep` (pierwszy otwarty krok, od aktora).
Toasty nigdy nie mają linii nagród ani kroku. `SessionSummary.sparks` = iskierki sesji, gdy nagrody są włączone (nigdy dla sesji demo: jej
karta czyta podgląd ze stanu). `store_config` woła `config_changed` (wstrzymanie, niższy cel dnia, naprawa pliku odłożona przy wyłączonych
nagrodach), `mark_goal_done` woła `goal_done` tylko dla ostatniego podsumowania tego uruchomienia (sesja demo: do podglądu); zapłacone iskierki
dochodzą do `sparks` tego podsumowania w pamięci i w `history.jsonl`.
Aktor co 60 s (także w Idle) sprawdza zmianę dnia nagród. `play_reward_sound`: syntetyczne `Chime::Pop` / `Sparkle` / `Fanfare` w pakiecie dzwonków
(`custom` → Midnight) z głośnością `chimeVolume × 0,6`; czeka do końca dzwonka fazy + 150 ms (w komendzie, nigdy w aktorze), a przy czekaniu
dłuższym niż 3 s rezygnuje.

`rewards.json` (w folderze danych, obok `history.jsonl`; tylko liczby, numery dni i id sesji):

```json
{ "version": 1, "seed": 1311768467294899695, "totalSparks": 1834, "lastDay": 20739,
  "days": [ { "day": 20739, "sparks": 62, "goalBonus": 10, "minuteSparks": 45, "stepSparks": 9, "starts": 2, "plans": 1,
              "goalMet": true, "rest": false, "pondFull": false } ],
  "streak": { "current": 6, "best": 14, "lastCountedDay": 20739, "restBank": 1, "restProgress": 3, "activeDays": 41 },
  "neutral": [ { "from": 20700, "to": 20705 } ], "pausedSince": null,
  "collection": { "bubbles": 12, "pearls": 3, "golden": 0 }, "surprises": { "sincePearl": 2, "firstDone": true },
  "counters": { "fullBlocks": 40, "breaksTaken": 12, "stepsDone": 51, "backs": 30 },
  "stickers": [ "firstStep" ], "goalRewarded": [ "1760342400000-4242-3" ] }
```

Brak pliku → nowa księga (ziarno z `RandomState` + czas + pid); plik uszkodzony (zły JSON albo nie tekst) → kopia z `.bak` (zapisana jako
`rewards.json` przez wątek zapisu, przed każdym innym zapisem), a gdy i ona nie działa → plik odkładany jako `rewards.unreadable-<ms>.json`, nowa
księga i `rewards.load`. Przy wyłączonych nagrodach ta naprawa i komunikat czekają na ich włączenie (nic nie jest zapisywane ani przemianowywane).
Plik zablokowany albo bez dostępu: dwie ponowne próby (200 ms), potem widać `.bak` (albo nową księgę), `rewards.load` i w tym uruchomieniu nic
się nie zapisuje (plik może być dobry, więc nigdy nie jest nadpisany ani odłożony). Sesja działa dalej (zasada 8). `Ledger::normalize`
po wczytaniu (dni ≤ 42, zakresy ≤ 20, id celów ≤ 50, bank ≤ 3). Bez bety plik nie powstaje i nie zmienia się.

### 6.15 Wyjątki odcienia, inteligentne przerwy, statystyki tygodnia i kreator (ADR-023)

**Wdrożone wszystkie cztery części.** Najpierw kontrakt (`/sync-contract`): typy w `api.rs`, pola configu z wartościami domyślnymi i walidacją
(`config.rs`), stałe (`tint::DEFAULT_PAUSE_APPS`, `stats::RECENT_WEEKS`, `stats::TOP_APPS_PER_WEEK`, `config::MAX_PAUSE_APPS` / `MAX_PROCESS_NAME_CHARS` /
`MAX_CITY_ID_CHARS` / `MAX_APP_CATEGORIES` / `LATITUDE_RANGE` / `LONGITUDE_RANGE`), `Config::first_run()`, trzy komendy (wtedy zaślepki `notImplemented`)
i wrappery w `api.ts` (`listSunCities`, `setSmartBreakOptOut`, `getWeeklyStats`, `onHistoryChanged`, `watchWeeklyStats`); potem funkcje (niżej). Dziś
żadna komenda nie jest zaślepką: `list_sun_cities` zwraca wbudowaną listę, `TintStatus` ma prawdziwe `pausedBy`, `schedule` i dzisiejsze okno,
`CalendarStatus.smartBreaks` / `smartOptOuts` i `SessionSnapshot.smartBreak` mają prawdziwe wartości, `get_weekly_stats` czyta historię,
a `history://changed` jest wysyłany.

```rust
// Config (nowe pola; serde default, więc stary config.json się wczytuje)
pub smart_breaks: bool,                                // domyślnie false
pub app_categories: BTreeMap<String, AppCategory>,     // exe małymi literami → kategoria; domyślnie puste; najwyżej 500
pub onboarding_done: bool,                             // domyślnie true (brak pola = istniejący użytkownik); Config::first_run() → false
// TintConfig (nowe pola)
pub schedule: TintSchedule,                            // fixed (domyślnie) | sun
pub location: Option<TintLocation>,                    // domyślnie None
pub pause_for_apps: bool,                              // domyślnie true
pub pause_apps: Vec<String>,                           // domyślnie tint::DEFAULT_PAUSE_APPS (17 nazw exe)
pub pause_for_fullscreen: bool,                        // domyślnie false
pub struct TintLocation { pub latitude: f64 /* -90..=90 */, pub longitude: f64 /* -180..=180 */, pub city: Option<String> /* SunCity.id */ }
pub struct SunCity { pub id: String, pub name: String, pub country: String /* ISO 3166-1 alfa-2 */, pub latitude: f64, pub longitude: f64 }
// TintStatus (nowe pola, #[serde(default)])
pub paused_by: Option<TintPause>,                      // TintPause { reason: TintPauseReason (app | fullscreen), app: Option<String> /* nazwa exe */ }
pub schedule: TintScheduleState,                       // fixed | sun | polarDay | polarNight | noLocation
pub window_start_minutes: u32, pub window_end_minutes: u32, // dzisiejsze okno (zachód / wschód albo stałe godziny)
// Inteligentne przerwy
pub struct SmartBreak { pub meeting_id: String, pub meeting_start_ms: i64, pub meeting_end_ms: i64, pub break_number: u32,
    pub kind: SmartBreakKind /* endsBeforeMeeting | startsAtMeeting */, pub focus_delta_secs: i32, pub break_start_ms: i64, pub break_end_ms: i64 }
// CalendarStatus: #[serde(default)] smart_breaks: Vec<SmartBreak>, smart_opt_outs: Vec<String>
// SessionSnapshot: #[serde(default)] smart_break: Option<SmartBreak>
// Statystyki tygodnia
// SessionSummary: #[serde(default, skip_serializing_if = "Vec::is_empty")] focus_spans: Vec<TimeSpan>   (TS: focusSpans?)
pub struct TimeSpan { pub start_ms: i64, pub end_ms: i64 }
pub enum AppCategory { Productive, Neutral, Distracting }
pub struct HistoryChanged { pub revision: u32 }
pub struct WeeklyStats {
    pub revision: u32, pub week_start: String /* poniedziałek RRRR-MM-DD */, pub iso_year: u32, pub iso_week: u32,
    pub current_week: bool, pub first_week_start: Option<String>, pub history_off: bool,
    pub days: Vec<DayStats> /* 7, pon..niedz */, pub focus_secs: u32, pub break_secs: u32, pub breaks_taken: u32, pub sessions: u32,
    pub best_day: Option<Weekday>, pub heatmap: Vec<Vec<u32>> /* 7 × 24, sekundy focusu */, pub recent_heatmap: Vec<Vec<u32>>,
    pub recent_weeks: u32 /* 8 */, pub top_apps: Vec<WeekApp> /* najwyżej 10 */, pub categories: CategoryTotals, pub estimated: bool,
}
pub struct DayStats { pub date: String, pub weekday: Weekday, pub focus_secs: u32, pub break_secs: u32, pub sessions: u32 }
pub struct WeekApp { pub process: String, pub secs: u32, pub category: Option<AppCategory> }
pub struct CategoryTotals { pub productive_secs: u32, pub neutral_secs: u32, pub distracting_secs: u32, pub unlabelled_secs: u32 }
```

**Walidacja configu** (`Config::normalized`, błędy `config.invalid` z polem): `tint.pauseApps` przycięte, puste usunięte, duplikaty bez względu na wielkość
liter usunięte (zostaje pierwsza pisownia), najwyżej 64 nazwy po najwyżej 128 znaków, bez `\`, `/`, `:` i znaków sterujących; `tint.location.latitude` /
`longitude` skończone i w zakresie; `tint.location.city` przycięte (puste → `null`), najwyżej 64 znaki, nie jest sprawdzane z listą miast;
`appCategories`: klucze przycięte i małymi literami (puste usunięte), te same reguły nazwy co `pauseApps`, najwyżej 500. Harmonogram `sun` bez lokalizacji
jest poprawny (do czasu jej podania działają stałe godziny).

**Wyjątki odcienia** (wdrożone). Rdzeń, `focus-core/src/tint.rs` (czysty, testy jednostkowe):
- `is_pause_app` / `pause_reason(cfg, foreground, full_screen) -> Option<TintPause>`: dokładna nazwa exe z `pauseApps` bez rozróżniania wielkości liter
  (bez ścieżek, prefiksów i tytułów okien); aplikacja ma pierwszeństwo przed pełnym ekranem; każdy przełącznik działa osobno.
- `TintExceptions` (histereza i płynne przejście na milisekundach monotonicznych z powłoki): wstrzymanie zaczyna się od razu, kończy się
  `PAUSE_RELEASE_MS` = 2000 ms po pierwszej próbce bez wyjątku (powrót w tym czasie liczy od nowa, więc Alt+Tab nie miga), a ciepło schodzi do 0
  i wraca liniowo przez `PAUSE_FADE_MS` = 1200 ms, od miejsca, w którym jest (odwrócenie w połowie nie skacze). Pierwsza próbka po `reset` (poza oknem
  ciepła, podgląd) nie ma przejścia: aplikacja do pracy z kolorem na pierwszym planie o zachodzie słońca w ogóle nie widzi ciepła.
- `tint_window(cfg, data lokalna, przesunięcie UTC) -> TintWindow { schedule, start_minutes, end_minutes }` i `window_level` (dawne `tint_level` dla
  dowolnego okna): `fixed` = stałe godziny; `sun` = zachód → wschód danej daty z `focus-core/src/sun.rs`, zaokrąglone do minuty; brak lokalizacji
  (albo nieprawidłowa data) → `noLocation`, dzień / noc polarna (też zachód i wschód w tej samej minucie) → `polarDay` / `polarNight`, wszystkie ze stałymi godzinami.
- `sun.rs`: równania NOAA Solar Calculator (Meeus): deklinacja i równanie czasu, kąt godzinny dla zenitu 90,833°, trzy przybliżenia w chwili zdarzenia,
  zdarzenia przypisane do daty lokalnej także daleko od UTC (UTC+14), zachód po północy raportowany jako > 1440 min (Reykjavík w czerwcu);
  `utc_offset_minutes` (zegar lokalny vs UTC, zaokrąglone do 15 min), `utc_date`, wbudowana lista `CITIES` (43: wszystkie 18 siedzib władz
  województw i 25 dużych miast świata; id `kk-nazwa`). Testy: Warszawa w czterech porach roku, Londyn, Nowy Jork, Los Angeles, Tokio, Auckland,
  Sydney, Singapur (±2 min od tablic USNO), Tromsø i bieguny, Kiribati, nieprawidłowe dane.

Powłoka, `src-tauri/src/tint.rs`: wątek odcienia liczy datę lokalną i przesunięcie (`clock::local_date_time` + `sun::utc_offset_minutes`, bez Windows:
data UTC) i dzisiejsze okno przy każdej ocenie. Tracker próbkuje tylko w czasie focusu, a odcień działa cały wieczór, więc ten sam wątek odcienia
(bez nowego wątku) próbkuje `foreground::probe()` najwyżej raz na `WATCH_EVERY` = 1 s i `notification_state::user_is_presenting()` (pełny ekran,
gra D3D, tryb prezentacji) — tylko gdy okno jest ciepłe i któryś wyjątek jest włączony; z próbki zostaje sama nazwa exe (tytuł od razu odrzucony).
W czasie przejścia ocena co `FADE_TICK` = 100 ms. Ten sam poziom trafia do mechanizmu ponownie najwyżej co `SCHEDULE_TICK` (gamma odświeżana jak
dotąd), zmieniony od razu. Poziom 0 przywraca mechanizm jak koniec okna (plik `tint-restore.json` i wpis RunOnce usuwane, przed powrotem ciepła zapisywane
na nowo), więc gwarancje zasady 6 się nie zmieniają. Podgląd w Ustawieniach i uruchomienie `--restore-shields` ignorują wyjątki (`pausedBy = null`).
Log mówi tylko „suspended by an exception” / „exception over”, nigdy nazwy aplikacji. Testy `Watch` na fałszywej sondzie (próbkowanie ≤ 1 Hz, przejście,
histereza, pierwsza próbka w oknie, przełączniki, wyjście z okna).

UI, `TintSettings.svelte`: „Kiedy ocieplać” (stałe godziny / od zachodu do wschodu słońca), miejsce z listy (grupa kraju z co najmniej 3 miastami,
reszta „Inne miasta”, nazwy krajów z `Intl.DisplayNames`) albo własne współrzędne (pole tekstowe, przecinek i typograficzny minus akceptowane, walidacja
lokalna jak w `config.rs`), „Dziś: zachód słońca 17:46, wschód 06:57” albo powód stałych godzin; stałe godziny opisane jako zapasowe. „Wyjątki”:
przełącznik programów do pracy z kolorem z listą chipów (usuń, dodaj z Enter, `.exe` dopisywane, „Przywróć domyślną listę” = kopia
`tint::DEFAULT_PAUSE_APPS` w `config.svelte.ts`), przełącznik pełnego ekranu; status „Wstrzymane: Photoshop na pierwszym planie” / „… na pełnym ekranie”
(pusta kropka). Strona Deweloper: „Warm colours paused · Photoshop in front” (próbka `tint.sample` tylko w pamięci, czyszczona z innymi próbkami).

**Inteligentne przerwy** (wdrożone). Przełącznik `smartBreaks` na stronie Skupienie przy wyborze czasów, aktywny tylko z połączonym kalendarzem (inaczej wyłączony,
z podpowiedzią i linkiem do Ustawień kalendarza). Czysta funkcja planowania w `focus-core` (`calendar.rs` / `session.rs`): dla spotkania kolidującego
z przerwą zmienia długość poprzedzającego bloku focusu tak, żeby przerwa skończyła się tuż przed spotkaniem (`endsBeforeMeeting`) albo zaczęła razem
z nim (`startsAtMeeting`), w granicach: skrócenie do ≥ 60 % długości focusu i nigdy poniżej 10 min (profil demo przeskalowany), wydłużenie o ≤ 25 %
(nazwane stałe w `timing.rs`, zasada 2). Wybiera wariant z mniejszą zmianą; bez dopasowania spotkanie dostaje zwykłe pytanie z ADR-016. Spotkania
z wyborem użytkownika (`MeetingChoice`) albo w `smartOptOuts` nie są dopasowywane; całodniowe, wolne i wstępne są pomijane jak dziś. Sesja dostaje
nowy koniec fazy istniejącą ścieżką wejść aktora (nowe `Input`, jak `SetMeetingHolds`), bez sztuczek z zegarem monotonicznym; kolejność efektów
ADR-013 się nie zmienia, a Stop, Pomiń fazę i Pauza działają jak zawsze. Testy: bez spotkania, spotkanie w środku focusu, w środku przerwy, spotkania
jedno po drugim, całodniowe, wolne i wstępne pominięte, dzień zmiany czasu, profil demo.

Wdrożenie inteligentnych przerw:
- **Rdzeń** (`focus-core/src/calendar.rs`): `SmartMeeting { id, startMs, endMs, eligible }` (bez tematu), `fit_break` dla jednej przerwy i
  `plan_smart_breaks` dla całego planu (blok po bloku; przesunięta przerwa przesuwa kolejne). Wariant jest możliwy, gdy blok mieści się w
  `TimingProfile::smart_min_focus_ms` … `smart_max_focus_ms` (`SMART_BREAK_MIN_FOCUS_PERCENT` = 60, `SMART_BREAK_MAX_EXTEND_PERCENT` = 25,
  `SMART_BREAK_MIN_FOCUS_SECS` = 600, w demo `DEMO_SMART_BREAK_MIN_FOCUS_SECS` = 10; focus krótszy niż ten próg tylko się wydłuża), przerwa nie
  zaczyna się w przeszłości, nie trafia na inne spotkanie (także z wyborem użytkownika), a spotkanie kończy się przed następną przerwą (dłuższe
  dostaje pytanie, gdzie najlepiej pasuje „Przedłuż skupienie”). Kolidujące spotkanie z wyborem, z „Zapytaj mnie” albo dłuższe niż 4 h → bez
  przesunięcia (pytanie). Wybór: mniejsza zmiana, przy remisie przerwa przed spotkaniem. Czas kalendarzowy w ms, więc zmiana czasu nic nie psuje.
- **Sesja** (`session.rs`): `Input::SetSmartMeetings(Vec<SmartMeeting>)` (bez efektów). Sesja dopasowuje bieżący blok sama, tą samą funkcją, przy
  Start, początku nowego bloku, Wznów i zmianie listy (pauza: jakby wznowić teraz): przesuwa się tylko jego termin (o różnicę dopasowań), więc
  koniec fazy przychodzi zwykłym Tickiem z efektami w kolejności ADR-013. Blok przedłużony przez `Extend` albo skrócony na stronie Deweloper jest
  stały. `SessionSnapshot.smartBreak` pokazuje przesuniętą przerwę bieżącego bloku i trwa przez nią (po Pomiń fazę znika); `phaseTotalSecs` ma już zmianę.
- **Powłoka** (`src-tauri/src/calendar.rs`, `calendar/model.rs`): `model::smart_meetings` (wszystkie spotkania; `eligible` = brak wyboru i brak
  „Zapytaj mnie”; pusta lista przy wyłączonych `smartBreaks`) idzie do aktora (`ActorCmd::SetSmartMeetings`, tylko po zmianie). Plan
  (`SessionPlan::smart_plan`) nie dopasowuje bieżącego bloku focusu (ma już dopasowanie sesji, `SessionPlan.smart` z migawki), a kolejne tak;
  `CalendarStatus.smartBreaks` = przerwa sesji + przesunięcia planu, a ich spotkania znikają z `conflicts`. Przypomnienia sprawdzają kolizję
  z tym samym planem. Całodniowe i wolne wydarzenia odrzuca już platforma (`meetings_only`); wstępne (tentative) liczą się jak zajęte, tak jak przy pytaniu.
- **UI**: przełącznik w karcie „Czas” (`TimingEditor.svelte`, wyłączony bez kalendarza, z linkiem do Ustawień kalendarzy), linia pod zegarem
  (`SmartBreakNote.svelte`: same godziny, „Zapytaj mnie”; w Idle pierwsza przerwa planu; w przesuniętej przerwie bez cofania), lista przesuniętych
  przerw w karcie spotkań (`MeetingConflicts.svelte`: temat jak przy pytaniu, zmiana bloku, „Zapytaj mnie”) i „Dopasuj automatycznie” przy
  spotkaniu z „Zapytaj mnie”. Teksty `calendar.smart` w 10 językach. Strona Deweloper: „Smart break · moved break + note” (próbka w pamięci).

**Statystyki tygodnia** (wdrożone). Czysta agregacja w `focus-core/src/stats.rs`: `WeekAggregator` dostaje kolejne sesje (strumieniowo,
w dowolnej kolejności), zapytanie `WeekQuery { day, now_ms, revision, history_off }` i funkcję „przesunięcie strefy w minutach w chwili `utc_ms`”
(pytana raz na kwadrans UTC okna, bo każda strefa i każda zmiana czasu wypada na pełny kwadrans, np. Nowa Fundlandia o 05:30 UTC; przycinana do ±18 h),
więc godziny są dobre także w dni zmiany czasu i w tygodniach sprzed niej. Sesje demo są pomijane.
Sesja liczy się do dnia, w którym się zaczęła (`sessions`, `breakSecs`, `breaksTaken` = suma `cyclesCompleted`, aplikacje), a sekundy focusu do dnia
i godziny, w której płynęły: z `focusSpans` (od ADR-023 `Session` zapisuje przedziały biegnącego focusu na zegarze kalendarzowym: bez pauz, przerw
i luk po uśpieniu; najwyżej `stats::MAX_FOCUS_SPANS` = 256, potem rozciąga ostatni), a dla starszych linii `focusedSecs` rozłożone równo od startu do
końca (`estimated: true`, gdy taka sesja wpada w okno 8 tygodni). Najlepszy dzień = pierwszy z największym focusem; top 10 aplikacji, ale kategorie
liczą wszystkie aplikacje tygodnia (`Config.appCategories`, klucz = nazwa exe małymi literami). Powłoka (`src-tauri/src/weekly.rs`) parsuje
`weekStart`, czyta `history.jsonl` linia po linii poza wątkami async (`Persistence::scan_history`: najwyżej ostatnie 64 MB pliku, linia > 1 MB
i linie nieczytelne pominięte, w logu tylko liczby), strefę bierze z `focus_platform::clock::utc_offset_minutes_at` (Windows
`SystemTimeToTzSpecificLocalTime`; poza Windows przesunięcie 0, ADR-008). `history://changed` (globalna rewizja `AtomicU32`) idzie po zapisie sesji,
„Zrobione?” zapisanym w pliku, „Usuń całą historię”, usunięciu historii w panelu prywatności i po zapisie configu ze zmianą `appCategories` albo
`keepHistory`. `keepHistory` wyłączone → `historyOff: true`: wcześniej zapisane sesje dalej się liczą, a UI dopisuje, że nowe nie trafią
do statystyk (bez historii: wyjaśnienie zamiast liczb). Tytuły okien nie są potrzebne.
UI: `src/lib/components/week/` (`WeekSection`, `WeekHeatmap`, `WeekApps`) na Podsumowaniu, stan w `src/lib/state/weekly.svelte.ts`
(`api.watchWeeklyStats` tylko wtedy, gdy sekcja jest na ekranie; poprzedni tydzień zostaje przyciemniony, dopóki nie przyjdzie wybrany),
daty `RRRR-MM-DD` w `src/lib/week.ts`. Heatmapa jest prawdziwą tabelą (wiersz = dzień, kolumna = godzina, w komórce czas dla czytników ekranu
i `title` po najechaniu), 5 stopni koloru z `--accent` (każdy motyw, także 8-bit), legenda „mniej → więcej”; słupki dni z tekstem dla czytników.
Etykiety: chipy przy aplikacji (`aria-pressed`; ponowne kliknięcie usuwa etykietę), zapis od razu do configu, kolor zawsze z podpisem. Pusty
tydzień = spokojny tekst bez oceniania („jeszcze nic tu nie ma”).
„Sample week” na stronie Deweloper to próbka w pamięci UI, jak inne próbki (bez komendy).

**Kreator pierwszego uruchomienia** (wdrożony). Bez nowych komend: tylko `Config.onboardingDone`. `Persistence::load_config` daje `Config::first_run()` wyłącznie wtedy,
gdy `config.json` nie istnieje (świeża instalacja, także po resecie fabrycznym); uszkodzony plik i „Usuń wszystko” dają `Config::default()`, więc kreator się
nie pokazuje. Flaga trafia na dysk z pierwszym zapisem configu, więc zamknięcie aplikacji w trakcie kreatora pokaże go przy następnym starcie, a zmiany
z kroków zostają. Po „Zacznij pierwsze skupienie”, „Pomiń” (z potwierdzeniem) albo linku „później w Ustawieniach” UI zapisuje `onboardingDone: true`
przez `update_config`. Kroki używają istniejących komend, sklepów i komponentów Ustawień (`get_system_language`, `open_extension_dir`, `update_config`;
`MusicSourcePicker`, `ExtensionGuide`, `LauncherSettings`, `CloseButtonSetting`, `Switch`), bez własnej logiki ustawień.

UI: `src/lib/state/onboarding.svelte.ts` (sklep: otwarcie raz na okno, gdy pierwszy wczytany config ma `onboardingDone: false`; kroki; zakończenie, pominięcie,
podgląd) i `src/lib/components/onboarding/` (`FirstRunWizard` + komponent na krok + `WizardMascot`). Kreator zakrywa całe okno główne (warstwa nad
nagłówkiem i paskiem muzyki, pod toastami; okna dialogowe są w top layer): strona pod nim nie jest renderowana (jedna kopia każdego `id`), nagłówek i pasek
muzyki są `inert`, a odtwarzacz gra dalej. Powłoka nie czeka na kreatora: zasobnik, okienka i sesja uruchomiona z zasobnika działają, a trwająca sesja
pokazuje się w kreatorze z przyciskiem Stop (zasada 7). Klawiatura: Enter = dalej (poza przyciskami, linkami, polami wielowierszowymi i listami), Esc = pomiń
z potwierdzeniem, Tab zostaje w kreatorze; nagłówek nowego kroku dostaje fokus. Aksolotl pokazuje pozę kroku (`poses:<nazwa>`, atlas póz także bez bety)
z „boing” z łaskotania (`mascot-boing` w `app.css`) i zdanie kroku w dymku (`useSpeech`, litera po literze; przy ograniczonym ruchu od razu i bez ruchu).
„Show first-run wizard” na stronie Deweloper pokazuje go od pierwszego kroku bez zmiany `onboardingDone`; zapisuje ją tylko ukończenie.

`TintStatus` (harmonogram słońca w Warszawie, ok. 18:30; na pierwszym planie Photoshop, więc ciepło zeszło do 0):

```json
{ "revision": 40, "enabled": true, "level": 0, "preview": false, "active": false, "mode": "nightLight", "capped": false, "error": null,
  "pausedBy": { "reason": "app", "app": "Photoshop.exe" }, "schedule": "sun", "windowStartMinutes": 1067, "windowEndMinutes": 417 }
```

`SmartBreak` (w `CalendarStatus.smartBreaks` i, w trakcie 2. bloku focusu, w `SessionSnapshot.smartBreak`; spotkanie z przykładu w §6.4: profil 45/15,
2. przerwa przesunięta o 400 s wcześniej, żeby skończyła się, gdy zaczyna się spotkanie; linia pod zegarem: „Przerwa przesunięta na 13:45 · spotkanie o 14:00”):

```json
{ "meetingId": "…", "meetingStartMs": 1760356800000, "meetingEndMs": 1760360400000, "breakNumber": 2, "kind": "endsBeforeMeeting",
  "focusDeltaSecs": -400, "breakStartMs": 1760355900000, "breakEndMs": 1760356800000 }
```

`WeeklyStats` (tydzień 42 roku 2026, we wtorek; `days` i heatmapy skrócone):

```json
{ "revision": 12, "weekStart": "2026-10-12", "isoYear": 2026, "isoWeek": 42, "currentWeek": true, "firstWeekStart": "2026-09-07", "historyOff": false,
  "days": [ { "date": "2026-10-12", "weekday": "mon", "focusSecs": 8100, "breakSecs": 1800, "sessions": 2 },
            { "date": "2026-10-13", "weekday": "tue", "focusSecs": 2700, "breakSecs": 900, "sessions": 1 },
            { "date": "2026-10-14", "weekday": "wed", "focusSecs": 0, "breakSecs": 0, "sessions": 0 } ],
  "focusSecs": 10800, "breakSecs": 2700, "breaksTaken": 3, "sessions": 3, "bestDay": "mon",
  "heatmap": [ [0, 0, 0, 0, 0, 0, 0, 0, 0, 2700, 3600, 1800, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] ],
  "recentHeatmap": [ [0, 0, 0, 0, 0, 0, 0, 0, 900, 9900, 12600, 7200, 0, 0, 3600, 5400, 1800, 0, 0, 0, 0, 0, 0, 0] ], "recentWeeks": 8,
  "topApps": [ { "process": "Code.exe", "secs": 6900, "category": "productive" }, { "process": "chrome.exe", "secs": 2400, "category": null },
               { "process": "Discord.exe", "secs": 600, "category": "distracting" } ],
  "categories": { "productiveSecs": 6900, "neutralSecs": 0, "distractingSecs": 600, "unlabelledSecs": 2400 },
  "estimated": true }
```

`HistoryChanged`: `{ "revision": 13 }`. `SunCity`: `{ "id": "pl-warszawa", "name": "Warszawa", "country": "PL", "latitude": 52.2297, "longitude": 21.0122 }`.
`SessionSummary`, tylko pole dodane przez ADR-023 (znika z JSON-a, gdy puste): `{ "focusSpans": [ { "startMs": 1760342400000, "endMs": 1760345100000 } ] }`.

## 7. Model wątków i współbieżności

```
Wątek główny (event loop Tauri)  ── okna, tray, WebView
Runtime tokio (tauri::async_runtime)
 ├── SessionActor: select! { cmd = rx.recv(), _ = interval(1 s).tick(), res = effect_results.recv() }
 │     → Session::handle → dispatch efektów (bez await na wolnych operacjach)
 ├── TintService: interval(60 s) → core::tint → okna tint
 ├── BreakWatchdog: heartbeaty okien przerwy (§6.3)
 ├── PersistenceWorker: jedna kolejka, zapisy po kolei (config, sessions.jsonl)
 └── DndWorker (Could): uzgadnia docelowy stan DND z Graph, po kolei
Wątek audio (std::thread)
 └── MixerDeviceSink + Player (rodio 0.22); AudioCmd przez kanał; AudioEvent z powrotem;
     błędy urządzenia z wątku cpal → kanał → ponowne otwarcie urządzenia domyślnego
```

- **Aktor nigdy nie czeka na wolny efekt.** Efekty trafiają do kolejek wykonawców (kanał, `send` bez czekania), więc Stop jest obsługiwany natychmiast.
- **Ignorowanie spóźnionego wyniku nie anuluje efektu** (R02). Rozpoczętego `spawn_blocking` nie da się przerwać `abort`em,
  a wysłanego żądania HTTP nie da się cofnąć. Dlatego kolejność i poprawność efektów zapewniają **wykonawcy**, a nie filtr w aktorze:
  - **Jeden wykonawca na zasób, operacje seryjnie** w kolejności zlecenia. Dwa zapisy configu nie mogą skończyć się w odwrotnej kolejności.
  - **Docelowy stan zamiast poleceń** tam, gdzie efekt jest zewnętrzny (DND): `SetDnd{on}` ustawia `desired` + numer generacji.
    DndWorker po zakończeniu każdego wywołania sprawdza, czy `desired` się nie zmieniło. Jeśli tak, wykonuje kolejne wywołanie dla aktualnego stanu.
    Timeout oznacza „stan nieznany” i wymusza ponowne uzgodnienie, a nie „anulowane”.
  - **`RecordSession` to trwałe zlecenie:** nie jest odrzucane dlatego, że sesja się skończyła. Klucz `(session_id, cycle)` jest unikalny,
    a PersistenceWorker pomija duplikat przy ponowieniu (zbiór kluczy wczytany przy starcie z `sessions.jsonl`).
  - Okna (`ShowBreak`/`HideBreak`, ostrzeżenie) wykonuje się w kolejności na wątku głównym (`run_on_main_thread`), a operacje niosą `break_id`.
- Wyniki od wykonawców wracają do aktora z `op_id`/generacją i służą **tylko** do aktualizacji stanu w UI (np. „Teams: DND aktywne”).
  Spóźniony wynik nie zmienia stanu aktora.
- Stan dla UI: jeden właściciel (aktor), snapshot przez event i `get_snapshot`. Nie ma `Arc<Mutex<Session>>` wołanego z wielu miejsc.

## 8. Audio (`focus-audio`) (ADR-003)

Ustalenia ze spike'a T0-10 (kod referencyjny: [spikes/audio-spike](../spikes/audio-spike/src/main.rs)):

- **Wersja: `rodio = { version = "=0.22.2", features = ["noise"] }`** (cpal 0.17, backend WASAPI na Windows).
  W spike'u dodatkowo `wav_output`, tylko do wygenerowania pliku testowego.
- **API rodio 0.22** (inne niż w starszych tutorialach, gdzie było `OutputStream` / `Sink`):
  - urządzenie: `DeviceSinkBuilder::from_default_device()?.with_error_callback(..).open_sink_or_fallback()?` → `MixerDeviceSink`
    (odtwarzanie trwa, dopóki ten obiekt żyje; `log_on_drop(false)` wycisza komunikat przy drop),
  - odtwarzacz: `Player::connect_new(sink.mixer())`, a potem `append`, `set_volume`, `pause`/`play`, `stop`,
  - `Source` to `Iterator<Item = Sample>` (`f32`), a `channels()` / `sample_rate()` zwracają `NonZero` (`ChannelCount`, `SampleRate`),
  - dekodowanie: `Decoder::try_from(File)` (symphonia: mp3/ogg/flac/wav/mp4 w domyślnych feature'ach).
- `MixerDeviceSink` i `Player` są **`Send + Sync`** na Windows (sprawdzone kompilacją). Dedykowany wątek zostaje jako
  **decyzja organizacyjna**: jeden właściciel urządzenia, prosty cykl życia i ponowne otwieranie urządzenia po błędzie.
- API: `AudioHandle::spawn(assets_dir) -> (AudioHandle, Receiver<AudioEvent>)`; `AudioCmd`: `Play{genre}`, `Pause`, `Resume`,
  `Stop`, `SetVolume(f32)`, `FadeTo{factor, ms}`, `Gong`, `Next`.
- **Docelowy stan odtwarzania (R04)**: wątek audio trzyma stan niezależny od tego, czy urządzenie istnieje:
  `desired: Stopped | Playing{genre} | Paused{genre}`, głośność użytkownika, bieżący i docelowy współczynnik fade oraz numer generacji.
  Komendy (`Play`/`Pause`/`Resume`/`Stop`/`SetVolume`/`FadeTo`) **zawsze** aktualizują ten stan, także gdy urządzenia nie ma.
  Reconnect **odtwarza** ten stan i nie jest nowym `Play`: przy `Stopped` tylko otwiera urządzenie, przy `Paused` tworzy odtwarzacz w pauzie,
  przy `Playing` gra z bieżącą głośnością × fade, a fade z przerwy nie jest resetowany. Ponawianie (co 2 s) nie blokuje odbierania komend.
- **Model głośności**: `efektywna = głośność_użytkownika × współczynnik_fade`, ustawiane przez `Player::set_volume`.
  `Resume` i `Play` przywracają fade do 1.0, więc po Postpone muzyka nie zostaje wyciszona. Pętla wątku robi `recv_timeout(20 ms)` i w każdym obiegu
  przesuwa fade. **Stop w trakcie 10-sekundowego fade'u jest obsłużony od razu** (zmierzone: < 20 ms opóźnienia pętli + ~2 µs sam Stop).
- Gatunki w `assets/music/genres.toml`: `kind = "noise"` (`brown | pink | white`) albo `"files"` (katalog; shuffle + pętla). M1: brown noise + 1 utwór CC0.
- **Szum**: rodio ma wbudowane `source::noise::{white, pink}` (feature `noise`) i ich używamy. **Brown noise rodio nie ma**,
  więc mamy własny `Source`: biały szum z rodio, całkowany z wyciekiem (`last = (last + 0.02·w) / 1.02`, wyjście ×3.5, clamp do [-1, 1]).
- **Błędy urządzenia**: callback z `with_error_callback` działa na wątku cpal, więc tylko przekazuje błąd kanałem do wątku audio.
  Wątek audio porzuca `Player` i `MixerDeviceSink`, co 2 s próbuje otworzyć **aktualne** urządzenie domyślne, a po sukcesie odtwarza
  docelowy stan (wyżej) i wysyła `AudioEvent::Reconnected`. Spike grał wtedy od nowa; w `focus-audio` obowiązuje reguła docelowego stanu. Sesja trwa dalej, a UI pokazuje „brak dźwięku” do czasu ponownego połączenia. rodio samo się nie łączy ponownie.
  **Sprawdzone (Windows 10, WASAPI):** wyłączenie urządzenia w `mmsys.cpl` → błąd „device is no longer available” w callbacku; potem „Could not find any output device” przy każdej próbie (brak urządzenia nie crashuje); po ponownym włączeniu szum wraca w ≤ 2 s.
- **Stabilność czasowa (40 min): niepotwierdzona na prawdziwym sprzęcie.** Spike działał w VirtualBoxie z emulowaną kartą HDA.
  40 min bez crasha i bez wycieku, ale emulowany zegar audio pływa: urządzenie pobierało od 0,75× do 1,3× nominalnego tempa,
  także w surowym cpal bez rodio, co wskazuje na środowisko (emulowana karta), a nie na rodio. Rozstrzygnie to dopiero pomiar na sprzęcie. Wątek callbacku działał regularnie (~20 ms), z 2 pauzami ~180 ms na 5 min.
  **Na VM nie mierzymy czasu audio ani okien.** Powtórka na fizycznym Windows: `audio-spike rawcpal 60` (oczekiwane ratio ≈ 1,000)
  i `audio-spike diag 2400 60` (metronom + pauzy callbacku); przy pauzach > 100 ms porównać z wariantem `--features boost`
  (MMCSS przez feature cpal `audio_thread_priority` + wyłączenie power throttlingu procesu) — task T0-11.
- Brak pliku / błąd dekodowania → `AudioEvent::Error`, bez crasha (sprawdzone).
- Zasoby dołączane przez `bundle.resources`; ścieżkę rozwiązuje powłoka (`BaseDirectory::Resource`).

## 9. Tint i integracje

### 9.1 Tint: overlay jako główny mechanizm (ADR-009)
> **Aktualnie (ADR-015):** overlay jest **usunięty** (właściciel produktu go odrzucił). Ciepło daje Oświetlenie nocne Windows (domyślnie,
> `focus-platform/src/night_light.rs`) albo rampa gamma (`gamma.rs`, kopia bazowej rampy każdego monitora, patrz §9.2), według harmonogramu z ADR-014
> (minuta doby z `clock.rs`, poziom 0..100 → `kelvin_for_level` / `rgb_gain_for_kelvin` z `focus_core::tint`). Stan sprzed zmiany wraca przy wyjściu,
> przy następnym starcie albo przy logowaniu (RunOnce), jak przy osłonach. Kontrakt w §6.6 i §6.7. Poniżej plan sprzed pivotu (nieaktualny).

- Okna `tint-{n}` na każdym monitorze: przezroczyste, bez dekoracji, `always_on_top`, `skip_taskbar`, nieaktywujące się,
  `set_ignore_cursor_events(true)`. Kolor i alfa z `core::tint` (kelvin → RGBA).
- To **efekt wizualny**, nie kalibracja barw: obniża kontrast i tak go opisujemy.
- Znika razem z procesem, więc kolory wracają same po każdym crashu i kill.
- Harmonogram: godziny od–do + przejście 30 min + siła; podgląd w ustawieniach (`preview_tint`).
- macOS wymaga `macos-private-api` do przezroczystości; Wayland nie ma always-on-top, więc tam tint jest niedostępny i UI to komunikuje.

### 9.2 Eksperymentalna gamma ramp (Could, S-02)
> **Aktualnie (ADR-015):** gamma wchodzi jako tryb do wyboru (`tint.mode: gamma`), w poprawnej wersji opisanej niżej: bazowa rampa każdego monitora
> zapisana przed pierwszą zmianą, a ustawiana rampa to baza × mnożniki kanałów. Gdy Windows odrzuci zbyt ciepłą rampę, działa słabsza (`TintStatus.capped`).
> Rekomendacja „nie wdrażać” poniżej przestaje obowiązywać, bo overlay został odrzucony.

- Tylko Windows, SDR, domyślnie wyłączona, poza ścieżką demo. Microsoft odradza `SetDeviceGammaRamp`: może zwrócić sukces
  bez efektu, zachowanie przy HDR jest niezdefiniowane, a system lub inne aplikacje mogą ją nadpisać.
- Sprzątanie opisujemy osobno dla każdego przypadku: **zwykłe wyjście**, **panika**, **zabicie procesu**.
- **Liniowa rampa to nie przywrócenie stanu** (R08): jeśli monitor miał własną kalibrację, reset do liniowej jej nie odtworzy,
  a po zmianie układu monitorów można zmienić niewłaściwy ekran. Poprawna wersja wymaga kopii bazowej rampy per monitor
  (`GetDeviceGammaRamp`, klucz = identyfikator urządzenia) zapisanej przed pierwszą zmianą oraz polityki konfliktu ze zmianami systemowymi po crashu.
- **Rekomendacja: nie wdrażać S-02 na hackathonie.** Overlay (§9.1) w pełni pokrywa funkcję.
  Weryfikacja ewentualnego eksperymentu: start z nieliniową rampą, zwykłe wyjście, kill + restart, zmiana monitora; patrzymy na wierność kalibracji, a nie na to, że „nie jest już pomarańczowo”.

### 9.3 Teams DND (Could, S-01, ADR-010)
> **Aktualnie (ADR-013 + ADR-014):** DND z wygaśnięciem focus + 5 min, automatyczne czyszczenie na przerwę i po Stop, ale tylko gdy FocusMode
> sam je ustawił; przy statusie `Offline` albo `DoNotDisturb` (odczyt `GET /me/presence`) statusu nie ruszamy. Poniżej plan sprzed pivotu.

- `Presence.ReadWrite` (delegowane), device code flow (public client), token w keychainie (`keyring`).
- Ustawienie: `POST /users/{id}/presence/setUserPreferredPresence` (`/me/...` sprawdzić w próbie),
  `{"availability":"DoNotDisturb","activity":"DoNotDisturb","expirationDuration":"PT10M"}`.
- **Bez automatycznego czyszczenia (R07).** `clearUserPreferredPresence` kasuje *każdy* preferowany status, a flaga „to my ustawiliśmy DND”
  nie dowodzi, że aktualny status nadal jest nasz (użytkownik mógł w międzyczasie ustawić np. „Zaraz wracam”). Odczyt przed czyszczeniem nie jest atomowy.
  Dlatego:
  - DND ustawiamy z **krótkim wygaśnięciem (10 min)** i odnawiamy co 5 min, tylko dopóki trwa Focus z włączoną integracją,
  - Stop, Pause i przerwa **przestają odnawiać** status, więc DND wygasa samo najpóźniej po 10 min,
  - UI ma przycisk „Wyczyść status Teams teraz” jako świadomą decyzję użytkownika, nigdy automatycznie.
- Preferred presence wymaga aktywnej sesji presence (zalogowany klient Teams). Zmiana ikony może trwać kilka minut, więc **w demo nie pokazujemy jej na żywo**.
- **Nie wykrywamy rozmów przez Graph** — od tego jest ręczny tryb spotkania.

## 10. Okna

> **Aktualnie:** okna `main`, `media`, `popup` i overlay odcienia (tabela w §6.3). Okien `warning` i `break-{n}` nie ma (ADR-013). Poniżej plan sprzed pivotu.

| Etykieta | URL | Cechy |
|---|---|---|
| `main` | `index.html#/` | zwykłe okno; zamknięcie = hide (tray) |
| `warning` | `index.html#/warning` | małe, w rogu głównego monitora, always-on-top, nieaktywujące się, bez paska zadań |
| `break-{n}` | `index.html#/break?primary=1` | per monitor: tworzone ukryte → `window_ready` → pokazane; fullscreen, always-on-top, bez dekoracji; heartbeat co 1 s |
| `tint-{n}` | `index.html#/tint` | per monitor: przezroczyste, klikalne na wylot, bez fokusu; **tylko odczyt** (`get_tint_state`, `window_ready`) |

- Monitory: `app.available_monitors()`. Testujemy 2 monitory, różne DPI, monitor po lewej (ujemne współrzędne),
  odłączenie monitora w trakcie i aplikację fullscreen w tle. **Nie obiecujemy przykrycia elementów systemu** (np. paska zadań).
- Klawiatura: przytrzymanie Esc przez 3 s w oknie przerwy = `skip_break`. W trakcie przerwy tray ma zawsze aktywne „Pomiń przerwę” i „Stop”,
  niezależnie od stanu WebView (§6.3).
- **Uprawnienia**: własne komendy z `invoke_handler` są domyślnie dostępne **każdemu** oknu. Samo capabilities dla pluginów ich nie ogranicza.
  Izolację (Should, B-12) włączamy przez manifest komend aplikacji w `build.rs` oraz capabilities per etykieta okna,
  według tabeli w §6.3. Weryfikacja: `stop` wywołane z `tint-*` jest odrzucane przez backend, a `get_tint_state` z `tint-*` działa.
- Pluginy: `notification` (tylko tekst; przyciski akcji w tym pluginie są tylko na mobile), `single-instance`, `log`;
  (Could) `global-shortcut`, `autostart`.

## 11. Dane i konfiguracja

| Co | Gdzie | Kto czyta/pisze |
|---|---|---|
| Ustawienia | `app_config_dir()/config.toml` | `persistence.rs` (zapis atomowy: tmp + rename); core parsuje i waliduje string |
| Historia sesji | `app_data_dir()/sessions.jsonl` | `persistence.rs`, append-only, jeden `SessionRecord` na cykl |
| Katalog ćwiczeń | zasób `assets/exercises.json` | `persistence.rs` → `ExerciseCatalog` → core |
| Znacznik gammy (Could) | `app_data_dir()/gamma.active` | tylko dla S-02 |
| Token Teams (Could) | systemowy keychain | `keyring` |
| Logi | `app_log_dir()` | `tauri-plugin-log` / `tracing` |

Brak pliku oznacza ustawienia domyślne. Uszkodzony config: log + domyślne, a stary plik zostaje przemianowany na `.bak`. Błąd zapisu kończy się `app://error`, a sesja trwa dalej.
Demo nie zapisuje się do prawdziwej historii (rekordy z `demo: true` są pomijane w statystykach).

## 12. Obsługa błędów i logowanie

- Biblioteki: `thiserror`. Powłoka: `anyhow` wewnętrznie, `AppError { code, message }` na granicy IPC.
- Brak `unwrap()/expect()` poza testami i setupem.
- Błąd integracji (audio, tint, Teams, zapis) **nie przerywa sesji**: log + `app://error` + degradacja.
- `tracing`, poziom przez `RUST_LOG`.

## 13. Testy

| Poziom | Co | Jak |
|---|---|---|
| Unit core | tabela §5.4, granice czasu, planer, config, agregaty | `cargo test -p focus-core`, jawne `Now` |
| Unit audio | szum w [-1, 1] bez NaN, parsowanie `genres.toml`, model fade | `cargo test -p focus-audio` bez urządzenia |
| Kontrakt | brak dryfu bindings, stałe nazw komend/eventów | CI: regeneracja + `git diff --exit-code` |
| Frontend | typy, składnia | `npm run check` |
| Instalator | pełny cykl z zainstalowanej paczki, poza repo, offline, odrzucone powiadomienia | ręcznie od M1 |
| Ręczne | checklista awarii (TASKS P-4) | przed każdą bramką |

## 14. Narzędzia i środowisko

Wymagane (wersje przypięte na czas hackathonu):
- **Rust** przez `rustup`, wersja w `rust-toolchain.toml` (start: 1.98.1) + `clippy`, `rustfmt`.
- **Node.js LTS**, wersja w `.nvmrc`. Tauri CLI jako devDependency (`@tauri-apps/cli`) → `npm run tauri dev|build`.
- Windows: MSVC (VS 2022 / Build Tools, komponent „C++ x64/x86 build tools”) + Windows SDK; WebView2 (jest w Win10/11).
- macOS: Xcode CLT. Linux: `libwebkit2gtk-4.1-dev build-essential libssl-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev libxdo-dev`.

Zalecane: VS Code (*rust-analyzer*, *Tauri*, *Svelte for VS Code*, *Even Better TOML*, *CodeLLDB*), `just`, `cargo-nextest`.
AI: Claude Code (CLAUDE.md + skille w `.claude/`) i drugi model do review ([CROSS_AI_REVIEW.md](CROSS_AI_REVIEW.md)).

### Zależności (dodawane przez `cargo add` / `npm install` na starcie, potem lockfile)

| Crate / pakiet | Gdzie | Po co |
|---|---|---|
| `tauri` 2 (`tray-icon`), `tauri-build` | src-tauri | powłoka |
| `tauri-plugin-notification`, `-single-instance`, `-log` | src-tauri | powiadomienia, 1 instancja, logi |
| `serde`, `serde_json`, `toml`, `ts-rs`, `thiserror`, `time` | core | typy, kontrakt |
| `rand` (z seedem, np. `rand_chacha`) | core | deterministyczne losowanie planu |
| `tokio` (przez `tauri::async_runtime`), `anyhow`, `tracing` | src-tauri | aktor, błędy, logi |
| `rodio` `=0.22.2` (feature `noise`) | audio | odtwarzanie, dekodowanie, szum white/pink |
| `reqwest` (rustls), `oauth2`, `keyring` | platform (Could) | Teams |
| `windows` | platform (Could) | gamma |
| `svelte` 5, `vite`, `typescript`, `@tauri-apps/api` | frontend | UI |

### Lockfile i bindings po merge'u
1. Najpierw scal manifesty (`Cargo.toml`, `package.json`).
2. Zregeneruj lockfile właściwym narzędziem (`cargo update -w` / `npm install`) i przejrzyj diff.
3. `cargo test -p focus-core` regeneruje bindings; nie rozwiązuj ich konfliktów ręcznie.
4. CI na czystym checkout: `npm ci`, `cargo … --locked`, brak dryfu bindings, check/test/build.

## 15. Build i dystrybucja

- Dev: `npm run tauri dev` (demo: `FOCUSMODE_DEMO=1`, tylko w dev).
- Zainstalowana aplikacja: tryb demo przez argument `--demo` lub przełącznik w ustawieniach. Aplikacja uruchomiona ze skrótu nie odziedziczy zmiennej środowiskowej z terminala.
- Release: `npm run tauri build` → Windows NSIS/MSI (**pierwszy w M1**). macOS/Linux budowane w CI (matrix, `tauri-action`), bez gwarancji UX.
- Buildy niepodpisane → ostrzeżenie SmartScreen/Gatekeeper; opisać w README.
