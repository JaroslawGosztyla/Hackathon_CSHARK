# FocusMode — cross-check założeń, 2026-09-27

Przejrzano w całości `DESIGN.md`, `ARCHITECTURE.md`, `DECISIONS.md`, `TASKS.md` i instrukcję `CROSS_AI_REVIEW.md`. Fakty techniczne sprawdzono w dokumentacji producentów i bibliotek. Numery linii odnoszą się do plików zastanych w dniu review.

**Wniosek: plan wymaga korekt przed implementacją.** Tauri + Svelte + Rust core można zachować. Największe problemy dotyczą gwarancji tint/overlay, semantyki Teams, czasu sesji i inicjalizacji nowych okien. Nie ma podstaw, żeby traktować opisane zachowanie jako zapewnione przez sam wybór bibliotek.

To review dokumentacji, nie działającej aplikacji: w udostępnionym folderze są tylko te pięć plików, bez kodu, manifestów i lockfile'ów. Nie wykonano builda ani testów sprzętowych. Autorstwa modelu nie da się ustalić z tych plików; nagłówki wspominają Claude, podczas gdy pytanie użytkownika wskazuje ChatGPT. Ocena dotyczy treści, niezależnie od pochodzenia.

Oznaczenia: **potwierdzone** — wynika ze źródeł lub sprzeczności w dokumentach; **ryzyko projektu** — możliwy błąd wymagający doprecyzowania kontraktu i testu; **rekomendacja** — proponowana decyzja. P1: rozstrzygnąć przed implementacją danej funkcji/demo. P2: uzupełnić plan i kryteria odbioru. Propozycje w tym raporcie nie są przyjętymi ADR-ami.

## 1. Pięć ryzyk, które mogą zepsuć demo

Oceny prawdopodobieństwa są jakościową oceną recenzenta, a nie wynikiem pomiarów.

| Ryzyko | Prawdopodobieństwo | Co może się wydarzyć | Konkretna mitigacja |
|---|---|---|---|
| Tint zależny od sterownika i trybu obrazu | wysokie bez testu sprzętu | Suwak działa w UI, obraz nie zmienia się albo po awarii zostaje zmieniony | Wyłączyć gamma z bazowego demo; opcjonalny overlay dopuścić po próbie na docelowym Windows; zawsze dostępne wyłączenie funkcji |
| Nowe okno przerwy nie otrzyma stanu | średnie | Pełnoekranowe puste okno, brak ćwiczenia lub działającego wyjścia | Snapshot zawierający aktualny plan, inicjalizacja po gotowości UI, identyfikator przerwy, awaryjne zamknięcie |
| Różne definicje timera i trybu demo | wysokie przy implementacji dosłownej | Przerwa startuje w innym czasie niż zapowiedź; odroczenie wydłuża pokaz o 2 minuty | Jedna tabela czasów i przejść; deterministyczny scenariusz bez losowania |
| Teams jako efekt widoczny natychmiast | wysokie bez wcześniejszej próby | Brak zgody/logowania, opóźniony status, błędna detekcja rozmowy | Próba na realnym tenancie w M0, wyraźnie oznaczona symulacja jako wariant demo, brak zależności głównego cyklu od Graph |
| Pierwszy instalator dopiero w M4 | średnie–wysokie | Dev działa, paczka nie znajduje muzyki lub powiadomienia zachowują się inaczej | Zbudować i zainstalować pierwszy Windows release w M1; test bez katalogu źródeł |

## 2. Ustalenia techniczne i niespójności

### F01 — P1, potwierdzone: gamma ramp nie jest pewną podstawą tint

**Miejsca:** ARCHITECTURE §7, linia 244; §9.1, linie 279–286; DECISIONS ADR-004, linie 59–64; TASKS A-07.

Microsoft odradza `SetDeviceGammaRamp`. Funkcja może zwrócić sukces bez zastosowania rampy; zachowanie w HDR jest niezdefiniowane, a inne aplikacje lub system mogą nadpisać ustawienie. Dokumentacja wspomina też czas wywołania do 200 ms. Stwierdzenie, że gamma jest zawsze szybka, jest zbyt mocne. Nie ma udokumentowanej gwarancji, że granica 3400 K rozwiązuje te problemy. [Microsoft: SetDeviceGammaRamp](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-setdevicegammaramp).

**Zmiana:** gamma jako eksperyment na sprawdzonym sprzęcie SDR, poza krytyczną ścieżką demo. Overlay to osobny efekt wizualny, nie odpowiednik kalibracji temperatury ekranu. Dopuszczać również stan „tint niedostępny”.

**Weryfikacja:** HDR on/off, Night Light on/off, podłączenie monitora, podgląd → anulowanie, wyłączenie tint. Sprawdzić widoczny rezultat, nie tylko kod sukcesu API.

### F02 — P1, potwierdzone: Drop i panic hook nie gwarantują sprzątania po kill

**Miejsca:** DESIGN §6.4, linia 92; ARCHITECTURE §9.1, linie 286–287; TASKS A-07 i P-4.

Produkt obiecuje przywrócenie kolorów przy crashu, a taski dopuszczają dopiero następny start. To różne kryteria. Wymuszone zakończenie procesu zatrzymuje jego wątki; kod `Drop`, hook paniki ani handler wyjścia nie zapewniają wtedy wykonania. [Microsoft: TerminateProcess](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-terminateprocess).

**Zmiana:** opisać osobno zwykłe wyjście, Rust panic i wymuszone zabicie. Jeśli gamma zostaje, potrzeba projektu odzyskiwania stanu per monitor oraz rozstrzygnięcia konfliktu z późniejszymi zmianami systemowymi. Zapis starej rampy i restart nie są gwarancją natychmiastowego odzyskania. Watchdog zwiększa złożoność i również nie daje gwarancji absolutnej. Na hackathonie prostsze jest wyłączenie globalnej gammy.

**Weryfikacja:** osobne próby Stop/Exit, paniki i wymuszonego zakończenia; nie uznawać jednej za dowód pozostałych.

### F03 — P1, potwierdzone: „fallback działa zawsze na trzech OS-ach” jest błędne

**Miejsca:** ARCHITECTURE §1, linie 17–19; §9.1, linie 282–283; DESIGN Must, linie 43–45; DECISIONS ADR-001/004.

Warstwa okien Tao opisuje `set_always_on_top` jako niewspierane na Waylandzie; na X11 jest to sugestia dla window managera. Tooltip tray w Tauri jest niewspierany na Linux. Samo udane zbudowanie aplikacji nie potwierdza więc wymagań UX. [Tao: always on top](https://docs.rs/tao/latest/tao/window/struct.Window.html#method.set_always_on_top), [Tauri: tray tooltip](https://docs.rs/tauri/latest/tauri/tray/struct.TrayIconBuilder.html#method.tooltip).

Uwaga o prywatnym API przezroczystości macOS już jest w ARCHITECTURE §10 i jest zasadna; trzeba przenieść jej konsekwencje do deklarowanego zakresu wsparcia. [Tauri: window options](https://v2.tauri.app/reference/javascript/api/namespacewindow/).

**Zmiana:** Windows jako platforma odbioru hackathonu; pozostałe systemy z jawną tabelą ograniczeń. Wayland: brak globalnego tint, zwykłe przypomnienie jako degradacja. Test dwóch monitorów, różnych DPI, monitora po lewej stronie, odłączenia monitora i aplikacji fullscreen. Nie obiecywać zakrycia wszystkich powierzchni systemowych.

### F04 — P1, potwierdzone: dokumenty opisują różne osie czasu

**Miejsca:** DESIGN §6.3, linia 82 i §9, linia 126; ARCHITECTURE §5, linie 151–175; TASKS B-05.

- DESIGN: ostrzeżenie w T−30 s. Maszyna stanów: dopiero po końcu Focus, a następnie dodatkowe 30 s. Wychodzi 40:00 albo 40:30 do przerwy.
- ARCHITECTURE: w demo ostrzeżenie 5 s. DESIGN demo i B-05: tekst 30 s.
- Jeden mnożnik `time_scale` nie daje jednocześnie 2400→40 s, 300→20 s i 30→5 s: to odpowiednio skale 1/60, 1/15 i 1/6.
- Odroczenie 120 s nie ma wariantu demo. Nie mieści się w pokazie, jeśli pozostanie nieskalowane.
- Obowiązkowy `stand-walk` trwa 60 s, a cała przerwa demo 20 s. Plan ćwiczeń też potrzebuje wariantu demo.

**Zmiana:** jawny profil czasów, nie jeden mnożnik. Proponowane wartości:

| Parametr | Normalny | Demo |
|---|---:|---:|
| Focus do rozpoczęcia przerwy | 2400 s | 40 s |
| Ostrzeżenie przed końcem focus | 30 s | 5 s |
| Przerwa | 300 s | 20 s |
| Jednorazowe odroczenie | 120 s | 10 s |

Ostrzeżenie zawiera się w focus. Pierwszy pokaz bez odroczenia: 0 start, 35 ostrzeżenie, 40 przerwa, 60 koniec. Osobno pokazać odroczenie. Demo oznaczyć jako skróconą prezentację; nie przedstawiać przyspieszonych ćwiczeń jako zaleceń ruchowych. Ustalić pozostały czas przerwy po 2–4 ćwiczeniach i `min_secs_before_finish`.

**Weryfikacja:** testy granic 34/35/39/40/60 s, odroczenia i sumy czasu planu. Te same wartości mają zasilać UI oraz testy.

### F05 — P1, ryzyko projektu: nowe okna mogą zgubić jedyny BreakPlan

**Miejsca:** ARCHITECTURE §6, linie 207–225 i §10, linie 308–309.

Okna powstają dynamicznie, a `BreakPlan` jest przekazywany eventem `break://show`. Snapshot nie zawiera tego planu. Jeśli emisja wyprzedzi rejestrację listenera w nowym WebView, dokument nie przewiduje odzyskania planu. To luka kontraktu, nie potwierdzony błąd istniejącego kodu. Tauri rozdziela komendy i eventy; samo `emit` nie opisuje tu protokołu inicjalizacji. [Tauri: komunikacja z frontendem](https://v2.tauri.app/develop/calling-frontend/).

**Zmiana:** `get_snapshot` musi umożliwić odtworzenie całego bieżącego widoku: aktywny plan, identyfikator przerwy, postęp i rewizja stanu. Listener instalowany przed pobraniem snapshotu; rewizja chroni przed nadpisaniem nowszego stanu starszą odpowiedzią. Okno pokazywać po gotowości widoku albo zapewnić od razu awaryjne wyjście. Komendy z wielu monitorów mają być idempotentne i odnosić się do konkretnej przerwy.

**Weryfikacja:** opóźnić start WebView o 2 s, odświeżyć widok w połowie przerwy, nacisnąć Stop/Pomiń z dwóch okien. Bez pustego overlay i bez podwójnego zapisu statystyk.

### F06 — P1, potwierdzone ograniczenia + wniosek: DND koliduje z wykrywaniem rozmowy

**Miejsca:** ARCHITECTURE §9.2, linie 291–297; TASKS S-01/S-02; DESIGN demo, linia 125.

Graph daje pierwszeństwo statusowi preferowanemu użytkownika przed stanami sesji. Dokumentuje też, że widoczna zmiana statusu w kliencie Teams może zająć kilka minut. **Wniosek dla tego projektu:** po ustawieniu własnego `DoNotDisturb/DoNotDisturb` nie można traktować zagregowanego `/me/presence` jako pewnego czujnika trwającej rozmowy. [Microsoft: zarządzanie presence](https://learn.microsoft.com/en-us/graph/manage-presence-state).

`clearUserPreferredPresence` usuwa preferowany status użytkownika; nie jest operacją cofnięcia wyłącznie „naszej” sesji ani przywrócenia wcześniejszego statusu. Bezwarunkowe czyszczenie może naruszyć intencję użytkownika. [Microsoft: clearUserPreferredPresence](https://learn.microsoft.com/en-us/graph/api/presence-clearuserpreferredpresence?view=graph-rest-1.0).

**Zmiana:** w MVP ręczne „jestem na spotkaniu” z ograniczonym odroczeniem. Integracja Teams wymaga własnej polityki odpowiedzialności za status oraz prób Pause/Resume/Postpone/Stop i ręcznej zmiany w Teams. Nie gwarantować natychmiastowej zmiany ikonki podczas 40-sekundowego demo.

`Presence.ReadWrite`, para DND/DND i jawne wygaśnięcie są poprawnym kierunkiem. Preferred presence wymaga jednak aktywnej sesji presence, np. zalogowanego klienta Teams. Dokumentacja operacji podaje `/users/{id}/presence/setUserPreferredPresence`; wariant `/me/...` z projektu trzeba sprawdzić w próbie integracyjnej, a nie uznawać bez testu za błędny lub poprawny. [Microsoft: setUserPreferredPresence](https://learn.microsoft.com/en-us/graph/api/presence-setuserpreferredpresence?view=graph-rest-1.0).

Nie zakładać automatycznej dostępności darmowego tenantu: sandbox M365 wymaga kwalifikacji. Tenant i device code flow sprawdzić w M0. [Microsoft: Developer Program](https://learn.microsoft.com/office/developer-program/microsoft-365-developer-program).

### F07 — P1, potwierdzona luka: odstęp między tickami nie dowodzi uśpienia

**Miejsca:** ARCHITECTURE §5, linie 170–175; TASKS A-01/A-09.

Przestój >60 s może oznaczać uśpienie, zatrzymanie w debuggerze lub problem wykonania. Nie dowodzi wykonania ćwiczeń. `Timestamp` nie określa rodzaju zegara: czas systemowy może się przestawić, a Rust nie gwarantuje jednolitego wliczania suspend do `Instant` na wszystkich platformach. [Rust: Instant](https://doc.rust-lang.org/std/time/struct.Instant.html), [Rust: SystemTime](https://doc.rust-lang.org/std/time/struct.SystemTime.html).

**Zmiana:** czas monotoniczny dla faz, czas kalendarzowy dla dziennych statystyk i harmonogramu, jawna polityka suspend/resume. Po niepewnej luce zatrzymać sesję i czekać na wznowienie; nie zaliczać automatycznie ćwiczeń ani czasu bezczynności jako focus. Regułę >60 s można zachować jako heurystykę przerwania, z taką nazwą.

**Weryfikacja:** uśpienie w każdej fazie, przestawienie zegara, przejście przez północ, długa pauza. Minuty focus wynikają z aktywnych odcinków, nie z samego czasu od Start do Stop.

### F08 — P2, potwierdzone: uzasadnienie rodio zależy od nieokreślonej wersji

**Miejsca:** DECISIONS ADR-003, linia 48; ARCHITECTURE §7, linie 237–243 oraz linia 378.

Zapis „OutputStream NIE jest Send” nie może być traktowany jako ponadczasowa właściwość rodio. Sprawdzona dokumentacja `latest` wskazuje rodio 0.22.2, z `MixerDeviceSink`, `DeviceSinkBuilder` i `Player`. `MixerDeviceSink` ma w dokumentacji dla prezentowanego targetu implementacje `Send` i `Sync`. [rodio: stream](https://docs.rs/rodio/latest/rodio/stream/index.html), [rodio: MixerDeviceSink](https://docs.rs/rodio/latest/rodio/stream/struct.MixerDeviceSink.html).

**Zmiana:** po krótkiej próbie przypiąć wersję i targety, dostosować nazwy API, potwierdzić cechy typów kompilacją. Dedykowany wątek można zachować jako wybór organizacyjny i sposób zarządzania życiem urządzenia. Nie uzasadniać go niezweryfikowanym zakazem `Send`. Fade powinien łączyć głośność użytkownika z osobnym współczynnikiem zanikania; po Postpone samo Resume nie może zostawić głośności równej zero.

**Weryfikacja:** odłączenie wyjścia audio, brak urządzenia, wyciszenie → odroczenie, Stop podczas fade. Wątek audio nie może blokować przyjmowania Stop na czas całej animacji.

### F09 — P2, potwierdzone: derive TS nie wystarcza do eksportu kontraktu

**Miejsca:** ARCHITECTURE §6, linie 179–201; TASKS T0-3/T0-4/T0-6.

Samo `#[derive(TS)]` nie sprawia, że `cargo test` wyeksportuje typy. Potrzebne jest `#[ts(export)]` albo jawny test/eksporter wywołujący API eksportu. `TS_RS_EXPORT_DIR` ustala miejsce, a nie uruchamia eksport. [ts-rs: eksport](https://docs.rs/ts-rs/latest/ts_rs/trait.TS.html).

**Zmiana:** dodać eksport do kontraktu, jednoznacznie ustawić ścieżkę względem workspace oraz sprawdzać w CI, czy regeneracja nie zmienia commitowanych bindings. Typy DTO nie sprawdzają automatycznie nazw `invoke`, argumentów komend i nazw eventów. Potrzeba małego testu integracyjnego `api.ts` ↔ Rust. Rozdzielić w tabeli `get_config()` i `update_config(config)`.

Skill `/sync-contract`, `CLAUDE.md` i `.claude/` nie występują w przekazanym folderze. Mogą istnieć w innym repo, ale tutaj nie można oprzeć na nich wykonalnego kroku procesu. Opisać zwykłą komendę regeneracji i review diffu; skill może ją później automatyzować.

### F10 — P2, potwierdzone: „core bez I/O” kłóci się z zadaniami zapisu

**Miejsca:** DECISIONS ADR-002, linie 35–36; ARCHITECTURE §11, linie 319–324; TASKS A-02/A-09; DESIGN §7.

ADR zabrania I/O w core, ale task A-02 umieszcza load/save w `focus-core::config`, a opis przechowywania przekazuje ścieżki do core. Samo dostarczenie ścieżki z zewnątrz nie czyni zapisu czystą logiką. Dodatkowo katalog ćwiczeń jest w frontendzie, chociaż Rust ma zwracać `BreakPlan`; nie określono, skąd Rust zna dostępne ćwiczenia i ich czasy.

**Zmiana:** core odpowiada za walidację, obliczenia i serializowalne struktury; shell za odczyt/zapis. Jeden wspólny katalog ćwiczeń dostarczany do planera. Losowanie z jawnym seedem lub gotowy plan jako wejście; w demo stały plan. To zachowuje deterministyczne testy bez dodatkowego frameworka.

**Weryfikacja:** core można testować bez filesystemu; brak pliku, uszkodzony config i błąd zapisu obsługiwane w warstwie persystencji; każde ID z planu występuje w katalogu UI.

### F11 — P1, luka kontraktu: wyjście awaryjne i zakończenie sesji są niedookreślone

**Miejsca:** DESIGN §6.4, linia 91; ARCHITECTURE §5, linie 145–168; TASKS B-02/B-03/P-4.

Diagram nie pokazuje Stop z BreakPending/Break, choć tabela sugeruje Stop z każdej fazy. `FinishBreak` nie ma przejścia; nie opisano efektów pauzy dla audio/DND. Overlay może przykryć tray, a hold-to-skip zależy od działającego frontendu. „Nigdy nie więzimy” nie jest jeszcze pokryte kryterium odbioru. Zapis statystyk po przerwie również wymaga rozróżnienia zakończenia, pominięcia i przerwania.

**Zmiana:** jawna tabela wszystkich komend w każdej fazie; Stop idempotentny i dostępny zawsze, także klawiaturą. Po przekroczeniu limitu czasu gotowości WebView powłoka zamyka niedziałający overlay. Zakończenie przerwy zapisuje dokładnie jeden rezultat z ID sesji. Wyjaśnić, czy kliknięcie „Zrobione” jest deklaracją użytkownika, czy tylko końcem odliczania.

Operacje Graph i wolne I/O nie mogą zatrzymywać obsługi Stop w aktorze. Jeśli efekty są wykonywane przez `await` w jego pętli, potrzebne są odrębne zadania z timeoutem i odpowiedzią oznaczoną ID sesji. Spóźniona odpowiedź integracji nie może ponownie włączyć efektu po Stop.

**Weryfikacja:** Stop w każdej fazie, dwukrotny Stop, Escape/klawiatura, niedziałający frontend, brak sieci, odpowiedź Graph po zatrzymaniu sesji, równoległe zakończenie z dwóch monitorów.

### F12 — P2, potwierdzone: powiadomienia i instalator trzeba sprawdzić wcześniej

**Miejsca:** DESIGN §6.3, linia 82; TASKS B-05 oraz P-1/P-4.

Tauri opisuje ograniczenia powiadomień Windows związane z instalacją; tryb developerski może pokazywać nazwę i ikonę PowerShell. API akcji powiadomień w tym pluginie jest dostępne tylko na platformach mobilnych. Nie jest jasne, czy dokument wymaga przycisków także w toastach, czy tylko w banerze aplikacji. [Tauri: notifications](https://v2.tauri.app/plugin/notification/).

**Zmiana:** tekstowy toast traktować pomocniczo; przyciski zapewnić we własnym UI dostępnym również przy ukrytym oknie głównym. Pierwszy zainstalowany build w M1, z zasobami muzyki i przetestowanym sposobem uruchomienia demo. Nie zakładać, że GUI uruchomione ze skrótu odziedziczy zmienną ustawioną wcześniej w innym terminalu.

**Weryfikacja:** uruchomić zainstalowaną paczkę poza repo, z wyłączoną siecią i ukrytym main window; odrzucić uprawnienie powiadomień; przejść pełen cykl.

### F13 — P2, potwierdzone: capabilities nie wystarczą bez konfiguracji własnych komend

**Miejsca:** ARCHITECTURE §10, linie 311–312.

Dokument obiecuje minimalny dostęp dla `break-*` i `tint-*`. Tauri wyjaśnia jednak, że własne komendy zarejestrowane przez `invoke_handler` są domyślnie dostępne wszystkim oknom/WebView aplikacji. Do ograniczania ich przez ACL trzeba skonfigurować także manifest komend aplikacji i uprawnienia. [Tauri: capabilities](https://tauri.app/security/capabilities/).

**Zmiana:** rozpisać komendy dozwolone dla każdego typu okna; jeśli izolacja jest wymaganiem, włączyć ją także dla custom commands. Dane logowania i tokeny pozostają w backendzie. Nie utożsamiać uprawnień pluginów z zabezpieczeniem wszystkich własnych komend.

**Weryfikacja:** próba nieuprawnionego wywołania z okna tint jest odrzucana przez backend, a potrzebne komendy okna break działają.

### F14 — P2, potwierdzone: zakres produktu, milestone'y i właściciele nie są spójni

**Miejsca:** DESIGN pitch i §5; TASKS Role, M1/M2; ARCHITECTURE tabela komend §6.

- Teams jest w głównej obietnicy produktu, choć w zakresie ma status Could. Tint również jest w pitchu, choć jest Should.
- Muzyka z plików CC0 jest Must, ale pozyskanie i bundlowanie w B-11 trafia dopiero do M2.
- Ćwiczenia z animacją są Must i w B-03/M1, ale opis bramki M2 ponownie sugeruje dopiero ich dostarczenie.
- `commands.rs` ma właściciela B, tabela IPC przypisuje większość implementacji A, a A-05 mówi o podmianie stubów B.
- Nie ma potwierdzonej liczby godzin pracy. Dwa tygodnie kalendarzowe nie są estymacją dostępności dwóch osób.

**Zmiana:** pitch ma opisywać zatwierdzone MVP. Wyraźnie wybrać: MVP z szumem, czy także z muzyką; zasoby potrzebne do M1 muszą trafić do M1. B posiada cienkie adaptery komend, A logikę i aktora. Wspólny kontrakt obejmuje też `show_break/hide_break`, błędy i gotowość okien. Teams po M2 nie powinien być warunkiem udanego pokazu.

### F15 — P2, ryzyko procesu: „latest + build” nie wystarcza do odtwarzalności

**Miejsca:** ARCHITECTURE linia 378; TASKS linia 108.

Instalacja aktualnych wersji na początku projektu jest rozsądna, ale późniejsze buildy muszą korzystać z ustalonego zestawu. Instrukcja „weź lockfile z main, odpal build” po konflikcie jest niepełna: lockfile może nie uwzględniać scalonych manifestów, a sam build frontendu nie jest procedurą jego aktualizacji.

**Zmiana:** najpierw scalić manifesty, potem uzgodnić/regenerować lockfile właściwym managerem pakietów i przejrzeć diff. CI na czystym checkout: `npm ci`, Rust z `--locked`, regeneracja bindings bez różnic, check/test/build. Ustalić wersje Rust i Node na czas hackathonu. To propozycja procesu; w folderze nie ma jeszcze manifestów, które można zweryfikować.

### F16 — P2, nieuzasadniona obietnica produktu: tint nie dowodzi ochrony oczu

**Miejsca:** DESIGN linie 8–22 i §7, linia 105.

Dokument łączy ciepłą barwę z dbaniem o oczy i mierzy benefit minutami timera. AAO wskazuje, że dyskomfort cyfrowy nie wynika z samego niebieskiego światła. Nie jest to badanie skuteczności FocusMode; nie uzasadnia ani obietnicy ochrony oczu, ani wniosku, że każdy tint jest bezwartościowy. [AAO: digital devices and your eyes](https://aao.org/eye-health/tips-prevention/blue-light-digital-eye-strain).

**Zmiana:** tint opisać jako preferencję wizualną; statystyki jako czas sesji i odnotowane przerwy, nie dowód lepszej produktywności czy zdrowia. Ćwiczenie patrzenia w dal wykonywane dopiero po 40 minutach nie realizuje rytmu 20-20-20, który zakłada przerwę co 20 minut. Albo zmienić nazwę na neutralną, albo dodać opcjonalną mikroprzerwę w połowie sesji. [AAO: zasada 20-20-20](https://aao.org/eye-health/tips-prevention/should-you-be-worried-about-blue-light).

## 3. Zakres na dwie osoby i dwa tygodnie

**Rekomendacja:** jeden dopracowany cykl na Windows, obsługa dwóch sprawdzonych monitorów, Tauri/Svelte, Rust core, jeden generowany szum i ewentualnie jeden zweryfikowany utwór, 3 proste karty przerwy, tray, niezawodny Stop i jawny profil demo. Zachować zwykłe ustawienia czasu i głośności. Ocena wykonalności pozostaje warunkowa do poznania dostępnych godzin i doświadczenia zespołu.

| Decyzja | Elementy |
|---|---|
| Dostarczyć w M1 | Pełen cykl, bootstrap overlay, wyjście awaryjne, audio, pierwszy instalator, test offline |
| Dodać po stabilnym M1 | Fade/gong, spójny motyw, prosta statystyka dzienna, przygotowane treści |
| Dopuszczać po próbie sprzętowej | Opcjonalny tint przez overlay na docelowym Windows |
| Przesunąć poza bazowy pokaz | Globalna gamma, Graph + wykrywanie rozmów, radio HTTP, geolokalizacja, autostart, wieloplatformowe wymagania UX, streak i rozbudowane statystyki |

Duży efekt małym kosztem daje zsynchronizowana zapowiedź, fade i czytelna karta przerwy. Nowe integracje OS zwiększają liczbę trybów awarii i nie są potrzebne do pokazania głównej pętli.

## 4. Alternatywy i ich koszt

| Obszar | Propozycja | Trade-off |
|---|---|---|
| Framework | Zachować Tauri + Svelte | Zmiana frameworka przed próbą okien nie rozwiązuje problemów specyficznych dla OS. Oceny „ładniejszy/szybszy” z ADR są opiniami zależnymi od kompetencji zespołu |
| UI w całości w Ruście | Rozważać egui/eframe tylko, jeśli zespół faktycznie nie zna frontendu webowego | Core można zachować, lecz widoki i integracja okien wymagają nowej pracy; nie traktować tego jako taniego awaryjnego przełączenia pod koniec |
| Audio | Rodio w jednej przypiętej wersji | Wymaga obsługi błędów urządzenia, ale daje wspólną kontrolę poza widokiem |
| Audio plan B | Element audio w trwałym WebView | Trzeba sprawdzić odtwarzanie po ukryciu okna, uśpieniu i odłączeniu urządzenia; samo ukryte okno nie dowodzi niezawodności |
| Tint | Brak globalnego tint albo opcjonalny overlay na sprawdzonym Windows | Mniejsza obietnica funkcjonalna, prostsze zakończenie działania; overlay nadal wymaga testów jakości i pozycjonowania |
| Teams | Ręczny tryb spotkania w MVP | Działa bez dostępu do tenantu, ale wymaga działania użytkownika |

`setPresence` to inny model: sesja presence należąca do aplikacji. Nie jest prostym zamiennikiem `setUserPreferredPresence`: ma inne dozwolone pary stanów i czas wygaśnięcia 5–240 minut. Jego dokumentowana para DND to `DoNotDisturb/Presenting`, więc nie należy jej bezrefleksyjnie używać do zwykłego focus. Ewentualna zmiana wymaga osobnego spike'a i ADR. [Microsoft: setPresence](https://learn.microsoft.com/en-us/graph/api/presence-setpresence?view=graph-rest-1.0).

## 5. Podział A/B i warunki pracy równoległej

Podział jest możliwy, ale sam plik z DTO nie usuwa zależności. A ma cięższe ryzyka techniczne (czas, audio, tint, persystencja); B ma dużo ekranów, okna, instalację pluginów i dodatkowo Teams. Obciążenie trzeba oszacować w godzinach, nie liczbą checkboxów.

| Właściciel | Odpowiedzialność po korekcie |
|---|---|
| A | Core, jawny model czasu, kontrakt aktora, audio, persystencja w shell, statystyki i testy domenowe |
| B | Okna, UI, cienkie `commands.rs`, bootstrap widoków, tray, uprawnienia i obsługa awaryjnego wyjścia |
| Razem na M0 | DTO + przykłady JSON, enumy i błędy, identyfikatory/rewizje, komendy i eventy, interfejs okien, tabela czasów, target demo |
| Wspólny odbiór M1 | Zainstalowany build i pełen cykl; oba konta developerskie potrafią go uruchomić |

Stuby powinny używać tych samych DTO i scenariuszy co backend. Na M0 wystarczy kontrakt MVP; projektowanie wszystkich przyszłych komend Teams/tint/stats w 30 minut nie jest wiarygodną bramką jakości.

## 6. Trzy pomysły produktowe do wyboru

1. **Przerwa bez patrzenia na ekran:** krótka karta instrukcji, następnie spokojne tło i gong końca. Lepsza zgodność z obietnicą odejścia od monitora niż ciągłe śledzenie animacji.
2. **Widoczny ręczny tryb spotkania:** odroczenie z podanym czasem następnej próby. Ma czytelny efekt podczas demo i nie zależy od Graph.
3. **Jedno zdanie podsumowania sesji:** „32 min aktywnej sesji, przerwa pominięta” albo „przerwa zakończona”. Bez utożsamiania odliczania z potwierdzonym wykonaniem ćwiczeń; dane demo wyraźnie oznaczone i oddzielone od prawdziwych statystyk.

To propozycje wyboru, nie kolejne trzy obowiązkowe funkcje.

## 7. Kolejność korekt dokumentów

| Plik | Co zmienić |
|---|---|
| DESIGN | Pitch zgodny z MVP; ograniczenia platform; jedna oś czasu; realistyczna obietnica tint i odzyskiwania po awarii; sposób wyjścia; neutralne obietnice zdrowotne |
| ARCHITECTURE | Snapshot odtwarzający przerwę; gotowość okien; pełna tabela przejść; dwa rodzaje czasu; persystencja poza core; wersja audio; eksport TS; izolacja własnych komend; wolne efekty poza pętlą aktora |
| DECISIONS | Po decyzji zespołu uzupełnić/supersedować ADR-003/004/005; dopisać decyzję o platformie demo i semantyce czasu; nie oznaczać propozycji jako przyjętych automatycznie |
| TASKS | Wcześniejszy spike okien i instalator; zasoby audio wymagane w M1; rozstrzygnięcie właściciela komend; kryteria awarii i wznowienia; ograniczony M2; dokładna procedura lockfile/bindings |

Najpierw rozstrzygnąć F01–F07 i F11, bo zmieniają zachowanie oraz kontrakty. Następnie uzupełnić pozostałe punkty przed rozpoczęciem odpowiadających im tasków. Fakty z dokumentacji nie zastępują prób na sprzęcie: overlay, audio, tenant Graph i odzyskiwanie po awarii pozostają do empirycznej weryfikacji.
