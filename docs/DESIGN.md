# FocusMode — Design Document (produkt)

> Status: **wersja 2** — po cross-review ([reviews/codex-crosscheck-2026-09-27.md](reviews/codex-crosscheck-2026-09-27.md)).
> Architektura: [ARCHITECTURE.md](ARCHITECTURE.md). Decyzje: [DECISIONS.md](DECISIONS.md). Podział pracy: [TASKS.md](TASKS.md).

## 1. Pitch (jedno zdanie)

**FocusMode** to desktopowe „lofi radio + trener przerw”: wybierasz klimat dźwięku, pracujesz 40 minut
w skupieniu, a potem aplikacja **wygania cię od ekranu** na 5 minut łagodnego ruchu.
Wieczorem może też ocieplić barwy ekranu.

Pitch opisuje MVP. Integracja z Teams jest opcjonalnym rozszerzeniem, a nie obietnicą produktu.

## 2. Problem i benefit

| Problem | Jak FocusMode pomaga |
|---|---|
| Ciągłe przerywanie rozbija głęboką pracę | Wyraźna sesja 40 min z muzyką; tryb „Jestem na spotkaniu” przesuwa przerwę zamiast jej zabierać |
| Siedzimy godzinami bez ruchu, bo „jeszcze tylko to” | Przerwa na pełnym ekranie z prowadzonymi ćwiczeniami, której nie da się przeoczyć (ale da się pominąć) |
| Jasny, zimny ekran wieczorem bywa nieprzyjemny | Ocieplenie barw jako **preferencja wizualna** (bez obietnic zdrowotnych) |
| Trudno wejść w flow | Rytuał: wybór dźwięku → start → dźwięk sam się wycisza na przerwę |

Co mierzymy (lokalnie): **czas aktywnej sesji** i **wynik przerwy** (ukończona / pominięta / przerwana).
Nie twierdzimy, że to dowód lepszej produktywności czy zdrowia. Odliczanie nie jest potwierdzeniem wykonania ćwiczeń.

## 3. Persona

**Programista/analityk w korporacji**: cały dzień przy dwóch monitorach, Teams włączony non stop,
słucha muzyki na słuchawkach, często zapomina o przerwach. W pracy używa Windowsa.

## 4. Wspierane platformy (ADR-008)

| Platforma | Status | Znane ograniczenia |
|---|---|---|
| **Windows 10/11** | **odbiór projektu i demo** — pełny UX, testy na sprzęcie | powiadomienia systemowe w trybie dev mogą mieć nazwę/ikonę PowerShell |
| macOS | best-effort: buduje się w CI, uruchamia | przezroczyste okna (tint) wymagają `macos-private-api`; bez testów UX |
| Linux X11 | best-effort | always-on-top to tylko sugestia dla menedżera okien; brak tooltipa w tray (czas pokazujemy w menu) |
| Linux Wayland | best-effort, z degradacją | brak always-on-top i tint → przerwa jako zwykłe okno + powiadomienie |

Architektura zostaje multiplatformowa (Tauri, kod per OS za traitami), ale **gwarantujemy tylko Windows**.

## 5. Główna pętla i czasy

```
Start ─▶ Focus ─────────────────────────▶ Przerwa ─────────▶ następna sesja (auto lub kliknięciem)
          │            ostrzeżenie           ćwiczenia, gong,
          │   (ostatnie 30 s focusa)         „odejdź od ekranu”
          └─ „Jestem na spotkaniu” przesuwa przerwę; „Jeszcze 2 min” raz na cykl
```

**Jeden profil czasów** zasila UI, logikę i testy (ADR-011). Ostrzeżenie **zawiera się** w czasie focusa.

| Parametr | Normalny | Demo |
|---|---:|---:|
| Focus (do startu przerwy) | 2400 s | 40 s |
| Ostrzeżenie przed końcem focusa | 30 s | 5 s |
| Przerwa | 300 s | 20 s |
| „Jeszcze 2 min” (raz na cykl) | 120 s | 10 s |
| „Jestem na spotkaniu” | 15 / 30 / 60 min | 15 s |

Oś czasu demo (bez odroczeń): **0 s** start → **35 s** ostrzeżenie → **40 s** przerwa → **60 s** koniec.
Odroczenie pokazujemy w osobnym przebiegu. Tryb demo jest wyraźnie oznaczony w UI jako „skrócona prezentacja”,
a skrócone ćwiczenia nie są zaleceniem ruchowym.

## 6. Funkcje (MoSCoW)

### Must (MVP, M1)
- **Timer** z profilem czasów (normalny/demo), start/pauza/stop, licznik cykli.
- **Dźwięk**: generowany szum (brown/pink, zero licencji) + **1 utwór CC0** z pliku (sprawdza ładowanie zasobów z instalatora).
- **Ostrzeżenie** w ostatnich 30 s: małe okienko na wierzchu (działa też przy ukrytym oknie głównym):
  **[Idę]** **[Jeszcze 2 min]** **[Jestem na spotkaniu]**. Toast systemowy jest tylko tekstowym dodatkiem.
- **Tryb „Jestem na spotkaniu”** (ręczny, bez Teams): przesuwa przerwę o wybrany czas, a UI pokazuje, kiedy nastąpi następna próba.
- **Przerwa na pełnym ekranie** na każdym monitorze (Windows), z odliczaniem i **3 prostymi kartami ćwiczeń**.
- **Niezawodne wyjście**: Stop działa w każdej fazie (przycisk, tray, klawiatura). Pominięcie przerwy wymaga przytrzymania
  przycisku lub klawisza Esc przez 3 s. Jeśli okno przerwy nie wystartuje, powłoka sama je zamyka.
- **Tray** z menu (i czasem do końca: w tooltipie na Windows, w pozycji menu na Linuksie).
- **Tryb demo**: `--demo` przy uruchomieniu lub przełącznik w ustawieniach (zmienna `FOCUSMODE_DEMO=1` tylko w dev).
- **Instalator Windows od M1**, przetestowany poza katalogiem repo i bez sieci.

### Should (M2 — „wow” małym kosztem)
- **Ocieplenie ekranu przez overlay** (ADR-009): przezroczyste okno klikalne na wylot, harmonogram godzinowy, suwak, płynne przejście.
  Po crashu znika razem z procesem, więc kolory zawsze wracają.
- Fade-in/fade-out muzyki, gong przy zmianie ćwiczenia (nie trzeba patrzeć w ekran), 2–3 gatunki.
- **Podsumowanie sesji jednym zdaniem**: „32 min aktywnej sesji, przerwa ukończona”.
- Prosta statystyka dzienna: czas aktywnej sesji dziś, liczba cykli, wyniki przerw.
- Ustawienia: długości faz, auto-start kolejnej sesji, domyślny gatunek, głośność, tint, tryb demo.
- Spójny styl lofi, ikona aplikacji.
- Ograniczenie komend IPC per typ okna (capabilities + manifest komend).

### Could (M3, stretch)
- **Teams „Nie przeszkadzać”** przez Microsoft Graph (ADR-010): tylko ustawianie DND z wygaśnięciem,
  z jawną polityką odpowiedzialności za status. Wymaga pozytywnego testu tenantu (T0-8).
- **Eksperymentalna gamma ramp** na Windows (SDR), domyślnie wyłączona, poza ścieżką demo.
- Wykrywanie bezczynności (auto-pauza), radio internetowe (URL), harmonogram tint wg zachodu słońca,
  globalny skrót, autostart, wykres 7 dni i seria dni, dopracowanie macOS/Linux.

### Won't (świadomie NIE robimy)
- **Automatyczne wykrywanie rozmów/spotkań przez Graph**: po ustawieniu własnego DND zagregowany presence
  nie jest wiarygodnym czujnikiem rozmowy (ADR-010). Zamiast tego jest ręczny tryb spotkania.
- Konta, chmura, synchronizacja; Spotify i własny streaming; grywalizacja; aplikacja mobilna.

## 7. Przepływy UX

### 7.1 Pierwsze uruchomienie
Okno główne: kafelki dźwięków + duży **Start**. Bez kreatora. Nota: „Za 40 min zaproponujemy przerwę.
Zawsze możesz ją pominąć, przytrzymując przycisk lub Esc.”

### 7.2 Sesja focus
- Pierścień z czasem, dźwięk, głośność, pauza/stop, przycisk „Jestem na spotkaniu”.
- Okno można zamknąć, a sesja trwa w tray.

### 7.3 Ostrzeżenie → przerwa
1. **T−30 s** (demo T−5 s): okienko w rogu ekranu, na wierzchu, bez kradzieży fokusu:
   „Przerwa za 30 s”, a pod nim **[Idę]** / **[Jeszcze 2 min]** (raz na cykl) / **[Jestem na spotkaniu ▾]**. Dźwięk zaczyna się wyciszać.
2. **T**: na każdym monitorze pojawia się overlay przerwy:
   - główny monitor: karta bieżącego ćwiczenia (krótkie kroki, prosta ilustracja), pasek postępu całej przerwy,
   - pozostałe: spokojne tło i duże odliczanie („Odejdź od ekranu 🙂”).
   - Zmiany ćwiczeń sygnalizuje **gong**. Kartę wystarczy przeczytać raz, dalej nie trzeba patrzeć w ekran.
3. **Pomiń**: przytrzymanie przycisku lub Esc przez 3 s. Zwykłe kliknięcie nie pomija przerwy.
4. Przerwa kończy się **wraz z odliczaniem**. Nie ma przycisku „Zrobione”, bo odliczanie nie jest deklaracją wykonania ćwiczeń.
   Potem następna sesja (auto-start) albo ekran „Wracamy?” i podsumowanie jednym zdaniem.

### 7.4 Tryb „Jestem na spotkaniu”
Włączany w focusie lub z okienka ostrzeżenia, na 15/30/60 min. Jeśli focus się skończy w trakcie spotkania,
przerwa czeka. UI pokazuje „Przerwa po spotkaniu, ok. 14:30”. Wyłączenie trybu po czasie focusa
uruchamia ostrzeżenie od razu.

### 7.5 Zasady bezpieczeństwa i uczciwości (kryteria odbioru — TASKS P-4)
- **Nigdy nie więzimy**: Stop dostępny w każdej fazie i idempotentny; pominięcie przerwy zawsze możliwe;
  niedziałający overlay zamykany przez powłokę po 5 s.
- **Kolory ekranu**: tint przez overlay znika razem z procesem. Eksperymentalna gamma (Could) ma osobno opisane
  zachowanie przy zwykłym wyjściu, panice i zabiciu procesu i jest domyślnie wyłączona.
- Ćwiczenia są ogólne i łagodne. Stopka: „To nie jest porada medyczna. Jeśli coś boli — przestań.”
- Dane (ustawienia, historia sesji, ewentualny token Teams) zostają **lokalnie**; token w systemowym keychainie.

## 8. Treści: ćwiczenia

Jeden katalog `assets/exercises.json` (id, nazwa, kroki, czas normalny i demo). Plan przerwy układa Rust
i wysyła go do UI z pełną treścią. UI dokłada tylko ilustrację po `id`, z ilustracją domyślną jako fallback.

| id | Nazwa | Czas | W MVP |
|---|---|---|---|
| `neck-tilt` | Skłony szyi na boki | 40 s | ✅ |
| `shoulder-rolls` | Krążenia barków | 30 s | ✅ |
| `stand-walk` | Wstań, przejdź się, napij wody | reszta przerwy | ✅ (zawsze na końcu) |
| `wrist-stretch` | Rozciąganie nadgarstków | 40 s | |
| `chest-opener` | Otwarcie klatki | 30 s | |
| `seated-twist` | Skręt tułowia na krześle | 40 s | |
| `eye-rest` | Odpocznij wzrok: popatrz w dal | 20 s | |
| `box-breathing` | Oddech 4-4-4-4 | 60 s | |

Plan normalny: 2 losowane ćwiczenia (z jawnym seedem, żeby testy były deterministyczne) + `stand-walk` do końca przerwy.
Plan demo: **stały**, 2 × 5 s + `stand-walk` 10 s. Ilustracje: własne SVG/CSS lub CC0. Niczego nie kopiujemy z internetu.

## 9. Styl wizualny

- Klimat „lofi”: ciemna, ciepła paleta, miękkie gradienty, animowany deszcz (CSS/Canvas), duża typografia timera.
- **Nie używamy grafik ani marki Lofi Girl.** Inspirujemy się tylko klimatem.
- Muzyka tylko z licencją pozwalającą na użycie (CC0 / CC-BY z atrybucją), opisana w `assets/music/CREDITS.md`.

## 10. Scenariusz demo (3 min, instalowany build, `--demo`)

1. (15 s) Problem: „ile razy dziś wstałeś od biurka?”
2. (60 s) Start z brown noise → 35 s ostrzeżenie w rogu → 40 s przerwa na obu monitorach, gong, karta ćwiczenia;
   próba „Pomiń” zwykłym kliknięciem (nie działa) → przytrzymanie. Koniec: podsumowanie jednym zdaniem.
3. (30 s) Drugi cykl: „Jestem na spotkaniu” → UI pokazuje, kiedy nastąpi następna próba.
4. (30 s) Tint: suwak w ustawieniach, ekran ocieplony na żywo (overlay).
5. (30 s) Architektura na 1 slajdzie (Rust core + Tauri, multiplatformowość z uczciwą tabelą ograniczeń).
6. (15 s) Co dalej (Teams DND, gamma, macOS/Linux).

Plan B: nagrane wideo z przebiegu (P-2).

## 11. Ryzyka

| Ryzyko | Prawdop. | Wpływ | Mitigacja |
|---|---|---|---|
| Okno przerwy startuje puste / bez stanu | średnie | wysoki | protokół gotowości okien + snapshot odtwarzający cały widok + timeout 5 s (ADR-012), spike w M0 |
| Niespójne czasy ostrzeżenia/przerwy | wysokie bez profilu | średni | jeden profil czasów, testy granic 34/35/39/40/60 s (ADR-011) |
| Dev działa, instalator nie (zasoby, powiadomienia) | średnie | wysoki | pierwszy instalator w M1, test poza repo i offline |
| Overlay nie przykrywa paska zadań / monitora | średnie | średni | spike okien w M0 na sprzęcie demo; nie obiecujemy zakrycia elementów systemu |
| Brak zgody w tenancie na Graph | wysokie | niski | Teams jest w Could; ręczny tryb spotkania w MVP |
| Licencje muzyki | średnie | wysoki | generowany szum + CC0, CREDITS.md |
| Za dużo funkcji | wysokie | wysoki | MoSCoW, feature freeze na 3 dni przed końcem |
| Konflikty przy pracy we dwóch | średnie | średni | kontrakt MVP w M0, właściciele plików, procedura lockfile/bindings |

## 12. Pomysły z review (do wyboru, nieobowiązkowe)

- ✅ przyjęte: gong i „nie trzeba patrzeć w ekran” (§7.3), ręczny tryb spotkania (§7.4), podsumowanie jednym zdaniem (Should).

## 13. Otwarte pytania

- [x] Czas hackathonu: **2 tygodnie, dozwolone wszelkie narzędzia AI**.
- [x] Platforma odbioru: **Windows** (ADR-008).
- [ ] **Ile godzin realnie ma każda osoba?** Dwa tygodnie kalendarzowe to nie estymacja; od tego zależy zakres M2/M3.
- [ ] Sprzęt demo: ile monitorów, jakie DPI, czy HDR?
- [ ] Język UI: PL czy EN (zależnie od jury)?
- [ ] Nazwa finalna: FocusMode? (sprawdzić kolizje)
- [ ] Czy tenant M365 pozwala na zgodę na `Presence.ReadWrite` (T0-8) — decyduje o S-01.
