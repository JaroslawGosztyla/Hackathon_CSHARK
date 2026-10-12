// All Polish UI texts live here (CLAUDE.md: code in English, UI in Polish).
// No health promises: the app suggests, it never claims effects.
import type { AppError, IntegrationState, MusicSourceKind, NoiseColor, Phase } from "./types";

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
    teamsHint: "Ustawia Twój status w Teams na czas skupienia i czyści go w przerwie. Wymaga jednorazowego połączenia konta.",
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
    sitesHint: "W czasie skupienia rozszerzenie przeglądarki zamienia te strony na stronę „Wracaj do pracy”. Nic nie jest wysyłane do internetu.",
    blocklist: "Blokowane domeny — jedna w każdej linii",
    blocklistCount: (n: number) => `${n} ${plural(n, "domena", "domeny", "domen")}`,
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

    demoNote: "Tryb prezentacji: skupienie trwa 40 s, przerwa 20 s.",
  },

  nudge: {
    title: "Wracaj do pracy",
    body: (domain: string) => `${domain} poczeka do przerwy.`,
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
      music: "Problem z muzyką.",
      media: "Problem z oknem YouTube.",
      persistence: "Nie udało się zapisać danych.",
      session: "Problem z sesją.",
      tracker: "Problem ze śledzeniem aplikacji.",
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
    "shields.blocklist": "lista blokowanych stron",
    "shields.teamsClientId": "Application (client) ID",
    "shields.teamsTenant": "tenant",
  } as Record<string, string>,

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
