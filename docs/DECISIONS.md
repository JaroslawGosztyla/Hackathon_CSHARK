# Decyzje architektoniczne (ADR)

Format: krótko. Status: `Proponowana` → `Przyjęta` → ewentualnie `Zastąpiona przez ADR-xxx`.
Nowa decyzja = nowy wpis na końcu; starych nie przepisujemy, tylko zmieniamy status i dopisujemy „Poprawka”.

**Stan na 2026-09-27:** ADR-008…010 przyjął zespół (decyzje po review). Pozostałe są proponowane i
**zatwierdzamy je razem w M0 (T0-4)**. Druga osoba też musi je zobaczyć.

Źródło poprawek z review: [reviews/codex-crosscheck-2026-09-27.md](reviews/codex-crosscheck-2026-09-27.md) (F01–F16).

---

## ADR-001: Tauri 2 jako framework aplikacji
**Status:** Proponowana (review: zachować)

**Kontekst:** aplikacja multiplatformowa w Ruście, bez UI na surowym Win32/HWND; tray, powiadomienia,
wiele okien (overlay per monitor), okna przezroczyste i klikalne na wylot, instalatory.

| Opcja | Plusy | Minusy |
|---|---|---|
| **Tauri 2** (Rust + WebView) | tray, powiadomienia, multi-window, transparent/click-through, instalatory, pluginy | frontend w TS; zachowanie okien różni się między OS (np. Wayland) |
| egui / eframe | 100% Rust | tray i multi-window słabsze; overlaye per monitor ręcznie |
| Iced | 100% Rust, architektura Elm | brak gotowego tray; mniej przykładów overlay |
| Slint | deklaratywne, natywne | tray/overlay poza frameworkiem; nowy DSL |
| Dioxus desktop | Rust + webview | młodszy ekosystem pluginów systemowych |
| Win32 (HWND) | pełna kontrola | tylko Windows; odrzucone |

**Decyzja:** Tauri 2, logika w crate'ach `focus-*`, UI w Svelte.

**Poprawka (F03, review):** Tauri nie zapewnia identycznego UX na każdym OS: always-on-top na Wayland nie działa,
tooltip w tray nie działa na Linuksie. Gwarancje dotyczą platformy odbioru (ADR-008).
Oceny „ładniej/szybciej” to opinie zależne od kompetencji zespołu. egui to **nie** jest tani plan awaryjny na koniec projektu:
core zostaje, ale widoki i okna trzeba by napisać od nowa.

---

## ADR-002: Functional core, imperative shell
**Status:** Proponowana

**Decyzja:** `focus-core` to czysta logika: `Session::handle(now, Input) -> Vec<Effect>`, bez zegara, plików, sieci, Tauri i OS.
Efekty wykonuje powłoka.

**Poprawka (F10):** „bez I/O” oznacza, że **odczyt i zapis plików (config, historia, katalog ćwiczeń) jest w powłoce**
(`persistence.rs`). Core daje typy, walidację i (de)serializację stringów. Losowanie planu przerwy przyjmuje seed z zewnątrz.

---

## ADR-003: Audio w Ruście (rodio) na dedykowanym wątku
**Status:** Proponowana

**Decyzja:** rodio, jeden dedykowany wątek jako właściciel urządzenia wyjściowego, komunikacja kanałami.

**Poprawka (F08):** usunięte uzasadnienie „OutputStream nie jest Send”, bo zależało od wersji, a aktualne rodio ma inne API.
Wątek to decyzja organizacyjna (jeden właściciel urządzenia, prosty cykl życia). Wersję przypinamy po spike'u T0-10.
Model głośności: głośność użytkownika × współczynnik fade; Stop przerywa fade natychmiast.

**Poprawka (T0-10, 2026-10-05):** spike potwierdził rodio `=0.22.2` (API `DeviceSinkBuilder` / `MixerDeviceSink` / `Player`).
Typy są `Send + Sync` na Windows, więc wątek zostaje z powodów organizacyjnych. Brown noise piszemy sami, white/pink są w rodio.
Po błędzie urządzenia rodio samo się nie łączy ponownie; robi to nasz wątek audio (szczegóły w ARCHITECTURE §8).

**Plan B:** `<audio>` w trwałym, ukrytym WebView. Wymaga osobnego sprawdzenia po ukryciu okna, uśpieniu i odłączeniu urządzenia.

---

## ADR-004: Tint — gamma ramp per OS + overlay jako fallback
**Status:** Zastąpiona przez ADR-009

---

## ADR-005: Teams DND przez Graph z wykrywaniem rozmów
**Status:** Zastąpiona przez ADR-010

---

## ADR-006: Kontrakt IPC jako DTO w Ruście + ts-rs
**Status:** Proponowana

**Decyzja:** typy przekraczające granicę UI↔Rust są w `focus-core/src/api.rs` (`serde` camelCase + `ts-rs`).
Wywołania tylko przez `src/lib/api.ts`.

**Poprawka (F09):** eksport wymaga `#[ts(export)]`; katalog `TS_RS_EXPORT_DIR` względem workspace. CI sprawdza brak dryfu bindings.
Nazwy komend i eventów są stałymi w Ruście eksportowanymi do TS, bo same DTO nie sprawdzają nazw `invoke` ani eventów.
Kontrakt M0 obejmuje tylko MVP (bez przyszłych komend Teams).

---

## ADR-007: Svelte 5 + TypeScript na frontend
**Status:** Proponowana

Mało boilerplate'u, reaktywne stores pod eventy z Rusta, animacje CSS. Bez ciężkiej biblioteki komponentów.
Alternatywy: vanilla TS, React, Leptos/Yew (Rust w WASM: spójność języka kosztem czasu buildów i startu).

---

## ADR-008: Platforma odbioru — Windows; macOS i Linux best-effort
**Status:** Przyjęta (2026-09-27, decyzja zespołu po review F03)

**Decyzja:** UX gwarantujemy i testujemy na sprzęcie tylko na Windows 10/11. Architektura pozostaje multiplatformowa;
CI buduje na 3 OS. Ograniczenia pozostałych platform są jawnie opisane (DESIGN §4). Na Wayland przerwa degraduje się do zwykłego okna + powiadomienia.

**Konsekwencje:** testy okien (2 monitory, DPI, monitor po lewej, odłączenie) tylko na Windows; nie obiecujemy przykrycia elementów systemu.

---

## ADR-009: Tint przez overlay; gamma ramp tylko jako eksperyment
**Status:** Przyjęta (2026-09-27, decyzja zespołu po review F01/F02)

**Kontekst:** Microsoft odradza `SetDeviceGammaRamp` (może zwrócić sukces bez efektu, HDR jest niezdefiniowany, system może nadpisać).
Drop, panic hook ani handler wyjścia nie wykonają się po zabiciu procesu, więc gamma nie daje gwarancji powrotu kolorów.

**Decyzja:** głównym mechanizmem jest przezroczyste, klikalne na wylot okno overlay na każdym monitorze (Should, M2).
Znika razem z procesem, więc kolory zawsze wracają. Opisujemy je jako efekt wizualny, nie kalibrację barw.
Gamma ramp: Could (S-02), tylko Windows SDR, domyślnie wyłączona, poza demo. Sprzątanie opisujemy osobno dla wyjścia, paniki i kill,
a naprawa po kill następuje przy następnym starcie.

---

## ADR-010: Ręczny tryb spotkania w MVP; Teams DND jako Could, bez wykrywania rozmów
**Status:** Przyjęta (2026-09-27, decyzja zespołu po review F06)

**Kontekst:** Graph daje pierwszeństwo preferowanemu statusowi użytkownika, więc po ustawieniu własnego DND zagregowany
`/me/presence` nie wykrywa wiarygodnie rozmowy. Zmiana ikony w Teams może trwać minuty. `clearUserPreferredPresence`
kasuje także ręczny status użytkownika. Firmowy tenant może blokować zgodę.

**Decyzja:**
- MVP: przycisk „Jestem na spotkaniu” (15/30/60 min) przesuwa przerwę, a UI pokazuje czas następnej próby.
- Could (S-01): tylko ustawianie DND z `expirationDuration`. Czyścimy wyłącznie DND ustawione przez nas w tej sesji, z ostrzeżeniem w ustawieniach.
  Warunek: pozytywny test tenantu (T0-8). Demo nie pokazuje zmiany ikony na żywo.
- Won't: wykrywanie rozmów przez Graph.

---

## ADR-011: Model czasu — profil czasów, ostrzeżenie w focusie, dwa zegary
**Status:** Proponowana (z review F04/F07)

**Decyzja:**
- Jeden `TimingProfile` (`normal()`/`demo()`) zasila core, UI i testy. Ostrzeżenie zawiera się w czasie focusa.
  Demo ma własne wartości (nie jest mnożnikiem): 40/5/20/10 s i stały plan przerwy.
- Fazy liczy zegar monotoniczny, statystyki i harmonogram zegar kalendarzowy.
- Luka > 60 s (albo rozjazd zegarów) to **heurystyka przerwania**: Focus → Paused(Interrupted), Break → Idle(`interrupted`).
  Luki nie liczymy jako focusa i nie zaliczamy ćwiczeń.
- Brak przycisku „Zrobione”. Przerwa kończy się z odliczaniem, a wynik przerwy to `completed | skipped | stopped | interrupted`.

---

## ADR-012: Okna odtwarzają stan ze snapshotu; protokół gotowości
**Status:** Proponowana (z review F05/F11)

**Decyzja:** `SessionSnapshot` zawiera wszystko, czego potrzebuje dowolne okno (z aktywnym planem przerwy i `break_id`) oraz `revision`.
Okno: listen → `get_snapshot` → stosuj tylko nowszą rewizję → `window_ready`. Okna przerwy tworzone są ukryte i pokazywane po gotowości,
a po 5 s bez gotowości powłoka je zamyka i degraduje przerwę do powiadomienia. Komendy z okien przerwy niosą `break_id` (idempotencja).
Aktor nie czeka na wolne efekty; ich wyniki są oznaczone `session_id`, a spóźnione są ignorowane.
