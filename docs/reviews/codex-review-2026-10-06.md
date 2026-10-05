# FocusMode — review zmian dokumentacji, 2026-10-06

## Zakres i wynik

Przejrzano aktualne `DESIGN.md`, `ARCHITECTURE.md`, `DECISIONS.md`, `TASKS.md`, `CROSS_AI_REVIEW.md`, `README.md` i `CLAUDE.md`. Punktem odniesienia jest poprzedni raport z 2026-09-27 oraz treść dokumentów odczytana podczas tamtego review. Szczególną uwagę poświęcono nowym opisom protokołu okien, tabeli stanów i spike'a audio z 2026-10-05.

**Wynik: 3 uwagi P1 oraz 6 uwag P2.** Zmiany usuwają wiele wcześniejszych sprzeczności, lecz kontrakt MVP nadal wymaga poprawek przed T0-4/A-01/A-06. P1 oznacza problem w podstawowym zachowaniu, który należy rozstrzygnąć przed implementacją. P2 obejmuje pozostałe scenariusze oraz funkcje opcjonalne; uwagi o Teams/gamma nie blokują MVP, jeśli te funkcje pozostają poza nim.

To review specyfikacji. Folder `Hackathon_docs` nie jest repozytorium Git i nie zawiera kodu aplikacji, spike'a ani manifestów. Nie ustalono diffu commitów i nie wykonano builda ani testów runtime. Deklaracje „sprawdzone na Windows” w §8 traktuję jako zapis autorów, a nie własny wynik pomiaru. Ustalenia poniżej opisują skutki dosłownej implementacji dokumentu lub brakujące reguły; nie dowodzą istnienia tych błędów w niedostępnym kodzie.

## R01 — [P1] Tryb spotkania nie chroni wszystkich przejść do przerwy

**Miejsce:** `docs/ARCHITECTURE.md:204–205`; wymaganie produktu: `docs/DESIGN.md:123–126`.

W wierszu Focus sprawdzenie spotkania jest przy deadline, lecz wejście w ostrzeżenie nie ma takiego warunku. Po przejściu do Focus + warning reguła ticka brzmi już bezwarunkowo `T → Break`. Dosłownie zaimplementowana tabela pozwala więc ominąć ochronę spotkania.

**Scenariusz:** demo startuje w t=0; w t=30 użytkownik włącza spotkanie na 15 s. W t=35 tabela ustawia warning, a w t=40 przechodzi do Break, chociaż spotkanie trwa do t=45. Dodatkowo wyłączenie spotkania po pierwotnym deadline ma według DESIGN dać nowe ostrzeżenie, podczas gdy tabela nie określa nowego deadline i następny tick może od razu rozpocząć przerwę.

**Poprawka:** wspólny warunek blokujący automatyczne ostrzeżenie i Break podczas spotkania; jawne wyzerowanie `warning` przy odroczeniu. Po wyłączeniu/wygaśnięciu spotkania po deadline ustalić nowy termin przerwy i czas ostrzeżenia. Oddzielnie zdecydować, czy świadome „Idę” może wymusić przerwę.

**Test:** spotkanie włączone przed ostrzeżeniem i w ostrzeżeniu, wygaśnięcie i ręczne wyłączenie po deadline; brak Break przed końcem spotkania i pełne uzgodnione ostrzeżenie po nim.

## R02 — [P1] Odrzucenie wyniku zadania nie anuluje jego efektu ubocznego

**Miejsce:** `docs/ARCHITECTURE.md:320–328`; reguła powtórzona w `CLAUDE.md:57–58` i ADR-012.

Dokument zakłada, że zadania wykonują I/O poza aktorem, a ignorowanie spóźnionego wyniku z `session_id` zapewnia, że po Stop efekt nie wróci. Filtr wyniku chroni stan aktora, ale nie cofa zapisu do pliku, pokazanego okna ani żądania wykonanego już po stronie Graph.

**Scenariusz:** wolne `SetDnd(on)` rusza po Start; użytkownik naciska Stop; `SetDnd(off)` kończy się pierwsze, a wcześniejsze `on` dopiero później. Aktor ignoruje odpowiedź `on`, lecz status zewnętrzny nadal jest DND. Podobnie dwa równoległe zapisy configu mogą zakończyć się w odwrotnej kolejności. Samo `session_id` nie rozróżnia też kolejnych operacji w tej samej sesji.

Tokio dokumentuje, że rozpoczętego `spawn_blocking` nie można zatrzymać przez `abort`; timeout nie jest gwarancją anulowania operacji. [Tokio: spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html).

**Poprawka:** zachować nieblokującego aktora, ale uporządkować efekty u ich wykonawców: generacja/ID operacji, docelowy stan i seryjne mutacje danego zasobu. Kontrola aktualności jest potrzebna przed zmianą lokalnego stanu, a dla rozpoczętej operacji zewnętrznej także po jej zakończeniu i ewentualne uzgodnienie stanu. Zapis końcowego rekordu sesji musi być trwałym zleceniem, którego nie odrzuca się tylko dlatego, że sesja już się zakończyła. Unikalny klucz rekordu chroni przed ponownym zapisem po retry.

**Test:** wymusić odwrotną kolejność zakończeń Start/Stop oraz dwóch zapisów configu. Sprawdzać końcowy efekt zewnętrzny i zawartość plików, a nie tylko snapshot aktora. Powtórzony zapis rekordu nie tworzy duplikatu.

## R03 — [P1] Timeout gotowości nie wykrywa zawieszenia już widocznej przerwy

**Miejsce:** `docs/ARCHITECTURE.md:289–296`; szersza obietnica: `docs/DESIGN.md:128–130`, `CLAUDE.md:64–65`, `docs/TASKS.md:110`.

Zaprojektowany timeout działa wyłącznie wtedy, gdy nowe okno nie wywoła `window_ready`. Po tej deklaracji nie ma sygnału żywotności ani kolejnego timeoutu. Obietnica „niedziałający overlay zamykany po 5 s” obejmuje więcej niż opisany mechanizm.

**Scenariusz:** okno zgłasza gotowość, pokazuje się i następnie jego JS przestaje odpowiadać. Powłoka nadal działa, ale przycisk i obsługiwany w tym WebView Esc przestają działać. Pierwotny timeout niczego już nie zamknie. Tray może zapewniać dodatkowe wyjście, ale nie realizuje automatycznego zamknięcia po 5 s.

**Poprawka:** albo ograniczyć deklarację i test do awarii startu, albo dodać okresowe potwierdzenie działania widoku, powiązane z instancją okna/`break_id`, oraz zamknięcie z poziomu powłoki po jego utracie. Ścieżka awaryjnego wyjścia nie może zależeć wyłącznie od JS tego samego okna. Nie rozszerzać tego na gwarancję działania przy zawieszeniu całego procesu.

**Test:** osobno brak `window_ready` i zawieszenie JS dopiero po poprawnym `window_ready`. Test P-4 musi rozróżniać te przypadki.

## R04 — [P2] Reconnect audio nie uwzględnia żądanego stanu odtwarzania

**Miejsce:** `docs/ARCHITECTURE.md:353–355`, w powiązaniu z `:347–349`.

Nowy opis nakazuje po odzyskaniu urządzenia wznowić bieżący gatunek. Nie uzależnia tego od Pause/Stop/Break ani od komend przyjętych w czasie braku urządzenia. Jednocześnie `Play` i `Resume` resetują fade do 1.0, więc użycie ich przy reconnect może też odciszyć muzykę podczas przerwy.

**Scenariusz:** Focus gra → odłączenie wyjścia → Pause albo Stop → powrót wyjścia. Bez dodatkowego stanu zapisana reguła ponownie uruchomi audio. W wariancie z Break może odtworzyć muzykę z pełnym współczynnikiem fade.

**Poprawka:** wątek audio trzyma żądany stan niezależnie od istnienia urządzenia: Playing/Paused/Stopped, gatunek, głośność, fade i aktualna generacja. Reconnect odtwarza ten stan; nie jest nowym poleceniem Play. Stop czyści zamiar odtwarzania. Retry nie blokuje odbierania komend.

**Test:** odłączenie → Pause/Stop/Break → ponowne podłączenie; urządzenie można otworzyć, lecz audio nie rusza ani nie zmienia wyciszenia wbrew bieżącemu stanowi. Rodio zapewnia osobne operacje pause/play/stop i głośność; regułę odtwarzania po reconnect musi dostarczyć aplikacja. [rodio 0.22.2: Player](https://docs.rs/rodio/0.22.2/rodio/struct.Player.html).

## R05 — [P2] Resume po przerwaniu nie określa, skąd odzyskać pozostały czas

**Miejsce:** `docs/ARCHITECTURE.md:170–175` oraz `:206`.

Heurystyka ma nie wliczać luki i przechodzić do Paused(Interrupted), ale Resume opisano tylko jako przesunięcie deadline o czas pauzy. Jeśli pauza jest rejestrowana dopiero przy wykryciu luki, upływ między ostatnim prawidłowym tickiem a wykryciem pozostaje w deadline. Nie opisano jego odtworzenia.

**Scenariusz:** deadline demo t=40, ostatni prawidłowy tick t=10, następny t=130 wykrywa przerwanie. Pause zaczyna się w t=130, Resume w t=135 przesuwa deadline o 5 s do t=45, czyli nadal w przeszłości. Przerwa zaczyna się od razu zamiast po zachowanych 30 s focusa.

**Poprawka:** przy przerwaniu zapisać remaining z ostatniego zaufanego momentu albo cofnąć logiczny początek pauzy do tego momentu. Resume ustawia nowy deadline z tego remaining. Dopisać także efekty Pause/Resume dla audio i ostrzeżenia; obecna tabela efektów ich nie zawiera.

**Test:** powyższa oś czasu daje po Resume 30 s do przerwy i 10 s aktywnego focusa; zwykła pauza w ostrzeżeniu nie zostawia aktywnego okna ostrzeżenia z niedziałającymi komendami.

## R06 — [P2] Okno tint ma zakazane komendy wymagane przez wspólny bootstrap

**Miejsce:** `docs/ARCHITECTURE.md:289–293`, `:399–406`; `docs/TASKS.md:87`.

Protokół wymaga od każdego okna wywołania `get_snapshot` i `window_ready`, ale `tint-*` ma nie mieć dostępu do żadnych komend. Po wdrożeniu B-12 ten sam `api.ts`/bootstrap nie może spełnić obu wymagań. Ponadto SessionSnapshot nie zawiera TintState — osobny event `tint://changed` emituje dane dopiero przy zmianie.

**Scenariusz:** okno tint powstaje lub jest odświeżane, gdy kolor nie zmienia się. Wywołania bootstrapu są odrzucane, a sam listener zmiany nie określa początkowego koloru. Dalszy efekt zależy od implementacji widoku, której tu nie ma.

**Poprawka:** osobny bootstrap tint z jawnym przekazaniem bieżącego TintState i potwierdzeniem gotowości albo minimalne, celowe uprawnienia do getterów/ready. Rozróżnić brak komend zmieniających stan od braku jakiegokolwiek IPC. Dla deklaracji „snapshot odtwarza cały widok” doprecyzować też inicjalizację NowPlaying, które obecnie jest osobnym eventem bez gettera.

**Test:** utworzenie i odświeżenie tint podczas stałego ustawienia daje od razu poprawny kolor; mutacje sesji z tego okna nadal są odrzucane. Ograniczenia własnych komend konfiguruje się manifestem i capabilities. [Tauri: capabilities](https://tauri.app/security/capabilities/).

## R07 — [P2, tylko S-01] Historyczna flaga ustawienia DND nie dowodzi własności aktualnego statusu

**Miejsce:** `docs/ARCHITECTURE.md:387–388`; `docs/DECISIONS.md:127`.

Warunek „w tej sesji sami ustawiliśmy DND” nie realizuje obietnicy „czyścimy wyłącznie nasze DND”, gdy użytkownik później zmieni status. Ostrzeżenie o nadpisaniu przy starcie nie usuwa tej rozbieżności w regule sprzątania.

**Scenariusz:** FocusMode ustawia DND; użytkownik w Teams ustawia Away; Stop nadal spełnia warunek historycznej flagi i usuwa nowy ręczny status. `clearUserPreferredPresence` usuwa preferowany status użytkownika, bez warunku własności naszej sesji. [Microsoft: clearUserPreferredPresence](https://learn.microsoft.com/en-us/graph/api/presence-clearuserpreferredpresence?view=graph-rest-1.0).

**Poprawka:** najprościej pozostawić krótkie jawne wygaśnięcie i zrezygnować z obietnicy bezpiecznego automatycznego clear. Jeśli clear zostaje, jego reguła i ograniczenia wymagają osobnej decyzji — odczyt przed clear ogranicza ryzyko, lecz sam nie zapewnia atomowego sprawdzenia i usunięcia tylko własnego statusu.

**Test:** DND aplikacji → ręczne Away/Busy/DND → Stop; nie kasować późniejszej decyzji użytkownika. Problem nie blokuje MVP bez integracji Teams.

## R08 — [P2, tylko S-02] Liniowa gamma nie jest przywróceniem wcześniejszych ustawień

**Miejsce:** `docs/ARCHITECTURE.md:379–381` oraz `:417`.

Po kill dokument nakazuje zastosować liniową rampę na podstawie samego znacznika `gamma.active`. Znacznik nie przechowuje poprzedniej rampy ani tożsamości monitora. Jeśli ekran miał wcześniej własną kalibrację, liniowa rampa jej nie przywróci, a po zmianie układu monitorów można zmodyfikować niewłaściwy wyświetlacz.

**Poprawka:** nie nazywać resetu liniowego przywracaniem stanu. Jeśli eksperyment zostaje, określić kopię bazowej rampy per monitor oraz politykę konfliktu z późniejszymi zmianami systemowymi. Windows ma API odczytu bieżącej rampy, lecz sam odczyt nie rozwiązuje konfliktu po crashu. [Microsoft: GetDeviceGammaRamp](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getdevicegammaramp). Prostszą decyzją dla hackathonu pozostaje niewdrażanie S-02.

**Test:** start z nieliniową rampą → tint → zwykłe wyjście oraz kill/restart; oddzielny przypadek odłączenia lub zmiany monitora. Nie wystarcza ocena, że obraz „nie jest już pomarańczowy”.

## R09 — [P2] Konfigurowalna krótka przerwa nie mieści stałych ćwiczeń

**Miejsce:** `docs/DESIGN.md:143–153`; `docs/ARCHITECTURE.md:166`; `docs/TASKS.md:58–59`.

Plan normalny to dwa ćwiczenia i spacer zajmujący resztę. W MVP pierwsze dwa trwają 40 + 30 s, a czas przerwy jest konfigurowalny. Nie podano minimalnej dopuszczalnej długości ani zasad pomijania ćwiczeń, które się nie mieszczą.

**Scenariusz:** użytkownik wybiera przerwę 60 s. Po dwóch wymaganych ćwiczeniach pozostaje −10 s na spacer. Wymaganie sumy planu równej `break_secs` jest niemożliwe do spełnienia bez dodatkowej reguły; przy odejmowaniu w typie bez znaku grozi także błąd obliczeń.

**Poprawka:** walidować minimalny czas zależny od wybranego planu albo dobierać tylko ćwiczenia, które mieszczą się razem z minimalnym spacerem. Osobno określić walidację `focus_secs` względem `warning_secs`. Nie skracać arbitralnie instrukcji ćwiczeń do wartości demo w normalnym trybie.

**Test:** break krótszy/równy/dłuższy od sumy ćwiczeń; dokładna suma bez ujemnej reszty, przewidywalny błąd walidacji albo jawnie zdefiniowany krótszy plan.

## Co z poprzedniego review zostało uwzględnione

Ocena dotyczy zapisów, nie dowodu wykonania funkcji.

| Poprzednie uwagi | Stan w obecnych dokumentach |
|---|---|
| F01–F03: gamma, crash, platformy | Zmieniono podstawowy mechanizm na overlay i wybrano Windows. Pozostała błędna naprawa gammy w opcjonalnym S-02 (R08) |
| F04: czasy i plan demo | Profil oraz stały plan demo są spójne. Pozostały przejścia spotkania (R01) i granice normalnego configu (R09) |
| F05: utrata BreakPlan przy otwarciu | Plan jest w snapshocie, dodano rewizje i bootstrap. Pozostały awaria po ready (R03) i wyjątek tint (R06) |
| F06: Graph i wykrywanie rozmów | Wycięto automatyczne wykrywanie rozmów, dodano ręczne spotkanie; własność statusu nadal nie jest rozwiązana (R07) |
| F07: czas i uśpienie | Dodano dwa zegary i heurystykę przerwania; trzeba dopisać odtwarzanie remaining (R05) |
| F08: rodio | Przypięto 0.22.2, poprawiono API/uzasadnienie wątku, zapisano ograniczenia próby VM; reconnect wymaga R04 |
| F09–F10: eksport TS i core bez I/O | Dodano `#[ts(export)]`, kontrolę dryfu, persystencję w shell i wspólny katalog ćwiczeń |
| F11: Stop i efekty | Dodano Stop/ID przerwy/tabelę wejść, lecz ochrona przed spóźnionymi efektami jest niepełna (R02), a effects nie opisuje Pause/Resume (R05) |
| F12–F15: instalator, ACL, role, lockfile | Wcześniejszy instalator, jawny manifest komend, uporządkowane role i procedura lockfile. ACL wymaga uzgodnienia bootstrapu (R06) |
| F16: obietnice zdrowotne | Zmieniono tint na preferencję wizualną i oddzielono czas sesji od dowodu produktywności/zdrowia |

## Ograniczenia i kwestie do sprawdzenia z kodem

- **Audio:** pomiar 40 min na fizycznym Windows nadal jest otwarty jako T0-11. Nie traktować zakończenia T0-10 jako potwierdzenia jakości audio na sprzęcie demo. Zapis o identycznym objawie w raw cpal wspiera hipotezę wspólnego problemu środowiska lub pomiaru, ale bez kodu i logów nie potwierdzam kategorycznego „to nie wina naszego kodu”.
- **Scaffold/routing:** TASKS wprost odnotowuje SvelteKit SPA i pozostawia decyzję o routingu, natomiast drzewo ARCHITECTURE nadal pokazuje App.svelte. To znana otwarta decyzja; trzeba zamknąć ją przed implementacją widoków, a nie liczyć jako nowe, odkryte teraz uszkodzenie kodu.
- **T0-2:** checkbox jest zamknięty, lecz DoD nadal wymaga uruchomienia także u B, którego notatka dopiero o to prosi. Odbiór obu środowisk pozostaje niepotwierdzony.
- **Brak implementacji w paczce:** link do `spikes/audio-spike/src/main.rs`, skrypt setupu, skille projektu, manifesty i lockfile nie są dostępne w tym folderze. Ich brak w kopii dokumentów nie dowodzi braku w repozytorium projektu.

## Zalecana kolejność

Przed T0-4 rozstrzygnąć R01–R03 i R05 oraz doprecyzować wspólny bootstrap R06. R04 dodać do kontraktu audio i kryteriów A-05; R09 do walidacji A-03 i planera A-02. R07–R08 rozstrzygać tylko przy wejściu w opcjonalne S-01/S-02. Sprawdzenie faktycznej implementacji i wyników spike'a wymaga osobnego dostępu do repozytorium z kodem.
