# FocusMode 🎧

Sesje skupienia w klimacie lofi: ustawiasz czasy pracy i przerwy (domyślnie 45 / 15 minut), wybierasz muzykę
i klikasz Start. Na czas skupienia FocusMode wycisza to, co rozprasza: powiadomienia Windows, status w Teams,
strony z memami i launchery gier. W przerwie wszystko wraca, a muzyka gra dalej. Jeśli w czasie przerwy masz spotkanie,
FocusMode powie ci o tym jeszcze przed startem i zapyta, co zrobić. Po zakończeniu sesji widzisz, w których
aplikacjach naprawdę spędziłeś czas.

Rust + Tauri 2 + Svelte 5 · Windows 10/11 (pełne wsparcie), macOS / Linux (best-effort) · projekt hackathonowy.
Zakres i decyzje: [ADR-013](docs/DECISIONS.md#adr-013-speedrun-pivot-produktu),
[ADR-014](docs/DECISIONS.md#adr-014-zasobnik-i-okienka-faz-odcień-ekranu-overlay-zamykanie-launcherów-gier-języki),
[ADR-015](docs/DECISIONS.md#adr-015-runda-2a-ciepło-ekranu-bez-overlayu-motywy-dźwięki-muzyka-przerwy-i-historia)
i [ADR-016](docs/DECISIONS.md#adr-016-kalendarze-i-spotkania-w-przerwach).

## Co potrafi

- **Pętla skupienie ⇄ przerwa** aż do Stop. Czasy ustawiasz sam (1–240 / 1–120 min) albo wybierasz preset: 25/5, 45/15, 50/10, 90/20.
- **Dzwonek i powiadomienie** przy każdej zmianie fazy. Dźwięk wybierasz z listy: pakiety pasujące do motywów (Midnight, Vaporwave, Lunar, Autumn),
  klasyczny ding-dong, łagodny ton albo 8-bitowe arpeggio. Możesz też wskazać **własny plik** (mp3, wav, ogg, flac, m4a, do 10 MB):
  FocusMode kopiuje go do swojego folderu danych i gra najwyżej 10 sekund.
- **Przerwa z pomysłem**: przy każdej przerwie FocusMode losuje poradę dopasowaną do jej długości (oczy, rozciąganie, ruch, woda, oddech, wyjście na zewnątrz…).
  Porady powstały na podstawie publicznych źródeł (m.in. HSE, OSHA, CIOP-PIB, AAO, WHO, CCOHS, NHS i badań nad przerwami w pracy), a źródło jest podane przy każdej z nich.
  To propozycje, nie porady medyczne. Możesz też ustawić **własne przypomnienia na kolejne przerwy**, np. 1. przerwa: rozciąganie, 2.: woda i spacer z psem,
  3.: obiad (opcjonalnie lista może się powtarzać).
- Zamiast zwykłego powiadomienia Windows możesz włączyć **okienka FocusMode**: małe okno w prawym dolnym rogu ekranu, w stylu aplikacji,
  z przyciskami „Pokaż okno” i „Pomiń fazę”. Nie zabiera fokusu i znika samo po kilku sekundach. Pojawia się też, gdy sesja zatrzymała się
  po uśpieniu komputera, gdy FocusMode zamknął launchery gier, gdy przerwa czeka na koniec spotkania i jako przypomnienie o spotkaniu w czasie przerwy. To okno aplikacji, a nie powiadomienie, więc „Nie przeszkadzać” go nie wycisza.
- **Spotkania z kalendarza** (opcjonalnie; [jak połączyć](#kalendarze-spotkania-w-czasie-przerw)): Microsoft 365 / Outlook, Google Calendar
  i każdy kalendarz z linkiem ICS (firmowy Exchange, iCloud, Proton, Zoho, Zimbra, Nextcloud…), także spotkania cykliczne. Gdy spotkanie wypada
  w czasie przerwy, strona Skupienie pokazuje to **przed Start** i pyta, czy przedłużyć skupienie na czas spotkania (z wyciszoną muzyką),
  wstrzymać przerwę, zignorować, czy przypomnieć później (30, 20 i 10 minut przed spotkaniem). Kalendarze są tylko odczytywane.
- **Pauza, Wznów, Pomiń fazę, Stop** w dowolnej chwili, w oknie albo z ikony w zasobniku (działa też przy ukrytym oknie).
  Uśpienie komputera w trakcie sesji automatycznie ją wstrzymuje.
- **Muzyka** (gra też w przerwie, chyba że wyłączysz to w ustawieniach):
  - **muzyka w zestawie** (domyślnie): 12 utworów lo-fi z kolekcji [Open Lo-Fi](#muzyka-w-zestawie-i-licencje) na licencji CC0, dołączonych do aplikacji;
    gra od razu, także bez internetu,
  - **stream**: np. publiczny live „lofi hip hop radio” kanału Lofi Girl na YouTube albo dowolny adres radia internetowego,
  - **linki**: filmy i playlisty YouTube, YouTube Music, `youtu.be`, bezpośrednie adresy plików audio (z mieszaniem),
  - **folder**: pliki mp3, m4a, aac, flac, wav, ogg, opus i webm z wybranego katalogu,
  - **szum**: brązowy, różowy lub biały, generowany lokalnie (działa offline),
  - albo cisza.

  YouTube gra w odtwarzaczu wbudowanym w okno FocusMode. Filmy, których właściciel nie pozwala osadzać w innych stronach
  (m.in. live Lofi Girl i część utworów YouTube Music), oraz linki do kanałów (`/@kanał`, `/@kanał/live`) otwierają się
  w **ukrytym oknie YouTube grającym w tle**, którym FocusMode steruje jak odtwarzaczem (odtwarzanie, pauza, głośność).
  Podpowiadany adres streamu to `https://www.youtube.com/@LofiGirl/live` (zawsze bieżąca transmisja kanału). Przy pierwszym odtwarzaniu w tym oknie YouTube pyta
  o zgodę na ciasteczka: okno pojawi się wtedy na środku ekranu, a po wyborze opcji schowa się samo (wybór zostaje zapamiętany).
  Przycisk „Pokaż okno” w odtwarzaczu pokazuje je na żądanie; zamknięcie (X) tylko je chowa, muzyka gra dalej.
- **Osłony skupienia** (każdą włączasz osobno w Ustawieniach; schodzą w przerwie, po Stop i przy wyjściu z aplikacji):
  - wyciszenie powiadomień Windows („Nie przeszkadzać”); w przerwie, po Stop i przy wyjściu powiadomienia są włączane z powrotem
    (jeśli „Nie przeszkadzać” było włączone już przed focusem, FocusMode go nie rusza),
  - status **„Nie przeszkadzać” w Microsoft Teams** (przez Microsoft Graph, wymaga jednorazowej konfiguracji),
  - **blokada rozpraszających stron** (kwejk, jbzd, reddit, facebook, tiktok… lista w pliku tekstowym
    [`websiteblacklist.txt`](#lista-blokowanych-stron-websiteblacklisttxt), z domenami i wyrażeniami regularnymi) przez rozszerzenie przeglądarki,
    które przekierowuje na stronę „Wracaj do pracy”; gdy rozszerzenie nie odezwało się w ostatnich 90 s (np. go nie ma),
    FocusMode rozpoznaje stronę po tytule okna przeglądarki i przywołuje cię powiadomieniem,
  - ekran i komputer nie usypiają się w trakcie skupienia,
  - **zamykanie launcherów gier** na start każdej fazy skupienia (domyślnie wyłączone). Zaznaczasz, które: Steam, Epic Games Launcher, GOG Galaxy,
    Ubisoft Connect, EA app, Battle.net, Riot Client, Xbox app, Amazon Games, itch. FocusMode zamyka tylko procesy samych launcherów
    (po dokładnej nazwie pliku, np. `steam.exe`, `EpicGamesLauncher.exe`), tylko twoje i bez praw administratora. Gier ani usług nie rusza
    i nie uruchamia launcherów ponownie w przerwie; w trakcie skupienia możesz je włączyć sam.
- **Ciepłe kolory ekranu według pory dnia** (domyślnie wyłączone; Ustawienia → „Ciepłe kolory ekranu”): wieczorem ekran robi się cieplejszy,
  rano wraca do normy. Ustawiasz godzinę początku i końca (domyślnie 18:00 → 9:00), siłę i długość płynnego przejścia (domyślnie 30 min),
  a przesunięcie suwaka siły od razu pokazuje podgląd. Działa niezależnie od sesji. Kolory zmienia naprawdę, na jeden z dwóch sposobów:
  - **Oświetlenie nocne Windows** (domyślnie): FocusMode włącza je w twoich godzinach i ustawia jego siłę, a poza nimi wyłącza,
  - **gamma monitora**: zmiana kolorów każdego monitora (także w grach na pełnym ekranie); nie działa na monitorach HDR, a bardzo ciepłe wartości Windows osłabia.

  Obraz nie traci kontrastu, a zrzuty ekranu mają normalne kolory. Twoje ustawienia Oświetlenia nocnego (albo kalibrację monitora) FocusMode zapamiętuje
  przed pierwszą zmianą i przywraca je po wyłączeniu, przy zamknięciu aplikacji, a po awarii przy następnym uruchomieniu albo najpóźniej przy następnym logowaniu.
- **Motywy**: Midnight (domyślny), Vaporwave, fazy księżyca, przytulna jesień, prosty ciemny, prosty jasny i 8-bit. Przełączanie zakładek
  Skupienie / Podsumowanie / Ustawienia jest animowane (podświetlenie przesuwa się, a tło lekko przesuwa się w bok); przy włączonym w systemie
  ograniczeniu animacji ruch jest wyłączony.
- **Wykres produktywności**: w trakcie skupienia FocusMode co sekundę sprawdza, która aplikacja jest na pierwszym planie.
  Po Stop pokazuje czas według procesu (np. `Code.exe`, `chrome.exe`), osobno bezczynność, a historię sesji zapisuje lokalnie.
  Elementy samego Windows (menu Start, wyszukiwanie, pasek zadań, pulpit, `ShellExperienceHost` itp.) nie są liczone.
  Po najechaniu myszą na słupek widzisz najczęstsze **tytuły okien** tej aplikacji (np. karty przeglądarki albo foldery w Eksploratorze);
  zapamiętywanie tytułów wyłączysz w Ustawieniach. Tam też możesz **wyłączyć zapisywanie historii** (podsumowanie po Stop nadal się pokaże)
  i **usunąć całą historię** jednym przyciskiem (z potwierdzeniem).
- **Zasobnik**: przy pierwszym kliknięciu X FocusMode pyta, czy schować się do zasobnika (obok zegara). „Tak” chowa okno (muzyka i sesja trwają dalej)
  i od razu włącza okienka FocusMode; „Nie” sprawia, że X zamyka aplikację. Wybór zapamiętuje i możesz go zmienić w Ustawieniach.
  Z menu ikony w zasobniku zawsze sterujesz sesją i zamkniesz aplikację („Zakończ”). Jedna instancja, opcjonalny start przy logowaniu
  (wtedy FocusMode startuje zminimalizowany do zasobnika).
- **Języki**: angielski, polski, niemiecki, francuski, włoski, ukraiński, rosyjski, hiszpański, chiński i japoński; domyślnie język Windows
  (gdy go nie ma na liście, angielski). Tłumaczenia są w przygotowaniu, na razie teksty są po polsku.

## Instalacja (Windows)

1. Uruchom instalator `FocusMode_0.1.0_x64-setup.exe` (jak go zbudować: [Rozwój](#rozwój)).
   Instaluje się tylko dla bieżącego użytkownika, bez praw administratora, do `%LOCALAPPDATA%\FocusMode`, ze skrótem w menu Start.
2. Instalator nie jest podpisany, więc SmartScreen może pokazać „System Windows ochronił ten komputer”. Wybierz **Więcej informacji → Uruchom mimo to**.
3. Wymagany jest Microsoft Edge WebView2 (jest w Windows 11 i aktualnym Windows 10). Jeśli go brakuje, instalator go pobierze.
4. Uruchom FocusMode, wybierz czasy i muzykę, kliknij **Start**.

Odinstalowanie: Ustawienia Windows → Aplikacje → Zainstalowane aplikacje → FocusMode.
Twoje dane (`%APPDATA%\com.focusmode.app`) możesz potem usunąć ręcznie.

## Rozszerzenie przeglądarki (blokada stron)

Działa w Chrome, Edge, Brave, Operze i Vivaldi (Chromium). Firefox nie jest na razie wspierany.

1. W FocusMode: **Ustawienia → Blokuj rozpraszające strony → Otwórz folder rozszerzenia**
   (po instalacji to `%LOCALAPPDATA%\FocusMode\browser-extension`, w repo: `browser-extension/`).
2. W przeglądarce otwórz stronę rozszerzeń: `chrome://extensions`, `edge://extensions`, `brave://extensions`, `opera://extensions` albo `vivaldi://extensions`.
3. Włącz **Tryb dewelopera** i kliknij **Załaduj rozpakowane**, a potem wskaż folder z punktu 1.
4. W Ustawieniach FocusMode status zmieni się na „Rozszerzenie połączone” (najpóźniej po 30 s).

Rozszerzenie co 30 s (i przy zmianie karty) pyta aplikację o stan pod `http://127.0.0.1:47615/v1/state`. W trakcie skupienia przekierowuje
strony z listy (niżej) na lokalną stronę „Wracaj do pracy”, a w przerwie wszystko otwiera się normalnie. Gdy aplikacja nie działa, rozszerzenie niczego nie blokuje.
Szczegóły: [browser-extension/README.md](browser-extension/README.md).

## Lista blokowanych stron (websiteblacklist.txt)

Listę stron blokowanych w trakcie skupienia trzymasz w zwykłym pliku tekstowym:

```
%APPDATA%\com.focusmode.app\websiteblacklist.txt
```

Przy pierwszym uruchomieniu FocusMode tworzy go z domyślnej listy popularnych rozpraszaczy, polskich i anglojęzycznych
(memy, media społecznościowe, plotki, sport, wideo, gry, zakupy). Otworzysz go z **Ustawienia → Blokuj rozpraszające strony**:
jeden przycisk otwiera plik w domyślnym edytorze tekstu, drugi wczytuje go ponownie. Po zapisaniu plik wczytuje się sam, bez restartu
(rozszerzenie dostaje nową listę najpóźniej po 30 s), a Ustawienia pokazują liczbę domen i wzorców oraz linie, które zostały pominięte (z numerem linii i powodem).

Format:

- jedna pozycja w linii; linie zaczynające się od `#` to komentarze, puste linie są pomijane,
- **zwykła linia to domena**: blokuje ją razem ze wszystkimi subdomenami (`https://` i ścieżka są ignorowane, liczy się tylko domena;
  adres ze znakiem `?`, np. `youtube.com/watch?v=…`, jest już wyrażeniem regularnym, więc wpisz samą domenę),
- **wyrażenie regularne**: linia ujęta w ukośniki `/…/` albo zawierająca któryś ze znaków `\ ^ $ * + ? ( ) [ ] { } |`.
  Jest dopasowywane do **całego adresu strony** (np. `https://www.youtube.com/shorts/abc`), bez rozróżniania wielkości liter.
  Składnia RE2, taka jak w Chrome: bez lookahead, lookbehind i odwołań wstecznych. Kropka oznacza „dowolny znak”, więc w domenach pisz `\.`.
  Wzorzec bez kotwic pasuje do dowolnego miejsca adresu; żeby trafić tylko w konkretną stronę, zacznij od `^https?://`.

Przykłady:

| Linia | Co blokuje |
|---|---|
| `kwejk.pl` | kwejk.pl i wszystkie subdomeny (`www.kwejk.pl`, `m.kwejk.pl`) |
| `youtube\.com/shorts` | tylko YouTube Shorts; zwykłe filmy działają |
| `^https?://(www\.)?onet\.pl/?$` | tylko stronę główną Onetu; `poczta.onet.pl` działa dalej |

Żeby odblokować stronę, usuń linię albo dopisz na jej początku `#`. Domyślna lista zawiera też zakomentowane pozycje
(cały YouTube, portale informacyjne, komunikatory), które włączysz, usuwając `#`.
Bez rozszerzenia FocusMode porównuje z listą tylko tytuł okna przeglądarki, więc wzorce pisane pod adres (np. z `^https?://`) wtedy nie zadziałają.

## Teams „Nie przeszkadzać” (opcjonalnie)

Microsoft Graph pozwala ustawić status tylko aplikacji zarejestrowanej w twoim tenancie, więc potrzebny jest jednorazowy setup
(czasem tylko administrator może go zrobić albo zatwierdzić):

1. [Entra ID](https://entra.microsoft.com) → **Rejestracje aplikacji → Nowa rejestracja**, typ kont: konta w katalogu organizacji.
2. **Uwierzytelnianie** → „Zezwalaj na przepływy klienta publicznego” = **Tak**.
3. **Uprawnienia interfejsu API** → Microsoft Graph → delegowane: `Presence.ReadWrite` i `User.Read`, a jeśli chcesz też
   [kalendarz Microsoft](#microsoft-365--outlook), `Calendars.Read` (jeśli tenant tego wymaga, ze zgodą administratora).
4. Skopiuj **Identyfikator aplikacji (klienta)** do FocusMode: **Ustawienia → Teams**, tenant zostaw `organizations` albo wpisz ID swojego tenanta.
5. Kliknij **Połącz**, otwórz pokazaną stronę Microsoft i wpisz kod.

W trakcie skupienia status to „Nie przeszkadzać” z wygaśnięciem (długość skupienia + 5 min), a na przerwę i po Stop FocusMode go czyści.
Przed zmianą FocusMode sprawdza twój bieżący status:

- jeśli jesteś **„Wyświetlaj jako offline”** (Teams pokazuje wtedy „offline”, tak samo jak przy braku zalogowania), FocusMode nic nie zmienia,
  żeby nie zdradzić, że jesteś przy komputerze,
- jeśli masz już **„Nie przeszkadzać”** (ustawione samodzielnie albo w trakcie prezentacji), FocusMode go nie rusza i potem nie czyści.

Uwaga: status zmieniony w Teams ręcznie **w trakcie** skupienia zostanie wyczyszczony na przerwę. Teams pokazuje zmianę statusu z opóźnieniem (do kilku minut).

## Kalendarze: spotkania w czasie przerw

FocusMode może czytać spotkania z twojego kalendarza i sprawdzać, czy któreś wypada w czasie przerwy. Kolizje widać na stronie **Skupienie**
już przed Start (godziny spotkania i numer przerwy) i dalej w trakcie sesji. Dla każdego takiego spotkania wybierasz:

| Wybór | Co się dzieje |
|---|---|
| **Przedłuż skupienie** | skupienie trwa do końca spotkania, muzyka milknie na czas spotkania, a pełna przerwa zaczyna się po nim |
| **Wstrzymaj przy przerwie** | gdy pora przerwy wypada w trakcie spotkania, przerwa zaczyna się wstrzymana (minutnik i muzyka stoją, okienko podaje temat spotkania) i rusza, gdy klikniesz **Wznów** |
| **Ignoruj** | sesja biegnie bez zmian |
| **Przypomnij później** | 30 minut przed spotkaniem pojawi się małe okienko FocusMode z tym samym pytaniem; „Przypomnij później” w okienku pokaże je znowu po 10 minutach, a ostatnie, 10 minut przed spotkaniem, nie ma już tej opcji |

Pomiń fazę, Pauza i Stop działają zawsze, także w przedłużonym skupieniu. Gdy kilka spotkań z wyborem „Przedłuż skupienie” następuje po sobie, przedłużenia się łączą.
FocusMode pomija spotkania odwołane, odrzucone, oznaczone jako „wolny” i całodniowe. Sprawdza najbliższe 10 godzin
(w Ustawieniach od 1 do 24; przy długiej sesji wybierz więcej, żeby objąć wszystkie jej przerwy).

Kalendarze włączasz w **Ustawienia → Kalendarze**, każdy osobno. FocusMode tylko je odczytuje: nie odpowiada na zaproszenia i niczego w nich nie zmienia.

### Microsoft 365 / Outlook

Kalendarz służbowy Microsoft 365 (Exchange Online, Outlook, spotkania w Teams) FocusMode czyta przez to samo konto Microsoft co [Teams](#teams-nie-przeszkadzać-opcjonalnie):

1. Zrób jednorazowy setup z sekcji Teams i w kroku 3 dodaj delegowane uprawnienie **`Calendars.Read`** (tylko odczyt kalendarza).
2. W **Ustawienia → Kalendarze** włącz **Microsoft 365 / Outlook**, a w osłonie „Microsoft Teams: Nie przeszkadzać” połącz konto Microsoft.
3. Jeśli konto było połączone wcześniej tylko dla Teams, kliknij **Połącz ponownie** w **Ustawienia → Kalendarze** (bez rozłączania,
   status w Teams działa dalej), żeby Microsoft zapytał o zgodę na odczyt kalendarza. Bez włączonego kalendarza FocusMode o nią nie prosi.

### Google Calendar

FocusMode nie ma własnego, zweryfikowanego przez Google identyfikatora klienta OAuth, więc potrzebny jest twój, z projektu Google Cloud.
Projekt zakładasz raz, bezpłatnie:

1. Otwórz [console.cloud.google.com](https://console.cloud.google.com) i utwórz projekt (albo wybierz istniejący).
2. **Interfejsy API i usługi → Biblioteka** → włącz **Google Calendar API**.
3. Skonfiguruj **ekran zgody OAuth** (Google Auth Platform): odbiorcy „Zewnętrzni” (konto prywatne) albo „Wewnętrzni” (firmowy Google Workspace),
   zakres `…/auth/calendar.readonly`. Dopóki aplikacja jest w trybie testowym, dodaj swój adres jako **użytkownika testowego**.
4. **Klienci** (Dane logowania) → utwórz **identyfikator klienta OAuth** typu **„Aplikacja komputerowa”** (Desktop app).
5. Skopiuj identyfikator klienta (kończy się na `.apps.googleusercontent.com`) i tajny klucz klienta (zaczyna się od `GOCSPX-`)
   do **Ustawienia → Kalendarze → Kalendarz Google** i kliknij **Połącz z Google**.
6. W przeglądarce wybierz konto i zezwól na odczyt kalendarza. Google pokaże ostrzeżenie **„Google nie zweryfikował tej aplikacji”**, bo projekt jest twój
   i nie przeszedł weryfikacji Google: wybierz **Kontynuuj** albo **Zaawansowane → Przejdź do …**. Po zgodzie przeglądarka wraca na lokalny adres `127.0.0.1`, a FocusMode pokazuje „Połączono”.

Logowanie idzie w twojej przeglądarce (OAuth dla aplikacji komputerowych z PKCE), więc FocusMode nie widzi twojego hasła.
W trybie testowym ekranu zgody Google unieważnia token po 7 dniach i trzeba połączyć się ponownie. Żeby tego uniknąć, opublikuj aplikację
(„W produkcji”; ostrzeżenie o braku weryfikacji zostaje) albo, w Google Workspace, wybierz odbiorców „Wewnętrznych” (bez ostrzeżenia).
Bez projektu Google Cloud możesz też dodać kalendarz Google jako link ICS (niżej), kosztem wolniejszego odświeżania.

### Inne kalendarze (link ICS / iCal)

Prawie każdy kalendarz, także firmowy Exchange na własnych serwerach, daje prywatny link subskrypcji w formacie iCalendar (`.ics`).
Wklej linki w **Ustawienia → Kalendarze → Inne kalendarze**, jeden w każdej linii (`https://…` albo `webcal://…`, najwyżej 10), i zapisz.
Spotkanie, które widać w kilku kalendarzach naraz (np. w Microsoft i w linku ICS), FocusMode liczy raz.
FocusMode uwzględnia też spotkania cykliczne (z wyjątkami i przeniesionymi terminami).

| Kalendarz | Gdzie znaleźć link |
|---|---|
| Outlook w przeglądarce (Microsoft 365, Outlook.com, Exchange) | Ustawienia → Kalendarz → Kalendarze udostępnione → **Publikuj kalendarz**: wybierz kalendarz i „Może wyświetlać wszystkie szczegóły”, kliknij „Publikuj” i skopiuj link ICS. Na firmowym serwerze Exchange publikowanie musi włączyć administrator |
| Google Calendar | Ustawienia → wybierz kalendarz po lewej → Integrowanie kalendarza → **Tajny adres w formacie iCal** |
| iCloud | w Kalendarzu udostępnij kalendarz jako **Kalendarz publiczny** i skopiuj link (`webcal://…`) |
| Proton, Zoho, Zimbra, Nextcloud i inne | w ustawieniach kalendarza szukaj opcji „Udostępnij”, „Subskrybuj”, „Link ICS” albo „iCal” |

Nazwy opcji różnią się między wersjami. **Taki link to klucz do kalendarza**: każdy, kto go zna, widzi twoje spotkania. FocusMode zapisuje linki
zaszyfrowane, poza plikiem ustawień, i nie pokazuje ich w logach ani w komunikatach o błędach. Gdy link wycieknie, unieważnij go u dostawcy
(zwykle „Zresetuj” albo „Cofnij publikację”) i wklej nowy.

## Discord i inne komunikatory

Discord nie udostępnia aplikacjom API do zmiany statusu użytkownika, a sterowanie kontem przez token użytkownika łamie regulamin Discorda
i grozi blokadą konta, więc FocusMode nie zmienia statusu w Discordzie. Ustaw „Nie przeszkadzać” w Discordzie ręcznie.
Banery powiadomień Discorda, które pokazuje Windows, wycisza osłona „Wycisz powiadomienia Windows”.
Inne komunikatory (np. Slack) nie są na razie obsługiwane.

## Tryb demo

Skrócone czasy do prezentacji: **40 s skupienia, 20 s przerwy**, a w UI oznaczenie „skrócona prezentacja”.

```powershell
# zainstalowana aplikacja
& "$env:LOCALAPPDATA\FocusMode\focusmode.exe" --demo

# dev (PowerShell)
$env:FOCUSMODE_DEMO=1; npm run tauri dev
```

W bashu: `FOCUSMODE_DEMO=1 npm run tauri dev`. Z `just`: `just demo`.

## Muzyka w zestawie i licencje

Domyślna muzyka to 12 utworów z kolekcji **Open Lo-Fi** autorstwa btahir (<https://github.com/btahir/open-lofi>, wydanie v1.0.0),
udostępnionej na licencji [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) (domena publiczna). Pliki są niezmienione;
leżą w `src-tauri/resources/music/open-lofi/` razem z `library.json` (tytuły i kategorie) i trafiają do instalatora (ok. 37 MB).
Pełna lista utworów, pochodzenie i pozostałe zasoby (maskotka, dźwięki): [CREDITS.md](CREDITS.md).

Dzwonki są syntezowane w kodzie, bez plików z zewnątrz. Stream Lofi Girl to tylko link do ich publicznej transmisji, odtwarzany oficjalnym playerem YouTube;
FocusMode nie zawiera grafik ani marki Lofi Girl.

## Prywatność

- **Nic nie opuszcza komputera.** Nie ma kont, telemetrii ani analityki.
- **Śledzenie aplikacji** sprawdza raz na sekundę, i tylko w trakcie biegnącego skupienia (nie w pauzie i nie w przerwie), nazwę procesu
  na pierwszym planie, czas bezczynności i tytuł okna. Elementy samego Windows (menu Start, pasek zadań, pulpit, wyszukiwanie) pomija.
  Zapisuje sumy czasu według procesu, a gdy włączone jest **zapamiętywanie tytułów** (domyślnie tak), także najczęstsze tytuły okien każdej aplikacji
  (przycięte do 120 znaków, w podsumowaniu najwyżej 5 na aplikację), żeby pokazać je w podpowiedzi na wykresie. Tytuły mogą zawierać prywatne informacje
  (np. temat wiadomości, nazwę dokumentu albo tytuł strony z trybu prywatnego), dlatego: zostają tylko na twoim komputerze w `history.jsonl`, nigdy nie trafiają
  do logów, do rozszerzenia ani do sieci, a w Ustawieniach możesz wyłączyć tytuły, wyłączyć zapisywanie historii albo usunąć całą historię (z potwierdzeniem).
  Wyłączenie tytułów nie usuwa tych, które już są w historii; do tego służy „Usuń historię”. Całe śledzenie wyłączysz w Ustawieniach.
- **Rozszerzenie nie wysyła historii przeglądania** ani do aplikacji, ani nigdzie indziej. Samo pobiera z aplikacji stan i listę domen
  oraz wzorców z `websiteblacklist.txt`, a adres karty porównuje lokalnie w przeglądarce. Nie ładuje zdalnego kodu. Jedynym adresem, z którym się łączy, jest `127.0.0.1:47615`.
- **Lokalny most** nasłuchuje tylko na `127.0.0.1` (nie jest widoczny w sieci) i udostępnia wyłącznie odczyt stanu.
- **Ruch sieciowy** jest tylko taki, jaki sam wybierzesz: odtwarzacz łączy się z YouTube / adresem streamu / linkami jak zwykła przeglądarka,
  a opcjonalna integracja z Teams łączy się z Microsoft Graph. YouTube gra w osadzonym iframe, którym aplikacja steruje przez `postMessage`.
  Skrypty Google nie są ładowane do samej aplikacji, a okno ma ustawione CSP. Player YouTube w iframe może zapisać swoje ciasteczka w danych WebView.
  Linki, których nie da się osadzić (np. live Lofi Girl), grają w małym osobnym oknie ze zwykłą stroną YouTube, jak karta przeglądarki:
  działają w nim skrypty i ciasteczka YouTube/Google (zapisane w profilu WebView2 aplikacji, także zgoda na ciasteczka). To okno nie ma dostępu
  do komend aplikacji, przechodzi tylko po domenach YouTube i Google, a inne linki otwiera w twojej przeglądarce.
  Token Teams jest zaszyfrowany DPAPI dla bieżącego użytkownika Windows. Przed ustawieniem „Nie przeszkadzać” FocusMode odczytuje twój bieżący
  status z Microsoft Graph (ta sama zgoda `Presence.ReadWrite`); odczytu nigdzie nie zapisuje.
  Włączone kalendarze łączą się tylko ze swoimi serwerami: Microsoft Graph, Google (logowanie i Calendar API) albo adresy twoich linków ICS.
- **Kalendarze** (tylko gdy je włączysz) są **tylko odczytywane**: `Calendars.Read` w Microsoft Graph, zakres tylko do odczytu w Google, zwykłe pobranie linku ICS.
  Token logowania Google i linki ICS (link ICS to klucz do kalendarza) są zaszyfrowane DPAPI dla bieżącego użytkownika Windows w osobnych plikach,
  nigdy w `config.json`; kalendarz Microsoft używa tokenu Teams. Tajny klucz klienta Google leży w `config.json` (Google nie traktuje go
  w aplikacjach komputerowych jako tajnego), ale nie trafia do logów. Tematy i godziny spotkań widzisz tylko w oknie i okienkach FocusMode: nie trafiają
  do logów, historii, rozszerzenia ani nigdzie indziej. Na dysku zostają tylko twoje wybory dla spotkań (sam identyfikator spotkania i wybór, bez tematu).
- **Dostęp do plików**: odtwarzacz może czytać z dysku tylko muzykę dołączoną do aplikacji i pliki audio znalezione w wybranym folderze muzyki.
  Własny dźwięk dzwonka jest kopiowany do folderu `sounds` w danych aplikacji; config przechowuje tylko nazwę pliku z tego folderu.
- **Zamykanie launcherów gier** (tylko gdy je włączysz): na start skupienia FocusMode przegląda lokalnie listę uruchomionych procesów
  i zamyka wyłącznie te, których nazwa pliku dokładnie pasuje do zaznaczonych launcherów, tylko w twojej sesji Windows i bez praw administratora.
  Listy procesów nie zapisuje ani nigdzie nie wysyła.
- **Ciepłe kolory ekranu, okienka FocusMode i język** działają lokalnie. Ciepłe kolory zmieniają ustawienie Oświetlenia nocnego bieżącego użytkownika
  albo rampę gamma monitorów (bez praw administratora). Zaraz po pierwszej zmianie FocusMode zapisuje, co trzeba przywrócić, w pliku w swoim folderze danych
  i dodaje ten sam wpis `FocusModeRestore` w `RunOnce` co przy wyciszeniu powiadomień, więc po awarii kolory wracają przy następnym uruchomieniu albo logowaniu.
  Godzina pochodzi z zegara systemu, a język z ustawień Windows.
- **Wyciszenie powiadomień** zmienia tylko ustawienie „Nie przeszkadzać” bieżącego użytkownika (bez praw administratora) i tylko wtedy,
  gdy było wyłączone. Zaraz po włączeniu FocusMode zapisuje, co trzeba cofnąć (`shields-restore.json`), i dodaje wpis `FocusModeRestore`
  w `HKCU\Software\Microsoft\Windows\CurrentVersion\RunOnce`, który przy następnym logowaniu uruchamia FocusMode z `--restore-shields`.
  Powiadomienia wracają w przerwie, po Stop i przy wyjściu, a po awarii przy następnym uruchomieniu aplikacji albo najpóźniej przy następnym logowaniu.
- Dane leżą lokalnie w `%APPDATA%\com.focusmode.app`: `config.json` (ustawienia, także język, wybór zasobnika, odcień, motyw, dzwonki, przypomnienia i launchery), `websiteblacklist.txt` (lista blokowanych stron),
  `history.jsonl` (podsumowania sesji z czasem według aplikacji i, jeśli włączone, tytułami okien; przy wyłączonym zapisie historii nowe sesje nie są dopisywane,
  a „Usuń historię” kasuje plik), `sounds\` (twoje własne dźwięki dzwonków), `teams-token.bin` (jeśli połączysz Teams; czyta też kalendarz Microsoft),
  `google-token.bin` i `ics-links.bin` (zaszyfrowane, jeśli połączysz Google albo zapiszesz linki ICS), `meeting-choices.json` (wybory dla spotkań: identyfikator i wybór, bez tematów),
  `shields-restore.json` (tylko w trakcie wyciszenia) i `tint-restore.json` (tylko gdy FocusMode zmienił kolory ekranu).
  Logi (bez tytułów okien, tokenów, linków ICS i tematów spotkań): `%LOCALAPPDATA%\com.focusmode.app\logs`,
  dane WebView2 (w tym ciasteczka YouTube z okna YouTube i z osadzonego playera): `%LOCALAPPDATA%\com.focusmode.app\EBWebView`.

## Znane ograniczenia

- Gwarantujemy tylko Windows 10/11. Na macOS i Linuksie osłony i śledzenie aplikacji są niedostępne.
- Wyciszenie powiadomień korzysta z nieudokumentowanego interfejsu COM Windows (`IQuietHoursSettings`, ten sam, którego używa centrum powiadomień)
  i może przestać działać po aktualizacji systemu. Wtedy osłona będzie oznaczona jako niedostępna albo z błędem, a sesja trwa dalej.
  Automatyczne reguły Windows (gry, pełny ekran, godziny) zostają pod kontrolą systemu.
- Część filmów YouTube (m.in. live Lofi Girl i część utworów YouTube Music) nie pozwala na odtwarzanie poza youtube.com.
  Takie linki grają w ukrytym oknie YouTube. W tym trybie FocusMode nie przechodzi sam do następnego linku po końcu utworu.
  Link, którego nie da się odtworzyć wcale, jest pomijany z komunikatem.
- Rozszerzenie trzeba raz załadować ręcznie. Zmiana fazy dociera do niego najpóźniej po 30 s (przy otwieraniu stron szybciej).
- Przerwa to dzwonek, powiadomienie (albo okienko FocusMode) i propozycje ruchu w oknie. Nie ma pełnoekranowej blokady ekranu.
- Ciepłe kolory ekranu zmieniają ustawienie Windows (Oświetlenie nocne) albo rampę gamma monitora, więc po awarii aplikacji zostają ciepłe
  do jej następnego uruchomienia albo najpóźniej do następnego logowania (Oświetlenie nocne możesz też od razu wyłączyć w szybkich ustawieniach Windows).
  Sterowanie Oświetleniem nocnym korzysta z nieudokumentowanego zapisu ustawień i może przestać działać po aktualizacji Windows; wtedy FocusMode spróbuje gammy
  albo pokaże błąd. **W Windows 11 24H2 i nowszych tego zapisu Windows już nie używa, więc tam FocusMode zawsze ociepla przez gammę** (Ustawienia to pokazują). W twoich godzinach FocusMode nadpisuje ręczne zmiany Oświetlenia nocnego. Tryb gamma nie działa na monitorach HDR, bardzo ciepłe wartości
  Windows osłabia, a gry, które same ustawiają gammę, mogą ją zmienić. To zmiana wyglądu, nie kalibracja barw. Działa tylko na Windows.
- Suma czasu na wykresie może być mniejsza od czasu skupienia, bo sekundy spędzone w menu Start, na pasku zadań czy na pulpicie nie są liczone.
- Zamknięcie launchera przerywa w nim pobieranie i aktualizacje. Gry uruchomionej z launchera FocusMode nie zamyka, ale gra może stracić
  nakładkę launchera, a niektóre gry kończą się razem z nim. Po aktualizacji launchera nazwy jego procesów mogą się zmienić.
- Teams: Microsoft Graph pokazuje „offline” tak samo dla „Wyświetlaj jako offline”, jak i dla braku zalogowania; w obu przypadkach FocusMode nie zmienia statusu.
  Bardzo krótka przerwa (np. 20 s w trybie demo) może sprawić, że w następnym cyklu Teams wciąż pokazuje nasze „Nie przeszkadzać” i FocusMode je pominie.
- Kalendarze: dostawca odświeża opublikowany link ICS z opóźnieniem (zależnie od dostawcy nawet kilka godzin), więc świeżo dodane albo przesunięte
  spotkanie może pojawić się później. Link ICS nie mówi, czy odrzuciłeś zaproszenie, więc odrzucone spotkanie może nadal być widoczne, a kalendarz
  opublikowany jako „tylko wolny/zajęty” daje spotkania bez tematu. Spotkania całodniowe są pomijane. Czasy ze strefą albo w UTC są przeliczane,
  a czas bez strefy jest traktowany jako czas tego komputera; nietypowa definicja strefy w pliku może przesunąć spotkanie, dlatego FocusMode zawsze pokazuje jego godziny.
  Plan przerw zakłada, że sesja biegnie bez pauz (po pauzie i pominięciu jest liczony na nowo). Przedłużone skupienie trzyma osłony przez całe spotkanie,
  a Teams „Nie przeszkadzać” (wygasa po długości skupienia + 5 min) może wygasnąć przed końcem długiego spotkania. Przypomnienia działają, dopóki FocusMode działa
  (także w zasobniku). Google w trybie testowym wymaga ponownego połączenia co 7 dni. Kalendarze Google i ICS działają tylko na Windows (szyfrowanie DPAPI).
- W trybie dev powiadomienia Windows mogą mieć nazwę i ikonę PowerShell. Po instalacji mają nazwę FocusMode.

## Rozwój

Wymagania: Rust stable (`rustup`), Node.js LTS, zależności systemowe Tauri. Lista:
[docs/ARCHITECTURE.md §14](docs/ARCHITECTURE.md#14-narzędzia-i-środowisko).
Na Windows wszystko instaluje skrypt (sprawdza, co już jest, i doinstalowuje resztę):

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-windows.ps1
```

Potem:

```bash
npm install                          # na VM: przy „Assertion failed … loop->time” usuń node_modules i powtórz
npm run tauri dev                    # aplikacja w trybie dev
FOCUSMODE_DEMO=1 npm run tauri dev   # tryb demo (PowerShell: $env:FOCUSMODE_DEMO=1; npm run tauri dev)
cargo test --workspace --locked      # testy Rust; regeneruje też bindingi TS w src/lib/bindings/
cargo clippy --workspace --all-targets -- -D warnings
npm run check                        # svelte-check / TypeScript
npm run tauri build                  # instalator NSIS → target/release/bundle/nsis/FocusMode_0.1.0_x64-setup.exe
```

Z [`just`](https://github.com/casey/just): `just dev`, `just demo`, `just check`, `just build` (lista: `just`).
W trybie dev rozszerzenie ładujesz z `browser-extension/` w repo; po zmianach w jego kodzie kliknij „Odśwież” na stronie rozszerzeń.

| Katalog | Zawartość |
|---|---|
| `crates/focus-core` | czysta logika: profil czasów, maszyna stanów (ze spotkaniami, które przedłużają focus albo wstrzymują przerwę), config (z przypomnieniami, nazwami plików dzwonków i kalendarzami), parser listy stron (domyślna lista w `assets/websiteblacklist.txt`), statystyki (z pomijaniem powłoki Windows i tytułami okien), harmonogram i temperatura odcienia, planowanie przerw i kolizje ze spotkaniami (`calendar.rs`), launchery gier, języki; kontrakt IPC w `api.rs` |
| `crates/focus-audio` | dzwonki (rodio) na dedykowanym wątku: syntezowane pakiety i własny plik |
| `crates/focus-platform` | integracje Windows: aktywne okno (z klasą okna), wyciszenie powiadomień, keep-awake, Teams i kalendarz Microsoft (Graph), Google Calendar i linki ICS (`calendar/`), pliki szyfrowane DPAPI (`secrets.rs`), zamykanie procesów, język systemu, czas lokalny, Oświetlenie nocne, rampa gamma |
| `src-tauri/` | powłoka Tauri: komendy, aktor sesji, osłony, tracker, most dla rozszerzenia, persystencja, tray, okienka faz, ciepło ekranu (Oświetlenie nocne / gamma), synchronizacja kalendarzy i przypomnienia o spotkaniach (`src/calendar/`); muzyka w zestawie w `resources/music/` |
| `src/` | frontend SvelteKit (SPA): ekrany, odtwarzacz muzyki, motywy (`lib/themes/`), porady na przerwę (`lib/tips.ts`), teksty UI |
| `browser-extension/` | rozszerzenie MV3 do blokady stron |
| `spikes/` | wyrzucalne eksperymenty z M0 |

## Dokumentacja

| Plik | Zawartość |
|---|---|
| [docs/DECISIONS.md](docs/DECISIONS.md) | decyzje architektoniczne (ADR); **ADR-013, ADR-014, ADR-015 i ADR-016 opisują aktualny zakres** |
| [CREDITS.md](CREDITS.md) | licencje i autorzy dołączonej muzyki, grafiki i dźwięków |
| [docs/TASKS.md](docs/TASKS.md) | taski (sekcja „Speedrun”), milestone'y i podział pracy A/B |
| [docs/DESIGN.md](docs/DESIGN.md) | produkt, UX, ryzyka (sprzed pivotu, w części nieaktualne) |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | stos, crate'y, wątki, narzędzia (sprzed pivotu, w części nieaktualne); kontrakt IPC w §6 aktualny |
| [docs/CROSS_AI_REVIEW.md](docs/CROSS_AI_REVIEW.md) | jak zrobić review planu drugim modelem AI |
| [CLAUDE.md](CLAUDE.md) | instrukcje dla Claude Code |

## Praca z Claude Code

- `CLAUDE.md` ładuje się automatycznie i zawiera zasady projektu.
- Stwórz lokalny `CLAUDE.local.md` (nie trafia do gita) z jedną linią: `Pracuję jako osoba A` albo `… B`.
- Skille: `/next-task A` (bierze kolejny task, robi go na własnej gałęzi), `/check` (fmt, clippy, testy),
  `/sync-contract <opis>` (spójna zmiana kontraktu UI↔Rust).
- Flow: gałąź na task → PR do `main` → review → squash merge. Szczegóły: [docs/TASKS.md — „Jak pracujemy”](docs/TASKS.md).
