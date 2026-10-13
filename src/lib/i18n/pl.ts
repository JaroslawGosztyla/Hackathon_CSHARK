// Polish UI texts: the source language of FocusMode. Every other locale file has exactly this shape (`Strings`).
// Tone: friendly, short and calm. No health promises: the app suggests, it never claims effects.
// "FocusMode" is a name and is never translated; neither is the file name websiteblacklist.txt.
import type { Theme } from "$lib/bindings/Theme";
import type { MoonPhaseName } from "$lib/themes/moon";
import type { TipCategory } from "$lib/tips";
import type {
  BlocklistIssueCode,
  CalendarSource,
  ChimeSound,
  GameLauncher,
  IntegrationState,
  MeetingChoice,
  MusicSourceKind,
  NoiseColor,
  Phase,
  PopupKind,
  TintMode,
  WorkItemKind,
  WorkReason,
  WorkSource,
} from "$lib/types";
import { pluralizer } from "./plural";

const pluralPl = pluralizer("pl");

/** Polish plural form: 1 minuta, 2–4 minuty, 5+ minut (with the 12–14 exception). */
function plural(n: number, one: string, few: string, many: string): string {
  return pluralPl(Math.trunc(n), { one, few, many, other: many });
}

export const pl = {
  appName: "FocusMode",
  tagline: "lofi sesje skupienia",

  nav: {
    label: "Główna nawigacja",
    focus: "Skupienie",
    summary: "Podsumowanie",
    settings: "Ustawienia",
  },

  phase: {
    idle: "Gotowe do startu",
    focus: "Skupienie",
    break: "Przerwa",
  } satisfies Record<Phase, string>,

  phaseAnnouncement: {
    focusStarted: (cycle: number) => `Zaczyna się skupienie, cykl ${cycle}.`,
    breakStarted: "Zaczyna się przerwa.",
    stopped: "Sesja zakończona.",
  },

  timer: {
    paused: "Pauza",
    cycle: (n: number) => `Cykl ${n}`,
    demo: "skrócona prezentacja",
    remainingLabel: (phase: string, time: string) => `${phase}: zostało ${time}`,
    idleHint: "Ustaw czasy i muzykę, a potem naciśnij Start.",
    focusedSoFar: (time: string) => `Skupienie w tej sesji: ${time}`,
  },

  controls: {
    start: "Start",
    pause: "Pauza",
    resume: "Wznów",
    skipToBreak: "Przerwa teraz",
    skipToFocus: "Wracam teraz",
    skip: "Pomiń fazę",
    stop: "Stop",
    sessionControls: "Sterowanie sesją",
  },

  timing: {
    title: "Czas",
    focus: "Skupienie (min)",
    break: "Przerwa (min)",
    presets: "Gotowe ustawienia",
    preset: (focus: number, brk: number) => `${focus} / ${brk}`,
    presetAria: (focus: number, brk: number) => `${focus} minut skupienia, ${brk} minut przerwy`,
    rangeFocus: "Podaj liczbę minut od 1 do 240.",
    rangeBreak: "Podaj liczbę minut od 1 do 120.",
  },

  music: {
    title: "Muzyka",
    sourceLabel: "Źródło muzyki",
    source: {
      library: "Open Lo-Fi (wbudowana)",
      stream: "Stream",
      links: "Linki",
      folder: "Folder",
      noise: "Szum",
      none: "Cisza",
    } satisfies Record<MusicSourceKind, string>,
    libraryName: "Open Lo-Fi",
    libraryHint: "Spokojne utwory lo-fi dołączone do FocusMode — grają bez internetu.",
    libraryCredit: "Muzyka: Open Lo-Fi, autor btahir, licencja CC0 (domena publiczna).",
    libraryCreditLink: "github.com/btahir/open-lofi",
    libraryCreditAria: "Strona Open Lo-Fi w serwisie GitHub (otwiera przeglądarkę)",
    libraryEmpty: "Nie znaleziono wbudowanej muzyki — wybierz inne źródło.",
    streamUrl: "Adres strumienia (YouTube na żywo albo radio internetowe)",
    streamPlaceholder: "https://www.youtube.com/watch?v=…",
    lofiPreset: "Lofi Girl — radio na żywo",
    lofiPresetHint:
      "Aktualna transmisja na żywo kanału Lofi Girl. Gra w tle przez ukryte okno YouTube (właściciel nie pozwala osadzać jej w aplikacji); pauzą i głośnością sterujesz z FocusMode. Przy pierwszym uruchomieniu YouTube pokaże okno wyboru plików cookie — po wyborze schowa się samo.",
    presetsLabel: "Propozycje",
    linksLabel: "Linki — jeden w każdej linii (YouTube, YouTube Music, playlisty, pliki audio)",
    linksPlaceholder: "https://music.youtube.com/watch?v=…\nhttps://youtu.be/…\nhttps://example.com/utwor.mp3",
    linksSummary: (ok: number, bad: number) =>
      bad > 0
        ? `${ok} ${plural(ok, "link do odtworzenia", "linki do odtworzenia", "linków do odtworzenia")}, ${bad} ${plural(bad, "nieobsługiwany", "nieobsługiwane", "nieobsługiwanych")}`
        : `${ok} ${plural(ok, "link do odtworzenia", "linki do odtworzenia", "linków do odtworzenia")}`,
    shuffle: "Losowa kolejność",
    pickFolder: "Wybierz folder",
    changeFolder: "Zmień folder",
    pickFolderTitle: "Wybierz folder z muzyką",
    noFolder: "Nie wybrano folderu.",
    loadingTracks: "Szukam plików audio…",
    trackCount: (n: number) => `${n} ${plural(n, "utwór", "utwory", "utworów")}`,
    noTracks: "W tym folderze nie ma obsługiwanych plików audio (mp3, m4a, flac, wav, ogg, opus…).",
    noise: {
      brown: "Brązowy",
      pink: "Różowy",
      white: "Biały",
    } satisfies Record<NoiseColor, string>,
    noiseLabel: "Rodzaj szumu",
    noiseHint: "Generowany na miejscu — działa bez internetu.",
    noneHint: "Bez muzyki — usłyszysz tylko dzwonki na początek i koniec przerwy.",
    detected: {
      youtubeVideo: "YouTube — film lub transmisja",
      youtubePlaylist: "YouTube — playlista",
      youtubeWindow: "YouTube — zagra w małym, osobnym oknie YouTube (tego linku nie da się odtworzyć w aplikacji).",
      audio: "Strumień lub plik audio",
      unsupported: "Ten serwis nie jest obsługiwany (np. Spotify) — użyj YouTube albo bezpośredniego linku audio.",
      invalid: "To nie wygląda na adres http(s).",
    },
    pauseDuringBreak: "Pauzuj muzykę w przerwie",
    pauseDuringBreakHint: "Domyślnie muzyka gra dalej, także w przerwie.",
  },

  player: {
    region: "Odtwarzacz muzyki",
    idle: "Muzyka zacznie grać po starcie sesji",
    pausedBySession: "Wstrzymane",
    pausedForBreak: "Wstrzymane na czas przerwy",
    pausedForMeeting: "Wyciszone na czas spotkania",
    loading: "Ładuję…",
    silence: "Cisza",
    next: "Następny",
    volume: "Głośność muzyki",
    expand: "Pokaż wideo",
    collapse: "Zwiń wideo",
    noiseTitle: (color: string) => `Szum ${color.toLowerCase()}`,
    youtubeFallbackTitle: "YouTube",
    trackOf: (i: number, n: number) => `${i} z ${n}`,
    sourceNotReady: "Wybierz źródło muzyki na ekranie Skupienie.",
    windowMode: "Gra w tle przez YouTube — właściciel nagrania nie pozwala osadzać go w aplikacji.",
    windowModeShort: "Gra w tle przez YouTube",
    showWindow: "Pokaż okno",
    showWindowAria: "Pokaż okno YouTube",
    windowClosed: "Okno YouTube zostało zamknięte",
    reopenWindow: "Otwórz ponownie",
    reopenWindowAria: "Otwórz ponownie okno YouTube",
  },

  focusView: {
    nowUsing: "Teraz używasz",
    nowUsingEmpty: "Zbieram dane o aplikacjach…",
    trackingOff: "Śledzenie aplikacji jest wyłączone w ustawieniach.",
    idle: "Bezczynność",
    unknown: "Nieznane",
    shields: "Osłony",
    shieldsNone: "Żadna osłona nie jest włączona — możesz je włączyć w Ustawieniach.",
  },

  shieldChip: {
    notifications: "Powiadomienia wyciszone",
    teams: "Teams: Nie przeszkadzać",
    sites: "Blokada stron",
    sitesFallback: "Blokada stron (tryb awaryjny)",
    keepAwake: "Ekran nie zaśnie",
    launchers: "Launchery gier",
    launchersClosed: (names: string) => `Zamknięto: ${names}`,
    launchersNothing: "Żaden z zaznaczonych launcherów nie był uruchomiony.",
    state: {
      off: "wyłączone",
      on: "aktywne",
      unavailable: "niedostępne",
      error: "błąd",
    } satisfies Record<IntegrationState, string>,
    chipAria: (name: string, state: string) => `${name}: ${state}`,
  },

  breakView: {
    title: "Czas na przerwę",
    lead: "Odejdź na chwilę od biurka. Muzyka może grać dalej.",
    reminderLabel: (n: number) => `Twoje przypomnienie · ${n}. przerwa`,
    suggestionLabel: "Propozycja na tę przerwę",
    another: "Inna propozycja",
    /** Text before the source's host, with its punctuation and spacing (fr "Source : ", zh "来源："). */
    source: "Źródło: ",
    sourceAria: (host: string) => `Źródło propozycji: ${host} (otwiera przeglądarkę)`,
    categories: {
      eyes: "Oczy",
      stretch: "Rozciąganie",
      move: "Ruch",
      hydrate: "Woda",
      breathe: "Oddech",
      posture: "Stanowisko",
      outside: "Na zewnątrz",
      social: "Ludzie",
      mind: "Chwila dla głowy",
      tidy: "Otoczenie",
    } satisfies Record<TipCategory, string>,
  },

  idleTips: {
    practiceTitle: "Dobra praktyka",
    lawTitle: "Warto wiedzieć",
    /** Text before the source's host, with its punctuation and spacing (fr "Source : ", zh "来源："). */
    source: "Źródło: ",
    sourceAria: (host: string) => `Źródło: ${host} (otwiera przeglądarkę)`,
  },

  summary: {
    title: "Podsumowanie",
    lastSession: "Ostatnia sesja",
    selectedSession: "Wybrana sesja",
    empty: "Jeszcze nie ma podsumowania. Zakończ sesję przyciskiem Stop, a zobaczysz tu wykres aplikacji.",
    focusTime: "Czas skupienia",
    cycles: "Ukończone cykle",
    cyclesCount: (n: number) => `${n} ${plural(n, "cykl", "cykle", "cykli")}`,
    breaks: "Czas przerw",
    span: "Start – koniec",
    chartTitle: "Czas skupienia według aplikacji",
    chartSubtitle: (time: string) => `Śledzony czas: ${time}`,
    chartEmpty: "Brak danych o aplikacjach (śledzenie było wyłączone albo sesja była bardzo krótka).",
    other: (n: number) => `Inne (${n})`,
    otherPlain: "Inne",
    idle: "Bezczynność",
    unknown: "Nieznane",
    barAria: (name: string, time: string, pct: string) => `${name}: ${time}, ${pct}`,
    titlesHeading: "Najczęstsze okna",
    titlesHint: "Najedź na aplikację albo przejdź do niej klawiszem Tab, aby zobaczyć jej najczęstsze okna.",
    notSaved: "Ta sesja nie trafiła do historii — zapisywanie historii jest wyłączone w Ustawieniach.",
    history: "Historia",
    historyEmpty: "Historia jest pusta.",
    historyLegend: "Kolory aplikacji w historii",
    demo: "demo",
    showSession: (date: string) => `Pokaż sesję z ${date}`,
    backToLast: "Wróć do ostatniej sesji",
    privacy: "Dane o aplikacjach zostają na tym komputerze.",
    goalLabel: "Cel sesji",
    goalQuestion: "Zrobione?",
    goalYes: "Tak",
    goalNo: "Nie",
    goalGroup: "Czy cel sesji został zrobiony?",
    goalDone: "Zrobione — brawo!",
    goalNotDone: "Nie tym razem — możesz wybrać go znowu w „Co dalej?”.",
    goalOpenAria: (name: string) => `Otwórz ${name} w przeglądarce`,
    /** History entry: short mark next to the stats. */
    goalMark: (done: boolean | null): string => (done === true ? "cel ✓" : done === false ? "cel ✗" : "cel"),
    goalMarkTitle: (name: string, done: boolean | null) =>
      `Cel: ${name}${done === true ? " — zrobione" : done === false ? " — niezrobione" : ""}`,
    /** Added to a history entry's label for screen readers. */
    goalAria: (done: boolean | null): string =>
      done === true ? "cel zrobiony" : done === false ? "cel niezrobiony" : "cel bez odpowiedzi",
  },

  settings: {
    title: "Ustawienia",
    saving: "Zapisuję…",
    saved: "Zapisano",
    shieldsTitle: "Osłony podczas skupienia",
    shieldsLead: "Włączają się na czas skupienia i same wyłączają w przerwie oraz po Stop.",

    notifications: "Wycisz powiadomienia Windows",
    notificationsHint:
      "Włącza tryb „Nie przeszkadzać” w Windows na czas skupienia i wyłącza go w przerwie, po Stop oraz przy zamykaniu aplikacji — także wtedy, gdy był włączony już wcześniej.",

    teams: "Microsoft Teams: „Nie przeszkadzać”",
    teamsHint:
      "Ustawia Twój status w Teams na czas skupienia i czyści go w przerwie. Gdy w Teams wyświetlasz się jako offline (niewidoczny) albo sam ustawisz „Nie przeszkadzać”, FocusMode nie zmienia Twojego statusu. Wymaga jednorazowego połączenia konta.",
    teamsClientId: "Application (client) ID",
    teamsTenant: "Tenant (ID katalogu albo „organizations”)",
    teamsConnect: "Połącz z Teams",
    teamsDisconnect: "Rozłącz",
    teamsConnected: (account: string | null) => (account ? `Połączono jako ${account}` : "Połączono"),
    teamsNotConnected: "Nie połączono",
    teamsNotConfigured: "Najpierw wpisz Application (client) ID.",
    teamsErrorLead: "Połączenie z Teams nie powiodło się:",
    teamsPendingLead: "Otwórz stronę logowania Microsoft i wpisz kod:",
    teamsOpenLink: "Otwórz stronę logowania",
    teamsCopyCode: "Kopiuj kod",
    teamsCopied: "Skopiowano",
    teamsCodeExpires: (min: number) => `Kod wygasa za ok. ${min} ${plural(min, "minutę", "minuty", "minut")}.`,
    teamsHelpTitle: "Jak przygotować Application ID?",
    teamsHelp: [
      "Wejdź na portal Azure → Microsoft Entra ID → Rejestracje aplikacji → Nowa rejestracja.",
      "Typ konta: konta w dowolnym katalogu organizacji. Platforma: „Aplikacje mobilne i klasyczne” (Mobile and desktop).",
      "W „Uwierzytelnianie” włącz „Zezwalaj na publiczne przepływy klientów” (Allow public client flows).",
      "W „Uprawnienia interfejsu API” dodaj delegowane uprawnienia Microsoft Graph: Presence.ReadWrite i User.Read, a dla kalendarza także Calendars.Read.",
      "Skopiuj „Application (client) ID” tutaj. Jeśli administrator wymaga zgody, poproś go o jej udzielenie.",
    ],

    sites: "Blokuj rozpraszające strony",
    sitesHint:
      "W czasie skupienia rozszerzenie przeglądarki zamienia strony z listy poniżej na stronę „Wracaj do pracy”. Nic nie jest wysyłane do internetu.",
    extensionTitle: "Rozszerzenie przeglądarki (Chrome, Edge, Brave, Opera, Vivaldi)",
    extensionStep1: "Otwórz folder rozszerzenia:",
    extensionOpenDir: "Otwórz folder rozszerzenia",
    extensionStep2:
      "W przeglądarce wejdź na chrome://extensions (Edge: edge://extensions, Opera: opera://extensions) i włącz „Tryb dewelopera”.",
    extensionStep3: "Kliknij „Załaduj rozpakowane” i wskaż otwarty folder.",
    extensionConnected: "Rozszerzenie połączone",
    extensionDisconnected: "Rozszerzenie niepołączone — używam awaryjnego wykrywania po tytule okna",
    extensionSeen: (ago: string) => `ostatni kontakt ${ago} temu`,

    keepAwake: "Nie usypiaj ekranu",
    keepAwakeHint: "Komputer i ekran nie przejdą w stan uśpienia w trakcie skupienia.",

    historyTitle: "Historia",
    trackApps: "Zapisuj, z których aplikacji korzystasz w trakcie skupienia",
    trackAppsHint:
      "Co sekundę sprawdzana jest nazwa procesu aktywnego okna. Pasek zadań, menu Start i inne elementy pulpitu Windows są pomijane. Dane zostają na tym komputerze.",
    trackTitles: "Zapisuj tytuły okien",
    trackTitlesHint:
      "Przy każdej aplikacji zapamiętuje kilka najczęstszych tytułów okien (np. kartę przeglądarki albo folder), żeby pokazać je po najechaniu na wykres w Podsumowaniu. Zostają tylko na tym komputerze i nie trafiają do logów.",
    trackTitlesNeedsApps: "Działa, gdy zapisywanie aplikacji jest włączone.",
    keepHistory: "Zapisuj historię sesji",
    keepHistoryHint:
      "Po wyłączeniu podsumowanie dalej pokaże się po Stop, ale sesja nie zostanie zapisana. Wcześniej zapisane sesje zostają, dopóki ich nie usuniesz.",
    clearHistory: "Usuń całą historię",
    clearHistoryHint: "Usuwa z tego komputera wszystkie zapisane sesje i ostatnie podsumowanie.",
    clearHistoryTitle: "Usunąć całą historię?",
    clearHistoryBody:
      "Wszystkie zapisane sesje, wykresy aplikacji i tytuły okien zostaną trwale usunięte z tego komputera. Tego nie da się cofnąć.",
    clearHistoryConfirm: "Tak, usuń historię",
    historyCleared: "Historia została usunięta.",

    appearanceTitle: "Wygląd",

    soundTitle: "Dźwięk",
    chimeSound: "Dźwięk dzwonka",
    chimeSounds: {
      midnight: "Midnight — miękki dzwonek",
      vaporwave: "Vaporwave — syntezatorowy akord",
      lunar: "Lunar — szklane tony",
      autumn: "Autumn — drewniana marimba",
      classic: "Klasyczny — ding-dong",
      soft: "Delikatny — łagodny ton",
      chiptune: "8-bit — arpeggio",
      custom: "Własny…",
    } satisfies Record<ChimeSound, string>,
    chimeSoundHint: "Nazwy pasują do motywów wyglądu, ale dźwięk możesz wybrać niezależnie od motywu.",
    customChimes: "Własne dźwięki",
    customBreak: "Początek przerwy",
    customFocus: "Początek skupienia (opcjonalnie)",
    customNone: "Nie wybrano pliku",
    customDefaultFallback: "Bez własnego pliku zagra dźwięk Midnight.",
    customBreakUsesFocus: "Bez osobnego pliku zagra ten sam dźwięk co na początku skupienia.",
    customFocusUsesBreak: "Bez osobnego pliku zagra ten sam dźwięk co na początku przerwy.",
    customPick: "Wybierz plik…",
    customChange: "Zmień plik…",
    customPickTitle: (label: string) => `Wybierz dźwięk: ${label}`,
    customFilter: "Dźwięk (mp3, wav, ogg, flac, m4a)",
    customRemove: "Usuń",
    customRemoveAria: (label: string) => `Usuń własny dźwięk: ${label}`,
    customHint: "Pliki mp3, wav, ogg, flac lub m4a do 10 MB. FocusMode kopiuje je do swojego folderu danych.",
    customImported: "Dodano własny dźwięk.",
    chimeVolume: "Głośność dzwonka",
    testChime: "Testuj dzwonek:",
    testBreak: "przerwa",
    testFocus: "skupienie",
    musicVolume: "Głośność muzyki",

    remindersTitle: "Przypomnienia w przerwach",
    remindersLead:
      "Własna lista na kolejne przerwy w sesji, np. 1. przerwa: rozciąganie, 2.: woda i spacer z psem, 3.: obiad. Przypomnienie pokaże się na ekranie przerwy i w okienku FocusMode.",
    reminderLabel: (n: number) => `${n}. przerwa`,
    reminderPlaceholders: ["np. rozciąganie", "np. woda i spacer z psem", "np. obiad"],
    reminderPlaceholderOther: "Bez przypomnienia",
    reminderAdd: "Dodaj przerwę",
    reminderRemove: (n: number) => `Usuń przypomnienie dla ${n}. przerwy`,
    reminderUp: (n: number) => `Przesuń przypomnienie ${n}. przerwy wyżej`,
    reminderDown: (n: number) => `Przesuń przypomnienie ${n}. przerwy niżej`,
    reminderChars: (used: number, max: number) => `${used}/${max}`,
    reminderCharsAria: (used: number, max: number) => `Wpisano ${used} z ${max} znaków`,
    reminderMoved: (n: number) => `Przeniesiono na ${n}. pozycję.`,
    reminderRemoved: "Usunięto przypomnienie.",
    remindersEmpty: "Brak przypomnień — w przerwach zobaczysz tylko propozycje.",
    remindersFull: (max: number) => `Lista ma już ${max} ${plural(max, "pozycję", "pozycje", "pozycji")} — to maksimum.`,
    remindersRepeat: "Powtarzaj listę od początku",
    remindersRepeatHint: "Po ostatniej pozycji lista zaczyna się od nowa. Bez tego kolejne przerwy nie mają przypomnienia.",
    remindersInvalid: (max: number, chars: number) =>
      `Najwięcej ${max} przypomnień, każde do ${chars} ${plural(chars, "znaku", "znaków", "znaków")}.`,

    systemTitle: "System",
    launchOnLogin: "Uruchamiaj FocusMode przy logowaniu",
    launchOnLoginHint: "Po zalogowaniu do systemu FocusMode uruchamia się zminimalizowany do zasobnika (ikona obok zegara).",

    launchersTitle: "Launchery gier",
    launchers: "Zamykaj launchery gier na początku skupienia",
    launchersHint:
      "Na starcie każdego skupienia FocusMode zamyka zaznaczone launchery. Zamykane są tylko launchery, nie same gry — ale gra uruchomiona przez launcher może przy tym stracić niezapisany postęp, więc najpierw zapisz grę.",
    launchersPick: "Które launchery zamykać",
    launchersNone: "Zaznacz co najmniej jeden launcher.",
    launchersLastClosed: (names: string) => `Przy ostatnim starcie skupienia zamknięto: ${names}.`,
    launchersLastNone: "Przy ostatnim starcie skupienia żaden z zaznaczonych launcherów nie był uruchomiony.",
    launchersUnavailable: "Zamykanie launcherów działa tylko w Windows.",
    launchersError: "Nie udało się zamknąć któregoś z launcherów.",

    windowTitle: "Okno i powiadomienia",
    closeButton: "Przycisk X (zamknij okno)",
    closeAsk: "Pytaj przy pierwszym zamknięciu",
    closeTray: "Chowaj do zasobnika",
    closeQuit: "Zamykaj aplikację",
    closeHint: "Schowany do zasobnika FocusMode dalej odmierza czas i gra muzykę. Otworzysz go ikoną obok zegara.",
    phasePopups: "Okienka FocusMode przy zmianie fazy",
    phasePopupsHint:
      "Małe okienko w rogu ekranu, gdy zaczyna się skupienie albo przerwa. Zastępuje powiadomienia Windows.",

    tintTitle: "Ciepłe kolory ekranu",
    tint: "Ocieplaj kolory ekranu według pory dnia",
    tintHint:
      "Wieczorem stopniowo ociepla barwy ekranu, a rano przywraca zwykłe. Ocieplenie działa już po złożeniu obrazu, więc nie trafia na zrzuty ekranu ani nagrania.",
    tintMode: "Sposób ocieplania",
    tintModes: {
      nightLight: "Oświetlenie nocne Windows (zalecane)",
      gamma: "Bezpośrednio na karcie graficznej (gamma)",
    } satisfies Record<TintMode, string>,
    tintModeShort: {
      nightLight: "Oświetlenie nocne Windows",
      gamma: "gamma karty graficznej",
    } satisfies Record<TintMode, string>,
    tintModeHints: {
      nightLight:
        "Wbudowane w Windows „Oświetlenie nocne”. FocusMode włącza je według harmonogramu i ustawia jego siłę — bez uprawnień administratora. W Windows 11 24H2 i nowszych nie da się nim sterować z zewnątrz, więc FocusMode ociepla wtedy ekran przez gammę.",
      gamma:
        "Zmienia krzywą kolorów każdego monitora, więc działa na cały ekran, także pasek zadań i gry pełnoekranowe. Nie działa na ekranach w trybie HDR, a Windows może nie pozwolić na bardzo ciepłe ustawienia.",
    } satisfies Record<TintMode, string>,
    tintFrom: "Od",
    tintTo: "Do",
    tintScheduleHint: "Godziny mogą obejmować północ, np. od 18:00 do 9:00.",
    tintStrength: "Siła",
    tintStrengthHint: "Przesuń suwak, aby zobaczyć podgląd na ekranie.",
    tintTransition: "Płynne przejście",
    tintTransitionNone: "Bez przejścia",
    tintTransitionMinutes: (m: number) => `${m} min`,
    tintStatusOff: "Teraz: wyłączone",
    tintStatusLevel: (pct: number) => `Teraz: ${pct}%`,
    tintStatusPreview: (pct: number) => `Podgląd: ${pct}%`,
    tintStatusFallback: (mode: string) => `Wybrany sposób jest niedostępny — używam: ${mode}.`,
    tintStatusCapped: "Windows nie pozwolił na pełną siłę — działa słabsze ocieplenie.",
    tintStatusError: "Nie udało się ocieplić kolorów ekranu — spróbuję ponownie za chwilę.",
    tintStatusRestoreError: "Nie udało się przywrócić zwykłych kolorów ekranu — spróbuję ponownie za chwilę.",
    tintTimeInvalid: "Podaj godzinę, np. 18:00.",
    tintSameTimes: "Godzina „Do” musi być inna niż „Od”.",
    tintStrengthRange: "Podaj siłę od 0 do 100.",
    tintTransitionRange: "Płynne przejście może trwać od 0 do 180 minut.",

    languageTitle: "Język",
    language: "Język aplikacji",
    languageSystem: (name: string | null) => (name ? `Systemowy (${name})` : "Systemowy"),

    demoNote: "Tryb prezentacji: skupienie trwa 40 s, przerwa 20 s.",
  },

  closePrompt: {
    title: "Zminimalizować FocusMode do zasobnika?",
    body: "Minutnik i muzyka będą działać w tle, a FocusMode pokaże małe okienko, gdy zacznie się skupienie albo przerwa.",
    note: "Możesz to zmienić w Ustawieniach.",
    toTray: "Tak, do zasobnika",
    quit: "Nie, zamknij aplikację",
  },

  popup: {
    title: {
      breakStarted: "Czas na przerwę",
      focusStarted: "Czas na skupienie",
      interrupted: "Sesja wstrzymana",
      launchersClosed: "Zamknięto launchery gier",
      meetingReminder: "Spotkanie w czasie przerwy",
      breakHeld: "Przerwa wstrzymana",
      workDigest: "Co dalej?",
    } satisfies Record<PopupKind, string>,
    /** Work digest: `5 nowych zadań i powiadomień czeka na przerwę`. */
    workDigest: (n: number) =>
      `${n} ${plural(n, "nowe zadanie lub powiadomienie czeka", "nowe zadania i powiadomienia czekają", "nowych zadań i powiadomień czeka")} na przerwę`,
    workDigestGeneric: "Nowe zadania i powiadomienia czekają na przerwę",
    /** Meeting reminder: `11:00–11:30 · Planowanie sprintu`. */
    meetingMessage: (range: string, subject: string) => `${range} · ${subject}`,
    meetingTitle: (n: number) => `Spotkanie w czasie ${n}. przerwy`,
    meetingBreakAt: (range: string) => `Planowana przerwa: ${range}`,
    meetingChoices: (subject: string) => `Co zrobić ze spotkaniem ${subject}?`,
    breakHeld: (subject: string) => `Spotkanie: ${subject}. Minutnik przerwy czeka — wznów go po spotkaniu.`,
    breakHeldGeneric: "Trwa spotkanie. Minutnik przerwy czeka — wznów go po spotkaniu.",
    breakHeldTitle: (time: string) => `Przerwa wstrzymana · ${time}`,
    choiceFailed: "Nie udało się zapisać wyboru — wybierz w oknie FocusMode.",
    /** Break popup with a tip: the length goes to the title, the reminder (or one of these lines) under it. */
    breakTitle: (time: string) => `Czas na przerwę · ${time}`,
    breakMusicOn: "Muzyka gra dalej — odejdź na chwilę od biurka.",
    breakMusicPaused: "Muzyka poczeka — odejdź na chwilę od biurka.",
    breakNoMusic: "Odejdź na chwilę od biurka.",
    reminderPrefix: "Twoje przypomnienie:",
    tipPrefix: (category: string) => `Propozycja (${category}):`,
    focusMessage: (cycle: number, time: string) => `Cykl ${cycle}: ${time} skupienia. Osłony włączone.`,
    focusMessageNoShields: (cycle: number, time: string) => `Cykl ${cycle}: ${time} skupienia.`,
    interrupted: "Komputer był uśpiony — wznów, kiedy wrócisz.",
    launchersClosed: (names: string) => `Zamknięto: ${names}`,
    showMain: "Pokaż FocusMode",
    skip: "Pomiń",
    skipBreak: "Pomiń przerwę",
    skipFocus: "Pomiń skupienie",
    dismiss: "Zamknij",
  },

  blocklist: {
    title: "Lista stron — websiteblacklist.txt",
    pathLabel: "Plik listy",
    loading: "Wczytuję listę stron…",
    summary: (domains: number, patterns: number) =>
      `${domains} ${plural(domains, "domena", "domeny", "domen")} · ${patterns} ${plural(patterns, "wyrażenie regularne", "wyrażenia regularne", "wyrażeń regularnych")}`,
    empty: "Lista jest pusta — w czasie skupienia żadna strona nie będzie blokowana.",
    open: "Otwórz websiteblacklist.txt",
    reload: "Wczytaj ponownie",
    reloaded: "Lista stron została wczytana ponownie.",
    autoReload: "Po zapisaniu pliku lista wczytuje się sama — nie trzeba niczego restartować.",
    allValid: "Wszystkie linie poprawne",
    issuesTitle: (n: number) =>
      `${n} ${plural(n, "linia pominięta", "linie pominięte", "linii pominiętych")} — popraw w pliku albo dopisz # na początku linii:`,
    issueLine: (line: number) => `Linia ${line}`,
    /** Limits mirror focus-core `blocklist.rs` (MAX_LINE_CHARS, MAX_DOMAINS, MAX_PATTERNS). */
    issueText: {
      invalidDomain: "Nieprawidłowa domena",
      invalidRegex: "Nieprawidłowe wyrażenie regularne",
      regexTooComplex: "Wyrażenie regularne jest zbyt złożone",
      matchesEverything: "Wyrażenie pasuje do każdego adresu",
      pastedUrl:
        "Adres ze znakiem ? to wyrażenie regularne, które nie pasuje do tego adresu — żeby zablokować tylko ten adres, zamień ? na \\?",
      lineTooLong: "Linia jest za długa (limit 500 znaków)",
      tooManyDomains: "Za dużo domen (limit 2000)",
      tooManyPatterns: "Za dużo wyrażeń regularnych (limit 500)",
    } satisfies Record<BlocklistIssueCode, string>,
    /** `pastedUrl` with the suggested domain (the issue's `message`). */
    pastedUrl: (domain: string) =>
      `Adres ze znakiem ? to wyrażenie regularne, które nie pasuje do tego adresu — żeby zablokować całą stronę, wpisz ${domain}; żeby tylko ten adres, zamień ? na \\?`,
    issueDetailLabel: "Szczegóły (po angielsku)",
    readError: "Nie udało się odczytać pliku — działa poprzednia lista:",
    helpTitle: "Jak zapisywać wpisy?",
    helpLines: "Jedna pozycja w linii. Linie zaczynające się od # to komentarze, puste linie są pomijane.",
    helpDomain: "Zwykła domena blokuje siebie i wszystkie subdomeny (np. www., m.).",
    helpRegex:
      "Wyrażenie regularne to linia w ukośnikach /…/ albo zawierająca któryś ze znaków \\ ^ $ * + ? ( ) [ ] { } |. Jest dopasowywane do całego adresu strony, bez rozróżniania wielkości liter. Kropka oznacza dowolny znak, więc w domenach pisz \\.",
    examplesLabel: "Przykłady:",
    examples: [
      { code: "kwejk.pl", note: "cały kwejk.pl razem z subdomenami" },
      { code: "youtube\\.com/shorts", note: "tylko YouTube Shorts, reszta YouTube działa" },
      { code: "linkedin\\.com/feed", note: "tylko tablica LinkedIn" },
    ],
    /** `BlocklistInfo.extensionSkipped`: regexes Chrome's rule engine rejected; the extension checks them per tab. */
    extensionSkippedTitle: "Przeglądarka nie obsługuje tych wyrażeń jako reguł — sprawdzam je przy wczytaniu karty",
  },

  mascot: {
    /** Settings → Wygląd. */
    setting: "Pokaż maskotkę (aksolotl)",
    settingHint:
      "Aksolotl w słuchawkach obok zegara, w przerwie, w podsumowaniu i w okienkach przy zmianie fazy. Podpowiada dobre praktyki i propozycje na przerwę.",
    /** Bubble labels on the Skupienie page. */
    practiceLabel: "Dobra praktyka",
    nextUpLabel: "Co dalej?",
    /** Said by the axolotl on the Skupienie page (one line picked at random each time): Start, a break, a completed cycle. */
    startLines: [
      "Do dzieła!",
      "Zaczynamy. Powodzenia!",
      "Słuchawki na uszy — działamy!",
      "Jedno zadanie naraz. Lecimy!",
    ],
    breakLines: [
      "Przerwa! Odejdź na chwilę od ekranu.",
      "Czas na przerwę. Zasłużoną!",
      "Przerwa — rozprostuj nogi.",
    ],
    cycleLines: [
      "Cykl zaliczony! Wracamy do skupienia.",
      "Brawo, kolejny cykl za nami!",
      "Świetnie idzie. Następna runda!",
    ],
    /** Idle: a meeting would fall into a break of a session started now (the card below has the choices). */
    conflictHint: (subject: string) =>
      `Spotkanie w czasie przerwy: ${subject}. Zdecyduj poniżej, co z nim zrobić.`,
    /** Skupienie page: the label of the axolotl button (a click tickles it). */
    tickle: "Połaskocz aksolotla",
    /** What the tickled axolotl says (one at random, never the same twice in a row). */
    tickleLines: [
      "Hej, to łaskocze!",
      "Hihi, przestań!",
      "Bup! Wracamy do pracy?",
      "Mam łaskotki na skrzelach!",
      "Głaskanie przyjęte. Skupienie +1!",
      "Ostrożnie, jestem mięciutki!",
      "Jeszcze raz? No dobra!",
      "Masz mnie! Mam tam łaskotki!",
    ],
  },

  calendar: {
    // --- Settings → Kalendarze ---
    title: "Kalendarze",
    lead: "FocusMode sprawdza spotkania z najbliższych godzin i jeszcze przed startem sesji ostrzega, gdy któreś wypada w czasie przerwy. Kalendarze są tylko odczytywane — FocusMode niczego w nich nie zmienia.",
    privacy:
      "Spotkania zostają na tym komputerze. Tokeny logowania i linki do kalendarzy są zapisane w postaci zaszyfrowanej, poza plikiem ustawień, i nie trafiają do logów.",
    horizon: "Sprawdzaj spotkania na najbliższe",
    horizonOption: (h: number) => `${h} ${plural(h, "godzinę", "godziny", "godzin")}`,
    horizonHint: "Przy długiej sesji wybierz dłuższy zakres, żeby objąć wszystkie jej przerwy.",
    lastSync: (time: string) => `Ostatnia synchronizacja: ${time}`,
    neverSynced: "Kalendarze nie były jeszcze odczytane.",
    refresh: "Odśwież teraz",
    refreshing: "Odświeżam…",
    sourceName: {
      microsoft: "Microsoft 365 / Outlook",
      google: "Kalendarz Google",
      ics: "Kalendarze z linku ICS",
    } satisfies Record<CalendarSource, string>,
    sourceShort: {
      microsoft: "Outlook",
      google: "Google",
      ics: "ICS",
    } satisfies Record<CalendarSource, string>,
    state: {
      off: "wyłączony",
      on: "połączony",
      unavailable: "niedostępny",
      error: "błąd",
    } satisfies Record<IntegrationState, string>,
    statusLine: (state: string, account: string | null) => (account ? `${state} · ${account}` : state),
    errorLead: "Nie udało się odczytać kalendarza:",

    microsoft: "Microsoft 365 / Outlook (konto Microsoft z Teams)",
    microsoftHint:
      "Czyta kalendarz przez to samo konto Microsoft, którego używa integracja z Teams (Microsoft Graph, tylko odczyt: Calendars.Read). Obejmuje spotkania w Teams i wszystkie inne wydarzenia z kalendarza.",
    microsoftReconnect:
      "O zgodę na odczyt kalendarza (Calendars.Read) FocusMode prosi tylko przy logowaniu z włączonym kalendarzem. Konto połączone wcześniej tylko dla Teams trzeba połączyć ponownie — bez rozłączania, status w Teams działa dalej. W rejestracji aplikacji w Azure dodaj uprawnienie Calendars.Read.",
    microsoftTeamsAbove: "Konto Microsoft łączysz wyżej, w osłonie „Microsoft Teams: Nie przeszkadzać”.",
    microsoftConnect: "Połącz konto Microsoft",
    /** The Microsoft source answered "reconnect needed": the sign-in has no calendar permission. */
    microsoftNeedsReconnect:
      "To logowanie Microsoft nie ma dostępu do kalendarza. Połącz konto ponownie i zgódź się na odczyt kalendarza.",
    microsoftReconnectButton: "Połącz ponownie",
    microsoftReconnecting: "Łączę…",
    /** Device code of the new sign-in, shown here when the Teams settings with the code are in the shield above. */
    microsoftReconnectCode: (code: string) =>
      `Wpisz kod ${code} na stronie logowania Microsoft (otwartej w przeglądarce, link także wyżej, w osłonie Teams).`,

    google: "Kalendarz Google",
    googleHint:
      "Logowanie odbywa się w przeglądarce, a FocusMode prosi tylko o odczyt kalendarza. Potrzebny jest własny, bezpłatny identyfikator klienta OAuth z Google Cloud.",
    googleClientId: "Identyfikator klienta OAuth (Client ID)",
    googleClientIdPlaceholder: "123456789012-abc….apps.googleusercontent.com",
    googleClientIdInvalid:
      "Identyfikator klienta Google kończy się na .apps.googleusercontent.com i zawiera tylko litery, cyfry, kropki, myślniki i podkreślenia.",
    googleClientSecret: "Tajny klucz klienta (Client secret)",
    googleClientSecretPlaceholder: "GOCSPX-…",
    googleClientSecretHint:
      "Z tego samego klienta „Aplikacja komputerowa” w Google Cloud, co identyfikator powyżej. Google wymaga go przy logowaniu, ale w aplikacjach instalowanych nie traktuje go jako tajnego, więc FocusMode zapisuje go w pliku ustawień (nigdy w logach).",
    googleClientSecretInvalid: (max: number) =>
      `Tajny klucz klienta zawiera tylko litery, cyfry, myślniki i podkreślenia (najwyżej ${max} znaków).`,
    googleConnect: "Połącz z Google",
    googleDisconnect: "Rozłącz",
    googlePending: "Dokończ logowanie w otwartej przeglądarce…",
    googleConnected: (account: string | null) => (account ? `Połączono jako ${account}` : "Połączono"),
    googleNotConnected: "Nie połączono",
    googleNeedsClientId: "Najpierw wpisz identyfikator klienta.",
    googleErrorLead: "Połączenie z Google nie powiodło się:",
    googleHelpTitle: "Jak przygotować identyfikator klienta Google?",
    googleHelp: [
      "Otwórz console.cloud.google.com i utwórz projekt (albo wybierz istniejący).",
      "W „Interfejsy API i usługi” → „Biblioteka” włącz Google Calendar API.",
      "Skonfiguruj ekran zgody OAuth (Google Auth Platform): odbiorcy „Zewnętrzni” (albo „Wewnętrzni” w firmowym Google Workspace), zakres …/auth/calendar.readonly. Dopóki aplikacja jest w trybie testowym, dodaj swój adres e-mail jako użytkownika testowego.",
      "W „Klienci” (Dane logowania) utwórz identyfikator klienta OAuth typu „Aplikacja komputerowa” (Desktop app).",
      "Skopiuj „Identyfikator klienta” (kończy się na .apps.googleusercontent.com) i „Tajny klucz klienta” (zaczyna się od GOCSPX-) do pól powyżej i naciśnij „Połącz z Google”.",
    ],

    ics: "Inne kalendarze (link ICS / iCal)",
    icsHint:
      "Dla kalendarzy z prywatnym linkiem subskrypcji: Outlook i Exchange w firmie, Google, iCloud, Proton, Zoho, Zimbra, Nextcloud i inne. Spotkania cykliczne też są uwzględniane.",
    icsLinks: "Linki do kalendarzy — jeden w każdej linii",
    icsPlaceholder: "https://outlook.office365.com/owa/calendar/…/calendar.ics\nwebcal://…",
    icsSave: "Zapisz linki",
    icsSaved: "Linki do kalendarzy zostały zapisane.",
    icsEdit: "Edytuj linki",
    icsAdd: "Dodaj linki",
    icsCancel: "Anuluj",
    icsRemoveAll: "Usuń wszystkie linki",
    icsRemoved: "Linki do kalendarzy zostały usunięte.",
    /** Status line part: `2 linki`, `brak linków`. */
    icsCount: (n: number) => (n === 0 ? "brak linków" : `${n} ${plural(n, "link", "linki", "linków")}`),
    icsLoading: "Wczytuję zapisane linki…",
    icsInvalidLines: (lines: readonly number[]) =>
      `${lines.length === 1 ? "Linia" : "Linie"} ${lines.join(", ")}: to nie wygląda na link do kalendarza (https:// albo webcal://).`,
    icsSecret:
      "Taki link daje wgląd w kalendarz każdemu, kto go zna. FocusMode zapisuje go zaszyfrowany i nigdy nie pokazuje go w komunikatach o błędach.",
    icsHelpTitle: "Gdzie znaleźć link ICS?",
    icsHelp: [
      {
        name: "Outlook w przeglądarce (Microsoft 365, Exchange)",
        text: "Ustawienia → Kalendarz → Kalendarze udostępnione → „Publikuj kalendarz”: wybierz kalendarz i „Może wyświetlać wszystkie szczegóły”, naciśnij „Publikuj” i skopiuj link ICS. Na firmowym serwerze Exchange publikowanie musi być włączone przez administratora.",
      },
      {
        name: "Kalendarz Google",
        text: "Ustawienia → wybierz kalendarz po lewej → „Integrowanie kalendarza” → „Tajny adres w formacie iCal”.",
      },
      {
        name: "iCloud",
        text: "W Kalendarzu udostępnij kalendarz jako „Kalendarz publiczny” i skopiuj jego link (webcal://).",
      },
      {
        name: "Proton, Zoho, Zimbra, Nextcloud i inne",
        text: "W ustawieniach kalendarza szukaj opcji „Udostępnij”, „Subskrybuj”, „Link ICS” albo „iCal”.",
      },
    ],

    // --- Skupienie: collisions and upcoming meetings ---
    cardTitle: "Spotkania w czasie przerw",
    leadPlanned: "Jeśli zaczniesz teraz, te spotkania wypadną w czasie przerw. Zdecyduj, co zrobić:",
    leadRunning: "Te spotkania wypadają w czasie przerw tej sesji:",
    noConflictsPlanned: "Jeśli zaczniesz teraz, żadne spotkanie nie wypadnie w czasie przerwy.",
    noConflictsRunning: "Żadne spotkanie nie wypada w czasie przerw tej sesji.",
    breakInfo: (n: number, range: string) => `${n}. przerwa: ${range}`,
    choice: {
      extend: "Przedłuż skupienie (muzyka wyciszona)",
      pauseAtBreak: "Wstrzymaj przy przerwie",
      ignore: "Ignoruj",
      remindLater: "Przypomnij później",
    } satisfies Record<MeetingChoice, string>,
    choiceExtend: (end: string) =>
      `Skupienie potrwa do końca spotkania (${end}), z wyciszoną muzyką. Przerwa zacznie się po spotkaniu.`,
    choicePauseAtBreak: "Gdy przyjdzie pora przerwy, minutnik się zatrzyma i poczeka, aż naciśniesz „Wznów”.",
    choiceIgnore: "Sesja biegnie bez zmian.",
    choiceRemindLater: (time: string | null) =>
      time ? `Przypomnę o ${time} — w małym okienku FocusMode.` : "Przypomnę bliżej spotkania.",
    choiceNone: "Wybierz, co zrobić.",
    /** A meeting longer than 4 h (`MeetingConflict.tooLong`): "extend" and "pause at break" are disabled. */
    tooLong: "Spotkanie dłuższe niż 4 h — nie przedłużam skupienia",
    choiceGroup: (subject: string) => `Co zrobić ze spotkaniem ${subject}`,
    upcomingTitle: "Najbliższe spotkania",
    upcomingEmpty: (hours: number) => `Brak spotkań w ciągu ${hours} ${plural(hours, "godziny", "godzin", "godzin")}.`,
    now: "teraz",
    online: "online",
    onlineAria: "spotkanie online",
    tomorrow: "jutro",
    untitled: "(bez tytułu)",
    connectHint: "Połącz kalendarz, a FocusMode przed sesją ostrzeże Cię o spotkaniach w czasie przerw.",
    connectLink: "Ustawienia kalendarzy",
    sourceErrors: (names: string) => `Nie udało się odczytać: ${names}.`,
    sourceErrorsLink: "Szczegóły w Ustawieniach",
    loading: "Wczytuję kalendarz…",

    // --- Banner during a held focus / break ---
    holdExtend: (time: string, subject: string) => `Skupienie przedłużone do ${time} — spotkanie: ${subject}`,
    holdExtendHint: "Muzyka jest wyciszona do końca spotkania, potem zacznie się przerwa.",
    holdPause: (subject: string) => `Przerwa wstrzymana — spotkanie: ${subject}`,
    holdPauseHint: (time: string) =>
      `Minutnik przerwy czeka. Naciśnij „Wznów”, gdy spotkanie się skończy (planowo o ${time}).`,
  },

  work: {
    // --- Settings → Narzędzia pracy ---
    title: "Narzędzia pracy (Co dalej?)",
    lead: "Połącz narzędzia, w których pracujesz, a FocusMode podpowie, czym zająć się w następnej sesji: najpierw zadania przypisane do Ciebie, potem to, na co ktoś czeka (recenzje, poprawki, wzmianki), na końcu pozostałe powiadomienia. Narzędzia są tylko odczytywane — FocusMode niczego w nich nie zmienia.",
    privacy:
      "Tokeny są zapisane w postaci zaszyfrowanej, poza plikiem ustawień, i nie trafiają do logów. FocusMode łączy się tylko z narzędziami włączonymi powyżej; zadania i powiadomienia zostają na tym komputerze i nie są wysyłane do żadnej AI ani innej usługi.",
    sourceName: {
      github: "GitHub",
      gitlab: "GitLab",
      jira: "Jira",
      azureDevops: "Azure DevOps",
      linear: "Linear",
    } satisfies Record<WorkSource, string>,
    /** Text badge in "Co dalej?" (no logos). */
    sourceShort: {
      github: "GitHub",
      gitlab: "GitLab",
      jira: "Jira",
      azureDevops: "Azure",
      linear: "Linear",
    } satisfies Record<WorkSource, string>,
    sourceHint: {
      github: "Zgłoszenia przypisane do Ciebie, pull requesty czekające na Twoją recenzję lub poprawki i — z tokenem klasycznym — powiadomienia.",
      gitlab: "Zgłoszenia przypisane do Ciebie, merge requesty do recenzji i Twoje merge requesty z nieudanym CI. Działa z GitLab.com i z własną instancją.",
      jira: "Zgłoszenia przypisane do Ciebie, z aktywnym sprintem, priorytetem i terminem. Jira Cloud albo Data Center.",
      azureDevops: "Elementy pracy przypisane do Ciebie (z bieżącą iteracją) i pull requesty czekające na Twoją recenzję.",
      linear: "Zgłoszenia przypisane do Ciebie, z bieżącym cyklem, priorytetem i terminem.",
    } satisfies Record<WorkSource, string>,
    state: {
      off: "wyłączone",
      on: "połączono",
      unavailable: "nie połączono",
      error: "błąd",
    } satisfies Record<IntegrationState, string>,
    connectedAs: (account: string | null) => (account ? `Połączono jako ${account}` : "Połączono"),
    syncing: "Token zapisany — trwa pierwsze odczytanie…",
    notConnected: "Nie połączono — zapisz token poniżej.",
    noSecretStore: "W tym systemie nie da się bezpiecznie zapisać tokenu, więc to narzędzie nie działa.",
    tokenInvalid: "Token może zawierać tylko widoczne znaki ASCII, bez spacji (najwyżej 4 KB).",
    needsUrl: {
      gitlab: "Najpierw wpisz adres GitLab.",
      jira: "Najpierw wpisz adres Jira.",
      azureDevops: "Najpierw wpisz nazwę organizacji.",
    },
    errorLead: "Nie udało się odczytać:",

    gitlabUrl: "Adres GitLab",
    gitlabUrlPlaceholder: "https://gitlab.com",
    gitlabUrlHint: "Dla własnej instancji wpisz jej adres, np. https://gitlab.firma.pl.",
    jiraUrl: "Adres Jira",
    jiraUrlPlaceholder: "https://firma.atlassian.net",
    jiraUrlHint: "Adres Twojej witryny Jira Cloud albo serwera Jira Data Center. Możesz wkleić adres z przeglądarki bez części po znaku ?.",
    jiraEmail: "E-mail konta Atlassian (tylko Jira Cloud)",
    jiraEmailPlaceholder: "jan.kowalski@firma.pl",
    jiraEmailHint: "Dla Jira Data Center z osobistym tokenem dostępu zostaw puste.",
    azureOrg: "Organizacja Azure DevOps",
    azureOrgPlaceholder: "firma",
    azureOrgHint: "Nazwa z adresu https://dev.azure.com/<nazwa> — możesz też wkleić cały adres.",
    urlInvalid:
      "Wpisz adres zaczynający się od https://, np. https://firma.atlassian.net — bez loginu, hasła oraz znaków ? i #.",
    emailInvalid: "Wpisz adres e-mail konta, np. jan@firma.pl (bez spacji i dwukropka), albo zostaw puste.",
    azureOrgInvalid: "Wpisz nazwę organizacji: litery, cyfry, myślniki, kropki i podkreślenia (do 64 znaków).",

    token: (name: string) => `Token dostępu ${name}`,
    tokenPlaceholder: "Wklej token",
    tokenPlaceholderReplace: "Wklej nowy token, aby zastąpić zapisany",
    tokenHint: "Token jest zapisywany zaszyfrowany i nie wraca do tego pola.",
    tokenSave: "Zapisz token",
    tokenSaving: "Zapisuję…",
    tokenRemove: "Usuń token",
    tokenSaved: (name: string) => `Token ${name} został zapisany.`,
    tokenRemoved: (name: string) => `Token ${name} został usunięty.`,
    helpTitle: "Jak utworzyć token tylko do odczytu?",
    help: {
      github: [
        "Na GitHubie: Settings → Developer settings → Personal access tokens → Fine-grained tokens → Generate new token.",
        "Resource owner: Twoje konto albo organizacja, w której pracujesz (token fine-grained widzi repozytoria tylko jednego właściciela). Repository access: All repositories albo wybrane repozytoria.",
        "Repository permissions, każde tylko „Read-only”: Issues, Pull requests i Metadata (Metadata GitHub dodaje sam). Nic więcej nie zaznaczaj. Ustaw termin ważności.",
      ],
      githubNotifications:
        "Powiadomienia GitHub (wzmianki, prośby o recenzję spoza repozytoriów tokenu) i wyniki GitHub Actions API udostępnia tylko tokenom klasycznym — token fine-grained ich nie odczyta. Jeśli chcesz je widzieć, utwórz zamiast niego token klasyczny (Tokens (classic)) z zakresem notifications. Do prywatnych repozytoriów token klasyczny potrzebuje też zakresu repo, który daje również prawo zapisu — FocusMode z niego nie korzysta, ale dlatego domyślnie polecamy token fine-grained.",
      gitlab: [
        "W GitLab: awatar → Edytuj profil → Tokeny dostępu (Personal access tokens) → Dodaj nowy token (w nowszych wersjach: „Legacy token”).",
        "Zaznacz tylko zakres read_api (odczyt API). Ustaw termin ważności.",
        "Dla własnej instancji GitLab wpisz wyżej jej adres.",
      ],
      jira: [
        "Jira Cloud: wejdź na id.atlassian.com/manage-profile/security/api-tokens i wybierz „Utwórz token API z zakresami” (Create API token with scopes), aplikację Jira i tylko zakresy read:jira-work i read:jira-user. Ustaw krótki termin ważności.",
        "Wyżej wpisz adres witryny i e-mail konta Atlassian. Token z zakresami działa tylko przez bramkę api.atlassian.com — FocusMode sam ją wybierze. Zwykły token bez zakresów też zadziała, ale ma wszystkie uprawnienia Twojego konta (FocusMode tylko odczytuje zgłoszenia).",
        "Jira Data Center: profil → Osobiste tokeny dostępu (Personal Access Tokens) → Utwórz token, z terminem ważności. Pole e-mail zostaw puste.",
      ],
      azureDevops: [
        "Na dev.azure.com/<organizacja>: Ustawienia użytkownika (ikona obok awatara) → Personal access tokens → New Token.",
        "Organization: Twoja organizacja. Scopes: Custom defined → Work Items: Read, Code: Read i Project and Team: Read — nic więcej. Ustaw termin ważności.",
      ],
      linear: [
        "W Linear: Settings → Security & access → Personal API keys → New API key.",
        "Permissions: „Only select permissions” → tylko Read. Dostęp do zespołów: wszystkie albo wybrane.",
      ],
    } satisfies Record<WorkSource, string[]> & { githubNotifications: string },

    holdDuringFocus: "Wstrzymuj powiadomienia podczas skupienia",
    holdDuringFocusHint:
      "Nowe zadania i powiadomienia z czasu skupienia FocusMode pokaże dopiero w przerwie albo przed następną sesją — nic nie przerywa skupienia.",
    lastSync: (time: string) => `Ostatnia synchronizacja: ${time}`,
    neverSynced: "Narzędzia nie były jeszcze odczytane.",
    refresh: "Odśwież teraz",
    refreshing: "Odświeżam…",

    // --- Skupienie: "Co dalej?" ---
    cardTitle: "Co dalej?",
    cardLead: "Najpierw to, co przypisane do Ciebie, potem to, na co ktoś czeka.",
    loading: "Wczytuję zadania…",
    connectHint: "Połącz GitHub, GitLab, Jirę, Azure DevOps albo Linear, a FocusMode podpowie, czym zająć się w tej sesji.",
    connectLink: "Ustawienia narzędzi pracy",
    noTokenHint: "Zapisz token w Ustawieniach, a pojawią się tu Twoje zadania.",
    empty: "Nic na Ciebie nie czeka — brak przypisanych zadań, próśb o recenzję i powiadomień.",
    sourceErrors: (names: string) => `Nie udało się odczytać: ${names}.`,
    sourceErrorsLink: "Szczegóły w Ustawieniach",
    held: (n: number) => `${n} ${plural(n, "nowe", "nowe", "nowych")} z czasu skupienia`,
    newCount: (n: number) => `${n} ${plural(n, "nowe", "nowe", "nowych")}`,
    unseen: "nowe",
    showMore: (n: number) => `Pokaż więcej (${n})`,
    showLess: "Pokaż mniej",
    open: "Otwórz",
    openAria: (name: string) => `Otwórz ${name} w przeglądarce`,
    setGoal: "Ustaw jako cel",
    setGoalAria: (name: string) => `Ustaw ${name} jako cel sesji`,
    isGoal: "Cel sesji",
    due: (date: string) => `termin: ${date}`,
    reasonsLabel: "Dlaczego teraz",
    reason: {
      assigned: "przypisane do Ciebie",
      inSprint: "sprint",
      highPriority: "wysoki priorytet",
      dueToday: "termin dziś",
      overdue: "po terminie",
      inProgress: "w toku",
      blockingOthers: "czeka na Twoją recenzję",
      waitingLong: "długo czeka",
      mentioned: "wzmianka",
    } satisfies Record<WorkReason, string>,
    /** Kinds that no reason names (`assigned`, `reviewRequested` and `mention` already have one). */
    kind: {
      assigned: "zadanie",
      reviewRequested: "recenzja",
      changesRequested: "prośba o poprawki",
      ciFailed: "CI nie przechodzi",
      mention: "wzmianka",
      notification: "powiadomienie",
    } satisfies Record<WorkItemKind, string>,
    /** `PROJ-123 Tytuł` for labels and the goal line. */
    itemName: (key: string, title: string) => (key && title ? `${key} ${title}` : key || title),

    // --- Goal under the timer ---
    goalLabel: "Cel sesji:",
    goalOpenAria: (name: string) => `Cel sesji: ${name}. Otwórz w przeglądarce`,
    goalClear: "Wyczyść cel sesji",
  },

  nudge: {
    title: "Wracaj do pracy",
    body: (domain: string) => `${domain} poczeka do przerwy.`,
    /** The match was a regex from websiteblacklist.txt, not a readable domain. */
    bodyPattern: "Ta strona jest na liście blokowanych i poczeka do przerwy.",
    dismiss: "Zamknij",
  },

  toasts: {
    region: "Powiadomienia",
    dismiss: "Zamknij powiadomienie",
  },

  errors: {
    generic: "Coś poszło nie tak.",
    fieldInvalid: (label: string) => `Nieprawidłowa wartość: ${label}.`,
    fixConfigFirst: "Popraw zaznaczone ustawienia, zanim zaczniesz sesję.",
    unplayable: "Nie można odtworzyć tego linku w aplikacji",
    allUnplayable: "Żadnego elementu z wybranej muzyki nie da się odtworzyć.",
    autoplayBlocked: "Odtwarzanie zostało zablokowane — naciśnij „Następny”, aby spróbować ponownie.",
    youtubeApi: "Nie udało się załadować odtwarzacza YouTube. Sprawdź połączenie z internetem.",
    clipboard: "Nie udało się skopiować do schowka.",
    codes: {
      "config.invalid": "Nieprawidłowe ustawienie.",
      "config.corrupt": "Plik ustawień był uszkodzony — przywrócono ustawienia domyślne (kopia: config.json.bak).",
      "config.read": "Nie udało się odczytać pliku ustawień — używam ustawień domyślnych.",
      "config.save": "Nie udało się zapisać ustawień na dysku — zmiany działają do zamknięcia aplikacji.",
      "audio.device": "Nie można odtworzyć dzwonka — sprawdź urządzenie audio.",
      "shields.notifications": "Nie udało się przełączyć trybu „Nie przeszkadzać” w Windows.",
      "shields.teams": "Nie udało się zmienić statusu w Teams.",
      "shields.keepAwake": "Nie udało się przełączyć blokady usypiania ekranu.",
      "bridge.port": "Port mostka rozszerzenia jest zajęty — blokada stron działa tylko w trybie awaryjnym.",
      "teams.auth": "Logowanie do Teams nie powiodło się.",
      "teams.network": "Brak połączenia z usługami Microsoft — spróbuj ponownie za chwilę.",
      "teams.notConfigured": "Najpierw wpisz Application (client) ID.",
      "teams.unsupported": "Integracja z Teams nie jest dostępna w tym systemie.",
      "teams.disconnect": "Nie udało się rozłączyć konta Teams.",
      "teams.unavailable": "Integracja z Teams nie odpowiada — uruchom aplikację ponownie.",
      "music.folder": "Nie można odczytać folderu z muzyką.",
      "media.url": "Tego linku nie da się otworzyć w oknie YouTube — działa tylko adres YouTube albo YouTube Music.",
      "media.window": "Nie udało się otworzyć okna YouTube albo nim sterować.",
      "persistence.history": "Problem z historią sesji (odczyt lub zapis).",
      "extension.missing": "Nie znaleziono folderu rozszerzenia przeglądarki.",
      "extension.open": "Nie udało się otworzyć folderu rozszerzenia.",
      "blocklist.read": "Nie udało się odczytać pliku websiteblacklist.txt — działa poprzednia lista stron.",
      "blocklist.open": "Nie udało się otworzyć websiteblacklist.txt w edytorze tekstu — otwórz plik ręcznie (ścieżka jest w Ustawieniach).",
      "shields.launchers": "Nie udało się zamknąć któregoś z launcherów gier — sesja trwa dalej.",
      "tint.apply": "Nie udało się ocieplić kolorów ekranu — spróbuję ponownie za chwilę.",
      "tint.restore": "Nie udało się przywrócić zwykłych kolorów ekranu po poprzednim uruchomieniu.",
      "chime.format": "Ten format nie jest obsługiwany — wybierz plik mp3, wav, ogg, flac albo m4a.",
      "chime.read": "Nie udało się odczytać wybranego pliku.",
      "chime.tooLarge": "Plik jest za duży — dzwonek może mieć najwyżej 10 MB.",
      "chime.save": "Nie udało się zapisać dźwięku w folderze danych FocusMode.",
      "chime.decode": "Tego pliku nie da się odtworzyć jako dźwięku — wybierz inny.",
      "music.library": "Nie udało się wczytać wbudowanej muzyki.",
      "popup.window": "Nie udało się pokazać okienka FocusMode.",
      // Codes of src-tauri/src/calendar.rs (+ calendar/model.rs, calendar/fetch.rs).
      "calendar.sync": "Nie udało się odczytać kalendarza — szczegóły w Ustawieniach → Kalendarze.",
      "calendar.unavailable": "Kalendarze nie odpowiadają — uruchom aplikację ponownie.",
      "calendar.unknownMeeting": "Tego spotkania nie ma już w kalendarzu.",
      "calendar.remindLater":
        "Na przypomnienie jest już za późno — spotkanie zaczyna się za 10 minut lub szybciej. Wybierz inną opcję.",
      "calendar.meetingTooLong":
        "Spotkanie dłuższe niż 4 h nie przedłuża skupienia ani nie wstrzymuje przerwy. Wybierz inną opcję.",
      "calendar.googleNotConfigured": "Najpierw wpisz identyfikator klienta Google.",
      "calendar.googleAuth": "Nie udało się rozpocząć logowania do Google.",
      "calendar.disconnect": "Nie udało się usunąć zapisanych danych kalendarza.",
      "calendar.icsUrl": "Któryś z linków nie jest poprawnym linkiem do kalendarza (https:// albo webcal://).",
      "calendar.icsSave": "Nie udało się bezpiecznie zapisać linków do kalendarza.",
      // Developer tools ("Co dalej?"); other `work.*` codes fall back to the `work` area below.
      // Codes of src-tauri/src/work.rs (+ work/model.rs, work/fetch.rs).
      "work.sync": "Nie udało się odczytać narzędzia pracy — szczegóły w Ustawieniach → Narzędzia pracy.",
      "work.unavailable": "Narzędzia pracy nie odpowiadają — uruchom aplikację ponownie.",
      "work.token": "Token może zawierać tylko widoczne znaki ASCII, bez spacji (najwyżej 4 KB) — wklej go ponownie.",
      "work.tokenSave": "Nie udało się bezpiecznie zapisać ani usunąć tokenu.",
      autostart: "Nie udało się zmienić uruchamiania przy logowaniu.",
      "session.unavailable": "Sesja nie odpowiada — uruchom aplikację ponownie.",
      "ipc.unavailable": "Ta funkcja działa tylko w aplikacji FocusMode.",
      "ipc.failed": "Aplikacja nie odpowiedziała poprawnie.",
    } as Record<string, string>,
    areas: {
      config: "Problem z ustawieniami.",
      audio: "Problem z dźwiękiem.",
      shields: "Jedna z osłon nie zadziałała — sesja trwa dalej.",
      teams: "Problem z połączeniem z Teams.",
      bridge: "Problem z mostkiem rozszerzenia przeglądarki.",
      blocklist: "Problem z listą blokowanych stron (websiteblacklist.txt).",
      music: "Problem z muzyką.",
      media: "Problem z oknem YouTube.",
      persistence: "Nie udało się zapisać danych.",
      session: "Problem z sesją.",
      tracker: "Problem ze śledzeniem aplikacji.",
      tint: "Problem z ciepłymi kolorami ekranu.",
      chime: "Problem z własnym dźwiękiem dzwonka.",
      popup: "Problem z okienkiem FocusMode.",
      launchers: "Problem z zamykaniem launcherów gier.",
      window: "Problem z oknem aplikacji.",
      calendar: "Problem z kalendarzem — sesja działa dalej.",
      google: "Problem z kalendarzem Google.",
      ics: "Problem z kalendarzem z linku ICS.",
      work: "Problem z narzędziami pracy (Co dalej?) — sesja działa dalej.",
    } as Record<string, string>,
  },

  fields: {
    focusMinutes: "czas skupienia",
    breakMinutes: "czas przerwy",
    chimeVolume: "głośność dzwonka",
    "music.volume": "głośność muzyki",
    "music.links": "linki",
    "music.streamUrl": "adres strumienia",
    "music.folder": "folder z muzyką",
    "shields.teamsClientId": "Application (client) ID",
    "shields.teamsTenant": "tenant",
    "shields.launchers": "launchery gier",
    "tint.startMinutes": "początek ciepłych kolorów",
    "tint.endMinutes": "koniec ciepłych kolorów",
    "tint.strength": "siła ciepłych kolorów",
    "tint.transitionMinutes": "płynne przejście",
    "tint.mode": "sposób ocieplania",
    language: "język",
    theme: "motyw",
    chimeSound: "dźwięk dzwonka",
    "customChimes.breakFile": "dźwięk początku przerwy",
    "customChimes.focusFile": "dźwięk początku skupienia",
    breakReminders: "przypomnienia w przerwach",
    "calendar.horizonHours": "zakres sprawdzania kalendarza",
    "calendar.googleClientId": "identyfikator klienta Google",
    "calendar.googleClientSecret": "tajny klucz klienta Google",
    "work.gitlabUrl": "adres GitLab",
    "work.jiraUrl": "adres Jira",
    "work.jiraEmail": "e-mail konta Jira",
    "work.azureOrg": "organizacja Azure DevOps",
  } as Record<string, string>,

  dialog: {
    cancel: "Anuluj",
  },

  /** Product names; the order is the order of the checkboxes in Settings (focus-core `launchers::ALL`). */
  launchers: {
    steam: "Steam",
    epic: "Epic Games Launcher",
    gog: "GOG Galaxy",
    ubisoft: "Ubisoft Connect",
    ea: "Aplikacja EA",
    battleNet: "Battle.net",
    riot: "Riot Client",
    xbox: "Aplikacja Xbox",
    amazon: "Amazon Games",
    itch: "itch",
  } satisfies Record<GameLauncher, string>,

  /** Settings → Wygląd: the theme picker. */
  themes: {
    legend: "Motyw aplikacji",
    hint: "Kolory i tło głównego okna oraz okienek faz. Zmiana zapisuje się od razu.",
    selected: "wybrany",
    names: {
      midnight: "Północ",
      vaporwave: "Vaporwave",
      lunar: "Fazy Księżyca",
      autumn: "Przytulna jesień",
      dark: "Ciemny",
      light: "Jasny",
      pixel: "8-bit",
    } satisfies Record<Theme, string>,
    descriptions: {
      midnight: "Ciepła, fioletowa noc z deszczem za oknem. Domyślny.",
      vaporwave: "Róż, cyjan i fiolet: paskowane słońce nad siatką w perspektywie.",
      lunar: "Srebro i indygo pod gwiazdami.",
      autumn: "Brązy i bursztyn, powoli opadające liście.",
      dark: "Neutralna szarość, bez dekoracji w tle.",
      light: "Jasne tło i ciemny tekst o wysokim kontraście, bez dekoracji.",
      pixel: "Kanciaste ramki, twarde cienie i pikselowe gwiazdy.",
    } satisfies Record<Theme, string>,
    /** Added to the "Fazy Księżyca" description: the moon in the background shows today's phase. */
    moonToday: (phase: string, litPct: number) => `Księżyc w tle ma dzisiejszą fazę: ${phase} (${litPct}% tarczy).`,
    moonPhases: {
      new: "nów",
      waxingCrescent: "przybywający sierp",
      firstQuarter: "pierwsza kwadra",
      waxingGibbous: "przybywający garb",
      full: "pełnia",
      waningGibbous: "ubywający garb",
      lastQuarter: "ostatnia kwadra",
      waningCrescent: "ubywający sierp",
    } satisfies Record<MoonPhaseName, string>,
  },

  /** Break suggestions by id (category, length and source in `$lib/tips`). Suggestions only, never health promises. */
  tips: {
    "eyes-20-20-20": "Spójrz przez okno na coś odległego o co najmniej 6 m i patrz tam spokojnie przez 20 sekund.",
    "eyes-green-view": "Znajdź za oknem drzewo, trawę albo kawałek nieba i popatrz na nie przez 40 sekund.",
    "eyes-slow-blinks": "Zamknij powoli oczy, policz do dwóch i otwórz. Powtórz 10 razy w spokojnym tempie.",
    "eyes-look-around":
      "Popatrz kolejno w górę, w dół, w lewo i w prawo, a potem zatocz oczami dwa powolne koła w każdą stronę.",
    "eyes-near-far":
      "Przez 10 sekund patrz na kciuk wyciągniętej ręki, potem 10 sekund na punkt za oknem. Powtórz 5 razy.",
    "eyes-screen-free": "Spędź tę przerwę bez ekranów — odłóż też telefon ekranem do dołu.",
    "stretch-shoulder-rolls": "Zrób 5 powolnych krążeń barkami do tyłu, a potem 5 do przodu.",
    "stretch-shoulder-shrug": "Unieś barki w stronę uszu, przytrzymaj 3–5 sekund i swobodnie opuść. Powtórz 3 razy.",
    "stretch-neck-turn":
      "Usiądź prosto i powoli obróć głowę w lewo, na ile to wygodne. Zostań tak 5 sekund, potem w prawo. 3 razy.",
    "stretch-neck-tilt":
      "Powoli przechyl głowę uchem w stronę barku i przytrzymaj 10 sekund, jeśli to wygodne. Potem druga strona.",
    "stretch-chin-glide":
      "Siedząc prosto, przesuń głowę poziomo do tyłu, nie unosząc brody. Przytrzymaj 5 sekund, powtórz 5 razy.",
    "stretch-chest-open":
      "Usiądź prosto bez opierania się, ściągnij łopatki i rozłóż ręce na boki. Przytrzymaj 10 s, powtórz 3 razy.",
    "stretch-upper-twist":
      "Skrzyżuj ręce na piersi i obróć tułów w lewo, na ile to wygodne. Zostań tak 5 sekund, potem w prawo. 5 razy.",
    "stretch-side-reach":
      "Spleć dłonie nad głową, sięgnij w górę, a potem przechyl się w lewo i w prawo, po 10 sekund, jeśli to wygodne.",
    "stretch-wrists":
      "Wyciągnij rękę i delikatnie zegnij nadgarstek w górę, potem w dół, po 5 sekund. 3 razy, potem druga ręka.",
    "stretch-fingers": "Zaciśnij dłonie w pięści, a potem szeroko rozłóż palce. Powtórz 5 razy, bez pośpiechu.",
    "stretch-prayer":
      "Złóż dłonie przed sobą i powoli opuszczaj nadgarstki do lekkiego rozciągnięcia, nie bólu. Powtórz 3 razy.",
    "stretch-ankles":
      "Na siedząco wyprostuj nogę nad podłogą i kieruj palce stopy na zmianę od siebie i do siebie. 5 razy na nogę.",
    "stretch-standing-back":
      "Stań z lekko ugiętymi kolanami, dłonie na dole pleców, i odchyl się lekko do tyłu, na ile wygodnie. 3 razy.",
    "stretch-routine":
      "Zrób spokojny 10-minutowy zestaw: barki, szyja, skręty tułowia, nadgarstki i kostki — w wygodnym zakresie.",
    "move-walk-5": "Wstań i przejdź się 5 minut — po mieszkaniu, korytarzu albo do kuchni i z powrotem.",
    "move-stairs": "Wejdź po schodach piętro wyżej i zejdź, we własnym tempie. Bez schodów? Pospaceruj po korytarzu.",
    "move-march-seated":
      "Usiądź prosto i na zmianę unoś kolana jak w marszu, na ile to wygodne — po 5 razy każdą nogą.",
    "move-march-standing": "Pomaszeruj w miejscu przez minutę, swobodnie machając rękami.",
    "move-calf-raises": "Stań przy biurku, oprzyj dłonie o blat i 10 razy powoli wspnij się na palce.",
    "move-sit-to-stand": "Na stabilnym krześle bez kółek wstań i usiądź 10 razy w spokojnym tempie.",
    "move-one-song":
      "Włącz jedną ulubioną piosenkę i ruszaj się do niej, jak masz ochotę — na stojąco lub na siedząco.",
    "move-chores": "Zrób coś ruchowego w domu: rozwieś pranie, podlej kwiaty albo wynieś śmieci.",
    "hydrate-glass": "Wypij szklankę wody — spokojnie, małymi łykami.",
    "hydrate-refill": "Przejdź się do kuchni i napełnij butelkę albo kubek wodą na następną sesję.",
    "hydrate-in-reach": "Zanim wrócisz do pracy, postaw szklankę wody w zasięgu ręki.",
    "breathe-count-5":
      "Usiądź wygodnie. Wdech nosem, licząc do 5, wydech ustami, licząc do 5. Powtarzaj przez 5 minut.",
    "breathe-belly-hand":
      "Połóż dłoń na brzuchu i przez minutę obserwuj, jak unosi się przy wdechu i opada przy wydechu.",
    "breathe-long-exhale":
      "Weź wdech nosem, na końcu dobierz jeszcze trochę powietrza, a potem zrób długi wydech ustami. 5 razy.",
    "breathe-open-window": "Otwórz okno i weź 5 spokojnych oddechów: wdech na cztery, wydech na sześć.",
    "breathe-muscle-release":
      "Napnij mięśnie stóp na 5 sekund i rozluźnij. Przejdź tak w górę ciała, aż do barków i twarzy.",
    "posture-reset": "Zanim wrócisz: plecy oparte o oparcie, stopy płasko na podłodze, ramiona luźno przy tułowiu.",
    "posture-screen-check":
      "Sprawdź monitor: górna krawędź na wysokości oczu lub trochę niżej, ekran na długość wyciągniętej ręki.",
    "posture-glare-check":
      "Sprawdź, czy okno albo lampa nie odbijają się w ekranie. Najlepiej, gdy okno jest z boku monitora.",
    "posture-shift": "Usiądź na chwilę inaczej: przesuń się na krześle, zmień ułożenie nóg, opuść barki.",
    "outside-step-out": "Wyjdź na balkon, przed dom albo stań przy otwartym oknie i pobądź tam 2 minuty.",
    "outside-sky": "Wyjdź na zewnątrz i popatrz przez minutę na niebo i chmury.",
    "outside-walk-15": "Wyjdź na 15-minutowy spacer, najlepiej tam, gdzie są drzewa albo trawa. Idź spokojnym tempem.",
    "outside-walk-noticing": "Wyjdź na spacer bez telefonu w ręce i licz po drodze drzewa, ptaki albo psy.",
    "social-chat": "Porozmawiaj chwilę z kimś o czymś zupełnie niezwiązanym z pracą.",
    "social-message": "Napisz krótką wiadomość do bliskiej osoby, po prostu żeby się przywitać.",
    "social-walk-together": "Zaproś kogoś na wspólny krótki spacer, nawet wokół budynku.",
    "social-pet": "Masz zwierzaka? Poświęć mu kilka minut pełnej uwagi.",
    "mind-do-nothing": "Usiądź wygodnie, zamknij oczy i przez 2 minuty po prostu nic nie rób.",
    "mind-one-song": "Posłuchaj uważnie jednej piosenki z zamkniętymi oczami, nie robiąc nic innego.",
    "mind-5-4-3": "Nazwij w myślach 5 rzeczy, które widzisz, 4, które słyszysz, i 3, których możesz dotknąć.",
    "mind-wander": "Popatrz przez okno i pozwól myślom swobodnie błądzić przez 3 minuty.",
    "mind-park-the-task": "Zapisz na kartce jedno zdanie: od czego zaczniesz po przerwie. Potem odłóż ten temat.",
    "mind-three-good": "Pomyśl o trzech drobnych rzeczach, które dziś poszły dobrze.",
    "mind-meal-away": "Zjedz posiłek z dala od biurka i ekranu — przy stole, na ławce albo w towarzystwie.",
    "mind-your-choice": "Zrób coś, na co naprawdę masz ochotę — ta przerwa jest Twoja.",
    "mind-lie-down": "Jeśli możesz, połóż się na 10–15 minut z zamkniętymi oczami. Nastaw budzik.",
    "tidy-desk": "Uprzątnij biurko: odnieś kubki, zbierz kartki i zostaw tylko to, czego potrzebujesz teraz.",
    "tidy-air": "Otwórz szeroko okno na kilka minut i przewietrz pokój.",
    "tidy-light": "Dopasuj światło: zasłoń okno, jeśli razi, albo zapal lampę, jeśli zrobiło się ciemno.",
  },

  /** Good working practices by id (one a day on the idle Skupienie page; sources in `$lib/tips`). */
  workingTips: {
    "gp-screen-position":
      "Ustaw monitor na wprost siebie, ok. 50–70 cm od oczu, z górną krawędzią na wysokości oczu lub trochę niżej.",
    "gp-light-and-glare": "Ustaw ekran bokiem do okna i dopasuj jego jasność do światła w pokoju.",
    "gp-sitting":
      "Plecy na oparciu, stopy płasko na podłodze, łokcie blisko tułowia, nadgarstki w linii z przedramionami.",
    "gp-short-often":
      "Rób krótkie przerwy często i zanim poczujesz zmęczenie: kilka minut co godzinę to lepiej niż długa przerwa co dwie.",
    "gp-20-20-20":
      "W trakcie skupienia co ok. 20 minut oderwij wzrok od ekranu i przez 20 sekund patrz w dal. Pamiętaj o mruganiu.",
    "gp-change-position": "Zmieniaj pozycję: choć raz na godzinę wstań od biurka, nawet na krótką chwilę.",
    "gp-batch-messages":
      "Sprawdzaj pocztę i komunikatory o stałych porach, np. w przerwach, a nie po każdym powiadomieniu.",
    "gp-single-task":
      "Na jedną sesję wybierz jedno zadanie i zamknij karty oraz okna, których do niego nie potrzebujesz.",
    "gp-phone-away": "Na czas skupienia wycisz telefon i odłóż go poza zasięg wzroku.",
    "gp-water-in-reach": "Trzymaj wodę w zasięgu ręki i popijaj ją po trochu przez cały dzień.",
  },

  /** Shown under every stretch suggestion. */
  stretchFootnote: "Ruszaj się w wygodnym zakresie: rozciąganie ma być odczuwalne, ale nie bolesne. Coś boli? Pomiń to ćwiczenie.",

  time: {
    hoursMinutes: (h: number, m: number) => `${h} h ${m} min`,
    hours: (h: number) => `${h} h`,
    minutes: (m: number) => `${m} min`,
    seconds: (s: number) => `${s} s`,
    today: "dziś",
  },
};

/** The shape of every locale file. */
export type Strings = typeof pl;
