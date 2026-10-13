// All Polish UI texts live here (CLAUDE.md: code in English, UI in Polish).
// No health promises: the app suggests, it never claims effects.
import type { TipCategory } from "./tips";
import type {
  AppError,
  BlocklistIssueCode,
  CalendarSource,
  ChimeSound,
  GameLauncher,
  IntegrationState,
  Language,
  MeetingChoice,
  MusicSourceKind,
  NoiseColor,
  Phase,
  PopupKind,
  TintMode,
  WorkItemKind,
  WorkReason,
  WorkSource,
} from "./types";

/** Polish plural form: 1 minuta, 2–4 minuty, 5+ minut (with the 12–14 exception). */
export function plural(n: number, one: string, few: string, many: string): string {
  const abs = Math.abs(Math.trunc(n));
  if (abs === 1) return one;
  const last = abs % 10;
  const lastTwo = abs % 100;
  if (last >= 2 && last <= 4 && !(lastTwo >= 12 && lastTwo <= 14)) return few;
  return many;
}

export const strings = {
  /** Language of these texts; content meant for one country only (the Polish labour-law note) checks it. */
  locale: "pl",
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
    source: "Źródło",
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
    source: "Źródło",
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
    goalMark: (done: boolean | null) => (done === true ? "cel ✓" : done === false ? "cel ✗" : "cel"),
    goalMarkTitle: (name: string, done: boolean | null) =>
      `Cel: ${name}${done === true ? " — zrobione" : done === false ? " — niezrobione" : ""}`,
    /** Added to a history entry's label for screen readers. */
    goalAria: (done: boolean | null) =>
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
    } as const,

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

  /** Native language names (never translated), in the order of the language picker. */
  languageNames: {
    en: "English",
    pl: "Polski",
    de: "Deutsch",
    fr: "Français",
    it: "Italiano",
    uk: "Українська",
    ru: "Русский",
    es: "Español",
    zh: "中文",
    ja: "日本語",
  } satisfies Record<Exclude<Language, "system">, string>,

  time: {
    hoursMinutes: (h: number, m: number) => `${h} h ${m} min`,
    hours: (h: number) => `${h} h`,
    minutes: (m: number) => `${m} min`,
    seconds: (s: number) => `${s} s`,
    today: "dziś",
  },
};

/** Polish text for an `AppError` (exact code, then its area prefix, then a generic line). */
export function errorText(error: AppError): string {
  if (error.code === "config.invalid" && error.field) {
    return strings.errors.fieldInvalid(strings.fields[error.field] ?? error.field);
  }
  const exact = strings.errors.codes[error.code];
  if (exact) return exact;
  const area = error.code.split(".")[0] ?? "";
  return strings.errors.areas[area] ?? strings.errors.generic;
}

/** Every game launcher, in the order of `strings.launchers` (focus-core `launchers::ALL`). */
export const GAME_LAUNCHERS = Object.keys(strings.launchers) as GameLauncher[];

/** Every concrete UI language, in the order of `strings.languageNames`. */
export const LANGUAGES = Object.keys(strings.languageNames) as Exclude<Language, "system">[];

/** `Steam, Epic Games Launcher` */
export function launcherList(launchers: readonly GameLauncher[]): string {
  return launchers.map((l) => strings.launchers[l] ?? l).join(", ");
}
