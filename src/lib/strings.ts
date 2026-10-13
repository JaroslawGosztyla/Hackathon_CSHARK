// All Polish UI texts live here (CLAUDE.md: code in English, UI in Polish).
// No health promises: the app suggests, it never claims effects.
import type {
  AppError,
  BlocklistIssueCode,
  GameLauncher,
  IntegrationState,
  Language,
  MusicSourceKind,
  NoiseColor,
  Phase,
  PopupKind,
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
      stream: "Stream",
      links: "Linki",
      folder: "Folder",
      noise: "Szum",
      none: "Cisza",
    } satisfies Record<MusicSourceKind, string>,
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
    suggestionLabel: "Propozycja na tę przerwę",
    another: "Inna propozycja",
    suggestions: [
      "Wstań i przejdź się kilka minut — choćby do kuchni i z powrotem.",
      "Spleć dłonie nad głową i powoli sięgnij w górę, potem kolejno na boki.",
      "Zrób kilka spokojnych krążeń barkami do przodu i do tyłu.",
      "Podejdź do okna i popatrz chwilę na coś odległego.",
      "Napełnij kubek wodą albo zaparz herbatę.",
      "Otwórz okno i weź kilka spokojnych, głębokich oddechów.",
      "Zaciśnij dłonie w pięści, a potem szeroko rozłóż palce. Powtórz kilka razy.",
      "Przejdź się po schodach albo zrób kilka przysiadów, jeśli masz ochotę.",
    ],
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
    history: "Historia",
    historyEmpty: "Historia jest pusta.",
    historyLegend: "Kolory aplikacji w historii",
    demo: "demo",
    showSession: (date: string) => `Pokaż sesję z ${date}`,
    backToLast: "Wróć do ostatniej sesji",
    privacy: "Dane o aplikacjach zostają na tym komputerze.",
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
      "W „Uprawnienia interfejsu API” dodaj delegowane uprawnienia Microsoft Graph: Presence.ReadWrite i User.Read.",
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

    trackingTitle: "Śledzenie aplikacji",
    trackApps: "Zapisuj, z których aplikacji korzystasz w trakcie skupienia",
    trackAppsHint: "Co sekundę sprawdzana jest tylko nazwa procesu aktywnego okna. Dane zostają na tym komputerze.",

    soundTitle: "Dźwięk",
    chimeVolume: "Głośność dzwonka",
    testChime: "Testuj dzwonek:",
    testBreak: "przerwa",
    testFocus: "skupienie",
    musicVolume: "Głośność muzyki",

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
      "Wieczorem nakłada na ekran półprzezroczystą, ciepłą warstwę, a rano ją zdejmuje. Nie zmienia ustawień monitora ani karty graficznej.",
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
    tintStatusError: "Nie udało się nałożyć ciepłej warstwy — spróbuję ponownie za chwilę.",
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
    } satisfies Record<PopupKind, string>,
    breakMessage: (time: string) => `Przerwa: ${time} — wstań, rozciągnij się, muzyka gra dalej.`,
    breakMessageMusicPaused: (time: string) => `Przerwa: ${time} — wstań, rozciągnij się, muzyka poczeka.`,
    breakMessageNoMusic: (time: string) => `Przerwa: ${time} — wstań i rozciągnij się.`,
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
      "tint.overlay": "Nie udało się nałożyć ciepłej warstwy na ekran.",
      "popup.window": "Nie udało się pokazać okienka FocusMode.",
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
      popup: "Problem z okienkiem FocusMode.",
      launchers: "Problem z zamykaniem launcherów gier.",
      window: "Problem z oknem aplikacji.",
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
    language: "język",
  } as Record<string, string>,

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
