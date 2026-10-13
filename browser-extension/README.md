# FocusMode — rozszerzenie blokujące rozpraszacze

Rozszerzenie dla przeglądarek opartych na Chromium (Chrome, Edge, Brave, Opera, Vivaldi). Podczas fazy
**Skupienie** w aplikacji FocusMode otwarcie strony z listy blokowanych (np. `reddit.com`, `9gag.com`,
`kwejk.pl`) kończy się lokalną stroną **„Wracaj do pracy 🎧”** z czasem pozostałym do przerwy. W przerwie,
po zakończeniu sesji albo gdy aplikacja jest wyłączona, nic nie jest blokowane, a otwarta strona „Wracaj do
pracy” po ok. 1,5 s sama wraca do pierwotnego adresu (i do tego czasu pokazuje link do niego).

Rozszerzenie nie ma własnych ustawień — pobiera listę z aplikacji.

## Lista blokowanych stron (`websiteblacklist.txt`)

Listę zmieniasz w zwykłym pliku tekstowym `websiteblacklist.txt` w folderze ustawień aplikacji
(`%APPDATA%\com.focusmode.app\websiteblacklist.txt`). Przy pierwszym uruchomieniu aplikacja tworzy go z
domyślną listą popularnych rozpraszaczy (kwejk, jbzd, wykop, reddit, facebook, TikTok, YouTube Shorts…).
Otworzysz go w aplikacji: **Ustawienia → Blokuj rozpraszające strony** — przycisk otwiera plik w domyślnym
edytorze tekstu, a obok widać liczbę domen i wzorców oraz linie, których nie udało się wczytać.
Po zapisaniu plik wczytuje się sam (albo przyciskiem „Wczytaj ponownie”); rozszerzenie dostaje nową listę
przy następnym sprawdzeniu stanu (najpóźniej po 30 s, szybciej po przełączeniu karty lub otwarciu popupu).

Format — jedna pozycja w linii:

| Linia                                   | Znaczenie                                                                   |
| --------------------------------------- | --------------------------------------------------------------------------- |
| `# komentarz`, pusta linia              | pomijana                                                                    |
| `kwejk.pl`                              | domena: blokuje `kwejk.pl` i wszystkie subdomeny (`www.`, `m.`…)            |
| `youtube\.com/shorts`                   | wyrażenie regularne (linia zawiera któryś ze znaków `\ ^ $ * + ? ( ) [ ] { } \|`) |
| `/reddit\.com/r/(funny\|memes)/`        | wyrażenie regularne zapisane w ukośnikach `/…/`                             |

Wyrażenia regularne:

- są dopasowywane do **całego adresu** strony, razem ze schematem, np. `https://www.youtube.com/shorts/abc`
  (`^` oznacza więc początek `https://…`), i **bez rozróżniania wielkości liter**;
- wystarczy, że pasują do fragmentu adresu — `youtube\.com/shorts` blokuje tylko Shorts, a nie cały YouTube;
- używają składni RE2 (tej samej co Chrome): bez lookahead/lookbehind (`(?=…)`, `(?<=…)`) i odwołań wstecznych
  (`\1`). Kropka to „dowolny znak”, więc w domenach pisz `\.`;
- wzorzec, którego przeglądarka nie przyjmie jako reguły (np. zbyt złożony), jest pomijany — popup pokazuje
  wtedy „pominięte: N”, a na liście jest przekreślony. Rozszerzenie zgłasza też takie wzorce aplikacji, a ta
  pokazuje je w **Ustawieniach** obok listy. Pominięty wzorzec zwykle nadal działa przy ładowaniu karty
  (rozszerzenie samo sprawdza jej adres, o ile JavaScript umie wykonać to wyrażenie), ale bez reguły sieciowej
  strona może zdążyć się na chwilę otworzyć.
  Rozszerzenie bierze pod uwagę najwyżej 500 wzorców.

## Instalacja

Rozszerzenie instaluje się jako „rozpakowane” (tryb dewelopera), bez Chrome Web Store.

1. W aplikacji FocusMode otwórz **Ustawienia → Blokuj rozpraszające strony → „Otwórz folder rozszerzenia”**.
   Zostanie otwarty folder `browser-extension` z plikiem `manifest.json`.
   Nie przenoś ani nie usuwaj tego folderu — przeglądarka wczytuje rozszerzenie bezpośrednio z niego.
   W wersji deweloperskiej (`npm run tauri dev`) to folder `browser-extension/` w repozytorium.
2. Otwórz stronę rozszerzeń w swojej przeglądarce (wpisz adres w pasku adresu):

   | Przeglądarka | Adres                   | Przełącznik                                   |
   | ------------ | ----------------------- | --------------------------------------------- |
   | Chrome       | `chrome://extensions`   | „Tryb dewelopera” (prawy górny róg)           |
   | Edge         | `edge://extensions`     | „Tryb dewelopera” (panel po lewej)            |
   | Brave        | `brave://extensions`    | „Tryb dewelopera” (prawy górny róg)           |
   | Opera        | `opera://extensions`    | „Tryb programisty” (prawy górny róg)          |
   | Vivaldi      | `vivaldi://extensions`  | „Tryb programisty” (prawy górny róg)          |

3. Włącz tryb dewelopera, kliknij **„Załaduj rozpakowane”** i wskaż folder z kroku 1.
4. (Opcjonalnie) Przypnij ikonę FocusMode (księżyc) do paska narzędzi — po kliknięciu pokazuje stan
   połączenia, fazę, pozostały czas oraz liczbę blokowanych domen i wzorców. Na ikonie widać też liczbę minut
   do końca fazy.
5. Sprawdź w aplikacji (Ustawienia), że status zmienił się na **„Rozszerzenie połączone”** (może to zająć do 30 s).

Rozszerzenie musi mieć dostęp do wszystkich witryn: **Szczegóły → Dostęp do witryn → „Na wszystkich
witrynach”** (tak jest domyślnie). Jeśli chcesz blokować także w oknach incognito/InPrivate, włącz
**„Zezwalaj w trybie incognito”** w szczegółach rozszerzenia. W tych oknach przeglądarka nie wyświetla
stron rozszerzeń, więc zamiast „Wracaj do pracy” zobaczysz jej własny komunikat o zablokowanej stronie —
blokada działa tak samo.

### Aktualizacja

Po aktualizacji aplikacji FocusMode wejdź na stronę rozszerzeń i kliknij **↻ (Załaduj ponownie)** przy
FocusMode. Jeśli ścieżka folderu się zmieniła, usuń rozszerzenie i załaduj je ponownie z nowego folderu.

Chrome może po uruchomieniu przypominać o rozszerzeniach w trybie dewelopera — to normalne dla rozszerzeń
instalowanych spoza sklepu.

## Jak to działa

- Rozszerzenie co 30 s, a także przy przełączeniu karty (najwyżej raz na sekundę) i ładowaniu strony
  (najwyżej raz na 5 s), pyta aplikację o stan: `GET http://127.0.0.1:47615/v1/state`. Odpowiedź
  zawiera tylko: czy blokować, listę domen, listę wzorców (regex), fazę i pozostały czas.
- Gdy aplikacja każe blokować, rozszerzenie ustawia reguły `declarativeNetRequest` — jedną dla wszystkich
  domen i po jednej na każdy wzorzec (wcześniej sprawdzony przez przeglądarkę, `isRegexSupported`).
  Przeglądarka sama przekierowuje otwierane strony z listy (oraz subdomeny, np. `old.reddit.com`) na
  `blocked.html`. Rozszerzenie nie widzi przy tym treści stron ani zapytań sieciowych.
- Po ustawieniu reguł rozszerzenie wysyła aplikacji listę wzorców, które nie dostały reguły:
  `POST http://127.0.0.1:47615/v1/report` z treścią `{"skipped": [...]}` — tylko wzorce z listy otrzymanej
  od aplikacji, dokładnie w tej postaci, w jakiej przyszły. Robi to tylko w fazie Skupienie, gdy lista się
  zmieni (po 2 s spokoju, raz na każdą nową listę w danym skupieniu; w kolejnym skupieniu zgłasza ją od nowa).
  Gdy aplikacja nie odpowie, rozszerzenie ponawia zgłoszenie przy następnym sprawdzeniu stanu; błąd
  zgłoszenia nie wpływa na blokowanie.
- Karty z blokowanymi stronami, które były już otwarte w chwili rozpoczęcia skupienia, są przeładowywane,
  żeby reguła zadziałała także dla nich. Tak samo przejście w obrębie strony bez ładowania jej od nowa
  (np. z filmu na YouTube do Shorts): rozszerzenie sprawdza nowy adres karty z listą domen i wzorców.
- Strona „Wracaj do pracy” pokazuje, która pozycja listy zablokowała stronę (domena albo wzorzec).
- Gdy zaczyna się przerwa, sesja się kończy albo aplikacja nie odpowiada (np. została zamknięta),
  reguły są usuwane i wszystkie strony działają normalnie.
- Otwarta strona „Wracaj do pracy” sprawdza stan co 5 s. Gdy zobaczy przerwę (albo koniec sesji, albo brak
  aplikacji), od razu prosi rozszerzenie o ponowne zapytanie aplikacji — to usuwa reguły bez czekania na
  kolejne 30 s — i jeśli drugie zapytanie to potwierdzi, po ok. 1,5 s wraca na pierwotny adres (tylko
  `http`/`https`). Strona blokady znika przy tym z historii karty, więc „Wstecz” nie wraca na nią.
  Gdyby strona od razu trafiła z powrotem na blokadę, drugi automatyczny powrót nie nastąpi — zostaje
  przycisk „Otwórz stronę”.
- Bez rozszerzenia aplikacja korzysta z awaryjnego wykrywania po tytule okna przeglądarki (pokazuje
  przypomnienie, ale nie zamyka strony).

## Prywatność

- **Nic nie opuszcza Twojego komputera.** Jedyne połączenia sieciowe, jakie wykonuje rozszerzenie, to
  zapytania do aplikacji FocusMode pod adresem `http://127.0.0.1:47615` (interfejs lokalny): odczyt stanu
  (`GET /v1/state`) i zgłoszenie pominiętych wzorców (`POST /v1/report`). Polityka bezpieczeństwa treści
  (CSP) rozszerzenia blokuje wszystkie inne połączenia.
- Rozszerzenie nie wysyła aplikacji żadnych danych o przeglądaniu — ani adresów, ani tytułów stron.
  Adres karty jest porównywany z listą domen i wzorców wyłącznie lokalnie, w przeglądarce. Zgłoszenie
  `/v1/report` zawiera tylko wzorce z Twojej własnej listy, które aplikacja wcześniej przysłała.
- Brak analityki, kodu ładowanego z sieci, ciasteczek i kont.
- W pamięci sesji przeglądarki (`chrome.storage.session`, czyszczonej po zamknięciu przeglądarki) trzymany
  jest tylko ostatni stan z aplikacji, ostatnio zgłoszona lista pominiętych wzorców (żeby nie wysyłać jej
  ponownie po każdym uśpieniu rozszerzenia) oraz — przez krótki czas — adres karty przekierowanej na stronę
  „Wracaj do pracy”, żeby można ją było otworzyć w przerwie. Strona blokady zapisuje też w pamięci sesji
  tej karty (`sessionStorage`, czyszczonej po zamknięciu karty) adres i czas ostatniego automatycznego
  powrotu — tylko po to, żeby nie wpaść w pętlę przekierowań.

### Po co te uprawnienia

| Uprawnienie                          | Do czego                                                                    |
| ------------------------------------ | --------------------------------------------------------------------------- |
| `declarativeNetRequestWithHostAccess` | reguły przekierowujące blokowane domeny i adresy pasujące do wzorców na stronę „Wracaj do pracy” |
| `host_permissions: <all_urls>`       | przekierowanie musi działać na dowolnej domenie z Twojej listy             |
| `tabs`                               | odczyt adresu karty, żeby przekierować już otwarte karty z blokowaną stroną |
| `alarms`                             | sprawdzanie stanu aplikacji co 30 s                                         |
| `storage`                            | zapamiętanie ostatniego stanu na czas sesji przeglądarki                    |

## Rozwiązywanie problemów

- **Popup pokazuje „Brak połączenia z aplikacją FocusMode”** — uruchom aplikację. Jeśli działa, sprawdź
  w przeglądarce adres `http://127.0.0.1:47615/v1/state` — powinien zwrócić JSON. Jeśli port jest zajęty
  przez inny program, aplikacja pokaże błąd `bridge.port`.
- **Przeglądarka pyta o dostęp do urządzeń w sieci lokalnej** — zezwól; `127.0.0.1` to Twój własny komputer.
- **Strona z listy się otwiera** — sprawdź, czy trwa faza Skupienie (a nie przerwa), czy blokowanie jest
  włączone w ustawieniach aplikacji i czy rozszerzenie ma dostęp do wszystkich witryn. Kliknij „Odśwież”
  w popupie. Po rozpoczęciu skupienia rozszerzenie może potrzebować do 30 s, zanim zacznie blokować
  (szybciej, gdy przełączysz kartę lub otworzysz popup).
- **Wzorzec (regex) nie blokuje** — sprawdź w Ustawieniach aplikacji, czy linia nie jest na liście
  problemów ani wśród wzorców, których przeglądarka nie przyjęła jako reguły (lista pojawia się w trakcie
  skupienia), a w popupie, czy wzorzec nie jest „pominięty” (przekreślony). Pamiętaj, że wzorzec jest
  dopasowywany do całego adresu z `https://` — `^youtube` nie zadziała, `^https://(www\.)?youtube` tak.
- **Blokowana jest za duża część strony** — kropka w wyrażeniu regularnym pasuje do dowolnego znaku, a wzorzec
  wystarczy, że pasuje do fragmentu adresu; dopisz `\.`, `/` albo kotwice `^…$`.
- **Firefox** nie jest obsługiwany (rozszerzenie korzysta z API Chromium). Tam działa tylko awaryjne
  wykrywanie po tytule okna.

## Języki

Rozszerzenie mówi w języku interfejsu przeglądarki (`chrome.i18n`), niezależnie od języka aplikacji: angielski,
polski, niemiecki, francuski, włoski, ukraiński, rosyjski, hiszpański, chiński (uproszczony) i japoński; dla innych
języków — angielski. Teksty są w `_locales/<język>/messages.json` (wspólne terminy: `docs/i18n-glossary.md`), a strony
wypełniają je z atrybutów `data-i18n` (`localizePage` w `common.js`).

## Pliki

| Plik                                   | Rola                                                                     |
| -------------------------------------- | ------------------------------------------------------------------------ |
| `manifest.json`                        | Manifest V3                                                              |
| `background.js`                        | service worker: odpytywanie aplikacji, reguły blokowania, obsługa kart   |
| `common.js`                            | wspólne funkcje (dopasowanie domen i wzorców, format czasu, teksty)       |
| `blocked.html`, `blocked.css`, `blocked.js` | strona „Wracaj do pracy”                                            |
| `popup.html`, `popup.css`, `popup.js`  | okienko po kliknięciu ikony                                              |
| `theme.css`                            | wspólny motyw kolorystyczny                                              |
| `icons/`                               | ikony 16/32/48/128 px                                                    |
| `_locales/<język>/messages.json`       | teksty w 10 językach (chrome.i18n); `en` to język domyślny               |
