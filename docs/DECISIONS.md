# Decyzje architektoniczne (ADR)

Format: krótko. Status: `Proponowana` → `Przyjęta` → ewentualnie `Zastąpiona przez ADR-xxx`.
Nowa decyzja = nowy wpis na końcu; starych nie przepisujemy, tylko zmieniamy status i dopisujemy „Poprawka”.

**Stan na 2026-09-27:** ADR-008…010 przyjął zespół (decyzje po review). Pozostałe są proponowane i
**zatwierdzamy je razem w M0 (T0-4)**. Druga osoba też musi je zobaczyć.

**Stan na 2026-10-12 (speedrun):** [ADR-013](#adr-013-speedrun-pivot-produktu) zmienia zakres produktu i częściowo zastępuje
ADR-003, 009, 010, 011 i 012. Tam, gdzie DESIGN.md lub ARCHITECTURE.md mówią co innego, wygrywa ADR-013 i `api.rs`.

**Stan na 2026-10-13:** [ADR-014](#adr-014-zasobnik-i-okienka-faz-odcień-ekranu-overlay-zamykanie-launcherów-gier-języki) dokłada do ADR-013
zasobnik z okienkami faz, odcień ekranu (tylko overlay, domyślnie wyłączony), zamykanie launcherów gier, Teams z poszanowaniem statusu użytkownika
i wybór języka. Zmienia przy tym ADR-009, 010, 012 i 013 (szczegóły w „Co zmienia”).

**Stan na 2026-10-13 (runda 2a):** [ADR-015](#adr-015-runda-2a-ciepło-ekranu-bez-overlayu-motywy-dźwięki-muzyka-przerwy-i-historia) usuwa overlay odcienia
(ciepło ekranu daje teraz Oświetlenie nocne Windows albo gamma ramp) i dokłada motywy, animowaną nawigację, pakiety dzwonków z własnym plikiem,
muzykę w zestawie (Open Lo-Fi, CC0) jako domyślne źródło, przypomnienia na kolejne przerwy, porady na przerwę ze źródłami
oraz opcje historii (pomijanie powierzchni powłoki Windows, tytuły okien w podpowiedzi wykresu, „nie zapisuj historii”, „usuń historię”).
Zmienia ADR-009, 012, 013 i 014.

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
**Status:** Częściowo zastąpiona przez ADR-013 (2026-10-12): rodio tylko do dzwonków, muzyka w WebView

**Decyzja:** rodio, jeden dedykowany wątek jako właściciel urządzenia wyjściowego, komunikacja kanałami.

**Poprawka (F08):** usunięte uzasadnienie „OutputStream nie jest Send”, bo zależało od wersji, a aktualne rodio ma inne API.
Wątek to decyzja organizacyjna (jeden właściciel urządzenia, prosty cykl życia). Wersję przypinamy po spike'u T0-10.
Model głośności: głośność użytkownika × współczynnik fade; Stop przerywa fade natychmiast.

**Poprawka (T0-10, 2026-10-05):** spike potwierdził rodio `=0.22.2` (API `DeviceSinkBuilder` / `MixerDeviceSink` / `Player`).
Typy są `Send + Sync` na Windows, więc wątek zostaje z powodów organizacyjnych. Brown noise piszemy sami, white/pink są w rodio.
Po błędzie urządzenia rodio samo się nie łączy ponownie; robi to nasz wątek audio (szczegóły w ARCHITECTURE §8).

**Poprawka (review R04, 2026-10-06):** wątek audio trzyma **docelowy stan** odtwarzania (Stopped/Playing/Paused, gatunek, głośność, fade, generacja),
niezależny od istnienia urządzenia. Reconnect odtwarza ten stan i nie wysyła nowego `Play`, więc po Pause, Stop albo w przerwie muzyka nie wraca.

**Plan B:** `<audio>` w trwałym, ukrytym WebView. Wymaga osobnego sprawdzenia po ukryciu okna, uśpieniu i odłączeniu urządzenia.

**Poprawka (ADR-013, 2026-10-12):** muzyka (YouTube, stream, linki, folder, szum) gra w WebView okna głównego, więc plan B staje się planem A.
rodio i dedykowany wątek zostają tylko dla syntetycznych dzwonków (`focus-audio::ChimePlayer`). Gatunki, fade i docelowy stan odtwarzania (R04) odpadają.

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
**Status:** Przyjęta (2026-09-27, decyzja zespołu po review F01/F02); zakres odłożony przez ADR-013 (2026-10-12); overlay przywrócony przez ADR-014 (2026-10-13); **zastąpiona przez ADR-015** (2026-10-13: overlay odrzucony, ciepło przez Oświetlenie nocne albo gamma)

**Kontekst:** Microsoft odradza `SetDeviceGammaRamp` (może zwrócić sukces bez efektu, HDR jest niezdefiniowany, system może nadpisać).
Drop, panic hook ani handler wyjścia nie wykonają się po zabiciu procesu, więc gamma nie daje gwarancji powrotu kolorów.

**Decyzja:** głównym mechanizmem jest przezroczyste, klikalne na wylot okno overlay na każdym monitorze (Should, M2).
Znika razem z procesem, więc kolory zawsze wracają. Opisujemy je jako efekt wizualny, nie kalibrację barw.
Gamma ramp: Could (S-02), tylko Windows SDR, domyślnie wyłączona, poza demo. Sprzątanie opisujemy osobno dla wyjścia, paniki i kill,
a naprawa po kill następuje przy następnym starcie.

**Poprawka (review R08, 2026-10-06):** reset do liniowej rampy **nie** przywraca wcześniejszej kalibracji monitora.
Poprawny S-02 wymagałby kopii bazowej rampy per monitor i polityki konfliktu. **Rekomendacja: nie wdrażać S-02 na hackathonie**,
bo overlay pokrywa funkcję.

**Poprawka (ADR-013, 2026-10-12):** tint (overlay i gamma) wypada z zakresu speedrunu. Jeśli wróci, obowiązuje ta decyzja: overlay, nie gamma.

**Poprawka (ADR-014, 2026-10-13):** tint wraca na prośbę właściciela produktu, **tylko jako overlay**, domyślnie wyłączony,
z harmonogramem według pory dnia (od–do, siła, przejście) i podglądem w Ustawieniach. Gamma ramp (S-02) nadal nie wchodzi.

**Poprawka (ADR-015, 2026-10-13):** właściciel produktu odrzucił overlay („półprzezroczysta warstwa koloru to nie jest odcień”), więc ta decyzja
przestaje obowiązywać. Ciepło ekranu daje teraz **Oświetlenie nocne Windows** (domyślnie) albo **gamma ramp** (do wyboru). Obawy z kontekstu
i z R08 rozwiązuje ADR-015: stan sprzed zmiany (ustawienia Oświetlenia nocnego albo bazowa rampa każdego monitora, a nie liniowa) jest zapisywany
zaraz po zmianie i przywracany przy wyjściu, przy następnym starcie albo najpóźniej przy następnym logowaniu (jak osłony, zasada 7).

---

## ADR-010: Ręczny tryb spotkania w MVP; Teams DND jako Could, bez wykrywania rozmów
**Status:** Przyjęta (2026-09-27, decyzja zespołu po review F06); w części zastąpiona przez ADR-013 (2026-10-12)

**Kontekst:** Graph daje pierwszeństwo preferowanemu statusowi użytkownika, więc po ustawieniu własnego DND zagregowany
`/me/presence` nie wykrywa wiarygodnie rozmowy. Zmiana ikony w Teams może trwać minuty. `clearUserPreferredPresence`
kasuje także ręczny status użytkownika. Firmowy tenant może blokować zgodę.

**Decyzja:**
- MVP: przycisk „Jestem na spotkaniu” (15/30/60 min) przesuwa przerwę, a UI pokazuje czas następnej próby.
- Could (S-01): tylko ustawianie DND z `expirationDuration`. Czyścimy wyłącznie DND ustawione przez nas w tej sesji, z ostrzeżeniem w ustawieniach.
  Warunek: pozytywny test tenantu (T0-8). Demo nie pokazuje zmiany ikony na żywo.
- Won't: wykrywanie rozmów przez Graph.

**Poprawka (review R07, 2026-10-06):** rezygnujemy z automatycznego `clearUserPreferredPresence`. Flaga „to my ustawiliśmy DND”
nie dowodzi, że aktualny status jest nasz, a odczyt przed czyszczeniem nie jest atomowy. DND ma krótkie wygaśnięcie (10 min) odnawiane
w trakcie Focus. Po Stop, Pause i w przerwie odnawianie się kończy i status wygasa sam. Czyszczenie statusu jest wyłącznie ręczne (przycisk w UI).

**Poprawka (ADR-013, 2026-10-12):** ręczny tryb spotkania odpada (nie ma wymuszonych okien przerwy, więc nie ma czego przesuwać; jest Pauza i „Pomiń fazę”).
Teams DND staje się opcjonalną osłoną skupienia: `setUserPreferredPresence` z `expirationDuration` = długość focusa + 5 min,
a na przerwę i po Stop **automatyczne** `clearUserPreferredPresence`. To uchyla R07 na wyraźne życzenie właściciela produktu; ryzyko opisuje ADR-013.
Wykrywanie rozmów przez Graph nadal Won't.

**Poprawka (ADR-014, 2026-10-13):** przed ustawieniem DND powłoka czyta bieżący status (`GET /me/presence`). Gdy użytkownik jest „offline”
(m.in. „Wyświetlaj jako offline”) albo ma już „Nie przeszkadzać”, FocusMode statusu nie zmienia i w przerwie ani po Stop go nie czyści.
Część ryzyka R07 znika: nie kasujemy już DND ani niewidoczności ustawionych przez użytkownika przed focusem.

---

## ADR-011: Model czasu — profil czasów, ostrzeżenie w focusie, dwa zegary
**Status:** Proponowana (z review F04/F07); w części zastąpiona przez ADR-013 (2026-10-12)

**Decyzja:**
- Jeden `TimingProfile` (`normal()`/`demo()`) zasila core, UI i testy. Ostrzeżenie zawiera się w czasie focusa.
  Demo ma własne wartości (nie jest mnożnikiem): 40/5/20/10 s i stały plan przerwy.
- Fazy liczy zegar monotoniczny, statystyki i harmonogram zegar kalendarzowy.
- Luka > 60 s (albo rozjazd zegarów) to **heurystyka przerwania**: Focus → Paused(Interrupted), Break → Idle(`interrupted`).
  Luki nie liczymy jako focusa i nie zaliczamy ćwiczeń.
- Brak przycisku „Zrobione”. Przerwa kończy się z odliczaniem, a wynik przerwy to `completed | skipped | stopped | interrupted`.

**Poprawka (review R01, R05, R09, 2026-10-06):**
- Spotkanie blokuje **i** ostrzeżenie, **i** przerwę. Po jego końcu, jeśli zostało mniej niż `warning_secs`, startuje pełne ostrzeżenie (`deadline = now + warning_secs`).
  „Idę” nie wymusza przerwy w trakcie spotkania.
- Pauza zamraża `remaining`. Przy luce `remaining` liczymy od ostatniego zaufanego ticka, a Resume ustawia `deadline = now + remaining`.
  Pause/Resume mają efekty audio i ostrzeżenia (tabela w ARCHITECTURE §5.4).
- Walidacja configu (zakresy) i planer z `min_walk_secs`: plan zawsze mieści się w `break_secs`, a przy krótkiej przerwie ma mniej ćwiczeń, nie krótsze.

**Poprawka (ADR-013, 2026-10-12):** zostają `TimingProfile` (`normal()` = 45/15 min, `demo()` = 40/20 s, `from_config`), dwa zegary i heurystyka luki.
Próg luki to 90 s, a luka w Focus **i** w Break kończy się automatyczną pauzą (nie Idle). Odpadają ostrzeżenie w focusie, Postpone, reguły spotkania,
plan przerwy i wyniki przerwy. Sesja jest pętlą Focus ⇄ Break aż do Stop; tabela faz jest w ADR-013.

---

## ADR-012: Okna odtwarzają stan ze snapshotu; protokół gotowości
**Status:** Proponowana (z review F05/F11); w części zastąpiona przez ADR-013 (2026-10-12); okna zmienione przez ADR-014 i ADR-015 (2026-10-13)

**Decyzja:** `SessionSnapshot` zawiera wszystko, czego potrzebuje dowolne okno (z aktywnym planem przerwy i `break_id`) oraz `revision`.
Okno: listen → `get_snapshot` → stosuj tylko nowszą rewizję → `window_ready`. Okna przerwy tworzone są ukryte i pokazywane po gotowości,
a po 5 s bez gotowości powłoka je zamyka i degraduje przerwę do powiadomienia. Komendy z okien przerwy niosą `break_id` (idempotencja).
Aktor nie czeka na wolne efekty; ich wyniki są oznaczone `session_id`, a spóźnione są ignorowane.

**Poprawka (review R02, R03, R06, 2026-10-06):**
- Ignorowanie spóźnionego wyniku chroni tylko stan aktora i **nie anuluje** efektu (`spawn_blocking` nie da się przerwać, HTTP nie da się cofnąć).
  Poprawność efektów zapewniają wykonawcy: jeden na zasób, operacje seryjnie, docelowy stan z generacją dla efektów zewnętrznych (DND),
  a `RecordSession` jako trwałe zlecenie z unikalnym kluczem `(session_id, cycle)`.
- Okna przerwy po `window_ready` wysyłają heartbeat co 1 s. Watchdog w powłoce zamyka okno po 5 s bez sygnału. Tray („Pomiń przerwę”, „Stop”)
  jest ścieżką wyjścia niezależną od WebView. Zawieszenie całego procesu nie jest objęte gwarancją.
- Każdy stan okna ma getter + event z rewizją (`get_snapshot`, `get_now_playing`, `get_tint_state`). Okna `tint-*` mają tylko odczyt
  (`get_tint_state`, `window_ready`), żadnych komend zmieniających stan.

**Poprawka (ADR-013, 2026-10-12):** zostaje jedno okno główne. Obowiązują dalej: snapshot z `revision` (także `ShieldStatus` i `TeamsStatus`),
wzorzec listen → getter → stosuj tylko nowszą rewizję oraz wykonawcy efektów z R02 (aktor nie czeka, jeden wykonawca na zasób, docelowy stan z generacją).
Odpadają okna przerwy, ostrzeżenia i `tint-*`, `window_ready`, heartbeat, watchdog i `break_id`.

**Poprawka (ADR-014, 2026-10-13):** wracają okna overlay odcienia (jedno na monitor) i dochodzi małe okno `popup` (okienko faz).
Oba stany mają parę getter + event z rewizją: `get_tint_status` + `tint://changed` (`TintStatus`) i `get_popup` + `popup://show` (`PopupInfo`).
Okna overlay tylko czytają stan, a okno `popup` dodatkowo wysyła `popup_action`. Protokół gotowości, heartbeat, watchdog i `break_id` nie wracają.

**Poprawka (ADR-015, 2026-10-13):** okna overlay odcienia (`tint-*`) i trasa `/tint` znikają. Para `get_tint_status` + `tint://changed` zostaje,
ale `TintStatus` opisuje teraz Oświetlenie nocne albo gamma ramp (nowe pola `mode` i `capped`), a czyta go tylko okno główne.
Poza oknem głównym zostają okno `popup` i okno YouTube `media`.

---

## ADR-013: Speedrun: pivot produktu
**Status:** Przyjęta (2026-10-12, B na prośbę właściciela produktu; A potwierdza przy review PR-a z gałęzi `b/speedrun-mvp`); uzupełniona 2026-10-13 o okno YouTube (pkt 2) i listę stron w pliku `websiteblacklist.txt` (pkt 4); w części zmieniona przez ADR-014 (2026-10-13: pkt 4, 6 i 7) i ADR-015 (2026-10-13: pkt 2, 3 i 5)

**Kontekst:** termin oddania to **jutro** (2026-10-13). Po T0-2/T0-3 nie było jeszcze kontraktu (T0-4), maszyny stanów ani okien przerwy,
a plan M1–M4 zakładał jeszcze ok. 10 dni pracy. Równocześnie właściciel produktu zmienił wymagania:
- czasy ustawia użytkownik (domyślnie **45 min skupienia / 15 min przerwy**), a sesja kręci się w pętli aż do Stop;
- muzyka z **folderu**, z **listy linków** (YouTube, YouTube Music, playlisty, bezpośrednie URL-e audio) albo ze **streamu** (np. Lofi Girl na YouTube);
- na czas skupienia aplikacja „wycisza świat”: powiadomienia Windows, status Teams, rozpraszające strony w przeglądarce.
  W przerwie wszystko wraca, a **muzyka gra dalej**;
- dzwonek i powiadomienie przy każdej zmianie fazy; Pauza i Stop w dowolnej chwili;
- na koniec sesji wykres produktywności według aktywnego procesu.

Plan z ADR-009…012 (pełnoekranowe okna przerwy na każdym monitorze, protokół gotowości okien, tint, tryb spotkania) jest za drogi na jeden dzień
i nie odpowiada na nowe wymagania. rodio nie odtworzy YouTube ani streamu bez pobierania strumienia zewnętrznymi narzędziami.

**Opcje — muzyka:**

| Opcja | Plusy | Minusy |
|---|---|---|
| **Odtwarzacz w WebView** (osadzony player YouTube, `<audio>`, WebAudio) | YouTube / YouTube Music / live przez oficjalny player; folder przez asset protocol; szum offline; bez nowych zależności | gra tylko, dopóki żyje WebView okna głównego (zamknięcie okna = ukrycie); część filmów blokuje osadzanie (błędy 101/150); polityka autoplay |
| rodio + `yt-dlp` / `ffmpeg` | jeden silnik audio w Ruście | pobieranie strumienia łamie regulamin YouTube; duże binarki; psuje się przy każdej zmianie YouTube |
| rodio dla folderu, YouTube w przeglądarce użytkownika | prosto | brak kontroli nad pauzą i przerwą; dwa osobne światy |

**Opcje — blokada stron („wstrzyknięcie się” w przeglądarkę):**

| Opcja | Plusy | Minusy |
|---|---|---|
| **Rozszerzenie MV3 + lokalny most HTTP** | oficjalne API (`declarativeNetRequest`, `tabs`); Chrome, Edge, Brave, Opera, Vivaldi; adres strony nie opuszcza przeglądarki | użytkownik raz ładuje rozszerzenie ręcznie (tryb dewelopera); brak Firefoksa w MVP |
| Wstrzyknięcie DLL / hook w proces przeglądarki | nic do instalowania | alarm antywirusa, niestabilne, zachowanie typowe dla malware; **odrzucone** |
| `--remote-debugging-port` (CDP) | pełna kontrola kart | restart przeglądarki z flagą; otwarty port debugowania to dziura bezpieczeństwa |
| Plik `hosts` / proxy / PAC | każda przeglądarka | prawa administratora albo zmiana systemowa; nie wraca po crashu; psuje inne aplikacje |
| Tylko tytuł okna przeglądarki | zero instalacji | wykrywa, ale nie blokuje; zależy od tytułu strony |

**Decyzja:**

1. **Pętla faz.** Idle → Focus (cykl 1) → Break → Focus (cykl 2) → … aż do Stop. Czasy z `TimingProfile::from_config`
   (domyślnie 45/15 min, presety 25/5, 45/15, 50/10, 90/20; zakresy 1–240 i 1–120 min). Demo (`--demo` / `FOCUSMODE_DEMO=1`): 40 s / 20 s.
   Pauza zamraża pozostały czas w Focus i w Break. „Pomiń fazę” działa w obie strony (pominięty Focus liczy się jako ukończony cykl).
   Stop jest idempotentny w każdej fazie i zawsze daje dokładnie jedno `SessionEnded`.

   | Faza | Start | Pause | Resume | Stop | SkipPhase | Tick po terminie |
   |---|---|---|---|---|---|---|
   | Idle | → Focus 1: osłony ↑, śledzenie ↑, dzwonek FocusStart | – | – | – | – | – |
   | Focus | – | → pauza: śledzenie ↓, osłony zostają | – | → Idle: śledzenie ↓, osłony ↓, podsumowanie | → Break (jak po terminie) | → Break: śledzenie ↓, osłony ↓, dzwonek BreakStart, toast |
   | Break | – | → pauza | – | → Idle, podsumowanie | → Focus n+1 | → Focus n+1: dzwonek FocusStart, toast, osłony ↑, śledzenie ↑ |
   | Focus/Break w pauzie | – | – | → bieg, `deadline = now + remaining` (w Focus śledzenie ↑) | → Idle, podsumowanie | → następna faza, bez pauzy | czas stoi |

   Kolejność efektów należy do kontraktu: przy przejściu do przerwy osłony schodzą **przed** toastem (inaczej wyciszone powiadomienia
   połknęłyby komunikat o przerwie), a przy powrocie do skupienia toast idzie **przed** podniesieniem osłon.
   Luka zegara > 90 s (uśpienie, hibernacja, skok zegara kalendarzowego) daje automatyczną pauzę z `remaining` liczonym od ostatniego zaufanego ticka
   i `Notify(Interrupted)`. Czasu luki nie liczymy ani jako skupienia, ani jako przerwy.
2. **Muzyka w WebView, nie w rodio.** Źródła: `stream` (domyślnie publiczny live Lofi Girl na YouTube: tylko link, bez ich grafik i marki),
   `links`, `folder`, `noise` (brown/pink/white w WebAudio, offline) i `none`. Odtwarzacz żyje w layoucie okna głównego. Zamknięcie okna je ukrywa,
   a WebView2 dostaje `--autoplay-policy=no-user-gesture-required`. Muzyka gra w Focus i w Break (chyba że włączono `pauseDuringBreak`),
   stoi w pauzie i zatrzymuje się po Stop. Asset protocol dopuszcza tylko pliki audio znalezione przy skanie folderu wybranego przez użytkownika
   (nie cały folder). YouTube gra w osadzonym iframe sterowanym przez `postMessage`, bez ładowania skryptów Google (`iframe_api`) do aplikacji,
   a okno ma ustawione CSP (`app.security.csp`). Link YouTube, który odmawia osadzania, gra w osobnym oknie YouTube (niżej);
   inny link, którego nie da się odtworzyć, jest pomijany z komunikatem.

   **Okno YouTube (fallback, 2026-10-13).**
   - *Dlaczego:* test end-to-end zainstalowanej aplikacji pokazał, że ówczesny domyślny stream (live Lofi Girl, `watch?v=jfKfPfyJRdk`) w osadzonym playerze
     kończy się błędem 150. Na stronie YouTube okazało się, że ta transmisja już się skończyła („Nagranie tej transmisji jest niedostępne”):
     identyfikatory transmisji Lofi Girl się zmieniają, więc domyślny adres to teraz `https://www.youtube.com/@LofiGirl/live` (zawsze bieżąca transmisja kanału). Sprawdzone 2026-10-13: tak samo z https://example.com i dla
     `/embed/live_stream?channel=…`, a zwykłe filmy w naszym embedzie grają. Osadzania odmawia też wiele utworów YouTube Music (kanały „Topic”).
   - *Jak:* gdy player zgłosi `onError` 101, 150 albo 153 albo link youtube.com / music.youtube.com / youtu.be nie daje się osadzić
     (np. `/@kanał/live`, `/channel/<id>/live`, `/@kanał`), odtwarzacz woła `open_media_window`. Powłoka otwiera osobne okno Tauri `media`,
     które ładuje stronę YouTube jako stronę najwyższego poziomu (jak zwykła karta przeglądarki). Okno nie ma żadnych uprawnień IPC.
     Powłoka steruje nim (`control_media_window`: play, pause, głośność, pokaż, zamknij) przez `eval` wstrzykniętego kontrolera,
     a stan (otwarte, adres, tytuł) wysyła jako `media://window` z rewizją (getter `get_media_window`). Nawigacja jest ograniczona do domen YouTube i Google,
     a inne linki otwierają się w przeglądarce systemowej. Okno dzieli profil WebView2 z oknem głównym, więc zgoda YouTube na ciasteczka zostaje zapamiętana.
     **Okno jest ukrytym odtwarzaczem w tle** (decyzja właściciela produktu: nie zaśmiecać ekranu): tworzone jako niewidoczne, X tylko je chowa
     (muzyka gra dalej), a aplikacja niszczy je dopiero przy zmianie źródła. Wyjątek: dopóki YouTube nie zapisał wyboru dotyczącego ciasteczek
     (ciasteczko `SOCS` z niezerowym pierwszym polem; przy pierwszej wizycie YouTube ustawia `SOCS` z zerem), okno pojawia się na środku ekranu,
     bo YouTube nic nie odtwarza, dopóki użytkownik nie wybierze opcji. Po zapisaniu wyboru okno chowa się samo. Aplikacja nie klika zgody za użytkownika.
   - *Konsekwencje:* ciasteczka YouTube/Google trafiają do profilu WebView2 aplikacji. W tym trybie aplikacja nie przechodzi sama do następnego linku
     po końcu utworu. Sprawdzone 2026-10-13 na buildzie release: ukryte okno odtwarza (WebView2 nie odkłada mediów w ukrytym oknie Tauri), pauza, głośność i przerwa działają.
     W oknie `media` działają skrypty YouTube (w oknie głównym nadal nie).
3. **rodio tylko do dzwonków.** `focus-audio::ChimePlayer` gra syntetyczny dzwonek (BreakStart opadający, FocusStart rosnący) na dedykowanym wątku.
   Urządzenie otwiera leniwie i ponownie po błędzie. Brak urządzenia = log + `app://error`, sesja trwa.
4. **Osłony skupienia** podnosimy na czas Focus (także w pauzie), a opuszczamy w przerwie, po Stop i przy wyjściu z aplikacji.
   Każdą włącza się osobno w ustawieniach. Wykonawcą jest jeden wątek powłoki (`shields.rs`) z docelowym stanem i generacją (zgodnie z R02).

   | Osłona | Mechanizm | Domyślnie | Powrót |
   |---|---|---|---|
   | Powiadomienia Windows | „Nie przeszkadzać” (Windows 11) / Focus Assist „Tylko priorytetowe” (Windows 10) przez nieudokumentowaną klasę COM bieżącego użytkownika `QuietHoursSettings` (`IQuietHoursSettings::put_UserSelectedProfile`, tej samej używa centrum powiadomień), bez praw administratora, bez rejestru i restartu Explorera. Stan sprzed focusa czytamy przez `get_UserSelectedProfile`, a awaryjnie ze stanu WNF `WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED` (tylko odczyt). DND włączamy tylko wtedy, gdy był wyłączony | wył. | w przerwie, po Stop i przy wyjściu DND jest wyłączany z powrotem, ale tylko jeśli włączył go FocusMode (włączonego wcześniej nie ruszamy). Zaraz po włączeniu `MuteToken` trafia do `shields-restore.json`, a w `HKCU\…\RunOnce` powstaje wpis `FocusModeRestore` (FocusMode z `--restore-shields`). Po crashu albo kill powiadomienia przywraca następny start aplikacji, a jeśli nikt jej nie uruchomi, następne logowanie |
   | Teams „Nie przeszkadzać” | Microsoft Graph `setUserPreferredPresence` (delegowane `Presence.ReadWrite`, device code flow, własna rejestracja aplikacji w Entra ID, token odświeżania szyfrowany DPAPI) z `expirationDuration` = focus + 5 min | wył. | `clearUserPreferredPresence` na przerwę i po Stop; gdy aplikacja zginie, status sam wygasa |
   | Blokada stron | rozszerzenie MV3 pyta `GET http://127.0.0.1:47615/v1/state` (co 30 s, przy zmianie karty i nawigacji) i przekierowuje strony z listy `websiteblacklist.txt` (domeny i regexy, niżej) na swoją stronę „Wracaj do pracy”. Awaryjnie, zawsze gdy rozszerzenie nie odpytało mostu w ostatnich 90 s (np. nie jest zainstalowane), tracker porównuje tytuł okna przeglądarki z listą → `focus://nudge`, toast i okno FocusMode na wierzch | wł. | most zwraca `active: false`, rozszerzenie zdejmuje reguły; strona blokady w przerwie pokazuje link do oryginału |
   | Nie usypiaj | `PowerCreateRequest` + `PowerSetRequest` (ekran + system) | wł. | `PowerClearRequest`; żądanie znika razem z procesem |

   Most HTTP słucha tylko na loopbacku, obsługuje tylko odczyt stanu (`GET`) i wysyła nagłówki CORS tylko originom rozszerzeń.
   Adresy odwiedzanych stron nie trafiają do aplikacji: rozszerzenie dopasowuje je u siebie.

   **Lista stron w pliku `websiteblacklist.txt` (2026-10-13).**
   - *Dlaczego:* właściciel produktu chce listy, którą edytuje się jak zwykły plik tekstowy, z obsługą wyrażeń regularnych
     i od razu wypełnionej popularnymi rozpraszaczami dla polskich i anglojęzycznych użytkowników (kwejk, jbzd, reddit, facebook…).
     Same domeny nie wystarczą, gdy trzeba zablokować tylko część serwisu (YouTube Shorts, tablica LinkedIn, strona główna portalu z pocztą).
   - *Gdzie:* `websiteblacklist.txt` w katalogu konfiguracji aplikacji (na Windows `%APPDATA%\com.focusmode.app\websiteblacklist.txt`).
     Przy pierwszym starcie (brak pliku) powłoka tworzy go z domyślnej listy wbudowanej w aplikację (`crates/focus-core/assets/websiteblacklist.txt`).
     **Plik to jedyne źródło listy**: pole `config.shields.blocklist` zostało usunięte, a `shields.blockSites` zostaje jako włącznik osłony.
     Stary `config.json` z tym polem nadal się wczytuje, a pole jest ignorowane.
   - *Format:* jedna pozycja w linii; linia zaczynająca się od `#` to komentarz, puste linie są pomijane.
     Zwykła linia to domena (normalizowana: bez schematu, ścieżki, portu i początkowego `www.`), która blokuje siebie i wszystkie subdomeny.
     Linia ujęta w ukośniki `/…/` (co najmniej 3 znaki) to regex ze środka. Linia zawierająca któryś ze znaków `\ ^ $ * + ? ( ) [ ] { } |`
     to regex w całości (sama kropka nie robi z linii regexu, więc `kwejk.pl` jest domeną). Wklejony adres ze znakiem `?`
     (np. `https://www.youtube.com/watch?v=…`) byłby regexem, który nie pasuje nawet do samego siebie, więc trafia do `issues` z podpowiedzią (sama domena albo `\?`).
     Regexy mają składnię RE2 (wspólną dla crate'u `regex` i Chrome: bez lookaround i odwołań wstecznych) i są dopasowywane bez rozróżniania
     wielkości liter: **w rozszerzeniu do pełnego adresu URL karty**, a **w awaryjnym nudge'u do tytułu okna przeglądarki**.
   - *Jak:* `focus-core` (`blocklist.rs`) parsuje tekst czystą funkcją, bez I/O. Linie z błędem (niepoprawna domena, niepoprawny albo zbyt złożony regex,
     przekroczony limit) są pomijane i trafiają do `issues` z numerem linii i polskim opisem, a reszta listy działa.
     Powłoka (`src-tauri/src/blocklist.rs`) wczytuje plik przy starcie, **automatycznie po każdym zapisie** i na żądanie (`reload_blocklist`).
     Zmiana wyniku daje `blocklist://changed` (`BlocklistInfo` z rewizją, getter `get_blocklist`), a most od razu podaje nowe `blocklist` i `patterns` (rozszerzenie bierze je przy następnym odpytaniu, najpóźniej po 30 s).
     Gdy pliku nie da się przeczytać, `BlocklistInfo.error` opisuje błąd, a w użyciu zostaje poprzednia lista (albo domyślna).
   - *UI:* w Ustawieniach (blokada stron) przycisk otwiera plik w **domyślnym edytorze tekstu systemu** (`open_blocklist_file`), drugi wczytuje go
     ponownie. Obok są liczba domen i wzorców, ścieżka pliku i lista pominiętych linii.
   - *Domyślna lista:* polskie i anglojęzyczne rozpraszacze w sekcjach (memy, media społecznościowe i fora, plotki, sport, wideo i streaming, gry, zakupy).
     LinkedIn jest blokowany tylko na tablicy (`linkedin\.com/feed`), a YouTube tylko w Shorts (cały YouTube jest zakomentowany).
     Portale (onet, wp, interia, o2, gazeta) są blokowane **tylko na stronie głównej** regexem `^https?://(www\.)?(onet|wp|interia|o2|gazeta)\.pl/?(\?.*)?$`,
     więc ich poczta i inne usługi działają dalej. Wiadomości i komunikatory są zakomentowane, do włączenia przez usunięcie `#`.
   - *Konsekwencje:* regexy wykonują się w czasie liniowym (RE2 / `regex`), więc wzorzec z pliku nie zawiesi aplikacji ani przeglądarki.
     Wzorce pisane pod adres URL (np. z `^https?://`) nie pasują do tytułu okna, więc bez rozszerzenia nudge łapie głównie domeny.
     Prywatność się nie zmienia: rozszerzenie dostaje listę z mostu i dopasowuje adres lokalnie, a tytułów okien nie zapisujemy.
     Nowa zależność: `regex` w `focus-core` (czysta biblioteka, więc zasada „core bez I/O” zostaje).
5. **Śledzenie aktywnej aplikacji.** Co 1 s w trakcie biegnącego Focus (nie w pauzie i nie w przerwie) pobieramy nazwę procesu okna na pierwszym planie
   i czas bezczynności (`GetLastInputInfo`; próg 120 s, w demo 30 s). Zapisujemy tylko sumy sekund per proces oraz „Bezczynność” i „Nieznane”.
   Tytułów okien nie zapisujemy. Po Stop `SessionSummary` trafia do `history.jsonl` i na ekran podsumowania z wykresem słupkowym według procesu.
   Śledzenie można wyłączyć (`trackApps`).
6. **Okno i tray.** Jedno okno główne (Skupienie / Podsumowanie / Ustawienia) i tray (Start, Pauza/Wznów, Pomiń fazę, Stop, Pokaż okno, Zakończ),
   którego pozycje idą kanałem aktora, niezależnie od WebView. Toasty z `tauri-plugin-notification`, single-instance, opcjonalny autostart
   (przy logowaniu aplikacja startuje zminimalizowana do zasobnika).
   Instalator NSIS per-user z rozszerzeniem jako zasobem.
7. **Odłożone:** pełnoekranowe okna przerwy na każdym monitorze i prowadzone ćwiczenia (`assets/exercises.json`, planer przerwy),
   okienko ostrzeżenia i „Jeszcze 2 min”, tryb „Jestem na spotkaniu”, tint (overlay i gamma), fade i gong w rodio, gatunki `genres.toml`,
   statystyka dnia jednym zdaniem, rozszerzenie dla Firefoksa, gwarancje UX na macOS/Linux (tam działają implementacje `Noop`).

**Konsekwencje:**
- `api.rs` jest napisany od nowa pod ten zakres i powstał w tej samej gałęzi co implementacja. To **wyjątek** od zasady osobnego PR-a z kontraktem.
  Dopóki `cargo test` nie ruszy, bindingi TS są pisane ręcznie w formacie ts-rs. Po pierwszym zielonym `cargo test -p focus-core`
  polecenie `git diff src/lib/bindings` musi być puste.
- Przerwa nie wygania już od ekranu: to dzwonek, toast i karta z propozycjami ruchu w oknie głównym. „Wymuszenie” przerwy jest słabsze,
  za to nic nie może uwięzić użytkownika.
- Wyciszenie powiadomień opiera się na nieudokumentowanym interfejsie COM Windows i może przestać działać po aktualizacji systemu.
  Gdy klasy COM nie ma, osłona dostaje status `Unavailable`, a gdy nie uda się samo przełączenie, `Error` + `app://error`. Sesja trwa w obu
  przypadkach. Dlatego osłona jest domyślnie wyłączona. Gdy DND było wyłączone przed focusem, a użytkownik zmieni je ręcznie w trakcie,
  koniec focusa i tak je wyłączy.
- Automatyczne `clearUserPreferredPresence` kasuje też status ustawiony w Teams ręcznie (ryzyko z ADR-010 R07). Przyjmujemy to świadomie,
  bo właściciel produktu chce zdjęcia DND w przerwie; ustawienia o tym ostrzegają. W długiej pauzie DND może wygasnąć przed końcem focusa
  (`TODO(hackathon)`: odnawianie).
- Teams wymaga własnej rejestracji aplikacji w Entra ID i zgody tenantu (T0-8 nadal nierozstrzygnięty), więc pozostaje funkcją opcjonalną.
- Rozszerzenie trzeba raz załadować ręcznie. Między zmianą fazy a nałożeniem albo zdjęciem reguł mija do 30 s (przy nawigacji szybciej).
  Gdy port 47615 jest zajęty, mostu nie ma i zostaje awaryjny nudge po tytule okna.
- Muzyka zależy od WebView2 i sieci (poza folderem i szumem). Część linków YouTube nie pozwala na osadzanie; grają w osobnym oknie YouTube (pkt 2).
- Nowe zależności: `windows` 0.62 i `reqwest` 0.13 w `focus-platform`; pluginy Tauri dialog, notification, single-instance, autostart i log,
  a także `tokio`, `tiny_http` i `anyhow` w powłoce; pluginy JS dialog, notification, autostart i log; `regex` w `focus-core` (lista stron, pkt 4).
  `rodio =0.22.2` zostaje, tylko dla dzwonków.
- DESIGN.md i ARCHITECTURE.md są w części nieaktualne (w ARCHITECTURE aktualny jest §6, kontrakt IPC). Do czasu ich aktualizacji wygrywa ten ADR i `api.rs`.

**Co zastępuje:**
- ADR-003 **częściowo**: rodio i dedykowany wątek zostają tylko dla dzwonków; muzyka, gatunki, fade i docelowy stan odtwarzania (R04) przechodzą do WebView.
- ADR-009 **w części**: tint wypada z zakresu. Jeśli wróci, obowiązuje overlay zamiast gammy.
- ADR-010 **w części**: ręczny tryb spotkania odpada. Teams DND z *Could* staje się opcjonalną osłoną z automatycznym czyszczeniem (uchyla R07)
  i wygaśnięciem focus + 5 min zamiast odnawianych 10 min. Wykrywanie rozmów nadal Won't.
- ADR-011 **w części**: zostają `TimingProfile`, dwa zegary i heurystyka luki (próg 90 s; w przerwie też auto-pauza, a nie Idle).
  Odpadają ostrzeżenie w focusie, Postpone, plan przerwy i wyniki przerwy. Demo to 40/20 s.
- ADR-012 **w części**: zostają snapshot z `revision`, wzorzec listen → getter → nowsza rewizja i wykonawcy efektów (R02).
  Odpadają okna przerwy, ostrzeżenia i tint, `window_ready`, heartbeat oraz `break_id`.
- ADR-001, 002, 006, 007 i 008 bez zmian: core nadal jest czysty, kontrakt idzie przez ts-rs, a platformą odbioru jest Windows.

**Poprawka (ADR-014, 2026-10-13):** pkt 6: pierwsze X na oknie głównym pyta, czy schować aplikację do zasobnika, a zamiast toastów
mogą się pojawiać okienka FocusMode. Pkt 7: tint nie jest już odłożony (wraca jako overlay). Pkt 4: dochodzi osłona „zamknij launchery gier”;
Teams nie rusza statusu „offline” ani własnego DND użytkownika; `BlocklistIssue` niesie kod (`code`), a polski opis wybiera UI.

**Poprawka (ADR-015, 2026-10-13):**
- Pkt 2: dochodzi źródło `library` (muzyka dołączona do aplikacji: Open Lo-Fi, CC0, offline) i to ono jest domyślne w nowym configu.
  Stream Lofi Girl zostaje jako wybór; stary `config.json` z zapisanym źródłem zachowuje je.
- Pkt 3: oprócz syntetycznych pakietów dzwonków rodio gra też własny plik użytkownika skopiowany do folderu danych aplikacji.
- Pkt 5: tracker pomija powierzchnie powłoki Windows (pasek zadań, pulpit, menu Start, wyszukiwanie, widok zadań, `ShellExperienceHost` itd.).
  **Tytuły okien zapisujemy** (zdanie „Tytułów okien nie zapisujemy” przestaje obowiązywać), ale tylko przy włączonym `trackTitles`,
  przycięte i tylko lokalnie, jako podpowiedź na wykresie. Zapis do `history.jsonl` można wyłączyć (`keepHistory`), a historię usunąć jednym przyciskiem.

---

## ADR-014: Zasobnik i okienka faz, odcień ekranu (overlay), zamykanie launcherów gier, języki
**Status:** Przyjęta (2026-10-13, B na prośbę właściciela produktu; A potwierdza przy review PR-a z gałęzi `b/speedrun-mvp`); w części zmieniona przez ADR-015 (2026-10-13: pkt 2 i 6, overlay odcienia odrzucony)

**Kontekst:** po pierwszych testach zakresu z ADR-013 właściciel produktu poprosił 2026-10-13 o:
1. możliwość schowania aplikacji do zasobnika i **okienka przy starcie skupienia i przerwy** w stylu FocusMode (małe okno w rogu ekranu,
   jak komunikaty antywirusa) zamiast toastów Windows. Pierwsze X na oknie ma zawsze zapytać, czy schować aplikację do zasobnika, a „tak” włącza też okienka;
2. **ciepły odcień ekranu** zależny od pory dnia (np. cieplej po 18:00, normalnie od 9:00), domyślnie wyłączony;
3. **zamykanie launcherów gier** (Steam, Epic Games Store, GOG Galaxy…) na start skupienia, z polami wyboru, które zamykać;
4. przełączniki „Nie przeszkadzać” także dla **innych komunikatorów** (np. Discord), ale nie wtedy, gdy użytkownik jest tam niewidoczny;
   jeśli się nie da, pominąć.

Do tego kolejny krok to tłumaczenie aplikacji na 10 języków, więc teksty dla użytkownika nie mogą powstawać w `focus-core`.
Stan z ADR-013 stał temu na przeszkodzie: X zawsze chowało okno (bez pytania, więc łatwo było przeoczyć, że aplikacja działa dalej), tint był odłożony
(twarda zasada 6 w CLAUDE.md), Teams DND przykrywało także „Wyświetlaj jako offline” i ręczne DND, a potem je czyściło,
a parser `websiteblacklist.txt` w core zwracał opisy błędów po polsku.

**Opcje — okienka faz:**

| Opcja | Plusy | Minusy |
|---|---|---|
| **Własne okno Tauri `popup` w rogu ekranu** | wygląd aplikacji; własne przyciski (pokaż okno, pomiń fazę); nie jest powiadomieniem Windows, więc „Nie przeszkadzać” go nie wycisza | kolejne okno WebView (pamięć); nie może kraść fokusu ani zasłaniać pracy |
| Toast Windows (`tauri-plugin-notification`, stan z ADR-013) | natywny, bez kodu UI | wygląd systemu; w trybie dev nazwa i ikona PowerShell; przyciski akcji w pluginie są tylko na mobile |
| Okno główne na wierzch | nic nowego | wyrywa z pracy i kradnie fokus |

**Opcje — odcień ekranu:**

| Opcja | Plusy | Minusy |
|---|---|---|
| **Overlay: przezroczyste, klikalne na wylot okno na każdym monitorze** (ADR-009) | znika z procesem, więc kolory zawsze wracają; działa z HDR i razem z „Oświetleniem nocnym” Windows; zwykłe API okien Tauri | nie przykrywa gier w wyłącznym pełnym ekranie; mieszanie alfa obniża kontrast; trafia na zrzuty ekranu |
| Gamma ramp (`SetDeviceGammaRamp`) | prawdziwa zmiana kolorów, także kursora | odrzucona w ADR-009 (R08): brak gwarancji powrotu po kill, gubi kalibrację, HDR niezdefiniowany |
| Sterowanie „Oświetleniem nocnym” Windows | systemowe | brak publicznego API (tylko nieudokumentowany zapis w rejestrze); odrzucone |

**Opcje — launchery gier:**

| Opcja | Plusy | Minusy |
|---|---|---|
| **Zamknięcie procesów o dokładnych nazwach, w sesji użytkownika, na start focusa** | bez praw administratora; użytkownik wybiera launchery | launcher można uruchomić ponownie w trakcie; nazwy procesów zmieniają się z wersjami |
| Blokada uruchamiania (IFEO `Debugger`, AppLocker, zatrzymanie usług) | trwała | prawa administratora, zmiana systemowa, nie wraca po crashu; odrzucone |
| Tylko przypomnienie | bezpieczne | nic nie zamyka |

**Opcje — inne komunikatory:**

| Opcja | Plusy | Minusy |
|---|---|---|
| **Teams przez Graph z odczytem statusu przed zmianą** | oficjalne API; ta sama zgoda `Presence.ReadWrite` | Graph nie odróżnia „Wyświetlaj jako offline” od braku zalogowania; status aktualizuje się z opóźnieniem |
| Discord | – | brak API do zmiany statusu użytkownika (OAuth2 i lokalne RPC Discorda tego nie dają); automatyzacja konta tokenem użytkownika („self-bot”) łamie warunki korzystania z Discorda i grozi blokadą konta; **odrzucone** |
| Slack (`dnd.setSnooze`) | oficjalne API | własna aplikacja Slack i zgoda workspace'u, kolejna integracja OAuth; poza zakresem, kandydat na później |

**Decyzja:**

1. **Zasobnik i okienka faz.**
   - `Config.closeToTray: Option<bool>`, domyślnie `None` (= zapytaj). Dopóki wyboru nie ma, X na oknie głównym go nie zamyka: powłoka przerywa
     zamknięcie (`CloseRequested` → `prevent_close`) i wysyła do okna `app://close-requested`. UI pyta, czy schować FocusMode do zasobnika,
     i odpowiada komendą `resolve_close { toTray }`. Powłoka zapisuje wybór w configu: `toTray: true` włącza też `phasePopups` i chowa okno,
     a `false` kończy aplikację (zwykłe wyjście, więc osłony schodzą). Kolejne X działa według zapisanego wyboru, bez pytania; wybór można zmienić w Ustawieniach.
     Bez odpowiedzi: drugie X w ciągu minuty od pytania (ale nie szybciej niż po 1 s, czyli nie dwuklik) chowa okno do zasobnika, a X po dłuższym czasie
     (pytanie zamknięte Escape albo zignorowane) pyta ponownie.
   - Okna głównego nie niszczymy, dopóki działa aplikacja, bo w nim gra muzyka (ADR-013 pkt 2). X albo je chowa, albo kończy cały proces.
     „Zakończ” w zasobniku działa zawsze, także gdy WebView nie odpowiada.
   - `Config.phasePopups`, domyślnie wyłączone; włącza je wybór zasobnika przy pierwszym X albo przełącznik w Ustawieniach. Gdy jest włączone,
     zamiast toastu Windows powłoka pokazuje okno `popup` przy starcie przerwy i skupienia, przy automatycznej pauzie po uśpieniu
     i po zamknięciu launcherów (`PopupKind`: `breakStarted`, `focusStarted`, `interrupted`, `launchersClosed`). Okno jest małe, w prawym dolnym rogu ekranu,
     w stylu aplikacji, zawsze na wierzchu, bez paska zadań i bez przejmowania fokusu. Znika samo po kilku sekundach albo po kliknięciu:
     `popup_action` z `dismiss`, `showMain` albo `skipPhase` (to samo, co „Pomiń fazę” w zasobniku).
   - Okno `popup` (`src-tauri/src/popup.rs`, własny wątek) powstaje widoczne i bez aktywacji (`focused(false)`) dla każdego okienka i jest niszczone,
     gdy okienko znika, bo `show()` na istniejącym oknie aktywowałoby je i zabrało fokus. Okienko, które przychodzi, gdy poprzednie jest jeszcze widoczne,
     tylko je aktualizuje. Gdy strona sama się nie zamknie, powłoka chowa okno po limicie bezpieczeństwa. Gdy okna nie da się utworzyć,
     powłoka pokazuje toast, który okienko zastępuje (degradacja, nigdy oba naraz).
   - Stan okienka: `get_popup` + `popup://show` (`PopupInfo` z rewizją; `kind: null` = schowaj). Kolejność efektów z tabeli ADR-013 się nie zmienia:
     core wysyła `Notify` w tym samym miejscu, a powłoka wybiera tylko kanał (okienko albo toast).
2. **Odcień ekranu wraca, ale tylko jako overlay (ADR-009).**
   - Przezroczyste, klikalne na wylot okna overlay, po jednym na monitor: bez dekoracji, zawsze na wierzchu, bez paska zadań i bez fokusu,
     w ciepłym kolorze z przezroczystością zależną od poziomu. Gamma ramp i „Oświetlenie nocne” Windows odpadają.
   - **Domyślnie wyłączony.** `Config.tint: TintConfig { enabled, startMinutes, endMinutes, strength, transitionMinutes }`; domyślnie od 18:00 (`1080`)
     do 09:00 (`540`), siła 45 ze 100, przejście 30 min (zakres 0–180). Okno czasu może przechodzić przez północ, ale początek i koniec muszą się różnić.
     Walidacja (`config.invalid` z polem `tint.*`) obowiązuje także przy wyłączonym odcieniu.
   - Poziom liczy czysta funkcja `focus_core::tint::tint_level(cfg, minute_of_day)`: od `startMinutes` rośnie liniowo przez `transitionMinutes`,
     a przed `endMinutes` tak samo opada (przejście zajmuje najwyżej połowę okna). Minutę doby czasu lokalnego daje `focus-platform`
     (`clock.rs`, `GetLocalTime`), bo core nie czyta zegara, a kalendarzowe `now` sesji jest w UTC. Bez nowej zależności (np. `chrono`).
   - Powłoka (`src-tauri/src/tint.rs`) przelicza poziom cyklicznie i po każdej zmianie configu, a przy poziomie 0 overlay znika.
     Odcień nie jest osłoną skupienia: działa według pory dnia niezależnie od sesji (także w Idle i w przerwie), dopóki działa aplikacja.
   - Podgląd w Ustawieniach: `preview_tint { strength }` pokazuje odcień o danej sile przez kilka sekund mimo harmonogramu, a `null` od razu kończy podgląd.
     Stan: `get_tint_status` + `tint://changed` (`TintStatus` z rewizją: `enabled`, `level`, `preview`, `active`, `error`).
   - W UI opisujemy go jako efekt wizualny: bez kalibracji barw i bez obietnic zdrowotnych (zasada 10 w CLAUDE.md).
3. **Zamykanie launcherów gier** (nowa osłona skupienia).
   - `shields.closeLaunchers` (domyślnie wyłączone) i `shields.launchers` (pola wyboru). Do wyboru: Steam, Epic Games Launcher, GOG Galaxy, Ubisoft Connect,
     EA app, Battle.net i Riot Client (domyślnie zaznaczone) oraz Xbox app, Amazon Games i itch.
   - Na **każdy start fazy Focus** (pierwszy cykl, każdy następny i „Pomiń fazę” z przerwy) powłoka zamyka procesy zaznaczonych launcherów.
     Dopasowanie tylko po **dokładnych nazwach plików `.exe`** z `focus_core::launchers::process_names`, bez rozróżniania wielkości liter
     (np. `steam.exe`, `steamwebhelper.exe`, `EpicGamesLauncher.exe`, `GalaxyClient.exe`; `steam.exe` nigdy nie trafi w `steamwebhelper.exe`).
     Ogólnych nazw (`agent`, `launcher`, `helper`, `updater`…) na liście nie ma i pilnuje tego test.
   - `focus-platform/src/processes.rs`: lista procesów z Toolhelp (`CreateToolhelp32Snapshot`), tylko procesy **bieżącej sesji Windows** (`ProcessIdToSessionId`),
     nigdy własny proces. Proces zamykamy przez `TerminateProcess` (po ponownym sprawdzeniu nazwy przez uchwyt, bo pid mógł zostać użyty ponownie),
     bez łagodnego `WM_CLOSE`, bo launchery na nie tylko chowają się do zasobnika. Główny proces launchera idzie przed procesami pomocniczymi,
     żeby rodzic ich nie wskrzesił. Usług (`steamservice`, `EABackgroundService`…), procesów wymagających praw administratora i procesów gier nie ruszamy.
     Proces, którego nie da się zamknąć, jest pomijany; błędem jest tylko nieudany odczyt listy procesów.
   - Wynik trafia do `ShieldStatus.launchers` (`IntegrationStatus`) i `ShieldStatus.closedLaunchers`, a gdy coś zamknięto, do okienka `launchersClosed`.
     Poza Windows: `Unavailable`.
   - To jednorazowe sprzątanie, nie blokada: launchera nie pilnujemy w trakcie focusa i nie uruchamiamy go ponownie w przerwie ani po Stop.
     Nie ma czego przywracać, więc zasada „osłony zawsze wracają” tej osłony nie dotyczy.
4. **Teams szanuje status użytkownika; Discord odrzucony.**
   - Na start focusa, przed `setUserPreferredPresence`, powłoka czyta bieżący status (`GET /me/presence`, w ramach zgody `Presence.ReadWrite`).
     Gdy `availability` to `Offline` („Wyświetlaj jako offline” albo brak zalogowania), FocusMode **nie ustawia** DND, bo DND zrobiłoby z niewidocznego
     użytkownika widocznego. Gdy `availability` to już `DoNotDisturb` (ustawione przez użytkownika, prezentacja, czas skupienia), FocusMode też go
     nie ustawia. W obu przypadkach **nie czyści** statusu w przerwie ani po Stop: czyścimy tylko DND, które ustawiliśmy sami.
   - Discord: nie robimy. Discord nie daje aplikacjom API do zmiany statusu użytkownika, a automatyzacja konta tokenem użytkownika („self-bot”)
     łamie warunki korzystania z Discorda i grozi blokadą konta. Banery, które Discord pokazuje jako powiadomienia Windows, wycisza osłona powiadomień Windows.
   - Inne komunikatory (np. Slack przez oficjalne `dnd.setSnooze`) są poza zakresem.
5. **Języki.**
   - `Config.language`: `system` (domyślnie) albo jeden z 10 języków: `en`, `pl`, `de`, `fr`, `it`, `uk`, `ru`, `es`, `zh`, `ja`.
     `system` oznacza język wyświetlania Windows (`get_system_language`: `focus-platform/src/locale.rs` + `Language::from_tag`), a gdy go nie ma na liście, angielski.
   - Język obejmuje UI, zasobnik, okienka faz i toasty. `focus-core` nie zawiera tekstów dla użytkownika: zwraca kody i ewentualny szczegół po angielsku
     (`AppError.code`, nowe `BlocklistIssue.code`), a tekst wybiera UI (`src/lib/strings.ts`) albo powłoka (`src-tauri/src/texts.rs`).
   - Tłumaczenia przyjdą osobnym przebiegiem. Do tego czasu wszystkie teksty są po polsku, niezależnie od `language`.
6. **Okna i kontrakt.**
   - Okno `popup` i okna overlay ładują lekkie strony z grupy tras `src/routes/(overlay)/`, bez odtwarzacza muzyki i nawigacji. Okno główne ładuje
     `src/routes/(app)/`, gdzie żyje odtwarzacz, więc muzyka nie zagra w kilku oknach naraz.
   - Kontrakt (`api.rs`; jak w ADR-013, w tej samej gałęzi co implementacja): komendy `resolve_close`, `get_system_language`, `get_popup`, `popup_action`,
     `get_tint_status` i `preview_tint`; eventy `app://close-requested`, `popup://show` i `tint://changed`; DTO `Language`, `TintConfig`, `GameLauncher`,
     `PopupKind`, `PopupInfo`, `PopupAction`, `TintStatus` i `BlocklistIssueCode`; nowe pola `Config` i `ShieldStatus`. Szczegóły: ARCHITECTURE §6.

**Konsekwencje:**
- Overlay nie zastąpi prawdziwej zmiany temperatury barwowej. Nie przykrywa gier i aplikacji w **wyłącznym pełnym ekranie** (omijają kompozycję pulpitu), ekranu blokady,
  okien UAC ani części elementów systemu (np. menu Start), a kursor zostaje w normalnych kolorach. To zwykłe okno, więc trafia na zrzuty ekranu i nagrania
  (chyba że powłoka wyłączy je z przechwytywania przez `content_protected`). Kolor jest nakładany przez mieszanie alfa, więc obraz traci trochę kontrastu,
  a czernie lekko się rozjaśniają (gamma tylko by przygasiła niebieski). W zamian kolory zawsze wracają: okna giną razem z procesem, także po crashu i kill.
- Okna overlay i `popup` to kolejne WebView, czyli więcej pamięci. Overlay na każdym monitorze trzeba odtworzyć po zmianie układu monitorów;
  gdy się nie uda, `TintStatus.error` opisuje błąd, a harmonogram próbuje dalej.
- Pierwsze X zachowuje się inaczej niż w ADR-013: przed wyborem nic się nie zamyka, a po „nie” X kończy aplikację (muzyka cichnie, osłony schodzą).
  Jeśli UI nie odpowie na `app://close-requested`, okno zostaje otwarte, a wyjściem jest „Zakończ” w zasobniku (zasada 7).
- Zamknięcie launchera może przerwać pobieranie albo aktualizację gry i czat w launcherze. Gra uruchomiona z launchera nie jest zamykana,
  ale może stracić nakładkę i synchronizację, a część gier kończy się razem z launcherem. Dlatego osłona jest domyślnie wyłączona.
  Nazwy procesów mogą się zmienić po aktualizacji launchera (lista w `crates/focus-core/src/launchers.rs`). Świadomie pominięte: Origin
  (EA wyłączyło tego klienta 2025-04-17, a nazwa koliduje z innym oprogramowaniem), zbyt ogólne `Agent.exe` (Battle.net) i `butler.exe` (itch),
  usługi oraz procesy uruchamiające gry.
- Teams: Graph zwraca `Offline` zarówno dla „Wyświetlaj jako offline”, jak i dla braku zalogowania. W obu przypadkach nic nie zmieniamy
  (bez sesji Teams preferowany status i tak nie byłby widoczny). Status w Graph aktualizuje się z opóźnieniem, więc odczyt tuż po naszym
  `clearUserPreferredPresence` (bardzo krótka przerwa, np. 20 s w demo) może jeszcze pokazać DND, a wtedy kolejny cykl pominie Teams.
  Ryzyko z ADR-013 zostaje w węższej postaci: status zmieniony ręcznie **w trakcie** focusa i tak zostanie wyczyszczony na przerwę.
- Discord i inne komunikatory: brak funkcji; README wyjaśnia dlaczego.
- `config.json` z nieznaną wartością enuma (np. język albo launcher z nowszej wersji po powrocie do starszej) nie wczytuje się i aplikacja wraca
  do ustawień domyślnych (`config.corrupt`), tak jak przy pozostałych enumach. Stary `config.json` bez nowych pól wczytuje się z wartościami domyślnymi.
- `BlocklistIssue` ma teraz `code` (`BlocklistIssueCode`), a `message` to angielski szczegół (np. powód z parsera regexu albo domena do podpowiedzi).
  Polski opis z ADR-013 pkt 4 wybiera UI. Podpowiedź dla linii `*.facebook.com` istnieje na razie tylko jako angielski dopisek do szczegółu `invalidRegex`;
  osobny kod (np. `wildcardDomain`) wymaga zmiany kontraktu przez `/sync-contract`.
- Nowych crate'ów nie ma. W `focus-platform` dochodzą feature'y `windows`: `Win32_System_Diagnostics_ToolHelp`, `Win32_Globalization` i `Win32_System_RemoteDesktop`.
- Twardej izolacji komend per okno (manifest komend w `build.rs`, ARCHITECTURE §10, B-12) nadal nie ma. Okna `popup` i overlay potrzebują tylko
  swoich getterów i eventów, a `popup` także `popup_action`.

**Co zmienia:**
- ADR-009: tint wraca do zakresu, tylko jako overlay i domyślnie wyłączony. Gamma ramp nadal nie.
- ADR-010 i ADR-013 pkt 4 (Teams): nie zmieniamy statusu „offline” ani DND ustawionego przez użytkownika i wtedy też go nie czyścimy.
- ADR-012: wracają okna overlay (tylko odczyt stanu) i dochodzi okno `popup`; oba stany mają getter + event z rewizją.
  Protokół gotowości, heartbeat i watchdog nie wracają.
- ADR-013: pkt 6 (X pyta raz; okienka FocusMode zamiast toastów, gdy włączone), pkt 7 (tint nie jest już odłożony),
  pkt 4 (nowa osłona: launchery gier; `BlocklistIssue.code`).
- CLAUDE.md, twarda zasada 6: „Bez pełnoekranowych okien przerwy, gammy i trybu spotkania (odłożone, ADR-013); odcień ekranu tylko jako overlay,
  domyślnie wyłączony (ADR-009, ADR-014)”.

**Poprawka (ADR-015, 2026-10-13):** pkt 2 nie obowiązuje w części o mechanizmie: overlay odcienia (okna `tint-*`, trasa `/tint`, wpis `tint-*`
w `capabilities/overlays.json`) jest usunięty, a ciepło ekranu daje Oświetlenie nocne Windows albo gamma ramp, z powrotem kolorów jak przy osłonach.
Harmonogram (`startMinutes`, `endMinutes`, `strength`, `transitionMinutes`), podgląd, domyślne wyłączenie i niezależność od sesji zostają.
Pkt 6: grupa tras `(overlay)` obsługuje już tylko okno `popup`. Konsekwencje dotyczące overlayu (pełny ekran, kontrast, zrzuty ekranu, okna per monitor) przestają obowiązywać,
a w ich miejsce wchodzą konsekwencje z ADR-015.

---

## ADR-015: Runda 2a: ciepło ekranu bez overlayu, motywy, dźwięki, muzyka, przerwy i historia
**Status:** Przyjęta (2026-10-13, B na prośbę właściciela produktu; A potwierdza przy review PR-a z gałęzi `b/speedrun-mvp`)

**Kontekst:** po testach zakresu z ADR-014 właściciel produktu poprosił 2026-10-13 o drugą rundę zmian (runda 2a):
1. **Odcień ekranu jako overlay jest odrzucony**: „półprzezroczysta warstwa koloru to nie jest odcień”. Warstwa mieszana alfą obniża kontrast i rozjaśnia czernie,
   nie zmienia kursora ani gier w wyłącznym pełnym ekranie, trafia na zrzuty ekranu i nagrania, a okna overlay były widoczne w Alt+Tab.
   Potrzebna jest prawdziwa zmiana temperatury barwowej obrazu.
2. **Motywy**: Midnight (obecny wygląd, domyślny), Vaporwave, fazy księżyca, przytulna jesień, prosty ciemny, prosty jasny i 8-bit.
3. **Nawigacja**: podświetlenie aktywnej zakładki płynnie przesuwa się między Skupieniem, Podsumowaniem i Ustawieniami, a tło lekko przesuwa się
   w lewo albo w prawo zgodnie z kolejnością zakładek.
4. **Dźwięki dzwonków**: lista pasująca do motywów i „własny”: użytkownik wybiera plik, aplikacja kopiuje go do swoich danych i go używa.
5. **Muzyka w zestawie**: utwory Open Lo-Fi (CC0) dołączone do aplikacji jako domyślna muzyka.
6. **Własne przypomnienia na kolejne przerwy**, np. „1. przerwa: rozciąganie; 2.: woda i spacer z psem; 3.: obiad”.
7. **Losowe porady na przerwę oparte na badaniach**, bez obietnic zdrowotnych.
8. **Historia** (prośba użytkownika): nie liczyć w historii skupienia elementów powłoki Windows (np. `ShellExperienceHost`); gdy na pierwszym planie jest Eksplorator,
   przeglądarka albo podobna aplikacja, zapamiętać tytuł okna i pokazać go po najechaniu myszą na wykres (z wyłącznikiem w Ustawieniach);
   przycisk usuwający historię (z potwierdzeniem); opcja, żeby historii w ogóle nie zapisywać.

Punkt 8 jest sprzeczny z twardą zasadą 11 w CLAUDE.md („tytułów okien nie zapisujemy”) i z ADR-013 pkt 5, więc wymaga tego ADR.
Punkt 1 odwraca ADR-014 pkt 2, który odrzucił Oświetlenie nocne (brak publicznego API) i gamma ramp (ADR-009, R08).

**Opcje — ciepło ekranu:**

| Opcja | Plusy | Minusy |
|---|---|---|
| Overlay (ADR-014) | kolory wracają razem z procesem | **odrzucony przez właściciela**: warstwa koloru zamiast zmiany temperatury; niższy kontrast, jaśniejsze czernie, widoczny na zrzutach, nie obejmuje pełnego ekranu ani kursora |
| **Oświetlenie nocne Windows** (domyślnie) | mechanizm systemu, który użytkownik zna; działa po kompozycji, więc zmienia cały obraz (także kursor) bez utraty kontrastu i nie trafia na zrzuty; ustawienie użytkownika, bez praw administratora | brak publicznego API: nieudokumentowany zapis ustawień w rejestrze, który może przestać działać po aktualizacji Windows (jak `IQuietHoursSettings` w ADR-013); zmienia ustawienie użytkownika, więc trzeba je przywracać także po crashu; jedna temperatura dla wszystkich monitorów |
| **Gamma ramp** (`SetDeviceGammaRamp`, do wyboru) | po kompozycji, per monitor, także w grach na pełnym ekranie i na pasku zadań; nie rusza ustawień Windows | Microsoft odradza; brak efektu na monitorach HDR; Windows odrzuca rampy zbyt odległe od liniowej (bardzo ciepłe wartości są słabsze); rampa zostaje po crashu; konflikt z kalibracją monitora (R08) |

**Opcje — tytuły okien w historii:**

| Opcja | Plusy | Minusy |
|---|---|---|
| **Najczęstsze tytuły każdej aplikacji, lokalnie, z wyłącznikiem** | odpowiada na prośbę; z „chrome.exe 40 min” wiadomo, co to było; ograniczona ilość danych | tytuły bywają prywatne (temat maila, nazwa dokumentu, strona w trybie prywatnym) i leżą w `history.jsonl` |
| Tylko wybrane procesy (przeglądarki, Eksplorator) | mniej danych | lista procesów do utrzymania; prośba dotyczy „Eksploratora, przeglądarki itp.”, więc i tak szeroko |
| Nie zapisywać (stan z ADR-013) | najprostsza prywatność | wykres nie mówi, co działo się w przeglądarce |

**Decyzja:**

1. **Ciepło ekranu: Oświetlenie nocne (domyślnie) albo gamma ramp; overlay usunięty.**
   - `TintConfig.mode: TintMode` = `nightLight` (domyślnie) albo `gamma`. Harmonogram, siła, przejście, podgląd i domyślne wyłączenie zostają z ADR-014,
     a odcień nadal działa według pory dnia, niezależnie od sesji. Siłę 0–100 zamienia na temperaturę `focus_core::tint::kelvin_for_level`
     (0 = 6500 K, czyli bez zmian; 100 = 1900 K; interpolacja w miredach, żeby suwak zmieniał obraz równomiernie), a dla gammy
     `rgb_gain_for_kelvin` daje mnożniki kanałów (dopasowanie krzywej ciała doskonale czarnego Tannera Hellanda, znormalizowane tak, że 6500 K = `[1, 1, 1]`).
   - **Oświetlenie nocne** (`focus-platform/src/night_light.rs`): FocusMode włącza je i ustawia jego temperaturę według poziomu, a przy poziomie 0 wyłącza.
     Zapis trafia do ustawień bieżącego użytkownika w rejestrze (`HKCU\…\CloudStore\…\windows.data.bluelightreduction.*`, nieudokumentowany format binarny),
     bez praw administratora; do Windows 11 23H2 Windows stosuje zmianę sam. Zapisywane są tylko wartości, których układ FocusMode rozumie w całości.
     **Od Windows 11 24H2 (kompilacja 26100+) zapis w rejestrze nie działa** (silnik czyta magazyn przez wewnętrzne API, a nie z rejestru; sprawdzone
     na kompilacji 26200), więc `NightLight::available()` zwraca tam `false`, nic nie jest zapisywane, a domyślny tryb w praktyce działa przez gammę (niżej).
   - **Gamma** (`focus-platform/src/gamma.rs`): dla każdego aktywnego monitora FocusMode przed pierwszą zmianą odczytuje **bazową rampę** (`GetDeviceGammaRamp`)
     i ustawia bazę × mnożniki (`SetDeviceGammaRamp`), więc kalibracja monitora zostaje (R08). Gdy Windows odrzuci rampę jako zbyt odległą od liniowej,
     FocusMode próbuje coraz słabszej i ustawia `TintStatus.capped` (na maszynie testowej najcieplej ok. 3300 K, czyli siła powyżej ok. 45 jest przycięta).
     Monitor podłączony później dostaje rampę zapamiętaną przy pierwszym użyciu; ona też trafia do pliku przywracania i wraca przy przywracaniu.
     Monitory HDR ignorują rampę (Windows przyjmuje wywołanie, obraz się nie zmienia), więc tam ocieplenie po prostu nie działa.
   - Gdy Oświetlenie nocne jest niedostępne, powłoka (`src-tauri/src/tint.rs`, napisany od nowa) przechodzi na gammę, a powód podaje w `TintStatus.error`;
     `TintStatus.mode` pokazuje mechanizm, który faktycznie działa. Przy `active: true` ten `error` to tylko informacja (UI pokazuje notkę o zastępstwie,
     a nie błąd); niedostępność na danej wersji Windows nie wysyła `app://error`. Oświetlenie nocne jest próbowane ponownie przy następnym ciepłym okresie albo zmianie configu.
     Gamma nie ma zastępstwa (błąd w `TintStatus.error`). Przy zmianie trybu (także przy przejściu na zastępstwo) powłoka najpierw przywraca stan poprzedniego
     mechanizmu, więc oba nigdy nie działają naraz.
   - **Powrót kolorów** (jak osłony, zasada 7): przed pierwszą zmianą FocusMode zapisuje to, co nadpisuje (kopię ustawień Oświetlenia nocnego
     albo bazowe rampy monitorów), w `tint-restore.json` w katalogu danych aplikacji i uzbraja wpis RunOnce `FocusModeRestore` (`--restore-shields` przywraca też kolory).
     Jeden wpis RunOnce obsługuje oba przywracania (`src-tauri/src/safety_net.rs`): istnieje, dopóki czeka którekolwiek, i znika, gdy nie czeka żadne.
     Stan wraca, gdy poziom spada do 0, po wyłączeniu odcienia, przy zmianie trybu i przy wyjściu z aplikacji. Po crashu albo kill przywraca go następny start aplikacji,
     a jeśli nikt jej nie uruchomi, następne logowanie. Plik jest usuwany dopiero po udanym przywróceniu.
     Ustawienia Oświetlenia nocnego wracają takie, jakie były przed pierwszą zmianą.
   - Usunięte: okna `tint-*`, trasa `src/routes/(overlay)/tint` i wpis `tint-*` w `capabilities/overlays.json`. `TintStatus` dostaje `mode` i `capped`,
     a `active` znaczy „ciepło jest teraz zastosowane”.
   - W UI „Ciepłe kolory ekranu” to zmiana wyglądu, bez kalibracji barw i bez obietnic zdrowotnych (zasada 10). Ustawienia uprzedzają,
     że w trybie Oświetlenia nocnego FocusMode steruje tym ustawieniem Windows w swoich godzinach.

2. **Motywy.** `Config.theme: Theme` = `midnight` (obecny wygląd, domyślny), `vaporwave`, `lunar` (fazy księżyca), `autumn` (przytulna jesień), `dark` (prosty ciemny),
   `light` (prosty jasny) albo `pixel` (8-bit). Motyw to zestaw zmiennych CSS (kolory, akcent, tło) i dekoracja tła w `src/lib/themes/`, nakładany atrybutem
   na korzeniu dokumentu w oknie głównym i w okienku `popup`. Tła powstają w kodzie (CSS/SVG); każda dołączona grafika z zewnątrz wymaga licencji i wpisu w `CREDITS.md` (zasada 10).
   Tekst zachowuje kontrast co najmniej WCAG AA także w motywie jasnym, a animacje tła respektują `prefers-reduced-motion`.

3. **Animowana nawigacja.** Podświetlenie aktywnej zakładki (kolor akcentu motywu, w Midnight pomarańczowy) przesuwa się płynnie między Skupieniem,
   Podsumowaniem i Ustawieniami, a warstwa tła przesuwa się o kilka procent w lewo albo w prawo według numeru zakładki. Animujemy tylko `transform` i `opacity`
   (bez przeliczania układu), a przy `prefers-reduced-motion: reduce` nic się nie przesuwa. Czysty frontend (layout `src/routes/(app)/`), bez zmian kontraktu.

4. **Dźwięki dzwonków.**
   - `Config.chimeSound: ChimeSound`: pakiety pasujące do motywów (`midnight`: miękki dzwon, dotychczasowy, domyślny; `vaporwave`: rozstrojony akord syntezatora;
     `lunar`: szklane, kryształowe tony; `autumn`: ciepła drewniana marimba) oraz `classic` (ding-dong), `soft` (łagodny sinus), `chiptune` (8-bitowe arpeggio) i `custom`.
     Każdy pakiet ma wariant przerwy (opadający) i skupienia (rosnący), syntezowany w `focus-audio`, bez plików. Dźwięk wybiera się osobno, więc zmiana motywu go nie zmienia.
   - Własny plik: `import_chime { path, chime }` sprawdza rozszerzenie (mp3, wav, ogg, flac, m4a), rozmiar (najwyżej 10 MB) i to, czy plik się dekoduje,
     kopiuje go do folderu `sounds` w danych aplikacji pod nową nazwą (`custom-<break|focus>-<ms>.<ext>`, więc plik, który wątek audio może jeszcze grać,
     nigdy nie jest nadpisywany), zapisuje nazwę w `Config.customChimes` i wybiera `chimeSound: custom` (`src-tauri/src/chimes.rs`).
     `remove_custom_chime { chime }` usuwa wpis; pliki, których config już nie wskazuje, powłoka usuwa (także przy starcie). W configu są tylko nazwy plików (`is_chime_file_name`: bez ścieżek, `..` i nazw urządzeń Windows),
     więc config nie wskaże pliku spoza folderu `sounds`. Gdy brakuje pliku jednego dzwonka, gra plik drugiego (`CustomChimes::file_for`), a gdy nie ma żadnego,
     pakiet `midnight`. Plik odtwarza rodio na wątku dzwonków, najwyżej 10 s.

5. **Muzyka w zestawie jako domyślne źródło.**
   - `MusicSourceKind::Library`: 12 utworów Open Lo-Fi (btahir, wydanie v1.0.0, **CC0**) w `src-tauri/resources/music/open-lofi/` z opisem `library.json`
     (tytuł, kategoria). Trafiają do instalatora jako zasób (`bundle.resources`, ok. 37 MB). Licencja i pochodzenie: `CREDITS.md`.
   - `list_library_tracks` → `Track[]` z bezwzględnymi ścieżkami. Powłoka dopuszcza w asset protocol dokładnie te pliki (jak przy folderze), a odtwarzacz gra je
     przez `<audio>` i `convertFileSrc`. Działa offline, bez ciasteczek i bez okna YouTube.
   - Nowy config (i config bez zapisanego źródła muzyki) dostaje `library`; zapisany wybór (np. `stream`) zostaje. Stream Lofi Girl (`@LofiGirl/live`) jest dalej jednym kliknięciem.

6. **Przypomnienia na kolejne przerwy.** `Config.breakReminders: string[]`: indeks 0 to pierwsza przerwa sesji, 1 druga itd.; pusty tekst = bez przypomnienia dla tej przerwy.
   Najwyżej 24 pozycje po 160 znaków (więcej → `config.invalid` z polem `breakReminders`); znaki sterujące zamieniane są na spacje.
   `Config.repeatReminders` (domyślnie wyłączone) po ostatniej pozycji zaczyna listę od początku. Tekst wybiera `focus_core::config::reminder_for_break(cfg, n)`,
   gdzie `n` to `cycle` ze snapshotu w chwili startu przerwy (przerwa po 1. cyklu to przerwa 1). Przypomnienie pokazuje okienko faz (`PopupInfo.reminder`)
   i karta przerwy w oknie głównym.

7. **Porady na przerwę ze źródłami.**
   - Dane frontendu w `src/lib/tips.ts` (bez zmian kontraktu): **60 porad na przerwę** w kategoriach oczy, rozciąganie, ruch, picie, oddech, postawa, na zewnątrz,
     ludzie, głowa i porządek, każda oznaczona jako na krótką przerwę, na długą albo na każdą; **10 porad do pracy** (ustawienie monitora, światło i odblaski, siedzenie,
     krótkie i częste przerwy, 20-20-20, zmiana pozycji, poczta o stałych porach, jedno zadanie, telefon poza zasięgiem wzroku, woda pod ręką); przypis do rozciągania
     („w wygodnym zakresie; coś boli, to pomiń”) i **nota prawna tylko po polsku** (kto pracuje przy monitorze co najmniej połowę dnia, ma 5 minut przerwy po każdej
     godzinie, wliczanej do czasu pracy; rozporządzenie w sprawie bhp na stanowiskach wyposażonych w monitory ekranowe, Dz.U. 1998 nr 148 poz. 973).
     Każda porada ma tekst PL i EN oraz źródło.
   - Wybór jest losowy, ale powtarzalny: każda sesja ma własną, przetasowaną (ziarno z `sessionId`) kolejność porad pasujących do długości przerwy
     (krótka < 10 min, długa ≥ 10 min), a kolejne przerwy idą po niej według numeru przerwy. Ten sam widok po odświeżeniu i każde okno pokazują tę samą poradę,
     a porada nie wraca, zanim nie pojawią się wszystkie z puli. Przycisk „Inna propozycja” przesuwa się po tej kolejności. Porada do pracy zmienia się raz dziennie.
   - Główne źródła: HSE (przerwy przy pracy z monitorem), OSHA eTools (stanowisko komputerowe), CIOP-PIB („Ergonomia pracy z komputerem”), AAO (zasada 20-20-20,
     zmęczenie oczu), wytyczne WHO 2020 dotyczące aktywności fizycznej i siedzącego trybu życia, metaanaliza mikroprzerw Albulescu i in. 2022 (PLOS ONE,
     [doi:10.1371/journal.pone.0272460](https://doi.org/10.1371/journal.pone.0272460)), CCOHS (rozciąganie w biurze) i NHS (ćwiczenia na siedząco, oddech, picie wody),
     a do tego badania o odpoczynku w pracy (m.in. Kim i in. 2017, Sianoja i in. 2018, Trougakos i in. 2014, Lee i in. 2015, Kushlev i Dunn 2015, Mark i in. 2008).
     Link do źródła jest przy każdej poradzie w `tips.ts`.
   - **Bez obietnic zdrowotnych** (zasada 10): porady to propozycje czynności („spójrz w dal przez 20 s”), bez „zapobiega”, „leczy” czy „chroni wzrok”.
     Źródło uzasadnia wybór porady, a nie obiecuje efektu.

8. **Historia.**
   - **Powierzchnie powłoki Windows nie trafiają do wykresu.** `focus_core::stats::is_ignored_process` pomija procesy powłoki (m.in. `ShellExperienceHost`,
     `StartMenuExperienceHost`, `SearchHost`, `TextInputHost`, `LockApp`, `ApplicationFrameHost`, `dwm`), a `is_shell_surface(process, class)` rozpoznaje dodatkowo
     po klasie okna pasek zadań, pulpit, przepełnienie zasobnika i widok zadań. `focus-platform` podaje klasę okna na pierwszym planie (`GetClassNameW`),
     a tracker pomija taką sekundę w całości: nie liczy jej ani jako aplikacji, ani jako „Nieznane” (bezczynność liczy się jak dotąd).
     `explorer.exe` liczy się tylko jako prawdziwe okno (np. folder), a nie jako pasek zadań czy pulpit.
   - **Tytuły okien w podpowiedzi wykresu.** `Config.trackTitles` (domyślnie włączone, wyłącznik w Ustawieniach). Gdy jest włączone, tracker przekazuje tytuł okna
     na pierwszym planie do `UsageAccumulator::add_sample`; core czyści go (znaki sterujące → spacje), przycina do 120 znaków i trzyma najwyżej 50 różnych tytułów
     na aplikację (przy przepełnieniu wypada najrzadszy). Podsumowanie (`AppUsage.titles: TitleUsage[]`) ma najwyżej 5 tytułów na aplikację, od najdłużej oglądanego,
     i razem z nim trafia do `history.jsonl`; wykres pokazuje je po najechaniu myszą na słupek. Tytuły zbieramy dla każdej aplikacji (najbardziej przydają się
     dla przeglądarki i Eksploratora). Przy wyłączonym `trackTitles` tytuł nie trafia do core. Wyłączenie nie usuwa tytułów z zapisanej historii; służy do tego „Usuń historię”.
   - **Tytułów nigdy nie logujemy** (`Debug` akumulatora pokazuje tylko liczby). Nie trafiają do mostu, rozszerzenia ani sieci i nie opuszczają komputera.
     Awaryjny nudge po tytule okna przeglądarki działa jak dotąd, w pamięci.
   - **Nie zapisuj historii.** `Config.keepHistory` (domyślnie włączone). Po wyłączeniu podsumowanie z wykresem nadal pokazuje się po Stop, ale nie trafia do `history.jsonl`.
     Wyłączenie nie usuwa wcześniejszej historii.
   - **Usuń historię.** Komenda `clear_history` usuwa plik `history.jsonl` i ostatnie podsumowanie. UI najpierw prosi o potwierdzenie (okno z przyciskiem „Usuń”;
     operacji nie da się cofnąć). Trwającej sesji nie przerywa.

9. **Kontrakt** (`api.rs`; jak w ADR-013 i ADR-014, w tej samej gałęzi co implementacja): komendy `import_chime`, `remove_custom_chime`, `list_library_tracks`
   i `clear_history`; nowe DTO `Theme`, `ChimeSound`, `CustomChimes`, `TintMode` i `TitleUsage`; nowe pola `Config` (`theme`, `chimeSound`, `customChimes`,
   `breakReminders`, `repeatReminders`, `trackTitles`, `keepHistory`), `TintConfig.mode`, `TintStatus.mode` i `capped`, `PopupInfo.reminder`, `AppUsage.titles`;
   wariant `MusicSourceKind::library`. Eventów nie przybywa. Szczegóły: ARCHITECTURE §6.7.

**Konsekwencje:**
- Ciepło ekranu zmienia teraz stan systemu (ustawienie Oświetlenia nocnego albo rampę gamma) i nie znika samo z procesem. Po kill kolory zostają ciepłe
  do następnego startu FocusMode albo najpóźniej do następnego logowania (RunOnce). Oświetlenie nocne można też od razu wyłączyć w szybkich ustawieniach Windows.
- Oświetlenie nocne opiera się na nieudokumentowanym formacie ustawień w rejestrze i może przestać działać po aktualizacji Windows. Wtedy powłoka przechodzi na gammę
  albo zgłasza `TintStatus.error` (+ `app://error`), a aplikacja działa dalej. W godzinach harmonogramu FocusMode nadpisuje ręczne zmiany Oświetlenia nocnego.
- Gamma nie działa na monitorach HDR, a bardzo ciepłych wartości Windows nie przyjmuje (`capped`). Gry i aplikacje, które same ustawiają rampę, mogą ją nadpisać.
- Oba mechanizmy działają po kompozycji: obraz nie traci kontrastu, kursor też robi się cieplejszy, a zrzuty ekranu i nagrania mają normalne kolory.
- Tytuły okien mogą zawierać dane prywatne (temat wiadomości, nazwa dokumentu, tytuł strony z trybu prywatnego). Leżą na dysku w `history.jsonl`
  (profil użytkownika Windows, bez szyfrowania), dopóki użytkownik nie usunie historii. Dlatego wyłącznik tytułów, „Nie zapisuj historii” i „Usuń historię”
  są obok siebie w Ustawieniach, a README to opisuje.
- Pominięte sekundy powłoki nie wchodzą do `trackedSecs`, więc suma na wykresie może być mniejsza od czasu skupienia.
- Instalator rośnie o ok. 37 MB (muzyka). Nowych crate'ów i pakietów npm nie ma; w `focus-platform` dochodzą feature'y `windows`: `Win32_UI_ColorSystem` i `Win32_Graphics_Gdi`.
- Nieznana wartość enuma w `config.json` (np. motyw z nowszej wersji po powrocie do starszej) nadal unieważnia cały config (`config.corrupt`, jak w ADR-014).

**Co zmienia:**
- ADR-009: zastąpiona. Overlay odpada; gamma ramp wraca jako tryb do wyboru, z kopią bazowej rampy każdego monitora (R08) i powrotem kolorów jak przy osłonach.
- ADR-012: okna overlay odcienia znikają; `TintStatus` z rewizją zostaje (czyta go tylko okno główne).
- ADR-013: pkt 2 (domyślne źródło `library`), pkt 3 (dzwonek z pliku użytkownika), pkt 5 (pomijanie powłoki Windows, tytuły okien przy `trackTitles`,
  `keepHistory`, `clear_history`).
- ADR-014: pkt 2 (odcień przez Oświetlenie nocne albo gammę zamiast overlayu) i pkt 6 (grupa tras `(overlay)` tylko dla okna `popup`).
- CLAUDE.md (zmiana czeka na zatwierdzenie przez użytkownika, notatka w TASKS z 2026-10-13):
  - twarda zasada 6: „Bez pełnoekranowych okien przerwy i trybu spotkania — odłożone (ADR-013). **Odcień ekranu tylko przez Oświetlenie nocne Windows albo
    gamma ramp, nigdy przez overlay**, domyślnie wyłączony; stan sprzed zmiany zapisany od razu i przywracany jak osłony (zasada 7; ADR-015)”;
  - twarda zasada 11, zdanie o trackerze: „Tracker zapisuje nazwy procesów i sumy czasu, a tytuły okien **tylko przy włączonym `trackTitles`**: przycięte,
    tylko lokalnie w `history.jsonl` (przy wyłączonym `keepHistory` nic), **nigdy w logach**, moście, rozszerzeniu ani w sieci (ADR-015). Powierzchnie powłoki Windows pomijamy.”;
  - Struktura: `focus-platform` (`night_light`, `gamma`, klasa okna w `foreground`), powłoka (`tint.rs`: Oświetlenie nocne / gamma i plik przywracania),
    `src/lib/themes/`, `src/lib/tips.ts`, `src-tauri/resources/music/`.
