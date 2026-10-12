# FocusMode — rozszerzenie blokujące rozpraszacze

Rozszerzenie dla przeglądarek opartych na Chromium (Chrome, Edge, Brave, Opera, Vivaldi). Podczas fazy
**Skupienie** w aplikacji FocusMode otwarcie strony z listy blokowanych (np. `reddit.com`, `9gag.com`,
`kwejk.pl`) kończy się lokalną stroną **„Wracaj do pracy 🎧”** z czasem pozostałym do przerwy. W przerwie,
po zakończeniu sesji albo gdy aplikacja jest wyłączona, nic nie jest blokowane, a otwarta strona „Wracaj do
pracy” po ok. 1,5 s sama wraca do pierwotnego adresu (i do tego czasu pokazuje link do niego).

Listę domen zmieniasz w aplikacji: **Ustawienia → Blokuj rozpraszające strony**. Rozszerzenie nie ma
własnych ustawień — pobiera listę z aplikacji.

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
   połączenia, fazę, pozostały czas i liczbę blokowanych domen. Na ikonie widać też liczbę minut do końca fazy.
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
  zawiera tylko: czy blokować, listę domen, fazę i pozostały czas.
- Gdy aplikacja każe blokować, rozszerzenie ustawia regułę `declarativeNetRequest`: przeglądarka sama
  przekierowuje otwierane strony z listy (oraz ich subdomeny, np. `old.reddit.com`) na `blocked.html`.
  Rozszerzenie nie widzi przy tym treści stron ani zapytań sieciowych.
- Karty z blokowanymi stronami, które były już otwarte w chwili rozpoczęcia skupienia, są przeładowywane,
  żeby reguła zadziałała także dla nich.
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

- **Nic nie opuszcza Twojego komputera.** Jedyne połączenie sieciowe, jakie wykonuje rozszerzenie, to
  zapytanie do aplikacji FocusMode pod adresem `http://127.0.0.1:47615` (interfejs lokalny). Polityka
  bezpieczeństwa treści (CSP) rozszerzenia blokuje wszystkie inne połączenia.
- Rozszerzenie nie wysyła aplikacji żadnych danych o przeglądaniu — ani adresów, ani tytułów stron.
  Adres karty jest porównywany z listą domen wyłącznie lokalnie, w przeglądarce.
- Brak analityki, kodu ładowanego z sieci, ciasteczek i kont.
- W pamięci sesji przeglądarki (`chrome.storage.session`, czyszczonej po zamknięciu przeglądarki) trzymany
  jest tylko ostatni stan z aplikacji oraz — przez krótki czas — adres karty przekierowanej na stronę
  „Wracaj do pracy”, żeby można ją było otworzyć w przerwie. Strona blokady zapisuje też w pamięci sesji
  tej karty (`sessionStorage`, czyszczonej po zamknięciu karty) adres i czas ostatniego automatycznego
  powrotu — tylko po to, żeby nie wpaść w pętlę przekierowań.

### Po co te uprawnienia

| Uprawnienie                          | Do czego                                                                    |
| ------------------------------------ | --------------------------------------------------------------------------- |
| `declarativeNetRequestWithHostAccess` | reguła przekierowująca blokowane domeny na stronę „Wracaj do pracy”        |
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
- **Firefox** nie jest obsługiwany (rozszerzenie korzysta z API Chromium). Tam działa tylko awaryjne
  wykrywanie po tytule okna.

## Pliki

| Plik                                   | Rola                                                                     |
| -------------------------------------- | ------------------------------------------------------------------------ |
| `manifest.json`                        | Manifest V3                                                              |
| `background.js`                        | service worker: odpytywanie aplikacji, reguły blokowania, obsługa kart   |
| `common.js`                            | wspólne funkcje (dopasowanie domen, format czasu, teksty)                 |
| `blocked.html`, `blocked.css`, `blocked.js` | strona „Wracaj do pracy”                                            |
| `popup.html`, `popup.css`, `popup.js`  | okienko po kliknięciu ikony                                              |
| `theme.css`                            | wspólny motyw kolorystyczny                                              |
| `icons/`                               | ikony 16/32/48/128 px                                                    |
