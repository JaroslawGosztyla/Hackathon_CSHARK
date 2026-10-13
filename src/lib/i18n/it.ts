// Italian UI texts (translated from `en.ts`, checked against the Polish source `pl.ts`; same shape).
// Tone: friendly, short and calm; informal "tu", gender-neutral wording where Italian allows it.
// "Pausa" is the break phase, so pausing the timer is "Sospendi" / "Sospeso" (never "Pausa").
// No health promises: the app suggests, it never claims effects.
// "FocusMode" is a name and is never translated; neither is the file name websiteblacklist.txt.
import type { Strings } from "./pl";
import { pluralizer } from "./plural";

const plural = pluralizer("it");

export const it: Strings = {
  appName: "FocusMode",
  tagline: "sessioni di concentrazione lofi",

  nav: {
    label: "Navigazione principale",
    focus: "Concentrazione",
    summary: "Riepilogo",
    settings: "Impostazioni",
    developer: "Sviluppatore",
  },

  phase: {
    idle: "Pronto per iniziare",
    focus: "Concentrazione",
    break: "Pausa",
  },

  phaseAnnouncement: {
    focusStarted: (cycle: number) => `Inizia la concentrazione, ciclo ${cycle}.`,
    breakStarted: "Inizia la pausa.",
    stopped: "Sessione terminata.",
  },

  timer: {
    paused: "Sospeso",
    cycle: (n: number) => `Ciclo ${n}`,
    demo: "demo breve",
    remainingLabel: (phase: string, time: string) => `${phase}: restano ${time}`,
    idleHint: "Imposta i tempi e la musica, poi premi Avvia.",
    focusedSoFar: (time: string) => `Concentrazione in questa sessione: ${time}`,
  },

  controls: {
    start: "Avvia",
    pause: "Sospendi",
    resume: "Riprendi",
    skipToBreak: "Pausa adesso",
    skipToFocus: "Torno a concentrarmi",
    skip: "Salta fase",
    stop: "Stop",
    sessionControls: "Controlli della sessione",
  },

  timing: {
    title: "Tempo",
    focus: "Concentrazione (min)",
    break: "Pausa (min)",
    presets: "Preimpostazioni",
    preset: (focus: number, brk: number) => `${focus} / ${brk}`,
    presetAria: (focus: number, brk: number) =>
      `${focus} ${plural(focus, { one: "minuto", other: "minuti" })} di concentrazione, ${brk} ${plural(brk, { one: "minuto", other: "minuti" })} di pausa`,
    rangeFocus: "Inserisci un numero di minuti da 1 a 240.",
    rangeBreak: "Inserisci un numero di minuti da 1 a 120.",
  },

  music: {
    title: "Musica",
    sourceLabel: "Sorgente musicale",
    source: {
      library: "Open Lo-Fi (integrata)",
      stream: "Stream",
      links: "Link",
      folder: "Cartella",
      noise: "Rumore",
      none: "Silenzio",
    },
    libraryName: "Open Lo-Fi",
    libraryHint: "Brani lo-fi rilassanti inclusi in FocusMode — suonano anche senza internet.",
    libraryCredit: "Musica: Open Lo-Fi di btahir, licenza CC0 (pubblico dominio).",
    libraryCreditLink: "github.com/btahir/open-lofi",
    libraryCreditAria: "Open Lo-Fi su GitHub (apre il browser)",
    libraryEmpty: "La musica integrata non è stata trovata — scegli un'altra sorgente.",
    streamUrl: "Indirizzo dello stream (YouTube in diretta o radio online)",
    streamPlaceholder: "https://www.youtube.com/watch?v=…",
    lofiPreset: "Lofi Girl — radio in diretta",
    lofiPresetHint:
      "La diretta attuale del canale Lofi Girl. Suona in sottofondo tramite una finestra nascosta di YouTube (il proprietario non ne consente l'incorporamento nelle app); puoi sospenderla e regolare il volume da FocusMode. La prima volta YouTube mostra la sua finestra di scelta dei cookie — dopo la scelta si nasconde da sola.",
    presetsLabel: "Proposte",
    linksLabel: "Link — uno per riga (YouTube, YouTube Music, playlist, file audio)",
    linksPlaceholder: "https://music.youtube.com/watch?v=…\nhttps://youtu.be/…\nhttps://example.com/brano.mp3",
    linksSummary: (ok: number, bad: number) =>
      bad > 0
        ? `${ok} ${plural(ok, { one: "link riproducibile", other: "link riproducibili" })}, ${bad} ${plural(bad, { one: "non supportato", other: "non supportati" })}`
        : `${ok} ${plural(ok, { one: "link riproducibile", other: "link riproducibili" })}`,
    shuffle: "Ordine casuale",
    pickFolder: "Scegli cartella",
    changeFolder: "Cambia cartella",
    pickFolderTitle: "Scegli una cartella di musica",
    noFolder: "Nessuna cartella scelta.",
    loadingTracks: "Cerco i file audio…",
    trackCount: (n: number) => `${n} ${plural(n, { one: "brano", other: "brani" })}`,
    noTracks: "In questa cartella non ci sono file audio supportati (mp3, m4a, flac, wav, ogg, opus…).",
    noise: {
      brown: "Marrone",
      pink: "Rosa",
      white: "Bianco",
    },
    noiseLabel: "Tipo di rumore",
    noiseHint: "Generato qui sul computer — funziona senza internet.",
    noneHint: "Niente musica — sentirai solo la campanella all'inizio e alla fine della pausa.",
    detected: {
      youtubeVideo: "YouTube — video o diretta",
      youtubePlaylist: "YouTube — playlist",
      youtubeWindow: "YouTube — suona in una piccola finestra di YouTube separata (questo link non si può riprodurre nell'app).",
      audio: "Stream o file audio",
      unsupported: "Questo servizio non è supportato (ad es. Spotify) — usa YouTube o un link audio diretto.",
      invalid: "Questo non sembra un indirizzo http(s).",
    },
    pauseDuringBreak: "Sospendi la musica durante le pause",
    pauseDuringBreakHint: "Di norma la musica continua a suonare, anche durante le pause.",
  },

  player: {
    region: "Lettore musicale",
    idle: "La musica parte con la sessione",
    pausedBySession: "Sospesa",
    pausedForBreak: "Sospesa durante la pausa",
    pausedForMeeting: "Silenziata durante la riunione",
    loading: "Caricamento…",
    silence: "Silenzio",
    next: "Successivo",
    volume: "Volume della musica",
    expand: "Mostra video",
    collapse: "Nascondi video",
    noiseTitle: (color: string) => `Rumore ${color.toLowerCase()}`,
    youtubeFallbackTitle: "YouTube",
    trackOf: (i: number, n: number) => `${i} di ${n}`,
    sourceNotReady: "Scegli una sorgente musicale nella schermata Concentrazione.",
    windowMode: "Suona in sottofondo tramite YouTube — il proprietario non ne consente l'incorporamento nelle app.",
    windowModeShort: "Suona in sottofondo tramite YouTube",
    showWindow: "Mostra finestra",
    showWindowAria: "Mostra la finestra di YouTube",
    windowClosed: "La finestra di YouTube è stata chiusa",
    reopenWindow: "Riapri",
    reopenWindowAria: "Riapri la finestra di YouTube",
  },

  youtube: {
    title: "Account YouTube",
    checking: "Controllo dell'accesso a YouTube…",
    signedIn: "Accesso a YouTube effettuato",
    signedInHint:
      "La musica di YouTube suona in sottofondo tramite la finestra di YouTube, con il tuo account — con Premium, senza pubblicità.",
    notSignedIn: "Accesso a YouTube non effettuato",
    notSignedInHint: "Hai YouTube Premium? Accedi e la musica di YouTube suonerà senza pubblicità.",
    signIn: "Accedi a YouTube",
    signInOpen: "Accedi nella finestra di Google che si è aperta — si chiuderà da sola dopo l'accesso.",
    showSignIn: "Mostra la finestra di accesso",
    signOut: "Esci",
    rejected:
      "Google ha rifiutato l'accesso nella finestra dell'app (“Questo browser o questa app potrebbero non essere sicuri”) — la musica continua senza accesso.",
    privacy:
      "Accedi sulla pagina di Google — FocusMode non vede mai la tua password. I cookie di accesso restano solo nel browser dell'app su questo computer; YouTube salva ciò che ascolti nella cronologia del tuo account.",
    playerWindowHint: "Suona in sottofondo tramite la finestra di YouTube, con il tuo account YouTube.",
    pickerHint: "Hai YouTube Premium?",
    pickerLink: "Accedi nelle Impostazioni",
    pickerSignedIn:
      "Accesso a YouTube effettuato — la musica di YouTube suona in sottofondo tramite la finestra di YouTube.",
  },

  focusView: {
    nowUsing: "Stai usando",
    nowUsingEmpty: "Raccolgo i dati sulle app…",
    trackingOff: "Il monitoraggio delle app è disattivato nelle Impostazioni.",
    idle: "Inattività",
    unknown: "Sconosciuta",
    shields: "Protezioni",
    shieldsNone: "Nessuna protezione è attiva — puoi attivarle nelle Impostazioni.",
  },

  shieldChip: {
    notifications: "Notifiche silenziate",
    teams: "Teams: Non disturbare",
    sites: "Blocco dei siti",
    sitesFallback: "Blocco dei siti (modalità di riserva)",
    keepAwake: "Lo schermo resta acceso",
    launchers: "Launcher di gioco",
    launchersClosed: (names: string) => `Chiusi: ${names}`,
    launchersNothing: "Nessuno dei launcher selezionati era in esecuzione.",
    state: {
      off: "disattivata",
      on: "attiva",
      unavailable: "non disponibile",
      error: "errore",
    },
    chipAria: (name: string, state: string) => `${name}: ${state}`,
  },

  breakView: {
    title: "È ora di una pausa",
    lead: "Allontanati un po' dalla scrivania. La musica può continuare a suonare.",
    reminderLabel: (n: number) => `Il tuo promemoria · pausa ${n}`,
    suggestionLabel: "Un suggerimento per questa pausa",
    another: "Un altro suggerimento",
    source: "Fonte: ",
    sourceAria: (host: string) => `Fonte del suggerimento: ${host} (apre il browser)`,
    categories: {
      eyes: "Occhi",
      stretch: "Stretching",
      move: "Movimento",
      hydrate: "Acqua",
      breathe: "Respiro",
      posture: "Postazione",
      outside: "All'aperto",
      social: "Persone",
      mind: "Un momento per la mente",
      tidy: "Ambiente",
    },
  },

  idleTips: {
    practiceTitle: "Buona pratica",
    lawTitle: "Buono a sapersi",
    source: "Fonte: ",
    sourceAria: (host: string) => `Fonte: ${host} (apre il browser)`,
  },

  summary: {
    title: "Riepilogo",
    lastSession: "Ultima sessione",
    selectedSession: "Sessione selezionata",
    empty: "Non c'è ancora un riepilogo. Termina una sessione con Stop e qui vedrai un grafico delle tue app.",
    focusTime: "Tempo di concentrazione",
    cycles: "Cicli completati",
    cyclesCount: (n: number) => `${n} ${plural(n, { one: "ciclo", other: "cicli" })}`,
    breaks: "Tempo di pausa",
    span: "Inizio – fine",
    chartTitle: "Tempo di concentrazione per app",
    chartSubtitle: (time: string) => `Tempo monitorato: ${time}`,
    chartEmpty: "Nessun dato sulle app (il monitoraggio era disattivato o la sessione è stata molto breve).",
    other: (n: number) => `Altre (${n})`,
    otherPlain: "Altre",
    idle: "Inattività",
    unknown: "Sconosciuta",
    barAria: (name: string, time: string, pct: string) => `${name}: ${time}, ${pct}`,
    titlesHeading: "Finestre più frequenti",
    titlesHint: "Passa il mouse su un'app, o raggiungila con Tab, per vedere le sue finestre più frequenti.",
    notSaved: "Questa sessione non è stata aggiunta alla cronologia — il salvataggio della cronologia è disattivato nelle Impostazioni.",
    history: "Cronologia",
    historyEmpty: "La cronologia è vuota.",
    historyLegend: "Colori delle app nella cronologia",
    demo: "demo",
    showSession: (date: string) => `Mostra la sessione del ${date}`,
    backToLast: "Torna all'ultima sessione",
    privacy: "I dati sulle app restano su questo computer.",
    goalLabel: "Obiettivo della sessione",
    goalQuestion: "Fatto?",
    goalYes: "Sì",
    goalNo: "No",
    goalGroup: "Hai completato l'obiettivo della sessione?",
    goalDone: "Fatto — ottimo lavoro!",
    goalNotDone: "Non questa volta — puoi sceglierlo di nuovo in “Cosa fare dopo?”.",
    goalOpenAria: (name: string) => `Apri ${name} nel browser`,
    goalMark: (done: boolean | null) => (done === true ? "obiettivo ✓" : done === false ? "obiettivo ✗" : "obiettivo"),
    goalMarkTitle: (name: string, done: boolean | null) =>
      `Obiettivo: ${name}${done === true ? " — fatto" : done === false ? " — non fatto" : ""}`,
    goalAria: (done: boolean | null) =>
      done === true ? "obiettivo completato" : done === false ? "obiettivo non completato" : "obiettivo senza risposta",
  },

  settings: {
    title: "Impostazioni",
    saving: "Salvataggio…",
    saved: "Salvato",
    shieldsTitle: "Protezioni durante la concentrazione",
    shieldsLead: "Si attivano durante la concentrazione e si disattivano da sole nelle pause e dopo lo Stop.",

    notifications: "Silenzia le notifiche di Windows",
    notificationsHint:
      "Attiva “Non disturbare” di Windows durante la concentrazione e lo disattiva nelle pause, dopo lo Stop e alla chiusura dell'app — anche se era già attivo prima.",

    teams: "Microsoft Teams: “Non disturbare”",
    teamsHint:
      "Imposta il tuo stato di Teams durante la concentrazione e lo rimuove nelle pause. Se in Teams risulti offline, o hai impostato tu “Non disturbare”, FocusMode non tocca il tuo stato. Serve collegare l'account una sola volta.",
    teamsClientId: "Application (client) ID",
    teamsTenant: "Tenant (ID della directory o “organizations”)",
    teamsConnect: "Connetti a Teams",
    teamsDisconnect: "Disconnetti",
    teamsConnected: (account: string | null) => (account ? `Connesso come ${account}` : "Connesso"),
    teamsNotConnected: "Non connesso",
    teamsNotConfigured: "Inserisci prima l'Application (client) ID.",
    teamsErrorLead: "Impossibile connettersi a Teams:",
    teamsPendingLead: "Apri la pagina di accesso Microsoft e inserisci il codice:",
    teamsOpenLink: "Apri la pagina di accesso",
    teamsCopyCode: "Copia codice",
    teamsCopied: "Copiato",
    teamsCodeExpires: (min: number) =>
      `Il codice scade tra circa ${min} ${plural(min, { one: "minuto", other: "minuti" })}.`,
    teamsHelpTitle: "Come ottengo un Application ID?",
    teamsHelp: [
      "Vai al portale di Azure → Microsoft Entra ID → Registrazioni app → Nuova registrazione.",
      "Tipo di account: account in qualsiasi directory organizzativa. Piattaforma: “Applicazioni per dispositivi mobili e desktop” (Mobile and desktop applications).",
      "In “Autenticazione” attiva “Consenti flussi client pubblici” (Allow public client flows).",
      "In “Autorizzazioni API” aggiungi le autorizzazioni delegate di Microsoft Graph Presence.ReadWrite e User.Read, più Calendars.Read per il calendario.",
      "Copia qui l'“ID applicazione (client)” (Application (client) ID). Se serve il consenso dell'amministratore, chiedigli di concederlo.",
    ],

    sites: "Blocca i siti che distraggono",
    sitesHint:
      "Durante la concentrazione, l'estensione del browser sostituisce i siti dell'elenco qui sotto con una pagina “Torna al lavoro”. Nulla viene inviato su internet.",
    extensionTitle: "Estensione del browser (Chrome, Edge, Brave, Opera, Vivaldi)",
    extensionStep1: "Apri la cartella dell'estensione:",
    extensionOpenDir: "Apri la cartella dell'estensione",
    extensionStep2:
      "Nel browser vai su chrome://extensions (Edge: edge://extensions, Opera: opera://extensions) e attiva la “Modalità sviluppatore”.",
    extensionStep3: "Fai clic su “Carica estensione non pacchettizzata” (Load unpacked) e scegli la cartella che hai aperto.",
    extensionConnected: "Estensione connessa",
    extensionDisconnected: "Estensione non connessa — uso il rilevamento di riserva dal titolo della finestra",
    extensionSeen: (ago: string) => `ultimo contatto ${ago} fa`,

    keepAwake: "Mantieni lo schermo acceso",
    keepAwakeHint: "Durante la concentrazione il computer e lo schermo non vanno in sospensione.",

    historyTitle: "Cronologia",
    trackApps: "Registra quali app usi durante la concentrazione",
    trackAppsHint:
      "Ogni secondo FocusMode controlla il nome del processo della finestra attiva. La barra delle applicazioni, il menu Start e le altre parti del desktop di Windows vengono ignorate. I dati restano su questo computer.",
    trackTitles: "Registra i titoli delle finestre",
    trackTitlesHint:
      "Per ogni app, FocusMode ricorda alcuni dei suoi titoli di finestra più frequenti (ad es. una scheda del browser o una cartella) e li mostra quando passi il mouse sul grafico nel Riepilogo. Restano solo su questo computer e non finiscono mai nei log.",
    trackTitlesNeedsApps: "Funziona quando la registrazione delle app è attiva.",
    keepHistory: "Salva la cronologia delle sessioni",
    keepHistoryHint:
      "Se è disattivato, il riepilogo compare comunque dopo lo Stop, ma la sessione non viene salvata. Le sessioni salvate in precedenza restano finché non le elimini.",
    clearHistory: "Elimina tutta la cronologia",
    clearHistoryHint: "Elimina da questo computer tutte le sessioni salvate e l'ultimo riepilogo.",
    clearHistoryTitle: "Eliminare tutta la cronologia?",
    clearHistoryBody:
      "Tutte le sessioni salvate, i grafici delle app e i titoli delle finestre verranno eliminati definitivamente da questo computer. L'operazione non si può annullare.",
    clearHistoryConfirm: "Sì, elimina la cronologia",
    historyCleared: "La cronologia è stata eliminata.",

    appearanceTitle: "Aspetto",

    soundTitle: "Suono",
    chimeSound: "Suono della campanella",
    chimeSounds: {
      midnight: "Mezzanotte — campana morbida",
      vaporwave: "Vaporwave — accordo di synth",
      lunar: "Fasi lunari — toni cristallini",
      autumn: "Autunno — marimba di legno",
      classic: "Classico — din-don",
      soft: "Delicato — tono morbido",
      chiptune: "8-bit — arpeggio",
      custom: "Personalizzato…",
    },
    chimeSoundHint: "I nomi corrispondono ai temi dell'aspetto, ma puoi scegliere il suono indipendentemente dal tema.",
    customChimes: "Suoni personalizzati",
    customBreak: "Inizio della pausa",
    customFocus: "Inizio della concentrazione (facoltativo)",
    customNone: "Nessun file scelto",
    customDefaultFallback: "Senza un file tuo, suona Mezzanotte.",
    customBreakUsesFocus: "Senza un file separato, suona lo stesso suono dell'inizio della concentrazione.",
    customFocusUsesBreak: "Senza un file separato, suona lo stesso suono dell'inizio della pausa.",
    customPick: "Scegli file…",
    customChange: "Cambia file…",
    customPickTitle: (label: string) => `Scegli un suono: ${label}`,
    customFilter: "Suono (mp3, wav, ogg, flac, m4a)",
    customRemove: "Rimuovi",
    customRemoveAria: (label: string) => `Rimuovi il suono personalizzato: ${label}`,
    customHint: "File mp3, wav, ogg, flac o m4a fino a 10 MB. FocusMode li copia nella sua cartella dati.",
    customImported: "Il tuo suono è stato aggiunto.",
    chimeVolume: "Volume della campanella",
    testChime: "Prova la campanella:",
    testBreak: "pausa",
    testFocus: "concentrazione",
    musicVolume: "Volume della musica",

    remindersTitle: "Promemoria per le pause",
    remindersLead:
      "Un elenco tutto tuo per le pause di una sessione, ad es. pausa 1: stretching, 2: acqua e passeggiata col cane, 3: pranzo. Il promemoria compare nella schermata della pausa e nel pop-up di FocusMode.",
    reminderLabel: (n: number) => `Pausa ${n}`,
    reminderPlaceholders: ["ad es. stretching", "ad es. acqua e passeggiata col cane", "ad es. pranzo"],
    reminderPlaceholderOther: "Nessun promemoria",
    reminderAdd: "Aggiungi una pausa",
    reminderRemove: (n: number) => `Rimuovi il promemoria della pausa ${n}`,
    reminderUp: (n: number) => `Sposta su il promemoria della pausa ${n}`,
    reminderDown: (n: number) => `Sposta giù il promemoria della pausa ${n}`,
    reminderChars: (used: number, max: number) => `${used}/${max}`,
    reminderCharsAria: (used: number, max: number) => `${used} di ${max} caratteri digitati`,
    reminderMoved: (n: number) => `Spostato in posizione ${n}.`,
    reminderRemoved: "Promemoria rimosso.",
    remindersEmpty: "Nessun promemoria — nelle pause vedrai solo i suggerimenti.",
    remindersFull: (max: number) =>
      `L'elenco ha già ${max} ${plural(max, { one: "voce", other: "voci" })} — è il massimo.`,
    remindersRepeat: "Ripeti l'elenco dall'inizio",
    remindersRepeatHint:
      "Dopo l'ultima voce, l'elenco ricomincia. Senza questa opzione, le pause successive non hanno promemoria.",
    remindersInvalid: (max: number, chars: number) =>
      `Al massimo ${max} promemoria, ognuno fino a ${chars} ${plural(chars, { one: "carattere", other: "caratteri" })}.`,

    systemTitle: "Sistema",
    launchOnLogin: "Avvia FocusMode all'accesso",
    launchOnLoginHint:
      "Dopo l'accesso, FocusMode si avvia ridotto a icona nell'area di notifica (l'icona accanto all'orologio).",

    developerTitle: "Sviluppatore",
    developerMode: "Attiva la modalità sviluppatore",
    developerModeHint:
      "Aggiunge una scheda Sviluppatore con pulsanti che mostrano i pop-up, le notifiche e le reazioni della mascotte, per i test e gli screenshot. I dati di esempio non vengono mai salvati.",

    launchersTitle: "Launcher di gioco",
    launchers: "Chiudi i launcher di gioco all'inizio della concentrazione",
    launchersHint:
      "Ogni volta che inizia la concentrazione, FocusMode chiude i launcher selezionati. Vengono chiusi solo i launcher, non i giochi — ma un gioco avviato da un launcher potrebbe perdere i progressi non salvati, quindi salva prima la partita.",
    launchersPick: "Quali launcher chiudere",
    launchersNone: "Seleziona almeno un launcher.",
    launchersLastClosed: (names: string) => `All'ultimo inizio della concentrazione, FocusMode ha chiuso: ${names}.`,
    launchersLastNone: "All'ultimo inizio della concentrazione, nessuno dei launcher selezionati era in esecuzione.",
    launchersUnavailable: "La chiusura dei launcher funziona solo su Windows.",
    launchersError: "Non è stato possibile chiudere uno dei launcher.",

    windowTitle: "Finestra e pop-up",
    closeButton: "Il pulsante X (chiudi finestra)",
    closeAsk: "Chiedi la prima volta",
    closeTray: "Nascondi nell'area di notifica",
    closeQuit: "Esci dall'app",
    closeHint:
      "Nascosto nell'area di notifica, FocusMode continua a contare il tempo e a suonare la musica. Lo riapri dall'icona accanto all'orologio.",
    phasePopups: "Pop-up di FocusMode al cambio di fase",
    phasePopupsHint:
      "Una piccola finestra nell'angolo dello schermo quando inizia la concentrazione o una pausa. Sostituisce le notifiche di Windows.",

    tintTitle: "Colori caldi dello schermo",
    tint: "Riscalda i colori dello schermo in base all'ora del giorno",
    tintHint:
      "La sera i colori dello schermo diventano gradualmente più caldi, e al mattino tornano quelli abituali. Il riscaldamento si applica dopo la composizione dell'immagine, quindi non compare negli screenshot né nelle registrazioni.",
    tintMode: "Come riscaldare",
    tintModes: {
      nightLight: "Luce notturna di Windows (consigliata)",
      gamma: "Direttamente sulla scheda grafica (gamma)",
    },
    tintModeShort: {
      nightLight: "Luce notturna di Windows",
      gamma: "gamma della scheda grafica",
    },
    tintModeHints: {
      nightLight:
        "La “Luce notturna” integrata in Windows. FocusMode la attiva secondo l'orario e ne imposta l'intensità — senza diritti di amministratore. In Windows 11 24H2 e versioni successive non si può controllare dall'esterno, quindi FocusMode riscalda lo schermo tramite la gamma.",
      gamma:
        "Modifica la curva dei colori di ogni monitor, quindi copre tutto lo schermo, compresi la barra delle applicazioni e i giochi a schermo intero. Non funziona sugli schermi in modalità HDR, e Windows potrebbe non consentire impostazioni molto calde.",
    },
    tintFrom: "Da",
    tintTo: "A",
    tintScheduleHint: "L'intervallo può superare la mezzanotte, ad es. dalle 18:00 alle 9:00.",
    tintStrength: "Intensità",
    tintStrengthHint: "Sposta il cursore per vedere un'anteprima sullo schermo.",
    tintTransition: "Transizione graduale",
    tintTransitionNone: "Nessuna transizione",
    tintTransitionMinutes: (m: number) => `${m} min`,
    tintStatusOff: "Ora: disattivato",
    tintStatusLevel: (pct: number) => `Ora: ${pct}%`,
    tintStatusPreview: (pct: number) => `Anteprima: ${pct}%`,
    tintStatusFallback: (mode: string) => `Il metodo scelto non è disponibile — uso: ${mode}.`,
    tintStatusCapped: "Windows non ha consentito l'intensità piena — viene applicato un riscaldamento più leggero.",
    tintStatusError: "Non è stato possibile riscaldare i colori dello schermo — riprovo tra poco.",
    tintStatusRestoreError: "Non è stato possibile ripristinare i colori abituali dello schermo — riprovo tra poco.",
    tintTimeInvalid: "Inserisci un orario, ad es. 18:00.",
    tintSameTimes: "“A” deve essere diverso da “Da”.",
    tintStrengthRange: "Inserisci un'intensità da 0 a 100.",
    tintTransitionRange: "La transizione graduale può durare da 0 a 180 minuti.",

    languageTitle: "Lingua",
    language: "Lingua dell'app",
    languageSystem: (name: string | null) => (name ? `Sistema (${name})` : "Sistema"),

    demoNote: "Modalità demo: la concentrazione dura 40 s, la pausa 20 s.",
  },

  closePrompt: {
    title: "Ridurre FocusMode nell'area di notifica?",
    body: "Il timer e la musica continuano in sottofondo, e FocusMode mostra un piccolo pop-up quando inizia la concentrazione o una pausa.",
    note: "Puoi cambiarlo nelle Impostazioni.",
    toTray: "Sì, nell'area di notifica",
    quit: "No, esci dall'app",
  },

  popup: {
    title: {
      breakStarted: "È ora di una pausa",
      focusStarted: "È ora di concentrarsi",
      interrupted: "Sessione sospesa",
      launchersClosed: "Launcher di gioco chiusi",
      meetingReminder: "Riunione durante una pausa",
      breakHeld: "Pausa in attesa",
      workDigest: "Cosa fare dopo?",
    },
    workDigest: (n: number) =>
      `${n} ${plural(n, { one: "nuova attività o notifica aspetta", other: "nuove attività e notifiche aspettano" })} la pausa`,
    workDigestGeneric: "Nuove attività e notifiche aspettano la pausa",
    meetingMessage: (range: string, subject: string) => `${range} · ${subject}`,
    meetingTitle: (n: number) => `Riunione durante la pausa ${n}`,
    meetingBreakAt: (range: string) => `Pausa prevista: ${range}`,
    meetingChoices: (subject: string) => `Cosa fare con la riunione ${subject}?`,
    breakHeld: (subject: string) =>
      `Riunione: ${subject}. Il timer della pausa aspetta — riprendilo dopo la riunione.`,
    breakHeldGeneric: "C'è una riunione in corso. Il timer della pausa aspetta — riprendilo dopo la riunione.",
    breakHeldTitle: (time: string) => `Pausa in attesa · ${time}`,
    choiceFailed: "Non è stato possibile salvare la scelta — scegli nella finestra di FocusMode.",
    breakTitle: (time: string) => `È ora di una pausa · ${time}`,
    breakMusicOn: "La musica continua — allontanati un po' dalla scrivania.",
    breakMusicPaused: "La musica ti aspetta — allontanati un po' dalla scrivania.",
    breakNoMusic: "Allontanati un po' dalla scrivania.",
    reminderPrefix: "Il tuo promemoria:",
    tipPrefix: (category: string) => `Suggerimento (${category}):`,
    focusMessage: (cycle: number, time: string) => `Ciclo ${cycle}: ${time} di concentrazione. Protezioni attive.`,
    focusMessageNoShields: (cycle: number, time: string) => `Ciclo ${cycle}: ${time} di concentrazione.`,
    interrupted: "Il computer era in sospensione — riprendi quando torni.",
    launchersClosed: (names: string) => `Chiusi: ${names}`,
    showMain: "Mostra FocusMode",
    skip: "Salta",
    skipBreak: "Salta la pausa",
    skipFocus: "Salta la concentrazione",
    dismiss: "Chiudi",
  },

  blocklist: {
    title: "Elenco dei siti — websiteblacklist.txt",
    pathLabel: "File dell'elenco",
    loading: "Caricamento dell'elenco dei siti…",
    summary: (domains: number, patterns: number) =>
      `${domains} ${plural(domains, { one: "dominio", other: "domini" })} · ${patterns} ${plural(patterns, { one: "espressione regolare", other: "espressioni regolari" })}`,
    empty: "L'elenco è vuoto — durante la concentrazione nessun sito verrà bloccato.",
    open: "Apri websiteblacklist.txt",
    reload: "Ricarica",
    reloaded: "L'elenco dei siti è stato ricaricato.",
    autoReload: "Quando salvi il file, l'elenco si ricarica da solo — non serve riavviare nulla.",
    allValid: "Tutte le righe sono valide",
    issuesTitle: (n: number) =>
      `${n} ${plural(n, { one: "riga ignorata", other: "righe ignorate" })} — correggi il file o metti # all'inizio della riga:`,
    issueLine: (line: number) => `Riga ${line}`,
    issueText: {
      invalidDomain: "Dominio non valido",
      invalidRegex: "Espressione regolare non valida",
      regexTooComplex: "L'espressione regolare è troppo complessa",
      matchesEverything: "L'espressione corrisponde a qualsiasi indirizzo",
      pastedUrl:
        "Un indirizzo con ? è un'espressione regolare che non corrisponde a quell'indirizzo — per bloccare solo questo indirizzo, sostituisci ? con \\?",
      lineTooLong: "La riga è troppo lunga (limite: 500 caratteri)",
      tooManyDomains: "Troppi domini (limite: 2000)",
      tooManyPatterns: "Troppe espressioni regolari (limite: 500)",
    },
    pastedUrl: (domain: string) =>
      `Un indirizzo con ? è un'espressione regolare che non corrisponde a quell'indirizzo — per bloccare tutto il sito, inserisci ${domain}; per bloccare solo questo indirizzo, sostituisci ? con \\?`,
    issueDetailLabel: "Dettagli (in inglese)",
    readError: "Non è stato possibile leggere il file — resta in uso l'elenco precedente:",
    helpTitle: "Come scrivo le voci?",
    helpLines: "Una voce per riga. Le righe che iniziano con # sono commenti; le righe vuote vengono ignorate.",
    helpDomain: "Un dominio semplice blocca sé stesso e tutti i suoi sottodomini (ad es. www., m.).",
    helpRegex:
      "Un'espressione regolare è una riga tra barre /…/ oppure una riga che contiene uno dei caratteri \\ ^ $ * + ? ( ) [ ] { } |. Viene confrontata con l'intero indirizzo della pagina, senza distinguere maiuscole e minuscole. Il punto indica un carattere qualsiasi, quindi nei domini scrivi \\.",
    examplesLabel: "Esempi:",
    examples: [
      { code: "reddit.com", note: "tutto reddit.com, sottodomini compresi" },
      { code: "youtube\\.com/shorts", note: "solo YouTube Shorts; il resto di YouTube funziona" },
      { code: "linkedin\\.com/feed", note: "solo il feed di LinkedIn" },
    ],
    extensionSkippedTitle:
      "Il browser non può usare queste espressioni come regole — le controllo quando si carica una scheda",
  },

  mascot: {
    setting: "Mostra la mascotte (assolotto)",
    settingHint:
      "Un assolotto con le cuffie accanto all'orologio, durante le pause, nel riepilogo e nei pop-up al cambio di fase. Condivide buone pratiche e suggerimenti per le pause.",
    practiceLabel: "Buona pratica",
    nextUpLabel: "Cosa fare dopo?",
    startLines: [
      "Al lavoro!",
      "Si comincia. In bocca al lupo!",
      "Cuffie in testa — si parte!",
      "Un compito alla volta. Via!",
    ],
    breakLines: [
      "Pausa! Allontanati un attimo dallo schermo.",
      "È ora di una pausa. Te la sei meritata!",
      "Pausa — sgranchisciti le gambe.",
    ],
    cycleLines: [
      "Ciclo completato! Si torna a concentrarsi.",
      "Ottimo, un altro ciclo fatto!",
      "Va alla grande. Prossimo giro!",
    ],
    conflictHint: (subject: string) =>
      `Una riunione cade durante una pausa: ${subject}. Decidi qui sotto cosa farne.`,
    tickle: "Fai il solletico all'assolotto",
    tickleLines: [
      "Ehi, mi fai il solletico!",
      "Ih ih, smettila!",
      "Bup! Torniamo al lavoro?",
      "Soffro il solletico alle branchie!",
      "Coccola ricevuta. Concentrazione +1!",
      "Piano, sono tutto morbidoso!",
      "Ancora? Va bene, un'ultima volta!",
      "Hai scoperto dove soffro il solletico!",
    ],
  },

  calendar: {
    title: "Calendari",
    lead: "FocusMode controlla le riunioni delle prossime ore e, prima che inizi la sessione, ti avvisa se una cade durante una pausa. I calendari vengono solo letti — FocusMode non vi modifica mai nulla.",
    privacy:
      "Le riunioni restano su questo computer. I token di accesso e i link dei calendari sono salvati cifrati, fuori dal file delle impostazioni, e non finiscono mai nei log.",
    horizon: "Controlla le riunioni delle prossime",
    horizonOption: (h: number) => `${h} ${plural(h, { one: "ora", other: "ore" })}`,
    horizonHint: "Per una sessione lunga scegli un intervallo più ampio, così copre tutte le sue pause.",
    lastSync: (time: string) => `Ultima sincronizzazione: ${time}`,
    neverSynced: "I calendari non sono ancora stati letti.",
    refresh: "Aggiorna ora",
    refreshing: "Aggiornamento…",
    sourceName: {
      microsoft: "Microsoft 365 / Outlook",
      google: "Google Calendar",
      ics: "Calendari da un link ICS",
    },
    sourceShort: {
      microsoft: "Outlook",
      google: "Google",
      ics: "ICS",
    },
    state: {
      off: "disattivato",
      on: "connesso",
      unavailable: "non disponibile",
      error: "errore",
    },
    statusLine: (state: string, account: string | null) => (account ? `${state} · ${account}` : state),
    errorLead: "Impossibile leggere il calendario:",

    microsoft: "Microsoft 365 / Outlook (l'account Microsoft di Teams)",
    microsoftHint:
      "Legge il tuo calendario tramite lo stesso account Microsoft usato dall'integrazione con Teams (Microsoft Graph, sola lettura: Calendars.Read). Include le riunioni di Teams e tutti gli altri eventi del calendario.",
    microsoftReconnect:
      "FocusMode chiede l'autorizzazione a leggere il calendario (Calendars.Read) solo quando accedi con il calendario attivo. Un account collegato in precedenza solo per Teams va collegato di nuovo — non serve disconnetterlo, e il tuo stato di Teams continua a funzionare. Nella registrazione dell'app in Azure, aggiungi l'autorizzazione Calendars.Read.",
    microsoftTeamsAbove: "L'account Microsoft si collega più in alto, nella protezione “Microsoft Teams: Non disturbare”.",
    microsoftConnect: "Collega un account Microsoft",
    microsoftNeedsReconnect:
      "Questo accesso Microsoft non può leggere il calendario. Collega di nuovo l'account e consenti la lettura del calendario.",
    microsoftReconnectButton: "Collega di nuovo",
    microsoftReconnecting: "Connessione…",
    microsoftReconnectCode: (code: string) =>
      `Inserisci il codice ${code} nella pagina di accesso Microsoft (è aperta nel browser; il link è anche più in alto, nella protezione Teams).`,

    google: "Google Calendar",
    googleHint:
      "Accedi nel browser, e FocusMode chiede solo di leggere il calendario. Ti serve un tuo ID client OAuth gratuito di Google Cloud.",
    googleClientId: "ID client OAuth",
    googleClientIdPlaceholder: "123456789012-abc….apps.googleusercontent.com",
    googleClientIdInvalid:
      "Un ID client di Google termina con .apps.googleusercontent.com e contiene solo lettere, cifre, punti, trattini e trattini bassi.",
    googleClientSecret: "Client secret",
    googleClientSecretPlaceholder: "GOCSPX-…",
    googleClientSecretHint:
      "Dallo stesso client “App desktop” di Google Cloud dell'ID qui sopra. Google lo richiede per l'accesso ma nelle app installate non lo considera segreto, quindi FocusMode lo salva nel file delle impostazioni (mai nei log).",
    googleClientSecretInvalid: (max: number) =>
      `Il client secret contiene solo lettere, cifre, trattini e trattini bassi (al massimo ${max} caratteri).`,
    googleConnect: "Connetti a Google",
    googleDisconnect: "Disconnetti",
    googlePending: "Completa l'accesso nella finestra del browser che si è aperta…",
    googleConnected: (account: string | null) => (account ? `Connesso come ${account}` : "Connesso"),
    googleNotConnected: "Non connesso",
    googleNeedsClientId: "Inserisci prima l'ID client.",
    googleErrorLead: "Impossibile connettersi a Google:",
    googleHelpTitle: "Come ottengo un ID client di Google?",
    googleHelp: [
      "Apri console.cloud.google.com e crea un progetto (o scegline uno esistente).",
      "In “API e servizi” → “Libreria”, abilita la Google Calendar API.",
      "Configura la schermata di consenso OAuth (Google Auth Platform): pubblico “Esterno” (o “Interno” in un Google Workspace aziendale), ambito …/auth/calendar.readonly. Finché l'app è in modalità di test, aggiungi il tuo indirizzo email come utente di test.",
      "In “Client” (Credenziali), crea un ID client OAuth di tipo “App desktop” (Desktop app).",
      "Copia l'“ID client” (termina con .apps.googleusercontent.com) e il “Client secret” (inizia con GOCSPX-) nei campi qui sopra e premi “Connetti a Google”.",
    ],

    ics: "Altri calendari (link ICS / iCal)",
    icsHint:
      "Per i calendari con un link di iscrizione privato: Outlook ed Exchange al lavoro, Google, iCloud, Proton, Zoho, Zimbra, Nextcloud e altri. Sono incluse anche le riunioni ricorrenti.",
    icsLinks: "Link dei calendari — uno per riga",
    icsPlaceholder: "https://outlook.office365.com/owa/calendar/…/calendar.ics\nwebcal://…",
    icsSave: "Salva i link",
    icsSaved: "I link dei calendari sono stati salvati.",
    icsEdit: "Modifica i link",
    icsAdd: "Aggiungi link",
    icsCancel: "Annulla",
    icsRemoveAll: "Rimuovi tutti i link",
    icsRemoved: "I link dei calendari sono stati rimossi.",
    icsCount: (n: number) => (n === 0 ? "nessun link" : `${n} link`),
    icsLoading: "Caricamento dei link salvati…",
    icsInvalidLines: (lines: readonly number[]) =>
      `${lines.length === 1 ? "Riga" : "Righe"} ${lines.join(", ")}: non sembra un link di calendario (https:// o webcal://).`,
    icsSecret:
      "Chiunque conosca un link come questo può vedere il tuo calendario. FocusMode lo salva cifrato e non lo mostra mai nei messaggi di errore.",
    icsHelpTitle: "Dove trovo il link ICS?",
    icsHelp: [
      {
        name: "Outlook sul web (Microsoft 365, Exchange)",
        text: "Impostazioni → Calendario → Calendari condivisi → “Pubblica un calendario”: scegli il calendario e “Può visualizzare tutti i dettagli”, premi “Pubblica” e copia il link ICS. Su un server Exchange aziendale, la pubblicazione deve essere attivata dall'amministratore.",
      },
      {
        name: "Google Calendar",
        text: "Impostazioni → scegli il calendario a sinistra → “Integra calendario” → “Indirizzo segreto in formato iCal”.",
      },
      {
        name: "iCloud",
        text: "In Calendario, condividi il calendario come “Calendario pubblico” e copia il suo link (webcal://).",
      },
      {
        name: "Proton, Zoho, Zimbra, Nextcloud e altri",
        text: "Nelle impostazioni del calendario cerca “Condividi”, “Iscriviti”, “Link ICS” o “iCal”.",
      },
    ],

    cardTitle: "Riunioni durante le pause",
    leadPlanned: "Se inizi ora, queste riunioni cadranno durante le pause. Decidi cosa fare:",
    leadRunning: "Queste riunioni cadono durante le pause di questa sessione:",
    noConflictsPlanned: "Se inizi ora, nessuna riunione cadrà durante una pausa.",
    noConflictsRunning: "Nessuna riunione cade durante le pause di questa sessione.",
    breakInfo: (n: number, range: string) => `Pausa ${n}: ${range}`,
    choice: {
      extend: "Prolunga la concentrazione (musica silenziata)",
      pauseAtBreak: "Metti in attesa la pausa",
      ignore: "Ignora",
      remindLater: "Ricordamelo più tardi",
    },
    choiceExtend: (end: string) =>
      `La concentrazione dura fino alla fine della riunione (${end}), con la musica silenziata. La pausa inizia dopo la riunione.`,
    choicePauseAtBreak: "Quando arriva l'ora della pausa, il timer si ferma e aspetta finché non premi “Riprendi”.",
    choiceIgnore: "La sessione continua senza cambiamenti.",
    choiceRemindLater: (time: string | null) =>
      time ? `Te lo ricordo in un piccolo pop-up di FocusMode (${time}).` : "Te lo ricordo più vicino alla riunione.",
    choiceNone: "Scegli cosa fare.",
    tooLong: "Riunione più lunga di 4 h — la concentrazione non verrà prolungata",
    choiceGroup: (subject: string) => `Cosa fare con la riunione ${subject}`,
    upcomingTitle: "Prossime riunioni",
    upcomingEmpty: (hours: number) =>
      hours === 1 ? "Nessuna riunione nella prossima ora." : `Nessuna riunione nelle prossime ${hours} ore.`,
    now: "ora",
    online: "online",
    onlineAria: "riunione online",
    tomorrow: "domani",
    untitled: "(senza titolo)",
    connectHint:
      "Collega un calendario e, prima di una sessione, FocusMode ti avviserà delle riunioni che cadono durante le pause.",
    connectLink: "Impostazioni dei calendari",
    sourceErrors: (names: string) => `Impossibile leggere: ${names}.`,
    sourceErrorsLink: "Dettagli nelle Impostazioni",
    loading: "Caricamento del calendario…",

    holdExtend: (time: string, subject: string) => `Concentrazione prolungata fino alle ${time} — riunione: ${subject}`,
    holdExtendHint: "La musica resta silenziata fino alla fine della riunione; poi inizia la pausa.",
    holdPause: (subject: string) => `Pausa in attesa — riunione: ${subject}`,
    holdPauseHint: (time: string) =>
      `Il timer della pausa aspetta. Premi “Riprendi” quando la riunione finisce (fine prevista: ${time}).`,
  },

  work: {
    title: "Strumenti di lavoro (Cosa fare dopo?)",
    lead: "Collega gli strumenti con cui lavori e FocusMode ti suggerirà cosa affrontare nella prossima sessione: prima le attività assegnate a te, poi ciò che gli altri aspettano da te (revisioni, correzioni, menzioni) e infine le altre notifiche. Gli strumenti vengono solo letti — FocusMode non vi modifica mai nulla.",
    privacy:
      "I token sono salvati cifrati, fuori dal file delle impostazioni, e non finiscono mai nei log. FocusMode si collega solo agli strumenti attivati qui sopra; attività e notifiche restano su questo computer e non vengono mai inviate ad alcuna IA o ad altri servizi.",
    sourceName: {
      github: "GitHub",
      gitlab: "GitLab",
      jira: "Jira",
      azureDevops: "Azure DevOps",
      linear: "Linear",
    },
    sourceShort: {
      github: "GitHub",
      gitlab: "GitLab",
      jira: "Jira",
      azureDevops: "Azure",
      linear: "Linear",
    },
    sourceHint: {
      github:
        "Issue assegnate a te, pull request in attesa della tua revisione o delle tue correzioni e — con un token classico — notifiche.",
      gitlab:
        "Issue assegnate a te, merge request da revisionare e le tue merge request con CI non riuscita. Funziona con GitLab.com e con le istanze self-managed.",
      jira: "Ticket assegnati a te, con lo sprint attivo, la priorità e la scadenza. Jira Cloud o Data Center.",
      azureDevops:
        "Elementi di lavoro assegnati a te (con l'iterazione corrente) e pull request in attesa della tua revisione.",
      linear: "Issue assegnate a te, con il ciclo corrente, la priorità e la scadenza.",
    },
    state: {
      off: "disattivato",
      on: "connesso",
      unavailable: "non connesso",
      error: "errore",
    },
    connectedAs: (account: string | null) => (account ? `Connesso come ${account}` : "Connesso"),
    syncing: "Token salvato — prima lettura in corso…",
    notConnected: "Non connesso — salva un token qui sotto.",
    noSecretStore: "Questo sistema non può salvare il token in modo sicuro, quindi questo strumento non funziona.",
    tokenInvalid: "Un token può contenere solo caratteri ASCII visibili, senza spazi (al massimo 4 KB).",
    needsUrl: {
      gitlab: "Inserisci prima l'indirizzo di GitLab.",
      jira: "Inserisci prima l'indirizzo di Jira.",
      azureDevops: "Inserisci prima il nome dell'organizzazione.",
    },
    errorLead: "Impossibile leggere:",

    gitlabUrl: "Indirizzo di GitLab",
    gitlabUrlPlaceholder: "https://gitlab.com",
    gitlabUrlHint: "Per un'istanza self-managed, inserisci il suo indirizzo, ad es. https://gitlab.example.com.",
    jiraUrl: "Indirizzo di Jira",
    jiraUrlPlaceholder: "https://tua-azienda.atlassian.net",
    jiraUrlHint:
      "L'indirizzo del tuo sito Jira Cloud o del server Jira Data Center. Puoi incollare l'indirizzo dal browser senza la parte dopo il ?.",
    jiraEmail: "Email dell'account Atlassian (solo Jira Cloud)",
    jiraEmailPlaceholder: "mario.rossi@example.com",
    jiraEmailHint: "Per Jira Data Center con un token di accesso personale, lascia vuoto.",
    azureOrg: "Organizzazione Azure DevOps",
    azureOrgPlaceholder: "tua-azienda",
    azureOrgHint: "Il nome nell'indirizzo https://dev.azure.com/<nome> — puoi anche incollare l'indirizzo intero.",
    urlInvalid:
      "Inserisci un indirizzo che inizia con https://, ad es. https://tua-azienda.atlassian.net — senza nome utente, password, ? o #.",
    emailInvalid: "Inserisci l'email dell'account, ad es. mario@example.com (senza spazi né due punti), oppure lascia vuoto.",
    azureOrgInvalid:
      "Inserisci il nome dell'organizzazione: lettere, cifre, trattini, punti e trattini bassi (fino a 64 caratteri).",

    token: (name: string) => `Token di accesso di ${name}`,
    tokenPlaceholder: "Incolla il token",
    tokenPlaceholderReplace: "Incolla un nuovo token per sostituire quello salvato",
    tokenHint: "Il token viene salvato cifrato e non ricompare mai in questo campo.",
    tokenSave: "Salva token",
    tokenSaving: "Salvataggio…",
    tokenRemove: "Rimuovi token",
    tokenSaved: (name: string) => `Il token di ${name} è stato salvato.`,
    tokenRemoved: (name: string) => `Il token di ${name} è stato rimosso.`,
    helpTitle: "Come creo un token di sola lettura?",
    help: {
      github: [
        "Su GitHub: Settings → Developer settings → Personal access tokens → Fine-grained tokens → Generate new token.",
        "Resource owner: il tuo account o l'organizzazione in cui lavori (un token fine-grained vede i repository di un solo proprietario). Repository access: All repositories oppure i repository selezionati.",
        "Repository permissions, ognuno solo “Read-only”: Issues, Pull requests e Metadata (GitHub aggiunge Metadata da solo). Non selezionare altro. Imposta una data di scadenza.",
      ],
      githubNotifications:
        "GitHub rende disponibili tramite API le notifiche (menzioni, richieste di revisione fuori dai repository del token) e i risultati di GitHub Actions solo ai token classici — un token fine-grained non può leggerli. Se vuoi vederli, crea invece un token classico (Tokens (classic)) con lo scope notifications. Per i repository privati, un token classico richiede anche lo scope repo, che consente anche la scrittura — FocusMode non la usa, ed è per questo che di default consigliamo un token fine-grained.",
      gitlab: [
        "In GitLab: avatar → Edit profile → Access tokens (Personal access tokens) → Add new token (nelle versioni più recenti: “Legacy token”).",
        "Seleziona solo lo scope read_api (accesso in lettura all'API). Imposta una data di scadenza.",
        "Per un'istanza GitLab self-managed, inserisci il suo indirizzo qui sopra.",
      ],
      jira: [
        "Jira Cloud: vai su id.atlassian.com/manage-profile/security/api-tokens e scegli “Create API token with scopes”, l'app Jira e solo gli scope read:jira-work e read:jira-user. Imposta una scadenza breve.",
        "Qui sopra inserisci l'indirizzo del sito e l'email del tuo account Atlassian. Un token con scope funziona solo tramite il gateway api.atlassian.com — FocusMode lo sceglie da solo. Funziona anche un token normale senza scope, ma ha tutti i permessi del tuo account (FocusMode legge solo i ticket).",
        "Jira Data Center: profilo → Personal Access Tokens → Create token, con una data di scadenza. Lascia vuoto il campo email.",
      ],
      azureDevops: [
        "Su dev.azure.com/<organizzazione>: User settings (l'icona accanto al tuo avatar) → Personal access tokens → New Token.",
        "Organization: la tua organizzazione. Scopes: Custom defined → Work Items: Read, Code: Read e Project and Team: Read — nient'altro. Imposta una data di scadenza.",
      ],
      linear: [
        "In Linear: Settings → Security & access → Personal API keys → New API key.",
        "Permissions: “Only select permissions” → Read only. Team access: tutti i team o quelli selezionati.",
      ],
    },

    holdDuringFocus: "Trattieni le notifiche durante la concentrazione",
    holdDuringFocusHint:
      "FocusMode mostra le nuove attività e notifiche arrivate durante la concentrazione solo nella pausa o prima della sessione successiva — niente interrompe la tua concentrazione.",
    lastSync: (time: string) => `Ultima sincronizzazione: ${time}`,
    neverSynced: "Gli strumenti non sono ancora stati letti.",
    refresh: "Aggiorna ora",
    refreshing: "Aggiornamento…",

    cardTitle: "Cosa fare dopo?",
    cardLead: "Prima ciò che è assegnato a te, poi ciò che gli altri aspettano.",
    loading: "Caricamento delle attività…",
    connectHint:
      "Collega GitHub, GitLab, Jira, Azure DevOps o Linear, e FocusMode ti suggerirà cosa affrontare in questa sessione.",
    connectLink: "Impostazioni degli strumenti di lavoro",
    noTokenHint: "Salva un token nelle Impostazioni e qui compariranno le tue attività.",
    empty: "Niente ti aspetta — nessuna attività assegnata, richiesta di revisione o notifica.",
    sourceErrors: (names: string) => `Impossibile leggere: ${names}.`,
    sourceErrorsLink: "Dettagli nelle Impostazioni",
    held: (n: number) => `${n} novità arrivate durante la concentrazione`,
    newCount: (n: number) => `${n} novità`,
    unseen: "novità",
    showMore: (n: number) => `Mostra altro (${n})`,
    showLess: "Mostra meno",
    open: "Apri",
    openAria: (name: string) => `Apri ${name} nel browser`,
    setGoal: "Imposta come obiettivo",
    setGoalAria: (name: string) => `Imposta ${name} come obiettivo della sessione`,
    isGoal: "Obiettivo della sessione",
    due: (date: string) => `scadenza: ${date}`,
    reasonsLabel: "Perché adesso",
    reason: {
      assigned: "assegnato a te",
      inSprint: "sprint",
      highPriority: "priorità alta",
      dueToday: "scade oggi",
      overdue: "scaduto",
      inProgress: "in corso",
      blockingOthers: "aspetta la tua revisione",
      waitingLong: "in attesa da tempo",
      mentioned: "menzione",
    },
    kind: {
      assigned: "attività",
      reviewRequested: "revisione",
      changesRequested: "modifiche richieste",
      ciFailed: "CI non superata",
      mention: "menzione",
      notification: "notifica",
    },
    itemName: (key: string, title: string) => (key && title ? `${key} ${title}` : key || title),

    goalLabel: "Obiettivo della sessione:",
    goalOpenAria: (name: string) => `Obiettivo della sessione: ${name}. Apri nel browser`,
    goalClear: "Cancella l'obiettivo della sessione",
  },

  // Impostazioni → Privacy e dati (ADR-020).
  privacy: {
    title: "Privacy e dati",
    lead: "Tutto ciò che FocusMode salva resta su questo computer. Qui vedi di cosa si tratta e quanto spazio occupa, e puoi portarlo con te o eliminarlo.",
    openFolder: "Apri la cartella dei dati",
    export: "Esporta i miei dati…",
    exportHint:
      "Un unico file JSON: impostazioni, cronologia delle sessioni, risposte alle riunioni ed elementi visti in “Cosa fare dopo?”. Mai token, link ICS, cookie o altri segreti.",
    exportDialogTitle: "Salva l'esportazione dei dati di FocusMode",
    exportFilterName: "File JSON",
    exported: "I tuoi dati sono stati esportati.",
    folders: "Cartelle dei dati:",
    busy: "In corso…",
    loading: "Controllo dei dati salvati…",
    groups: {
      app: "Impostazioni e cronologia",
      connections: "Connessioni (cifrate da Windows)",
      browser: "Browser e log",
      restore: "File di ripristino",
    },
    restoreHint:
      "Esistono solo mentre le protezioni o i colori caldi sono attivi (o dopo un arresto anomalo) e spariscono da soli quando FocusMode ha ripristinato le impostazioni di Windows. Per questo qui non si eliminano.",
    items: {
      settings: "Impostazioni",
      history: "Cronologia delle sessioni",
      meetingChoices: "Risposte alle riunioni",
      workSeen: "Elementi visti in “Cosa fare dopo?”",
      microsoftToken: "Account Microsoft (Teams, calendario)",
      googleToken: "Account Google (calendario)",
      icsLinks: "Link ai calendari ICS",
      githubToken: "Token GitHub",
      gitlabToken: "Token GitLab",
      jiraToken: "Token Jira",
      azureDevopsToken: "Token Azure DevOps",
      linearToken: "Token Linear",
      customChimes: "Suoni personalizzati",
      blocklist: "Elenco dei siti (websiteblacklist.txt)",
      shieldsRestore: "Ripristino delle notifiche",
      tintRestore: "Ripristino dei colori dello schermo",
      browserData: "Dati del browser integrato",
      logs: "Log dell'app",
    },
    explain: {
      settings: "Le tue scelte in questa pagina (config.json). Tornano ai valori predefiniti solo con “Elimina tutto”.",
      history: "Sessioni concluse: tempi, cicli e app in cui hai lavorato.",
      meetingChoices: "Come hai risposto alle riunioni che cadono in una pausa. Solo identificativi, senza oggetti.",
      workSeen: "Quali elementi di “Cosa fare dopo?” hai già visto. Solo identificativi, senza titoli.",
      microsoftToken: "Accesso per Teams e il calendario Microsoft. Cifrato, mai mostrato.",
      googleToken: "Accesso a Google Calendar. Cifrato, mai mostrato.",
      icsLinks: "Link privati ai calendari. Cifrati, mai mostrati.",
      githubToken: "Token di sola lettura per “Cosa fare dopo?”. Cifrato.",
      gitlabToken: "Token di sola lettura per “Cosa fare dopo?”. Cifrato.",
      jiraToken: "Token di sola lettura per “Cosa fare dopo?”. Cifrato.",
      azureDevopsToken: "Token di sola lettura per “Cosa fare dopo?”. Cifrato.",
      linearToken: "Chiave di sola lettura per “Cosa fare dopo?”. Cifrata.",
      customChimes: "Copie dei suoni importati per la campanella (gli originali restano dove sono).",
      blocklist: "Siti bloccati durante la concentrazione. Eliminarlo ripristina l'elenco predefinito.",
      shieldsRestore: "Lo stato delle notifiche di Windows prima della concentrazione, perché torni anche dopo un arresto anomalo.",
      tintRestore: "I colori dello schermo prima del riscaldamento, perché tornino anche dopo un arresto anomalo.",
      browserData: "Cookie (inclusi accesso e consenso di YouTube), cache e dati dei siti del lettore.",
      logs: "Un registro di ciò che ha fatto l'app, per diagnosticare i problemi. Senza token, titoli di finestre né oggetti delle riunioni.",
    },
    withTitles: "con titoli delle finestre",
    present: "Salvato",
    absent: "Nessuno",
    counts: {
      sessions: (n: number) => `${n} ${plural(n, { one: "sessione", other: "sessioni" })}`,
      answers: (n: number) => `${n} ${plural(n, { one: "risposta", other: "risposte" })}`,
      ids: (n: number) => `${n} ${plural(n, { one: "elemento", other: "elementi" })}`,
      links: (n: number) => `${n} link`,
      files: (n: number) => `${n} file`,
      entries: (n: number) => `${n} ${plural(n, { one: "voce", other: "voci" })}`,
    },
    delete: "Elimina",
    clear: "Svuota",
    restoreDefault: "Ripristina predefinito",
    deleteTitle: (name: string) => `Eliminare: ${name}?`,
    resetTitle: "Ripristinare l'elenco dei siti predefinito?",
    consequences: {
      history: "Tutte le sessioni salvate spariscono dal Riepilogo. Non si può annullare.",
      meetingChoices: "FocusMode dimentica le tue risposte e chiederà di nuovo quando una riunione cade in una pausa.",
      workSeen: "Gli elementi attuali di “Cosa fare dopo?” appariranno come nuovi finché non li guardi.",
      microsoftToken:
        "Disconnette l'account Microsoft: lo stato di Teams e il calendario Microsoft smettono di funzionare finché non lo ricolleghi.",
      googleToken: "Disconnette Google Calendar (il token viene revocato anche presso Google).",
      icsLinks: "Questi calendari non vengono più controllati finché non aggiungi di nuovo i link.",
      githubToken: "Disconnette GitHub: “Cosa fare dopo?” non mostra più i suoi elementi.",
      gitlabToken: "Disconnette GitLab: “Cosa fare dopo?” non mostra più i suoi elementi.",
      jiraToken: "Disconnette Jira: “Cosa fare dopo?” non mostra più i suoi elementi.",
      azureDevopsToken: "Disconnette Azure DevOps: “Cosa fare dopo?” non mostra più i suoi elementi.",
      linearToken: "Disconnette Linear: “Cosa fare dopo?” non mostra più i suoi elementi.",
      customChimes: "I suoni importati spariscono e la campanella torna al suono predefinito.",
      blocklist: "Il tuo elenco dei siti viene sostituito da quello predefinito. Le tue modifiche al file vanno perse.",
      browserData:
        "Elimina cookie, cache e dati dei siti. Ti disconnette da YouTube, che chiederà di nuovo il consenso ai cookie.",
      logs: "I log precedenti spariscono e quello attuale viene svuotato.",
    },
    sessionStops: "La sessione in corso viene prima interrotta.",
    deleted: (name: string) => `Eliminato: ${name}.`,
    blocklistReset: "L'elenco dei siti predefinito è tornato.",
    deleteAll: "Elimina tutti i dati",
    deleteAllHint: "Elimina tutto ciò che è elencato sopra e ripristina le impostazioni predefinite. L'app resta installata.",
    deleteAllButton: "Elimina tutto…",
    deleteAllTitle: "Eliminare tutti i dati di FocusMode?",
    deleteAllBody:
      "L'app resta installata; l'estensione del browser la rimuovi nel browser. Da questo computer spariranno:",
    deleteAllSettings: "Impostazioni (tornano ai valori predefiniti, compresi lingua, tema e avvio all'accesso)",
    deleteAllConfirm: "Sì, elimina tutto",
    deleteAllDone: "Tutti i dati sono stati eliminati. Le impostazioni sono tornate ai valori predefiniti.",
  },

  nudge: {
    title: "Torna al lavoro",
    body: (domain: string) => `${domain} può aspettare fino alla pausa.`,
    bodyPattern: "Questo sito è nell'elenco dei siti bloccati e può aspettare fino alla pausa.",
    dismiss: "Chiudi",
  },

  toasts: {
    region: "Notifiche",
    dismiss: "Chiudi la notifica",
  },

  errors: {
    generic: "Qualcosa è andato storto.",
    fieldInvalid: (label: string) => `Valore non valido: ${label}.`,
    fixConfigFirst: "Correggi le impostazioni evidenziate prima di iniziare una sessione.",
    unplayable: "Questo link non si può riprodurre nell'app",
    allUnplayable: "Nessun brano della musica scelta si può riprodurre.",
    autoplayBlocked: "La riproduzione è stata bloccata — premi “Successivo” per riprovare.",
    youtubeApi: "Impossibile caricare il lettore di YouTube. Controlla la connessione a internet.",
    clipboard: "Impossibile copiare negli appunti.",
    codes: {
      "config.invalid": "Impostazione non valida.",
      "config.corrupt":
        "Il file delle impostazioni era danneggiato — sono state ripristinate le impostazioni predefinite (backup: config.json.bak).",
      "config.read": "Impossibile leggere il file delle impostazioni — uso le impostazioni predefinite.",
      "config.save": "Impossibile salvare le impostazioni su disco — le modifiche valgono fino alla chiusura dell'app.",
      "audio.device": "Impossibile riprodurre la campanella — controlla il dispositivo audio.",
      "shields.notifications": "Impossibile attivare o disattivare “Non disturbare” di Windows.",
      "shields.teams": "Impossibile cambiare il tuo stato di Teams.",
      "shields.keepAwake": "Impossibile attivare o disattivare lo schermo sempre acceso.",
      "bridge.port":
        "La porta del ponte dell'estensione è occupata — il blocco dei siti funziona solo in modalità di riserva.",
      "teams.auth": "L'accesso a Teams non è riuscito.",
      "teams.network": "Impossibile raggiungere i servizi Microsoft — riprova tra poco.",
      "teams.notConfigured": "Inserisci prima l'Application (client) ID.",
      "teams.unsupported": "L'integrazione con Teams non è disponibile su questo sistema.",
      "teams.disconnect": "Impossibile disconnettere l'account Teams.",
      "teams.unavailable": "L'integrazione con Teams non risponde — riavvia l'app.",
      "music.folder": "Impossibile leggere la cartella della musica.",
      "media.url":
        "Questo link non si apre nella finestra di YouTube — funzionano solo gli indirizzi di YouTube o YouTube Music.",
      "media.window": "Impossibile aprire o controllare la finestra di YouTube.",
      "youtube.signIn": "Impossibile aprire la finestra di accesso a YouTube.",
      "youtube.signOut": "Impossibile uscire da YouTube — riprova.",
      "persistence.history": "Un problema con la cronologia delle sessioni (lettura o salvataggio).",
      "extension.missing": "La cartella dell'estensione del browser non è stata trovata.",
      "extension.open": "Impossibile aprire la cartella dell'estensione.",
      "blocklist.read": "Impossibile leggere websiteblacklist.txt — resta in uso l'elenco dei siti precedente.",
      "blocklist.open":
        "Impossibile aprire websiteblacklist.txt in un editor di testo — apri il file manualmente (il percorso è nelle Impostazioni).",
      "shields.launchers": "Non è stato possibile chiudere uno dei launcher di gioco — la sessione continua.",
      "tint.apply": "Impossibile riscaldare i colori dello schermo — riprovo tra poco.",
      "tint.restore": "Impossibile ripristinare i colori abituali dello schermo dopo l'esecuzione precedente.",
      "chime.format": "Questo formato non è supportato — scegli un file mp3, wav, ogg, flac o m4a.",
      "chime.read": "Impossibile leggere il file scelto.",
      "chime.tooLarge": "Il file è troppo grande — una campanella può essere al massimo di 10 MB.",
      "chime.save": "Impossibile salvare il suono nella cartella dati di FocusMode.",
      "chime.decode": "Questo file non si può riprodurre come suono — scegline un altro.",
      "music.library": "Impossibile caricare la musica integrata.",
      "popup.window": "Impossibile mostrare il pop-up di FocusMode.",
      "calendar.sync": "Impossibile leggere il calendario — dettagli in Impostazioni → Calendari.",
      "calendar.unavailable": "I calendari non rispondono — riavvia l'app.",
      "calendar.unknownMeeting": "Questa riunione non è più nel calendario.",
      "calendar.remindLater":
        "È troppo tardi per un promemoria — la riunione inizia tra 10 minuti o meno. Scegli un'altra opzione.",
      "calendar.meetingTooLong":
        "Una riunione più lunga di 4 h non prolunga la concentrazione né mette in attesa la pausa. Scegli un'altra opzione.",
      "calendar.googleNotConfigured": "Inserisci prima l'ID client di Google.",
      "calendar.googleAuth": "Impossibile avviare l'accesso a Google.",
      "calendar.disconnect": "Impossibile rimuovere i dati del calendario salvati.",
      "calendar.icsUrl": "Uno dei link non è un link di calendario valido (https:// o webcal://).",
      "calendar.icsSave": "Impossibile salvare in modo sicuro i link dei calendari.",
      "work.sync": "Impossibile leggere uno strumento di lavoro — dettagli in Impostazioni → Strumenti di lavoro.",
      "work.unavailable": "Gli strumenti di lavoro non rispondono — riavvia l'app.",
      "work.token":
        "Un token può contenere solo caratteri ASCII visibili, senza spazi (al massimo 4 KB) — incollalo di nuovo.",
      "work.tokenSave": "Impossibile salvare o rimuovere il token in modo sicuro.",
      // Codes of src-tauri/src/privacy.rs (+ focus-core privacy.rs).
      "privacy.export": "Impossibile salvare l'esportazione — scegli un'altra cartella (fuori dalla cartella dei dati di FocusMode).",
      "privacy.delete": "Non è stato possibile eliminare una parte dei dati (il resto sì) — riprova.",
      "privacy.notDeletable": "Questi dati non si eliminano separatamente.",
      "privacy.openFolder": "Impossibile aprire la cartella dei dati.",
      "privacy.report": "Impossibile controllare i dati salvati.",
      autostart: "Impossibile modificare l'avvio all'accesso.",
      "session.unavailable": "La sessione non risponde — riavvia l'app.",
      "ipc.unavailable": "Questa funzione è disponibile solo nell'app FocusMode.",
      "ipc.failed": "L'app non ha risposto correttamente.",
    },
    areas: {
      config: "Un problema con le impostazioni.",
      audio: "Un problema con l'audio.",
      shields: "Una delle protezioni non ha funzionato — la sessione continua.",
      teams: "Un problema con la connessione a Teams.",
      bridge: "Un problema con il ponte dell'estensione del browser.",
      blocklist: "Un problema con l'elenco dei siti bloccati (websiteblacklist.txt).",
      music: "Un problema con la musica.",
      media: "Un problema con la finestra di YouTube.",
      youtube: "Un problema con l'account YouTube.",
      persistence: "Impossibile salvare i dati.",
      session: "Un problema con la sessione.",
      tracker: "Un problema con il monitoraggio delle app.",
      tint: "Un problema con i colori caldi dello schermo.",
      chime: "Un problema con il suono personalizzato della campanella.",
      popup: "Un problema con il pop-up di FocusMode.",
      launchers: "Un problema nella chiusura dei launcher di gioco.",
      window: "Un problema con la finestra dell'app.",
      calendar: "Un problema con il calendario — la sessione continua.",
      google: "Un problema con Google Calendar.",
      ics: "Un problema con un calendario da link ICS.",
      work: "Un problema con gli strumenti di lavoro (Cosa fare dopo?) — la sessione continua.",
      privacy: "Un problema con i dati salvati da FocusMode.",
    },
  },

  fields: {
    focusMinutes: "tempo di concentrazione",
    breakMinutes: "tempo di pausa",
    chimeVolume: "volume della campanella",
    "music.volume": "volume della musica",
    "music.links": "link",
    "music.streamUrl": "indirizzo dello stream",
    "music.folder": "cartella della musica",
    "shields.teamsClientId": "Application (client) ID",
    "shields.teamsTenant": "tenant",
    "shields.launchers": "launcher di gioco",
    "tint.startMinutes": "inizio dei colori caldi",
    "tint.endMinutes": "fine dei colori caldi",
    "tint.strength": "intensità dei colori caldi",
    "tint.transitionMinutes": "transizione graduale",
    "tint.mode": "metodo di riscaldamento",
    language: "lingua",
    theme: "tema",
    chimeSound: "suono della campanella",
    "customChimes.breakFile": "suono di inizio pausa",
    "customChimes.focusFile": "suono di inizio concentrazione",
    breakReminders: "promemoria per le pause",
    "calendar.horizonHours": "intervallo di controllo del calendario",
    "calendar.googleClientId": "ID client di Google",
    "calendar.googleClientSecret": "client secret di Google",
    "work.gitlabUrl": "indirizzo di GitLab",
    "work.jiraUrl": "indirizzo di Jira",
    "work.jiraEmail": "email dell'account Jira",
    "work.azureOrg": "organizzazione Azure DevOps",
  },

  dialog: {
    cancel: "Annulla",
  },

  launchers: {
    steam: "Steam",
    epic: "Epic Games Launcher",
    gog: "GOG Galaxy",
    ubisoft: "Ubisoft Connect",
    ea: "App EA",
    battleNet: "Battle.net",
    riot: "Riot Client",
    xbox: "App Xbox",
    amazon: "Amazon Games",
    itch: "itch",
  },

  themes: {
    legend: "Tema dell'app",
    hint: "Colori e sfondo della finestra principale e dei pop-up di fase. Le modifiche vengono salvate subito.",
    selected: "selezionato",
    names: {
      midnight: "Mezzanotte",
      vaporwave: "Vaporwave",
      lunar: "Fasi lunari",
      autumn: "Autunno accogliente",
      dark: "Scuro",
      light: "Chiaro",
      pixel: "8-bit",
    },
    descriptions: {
      midnight: "Una calda notte viola con la pioggia fuori dalla finestra. Il tema predefinito.",
      vaporwave: "Rosa, ciano e viola: un sole a strisce sopra una griglia in prospettiva.",
      lunar: "Argento e indaco sotto le stelle.",
      autumn: "Marroni e ambra, foglie che cadono lentamente.",
      dark: "Grigio neutro, senza decorazioni sullo sfondo.",
      light: "Sfondo chiaro e testo scuro ad alto contrasto, senza decorazioni.",
      pixel: "Cornici squadrate, ombre nette e stelle in pixel.",
    },
    moonToday: (phase: string, litPct: number) =>
      `La luna sullo sfondo mostra la fase di oggi: ${phase} (illuminata al ${litPct}%).`,
    moonPhases: {
      new: "luna nuova",
      waxingCrescent: "falce crescente",
      firstQuarter: "primo quarto",
      waxingGibbous: "gibbosa crescente",
      full: "luna piena",
      waningGibbous: "gibbosa calante",
      lastQuarter: "ultimo quarto",
      waningCrescent: "falce calante",
    },
  },

  /** Break suggestions by id (category, length and source in `$lib/tips`). Suggestions only, never health promises. */
  tips: {
    "eyes-20-20-20":
      "Guarda fuori dalla finestra qualcosa ad almeno 6 metri di distanza e tieni lo sguardo lì per 20 secondi.",
    "eyes-green-view": "Cerca fuori dalla finestra un albero, un po' d'erba o un pezzo di cielo e guardalo per 40 secondi.",
    "eyes-slow-blinks": "Chiudi lentamente gli occhi, conta fino a due e riaprili. Ripeti 10 volte, con calma.",
    "eyes-look-around":
      "Guarda in alto, in basso, a sinistra e a destra, poi fai lentamente due cerchi con gli occhi in ogni direzione.",
    "eyes-near-far":
      "Guarda il pollice con il braccio teso per 10 secondi, poi un punto fuori dalla finestra per 10 secondi. Ripeti 5 volte.",
    "eyes-screen-free": "Passa questa pausa lontano dagli schermi — metti a faccia in giù anche il telefono.",
    "stretch-shoulder-rolls": "Ruota lentamente le spalle all'indietro 5 volte, poi in avanti 5 volte.",
    "stretch-shoulder-shrug":
      "Solleva le spalle verso le orecchie, tienile su per 3–5 secondi e lasciale ricadere. Ripeti 3 volte.",
    "stretch-neck-turn":
      "Con la schiena dritta, gira lentamente la testa a sinistra, fin dove è comodo. Resta così 5 secondi, poi a destra. 3 volte.",
    "stretch-neck-tilt":
      "Inclina lentamente la testa, con l'orecchio verso la spalla, e resta così 10 secondi, se è comodo. Poi dall'altro lato.",
    "stretch-chin-glide":
      "Con la schiena dritta, porta la testa indietro in orizzontale senza alzare il mento. Resta così 5 secondi, ripeti 5 volte.",
    "stretch-chest-open":
      "Siediti con la schiena dritta, staccata dallo schienale, avvicina le scapole e apri le braccia. Resta così 10 s, 3 volte.",
    "stretch-upper-twist":
      "Incrocia le braccia sul petto e ruota il busto a sinistra, fin dove è comodo. Resta così 5 s, poi a destra. 5 volte.",
    "stretch-side-reach":
      "Intreccia le dita sopra la testa, allungati verso l'alto, poi piegati a sinistra e a destra, 10 secondi per lato, se è comodo.",
    "stretch-wrists":
      "Distendi un braccio e piega delicatamente il polso verso l'alto, poi verso il basso, 5 secondi ciascuno. 3 volte, poi l'altra mano.",
    "stretch-fingers": "Chiudi le mani a pugno, poi apri bene le dita. Ripeti 5 volte, senza fretta.",
    "stretch-prayer":
      "Unisci i palmi davanti a te e abbassa lentamente i polsi fino a un leggero allungamento, non al dolore. 3 volte.",
    "stretch-ankles":
      "Sulla sedia, distendi una gamba sollevandola dal pavimento e porta le punte dei piedi avanti e indietro. 5 volte per gamba.",
    "stretch-standing-back":
      "In piedi con le ginocchia leggermente piegate e le mani sulla parte bassa della schiena, inclinati delicatamente all'indietro, fin dove è comodo. 3 volte.",
    "stretch-routine":
      "Fai una serie tranquilla di 10 minuti: spalle, collo, torsioni del busto, polsi e caviglie — sempre fin dove è comodo.",
    "move-walk-5": "Alzati e cammina per 5 minuti — per casa, lungo il corridoio o fino in cucina e ritorno.",
    "move-stairs":
      "Sali una rampa di scale e riscendi con il tuo ritmo. Niente scale? Fai due passi lungo il corridoio.",
    "move-march-seated":
      "Con la schiena dritta, solleva le ginocchia a turno come se marciassi, fin dove è comodo — 5 volte per gamba.",
    "move-march-standing": "Marcia sul posto per un minuto, facendo oscillare le braccia rilassate.",
    "move-calf-raises": "In piedi davanti alla scrivania, appoggia le mani sul piano e sollevati lentamente sulle punte 10 volte.",
    "move-sit-to-stand": "Su una sedia stabile senza rotelle, alzati e siediti 10 volte, con calma.",
    "move-one-song": "Metti una canzone che ami e muoviti come ti va — in piedi o sulla sedia.",
    "move-chores": "Fai qualcosa di attivo in casa: stendi il bucato, annaffia le piante o porta fuori la spazzatura.",
    "hydrate-glass": "Bevi un bicchiere d'acqua — lentamente, a piccoli sorsi.",
    "hydrate-refill": "Vai in cucina e riempi d'acqua una bottiglia o una tazza per la prossima sessione.",
    "hydrate-in-reach": "Prima di tornare al lavoro, metti un bicchiere d'acqua a portata di mano.",
    "breathe-count-5":
      "Siediti comodamente. Inspira dal naso contando fino a 5, espira dalla bocca contando fino a 5. Continua per 5 minuti.",
    "breathe-belly-hand":
      "Appoggia una mano sulla pancia e per un minuto osserva come si alza quando inspiri e si abbassa quando espiri.",
    "breathe-long-exhale":
      "Inspira dal naso, aggiungi un secondo breve sorso d'aria, poi espira lentamente dalla bocca. 5 volte.",
    "breathe-open-window": "Apri la finestra e fai 5 respiri tranquilli: inspira contando fino a quattro, espira fino a sei.",
    "breathe-muscle-release":
      "Contrai i piedi per 5 secondi, poi rilassali. Risali così lungo il corpo, fino alle spalle e al viso.",
    "posture-reset":
      "Prima di tornare: schiena contro lo schienale, piedi ben appoggiati a terra, braccia rilassate lungo i fianchi.",
    "posture-screen-check":
      "Controlla il monitor: bordo superiore all'altezza degli occhi o poco sotto, schermo a circa un braccio di distanza.",
    "posture-glare-check":
      "Controlla che nessuna finestra o lampada si rifletta sullo schermo. L'ideale è avere la finestra di lato rispetto al monitor.",
    "posture-shift": "Cambia posizione per un po': spostati sulla sedia, muovi le gambe, abbassa le spalle.",
    "outside-step-out":
      "Esci sul balcone o davanti all'edificio, oppure affacciati a una finestra aperta, e resta lì per 2 minuti.",
    "outside-sky": "Esci all'aperto e guarda il cielo e le nuvole per un minuto.",
    "outside-walk-15":
      "Fai una passeggiata di 15 minuti, se puoi dove ci sono alberi o prati. Mantieni un passo tranquillo.",
    "outside-walk-noticing":
      "Fai una passeggiata senza il telefono in mano e conta alberi, uccelli o cani lungo la strada.",
    "social-chat": "Fai una breve chiacchierata con qualcuno su qualcosa che non c'entra niente con il lavoro.",
    "social-message": "Manda un breve messaggio a una persona cara, solo per salutarla.",
    "social-walk-together": "Invita qualcuno a fare due passi insieme, anche solo intorno all'edificio.",
    "social-pet": "Hai un animale? Dedicagli qualche minuto di attenzione piena.",
    "mind-do-nothing": "Siediti comodamente, chiudi gli occhi e per 2 minuti semplicemente non fare nulla.",
    "mind-one-song": "Ascolta con attenzione una canzone a occhi chiusi, senza fare nient'altro.",
    "mind-5-4-3": "Nomina mentalmente 5 cose che vedi, 4 che senti e 3 che puoi toccare.",
    "mind-wander": "Guarda fuori dalla finestra e lascia vagare liberamente i pensieri per 3 minuti.",
    "mind-park-the-task":
      "Scrivi su un foglio una frase: da cosa inizierai dopo la pausa. Poi metti da parte l'argomento.",
    "mind-three-good": "Pensa a tre piccole cose che oggi sono andate bene.",
    "mind-meal-away": "Mangia lontano dalla scrivania e dallo schermo — a tavola, su una panchina o in compagnia.",
    "mind-your-choice": "Fai qualcosa che hai davvero voglia di fare — questa pausa è tua.",
    "mind-lie-down": "Se puoi, sdraiati per 10–15 minuti con gli occhi chiusi. Imposta una sveglia.",
    "tidy-desk":
      "Libera la scrivania: porta via le tazze, raccogli i fogli sparsi e tieni solo ciò che ti serve adesso.",
    "tidy-air": "Spalanca la finestra per qualche minuto e cambia l'aria nella stanza.",
    "tidy-light": "Regola la luce: abbassa la tenda se c'è riverbero, o accendi una lampada se sta facendo buio.",
  },

  /** Good working practices by id (one a day on the idle Focus page). */
  workingTips: {
    "gp-screen-position":
      "Metti il monitor proprio davanti a te, a circa 50–70 cm dagli occhi, con il bordo superiore all'altezza degli occhi o poco sotto.",
    "gp-light-and-glare": "Tieni lo schermo di lato rispetto alla finestra e adatta la sua luminosità alla luce della stanza.",
    "gp-sitting":
      "Schiena contro lo schienale, piedi ben appoggiati a terra, gomiti vicini al corpo, polsi in linea con gli avambracci.",
    "gp-short-often":
      "Fai pause brevi e frequenti, prima che arrivi la stanchezza: qualche minuto ogni ora è meglio di una lunga pausa ogni due ore.",
    "gp-20-20-20":
      "Mentre ti concentri, distogli lo sguardo dallo schermo circa ogni 20 minuti e guarda lontano per 20 secondi. Ricordati di sbattere le palpebre.",
    "gp-change-position": "Cambia posizione: alzati dalla scrivania almeno una volta all'ora, anche solo per poco.",
    "gp-batch-messages":
      "Controlla email e chat in orari fissi, ad esempio durante le pause, invece che dopo ogni notifica.",
    "gp-single-task": "Scegli un'attività per sessione e chiudi le schede e le finestre che non ti servono per quella.",
    "gp-phone-away": "Durante la concentrazione, silenzia il telefono e mettilo fuori dalla tua vista.",
    "gp-water-in-reach": "Tieni l'acqua a portata di mano e bevine un sorso ogni tanto durante la giornata.",
  },

  /** Shown under every stretch suggestion. */
  stretchFootnote:
    "Muoviti fin dove è comodo: dovresti sentire un allungamento, mai dolore. Se qualcosa fa male, salta questo esercizio.",

  time: {
    hoursMinutes: (h: number, m: number) => `${h} h ${m} min`,
    hours: (h: number) => `${h} h`,
    minutes: (m: number) => `${m} min`,
    seconds: (s: number) => `${s} s`,
    today: "oggi",
  },
};
