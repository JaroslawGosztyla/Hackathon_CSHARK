// French UI texts (translated from the Polish source, `pl.ts`, via `en.ts`; same shape).
// Tone: friendly, short and calm; the user is addressed with "vous" everywhere. No health promises: the app
// suggests, it never claims effects. "FocusMode" is a name and is never translated; neither is the file name
// websiteblacklist.txt. French typography: « … » and a no-break space before : ; ? ! % and units.
// Terms: docs/i18n-glossary.md (column FR).
import type { Strings } from "./pl";
import { pluralizer } from "./plural";

const plural = pluralizer("fr");

export const fr: Strings = {
  appName: "FocusMode",
  tagline: "sessions de concentration lofi",

  nav: {
    label: "Navigation principale",
    focus: "Concentration",
    summary: "Résumé",
    settings: "Paramètres",
  },

  phase: {
    idle: "Prêt à démarrer",
    focus: "Concentration",
    break: "Pause",
  },

  phaseAnnouncement: {
    focusStarted: (cycle: number) => `La concentration commence, cycle ${cycle}.`,
    breakStarted: "La pause commence.",
    stopped: "Session terminée.",
  },

  timer: {
    paused: "Suspendu",
    cycle: (n: number) => `Cycle ${n}`,
    demo: "démo courte",
    remainingLabel: (phase: string, time: string) => `${phase} : encore ${time}`,
    idleHint: "Réglez les durées et la musique, puis appuyez sur Démarrer.",
    focusedSoFar: (time: string) => `Concentration pendant cette session : ${time}`,
  },

  controls: {
    start: "Démarrer",
    pause: "Suspendre",
    resume: "Reprendre",
    skipToBreak: "Pause maintenant",
    skipToFocus: "Je m'y remets",
    skip: "Passer la phase",
    stop: "Arrêter",
    sessionControls: "Commandes de la session",
  },

  timing: {
    title: "Durée",
    focus: "Concentration (min)",
    break: "Pause (min)",
    presets: "Préréglages",
    preset: (focus: number, brk: number) => `${focus} / ${brk}`,
    presetAria: (focus: number, brk: number) => `${focus} minutes de concentration, ${brk} minutes de pause`,
    rangeFocus: "Indiquez un nombre de minutes entre 1 et 240.",
    rangeBreak: "Indiquez un nombre de minutes entre 1 et 120.",
  },

  music: {
    title: "Musique",
    sourceLabel: "Source musicale",
    source: {
      library: "Open Lo-Fi (intégrée)",
      stream: "Flux",
      links: "Liens",
      folder: "Dossier",
      noise: "Bruit",
      none: "Silence",
    },
    libraryName: "Open Lo-Fi",
    libraryHint: "Des morceaux lo-fi apaisants fournis avec FocusMode — ils fonctionnent sans Internet.",
    libraryCredit: "Musique : Open Lo-Fi par btahir, sous licence CC0 (domaine public).",
    libraryCreditLink: "github.com/btahir/open-lofi",
    libraryCreditAria: "Open Lo-Fi sur GitHub (ouvre le navigateur)",
    libraryEmpty: "La musique intégrée est introuvable — choisissez une autre source.",
    streamUrl: "Adresse du flux (YouTube en direct ou radio Internet)",
    streamPlaceholder: "https://www.youtube.com/watch?v=…",
    lofiPreset: "Lofi Girl — radio en direct",
    lofiPresetHint:
      "Le direct actuel de la chaîne Lofi Girl. Il est lu en arrière-plan dans une fenêtre YouTube masquée (la chaîne n'autorise pas son intégration dans des applications) ; vous gérez la pause et le volume depuis FocusMode. La première fois, YouTube affiche sa fenêtre de choix des cookies — une fois votre choix fait, elle se masque d'elle-même.",
    presetsLabel: "Suggestions",
    linksLabel: "Liens — un par ligne (YouTube, YouTube Music, playlists, fichiers audio)",
    linksPlaceholder: "https://music.youtube.com/watch?v=…\nhttps://youtu.be/…\nhttps://example.com/morceau.mp3",
    linksSummary: (ok: number, bad: number) =>
      bad > 0
        ? `${ok} ${plural(ok, { one: "lien lisible", other: "liens lisibles" })}, ${bad} non pris en charge`
        : `${ok} ${plural(ok, { one: "lien lisible", other: "liens lisibles" })}`,
    shuffle: "Lecture aléatoire",
    pickFolder: "Choisir un dossier",
    changeFolder: "Changer de dossier",
    pickFolderTitle: "Choisir un dossier de musique",
    noFolder: "Aucun dossier choisi.",
    loadingTracks: "Recherche de fichiers audio…",
    trackCount: (n: number) => `${n} ${plural(n, { one: "morceau", other: "morceaux" })}`,
    noTracks: "Ce dossier ne contient aucun fichier audio pris en charge (mp3, m4a, flac, wav, ogg, opus…).",
    noise: {
      brown: "Brun",
      pink: "Rose",
      white: "Blanc",
    },
    noiseLabel: "Type de bruit",
    noiseHint: "Généré ici même — fonctionne sans Internet.",
    noneHint: "Pas de musique — vous n'entendrez que les sonneries au début et à la fin de la pause.",
    detected: {
      youtubeVideo: "YouTube — vidéo ou direct",
      youtubePlaylist: "YouTube — playlist",
      youtubeWindow:
        "YouTube — se lit dans une petite fenêtre YouTube séparée (ce lien ne peut pas être lu dans l'application).",
      audio: "Flux ou fichier audio",
      unsupported: "Ce service n'est pas pris en charge (p. ex. Spotify) — utilisez YouTube ou un lien audio direct.",
      invalid: "Ceci ne ressemble pas à une adresse http(s).",
    },
    pauseDuringBreak: "Suspendre la musique pendant les pauses",
    pauseDuringBreakHint: "Par défaut, la musique continue, y compris pendant les pauses.",
  },

  player: {
    region: "Lecteur de musique",
    idle: "La musique démarre avec la session",
    pausedBySession: "Suspendu",
    pausedForBreak: "Suspendu pendant la pause",
    pausedForMeeting: "En sourdine pendant la réunion",
    loading: "Chargement…",
    silence: "Silence",
    next: "Suivant",
    volume: "Volume de la musique",
    expand: "Afficher la vidéo",
    collapse: "Masquer la vidéo",
    noiseTitle: (color: string) => `Bruit ${color.toLowerCase()}`,
    youtubeFallbackTitle: "YouTube",
    trackOf: (i: number, n: number) => `${i} sur ${n}`,
    sourceNotReady: "Choisissez une source musicale sur l'écran Concentration.",
    windowMode:
      "Lecture en arrière-plan via YouTube — le propriétaire de la vidéo n'autorise pas son intégration dans des applications.",
    windowModeShort: "Lecture en arrière-plan via YouTube",
    showWindow: "Afficher la fenêtre",
    showWindowAria: "Afficher la fenêtre YouTube",
    windowClosed: "La fenêtre YouTube a été fermée",
    reopenWindow: "Rouvrir",
    reopenWindowAria: "Rouvrir la fenêtre YouTube",
  },

  focusView: {
    nowUsing: "En cours d'utilisation",
    nowUsingEmpty: "Collecte des données des applications…",
    trackingOff: "Le suivi des applications est désactivé dans les Paramètres.",
    idle: "Inactivité",
    unknown: "Inconnu",
    shields: "Boucliers",
    shieldsNone: "Aucun bouclier n'est activé — vous pouvez les activer dans les Paramètres.",
  },

  shieldChip: {
    notifications: "Notifications en sourdine",
    teams: "Teams : Ne pas déranger",
    sites: "Blocage des sites",
    sitesFallback: "Blocage des sites (mode de secours)",
    keepAwake: "L'écran reste allumé",
    launchers: "Lanceurs de jeux",
    launchersClosed: (names: string) => `Fermés : ${names}`,
    launchersNothing: "Aucun des lanceurs sélectionnés n'était ouvert.",
    state: {
      off: "désactivé",
      on: "activé",
      unavailable: "indisponible",
      error: "erreur",
    },
    chipAria: (name: string, state: string) => `${name} : ${state}`,
  },

  breakView: {
    title: "C'est l'heure de la pause",
    lead: "Éloignez-vous un peu de votre bureau. La musique peut continuer.",
    reminderLabel: (n: number) => `Votre rappel · pause ${n}`,
    suggestionLabel: "Une suggestion pour cette pause",
    another: "Autre suggestion",
    source: "Source : ",
    sourceAria: (host: string) => `Source de la suggestion : ${host} (ouvre le navigateur)`,
    categories: {
      eyes: "Yeux",
      stretch: "Étirements",
      move: "Mouvement",
      hydrate: "Eau",
      breathe: "Respiration",
      posture: "Poste de travail",
      outside: "Dehors",
      social: "Lien social",
      mind: "Un moment pour soi",
      tidy: "Environnement",
    },
  },

  idleTips: {
    practiceTitle: "Bonne pratique",
    lawTitle: "Bon à savoir",
    source: "Source : ",
    sourceAria: (host: string) => `Source : ${host} (ouvre le navigateur)`,
  },

  summary: {
    title: "Résumé",
    lastSession: "Dernière session",
    selectedSession: "Session sélectionnée",
    empty: "Pas encore de résumé. Terminez une session avec Arrêter et vous verrez ici un graphique de vos applications.",
    focusTime: "Temps de concentration",
    cycles: "Cycles terminés",
    cyclesCount: (n: number) => `${n} ${plural(n, { one: "cycle", other: "cycles" })}`,
    breaks: "Temps de pause",
    span: "Début – fin",
    chartTitle: "Temps de concentration par application",
    chartSubtitle: (time: string) => `Temps suivi : ${time}`,
    chartEmpty: "Aucune donnée d'application (le suivi était désactivé ou la session très courte).",
    other: (n: number) => `Autres (${n})`,
    otherPlain: "Autres",
    idle: "Inactivité",
    unknown: "Inconnu",
    barAria: (name: string, time: string, pct: string) => `${name} : ${time}, ${pct}`,
    titlesHeading: "Fenêtres les plus fréquentes",
    titlesHint: "Survolez une application, ou atteignez-la avec la touche Tab, pour voir ses fenêtres les plus fréquentes.",
    notSaved:
      "Cette session n'a pas été ajoutée à l'historique — l'enregistrement de l'historique est désactivé dans les Paramètres.",
    history: "Historique",
    historyEmpty: "L'historique est vide.",
    historyLegend: "Couleurs des applications dans l'historique",
    demo: "démo",
    showSession: (date: string) => `Afficher la session du ${date}`,
    backToLast: "Revenir à la dernière session",
    privacy: "Les données des applications restent sur cet ordinateur.",
    goalLabel: "Objectif de la session",
    goalQuestion: "Atteint ?",
    goalYes: "Oui",
    goalNo: "Non",
    goalGroup: "L'objectif de la session a-t-il été atteint ?",
    goalDone: "Atteint — bravo !",
    goalNotDone: "Pas cette fois — vous pourrez le choisir à nouveau dans « Et ensuite ? ».",
    goalOpenAria: (name: string) => `Ouvrir ${name} dans le navigateur`,
    goalMark: (done: boolean | null) => (done === true ? "objectif ✓" : done === false ? "objectif ✗" : "objectif"),
    goalMarkTitle: (name: string, done: boolean | null) =>
      `Objectif : ${name}${done === true ? " — atteint" : done === false ? " — non atteint" : ""}`,
    goalAria: (done: boolean | null) =>
      done === true ? "objectif atteint" : done === false ? "objectif non atteint" : "objectif sans réponse",
  },

  settings: {
    title: "Paramètres",
    saving: "Enregistrement…",
    saved: "Enregistré",
    shieldsTitle: "Boucliers pendant la concentration",
    shieldsLead:
      "Ils s'activent pendant la concentration et se désactivent d'eux-mêmes pendant les pauses et après l'arrêt.",

    notifications: "Mettre en sourdine les notifications Windows",
    notificationsHint:
      "Active « Ne pas déranger » de Windows pendant la concentration et le désactive pendant les pauses, après l'arrêt et à la fermeture de l'application — même s'il était déjà activé avant.",

    teams: "Microsoft Teams : « Ne pas déranger »",
    teamsHint:
      "Définit votre statut Teams pendant la concentration et le réinitialise pendant les pauses. Si vous apparaissez hors ligne dans Teams, ou si vous avez choisi vous-même « Ne pas déranger », FocusMode ne touche pas à votre statut. Nécessite de connecter votre compte une seule fois.",
    teamsClientId: "Application (client) ID",
    teamsTenant: "Locataire (ID de l'annuaire ou « organizations »)",
    teamsConnect: "Se connecter à Teams",
    teamsDisconnect: "Se déconnecter",
    teamsConnected: (account: string | null) => (account ? `Connecté en tant que ${account}` : "Connecté"),
    teamsNotConnected: "Non connecté",
    teamsNotConfigured: "Saisissez d'abord l'Application (client) ID.",
    teamsErrorLead: "Impossible de se connecter à Teams :",
    teamsPendingLead: "Ouvrez la page de connexion Microsoft et saisissez le code :",
    teamsOpenLink: "Ouvrir la page de connexion",
    teamsCopyCode: "Copier le code",
    teamsCopied: "Copié",
    teamsCodeExpires: (min: number) =>
      `Le code expire dans environ ${min} ${plural(min, { one: "minute", other: "minutes" })}.`,
    teamsHelpTitle: "Comment obtenir un Application ID ?",
    teamsHelp: [
      "Ouvrez le portail Azure → Microsoft Entra ID → Inscriptions d'applications → Nouvelle inscription.",
      "Type de compte : comptes dans un annuaire d'organisation. Plateforme : « Applications mobiles et de bureau » (Mobile and desktop applications).",
      "Sous « Authentification », activez « Autoriser les flux de clients publics » (Allow public client flows).",
      "Sous « Autorisations de l'API », ajoutez les autorisations déléguées Microsoft Graph Presence.ReadWrite et User.Read, ainsi que Calendars.Read pour le calendrier.",
      "Copiez ici l'« ID d'application (client) » (Application (client) ID). Si votre administrateur doit donner son consentement, demandez-le-lui.",
    ],

    sites: "Bloquer les sites qui distraient",
    sitesHint:
      "Pendant la concentration, l'extension du navigateur remplace les sites de la liste ci-dessous par une page « Retour au travail ». Rien n'est envoyé sur Internet.",
    extensionTitle: "Extension du navigateur (Chrome, Edge, Brave, Opera, Vivaldi)",
    extensionStep1: "Ouvrez le dossier de l'extension :",
    extensionOpenDir: "Ouvrir le dossier de l'extension",
    extensionStep2:
      "Dans votre navigateur, allez sur chrome://extensions (Edge : edge://extensions, Opera : opera://extensions) et activez le « Mode développeur ».",
    extensionStep3: "Cliquez sur « Charger l'extension non empaquetée » et sélectionnez le dossier ouvert.",
    extensionConnected: "Extension connectée",
    extensionDisconnected: "Extension non connectée — détection de secours par le titre de la fenêtre",
    extensionSeen: (ago: string) => `dernier contact il y a ${ago}`,

    keepAwake: "Garder l'écran allumé",
    keepAwakeHint: "Votre ordinateur et votre écran ne se mettront pas en veille pendant la concentration.",

    historyTitle: "Historique",
    trackApps: "Enregistrer les applications utilisées pendant la concentration",
    trackAppsHint:
      "Chaque seconde, FocusMode vérifie le nom du processus de la fenêtre active. La barre des tâches, le menu Démarrer et les autres éléments du bureau Windows sont ignorés. Les données restent sur cet ordinateur.",
    trackTitles: "Enregistrer les titres des fenêtres",
    trackTitlesHint:
      "Pour chaque application, FocusMode retient quelques-uns de ses titres de fenêtre les plus fréquents (p. ex. un onglet du navigateur ou un dossier) et les affiche quand vous survolez le graphique du Résumé. Ils restent uniquement sur cet ordinateur et n'apparaissent jamais dans les journaux.",
    trackTitlesNeedsApps: "Fonctionne quand l'enregistrement des applications est activé.",
    keepHistory: "Enregistrer l'historique des sessions",
    keepHistoryHint:
      "Si cette option est désactivée, le résumé s'affiche quand même après l'arrêt, mais la session n'est pas enregistrée. Les sessions enregistrées auparavant restent jusqu'à ce que vous les supprimiez.",
    clearHistory: "Supprimer tout l'historique",
    clearHistoryHint: "Supprime de cet ordinateur toutes les sessions enregistrées et le dernier résumé.",
    clearHistoryTitle: "Supprimer tout l'historique ?",
    clearHistoryBody:
      "Toutes les sessions enregistrées, les graphiques des applications et les titres des fenêtres seront définitivement supprimés de cet ordinateur. Cette action est irréversible.",
    clearHistoryConfirm: "Oui, supprimer l'historique",
    historyCleared: "L'historique a été supprimé.",

    appearanceTitle: "Apparence",

    soundTitle: "Son",
    chimeSound: "Sonnerie",
    chimeSounds: {
      midnight: "Minuit — cloche douce",
      vaporwave: "Vaporwave — accord de synthé",
      lunar: "Lune — notes cristallines",
      autumn: "Automne — marimba en bois",
      classic: "Classique — ding-dong",
      soft: "Douce — son feutré",
      chiptune: "8 bits — arpège",
      custom: "Personnalisée…",
    },
    chimeSoundHint:
      "Les noms correspondent aux thèmes d'apparence, mais vous pouvez choisir une sonnerie indépendamment du thème.",
    customChimes: "Vos propres sons",
    customBreak: "Début de la pause",
    customFocus: "Début de la concentration (facultatif)",
    customNone: "Aucun fichier choisi",
    customDefaultFallback: "Sans fichier personnel, c'est la sonnerie Minuit qui retentit.",
    customBreakUsesFocus: "Sans fichier distinct, c'est le même son qu'au début de la concentration.",
    customFocusUsesBreak: "Sans fichier distinct, c'est le même son qu'au début de la pause.",
    customPick: "Choisir un fichier…",
    customChange: "Changer de fichier…",
    customPickTitle: (label: string) => `Choisir un son : ${label}`,
    customFilter: "Son (mp3, wav, ogg, flac, m4a)",
    customRemove: "Supprimer",
    customRemoveAria: (label: string) => `Supprimer votre son : ${label}`,
    customHint: "Fichiers mp3, wav, ogg, flac ou m4a jusqu'à 10 Mo. FocusMode les copie dans son dossier de données.",
    customImported: "Votre son a été ajouté.",
    chimeVolume: "Volume de la sonnerie",
    testChime: "Tester la sonnerie :",
    testBreak: "pause",
    testFocus: "concentration",
    musicVolume: "Volume de la musique",

    remindersTitle: "Rappels pendant les pauses",
    remindersLead:
      "Votre propre liste pour les pauses d'une session, p. ex. pause 1 : étirements, 2 : de l'eau et une promenade avec le chien, 3 : déjeuner. Le rappel s'affiche sur l'écran de pause et dans la fenêtre pop-up FocusMode.",
    reminderLabel: (n: number) => `Pause ${n}`,
    reminderPlaceholders: ["p. ex. étirements", "p. ex. de l'eau et une promenade avec le chien", "p. ex. déjeuner"],
    reminderPlaceholderOther: "Pas de rappel",
    reminderAdd: "Ajouter une pause",
    reminderRemove: (n: number) => `Supprimer le rappel de la pause ${n}`,
    reminderUp: (n: number) => `Monter le rappel de la pause ${n}`,
    reminderDown: (n: number) => `Descendre le rappel de la pause ${n}`,
    reminderChars: (used: number, max: number) => `${used}/${max}`,
    reminderCharsAria: (used: number, max: number) =>
      `${used} ${plural(used, { one: "caractère saisi", other: "caractères saisis" })} sur ${max}`,
    reminderMoved: (n: number) => `Déplacé en position ${n}.`,
    reminderRemoved: "Rappel supprimé.",
    remindersEmpty: "Aucun rappel — pendant les pauses, vous ne verrez que des suggestions.",
    remindersFull: (max: number) =>
      `La liste contient déjà ${max} ${plural(max, { one: "élément", other: "éléments" })} — c'est le maximum.`,
    remindersRepeat: "Reprendre la liste depuis le début",
    remindersRepeatHint:
      "Après le dernier élément, la liste recommence. Sinon, les pauses suivantes n'ont pas de rappel.",
    remindersInvalid: (max: number, chars: number) =>
      `${max} rappels au maximum, de ${chars} ${plural(chars, { one: "caractère", other: "caractères" })} chacun au plus.`,

    systemTitle: "Système",
    launchOnLogin: "Lancer FocusMode à l'ouverture de session",
    launchOnLoginHint:
      "Après l'ouverture de session, FocusMode démarre réduit dans la zone de notification (l'icône à côté de l'horloge).",

    launchersTitle: "Lanceurs de jeux",
    launchers: "Fermer les lanceurs de jeux au début de la concentration",
    launchersHint:
      "À chaque début de concentration, FocusMode ferme les lanceurs sélectionnés. Seuls les lanceurs sont fermés, pas les jeux eux-mêmes — mais un jeu lancé depuis un lanceur peut alors perdre sa progression non enregistrée, sauvegardez donc d'abord votre partie.",
    launchersPick: "Lanceurs à fermer",
    launchersNone: "Sélectionnez au moins un lanceur.",
    launchersLastClosed: (names: string) => `Au dernier début de concentration, FocusMode a fermé : ${names}.`,
    launchersLastNone: "Au dernier début de concentration, aucun des lanceurs sélectionnés n'était ouvert.",
    launchersUnavailable: "La fermeture des lanceurs ne fonctionne que sous Windows.",
    launchersError: "Un des lanceurs n'a pas pu être fermé.",

    windowTitle: "Fenêtre et pop-ups",
    closeButton: "Le bouton X (fermer la fenêtre)",
    closeAsk: "Demander la première fois",
    closeTray: "Réduire dans la zone de notification",
    closeQuit: "Quitter l'application",
    closeHint:
      "Réduit dans la zone de notification, FocusMode continue de minuter et de jouer la musique. Rouvrez-le depuis l'icône à côté de l'horloge.",
    phasePopups: "Fenêtres pop-up FocusMode au changement de phase",
    phasePopupsHint:
      "Une petite fenêtre dans le coin de l'écran quand la concentration ou une pause commence. Elle remplace les notifications Windows.",

    tintTitle: "Couleurs chaudes de l'écran",
    tint: "Réchauffer les couleurs de l'écran selon l'heure",
    tintHint:
      "Le soir, les couleurs de l'écran deviennent peu à peu plus chaudes, et le matin les couleurs habituelles reviennent. Le réchauffement est appliqué après la composition de l'image, il n'apparaît donc ni sur les captures d'écran ni dans les enregistrements.",
    tintMode: "Méthode de réchauffement",
    tintModes: {
      nightLight: "Éclairage nocturne de Windows (recommandé)",
      gamma: "Directement sur la carte graphique (gamma)",
    },
    tintModeShort: {
      nightLight: "Éclairage nocturne de Windows",
      gamma: "gamma de la carte graphique",
    },
    tintModeHints: {
      nightLight:
        "L'« Éclairage nocturne » intégré à Windows. FocusMode l'active selon un horaire et règle son intensité — sans droits d'administrateur. À partir de Windows 11 24H2, il ne peut plus être piloté de l'extérieur ; FocusMode réchauffe alors l'écran via le gamma.",
      gamma:
        "Modifie la courbe des couleurs de chaque moniteur, ce qui couvre tout l'écran, y compris la barre des tâches et les jeux en plein écran. Ne fonctionne pas sur les écrans en mode HDR, et Windows peut refuser les réglages très chauds.",
    },
    tintFrom: "De",
    tintTo: "À",
    tintScheduleHint: "Les horaires peuvent passer minuit, p. ex. de 18:00 à 9:00.",
    tintStrength: "Intensité",
    tintStrengthHint: "Déplacez le curseur pour un aperçu à l'écran.",
    tintTransition: "Transition progressive",
    tintTransitionNone: "Sans transition",
    tintTransitionMinutes: (m: number) => `${m} min`,
    tintStatusOff: "Actuellement : désactivé",
    tintStatusLevel: (pct: number) => `Actuellement : ${pct} %`,
    tintStatusPreview: (pct: number) => `Aperçu : ${pct} %`,
    tintStatusFallback: (mode: string) => `La méthode choisie n'est pas disponible — utilisation de : ${mode}.`,
    tintStatusCapped: "Windows n'a pas autorisé l'intensité maximale — un réchauffement plus léger est appliqué.",
    tintStatusError: "Impossible de réchauffer les couleurs de l'écran — nouvel essai dans un instant.",
    tintStatusRestoreError: "Impossible de rétablir les couleurs habituelles de l'écran — nouvel essai dans un instant.",
    tintTimeInvalid: "Indiquez une heure, p. ex. 18:00.",
    tintSameTimes: "« À » doit être différent de « De ».",
    tintStrengthRange: "Indiquez une intensité entre 0 et 100.",
    tintTransitionRange: "La transition progressive peut durer de 0 à 180 minutes.",

    languageTitle: "Langue",
    language: "Langue de l'application",
    languageSystem: (name: string | null) => (name ? `Système (${name})` : "Système"),

    demoNote: "Mode démo : la concentration dure 40 s, la pause 20 s.",
  },

  closePrompt: {
    title: "Réduire FocusMode dans la zone de notification ?",
    body: "Le minuteur et la musique continuent en arrière-plan, et FocusMode affiche une petite fenêtre pop-up quand la concentration ou une pause commence.",
    note: "Vous pouvez modifier ce choix dans les Paramètres.",
    toTray: "Oui, dans la zone de notification",
    quit: "Non, quitter l'application",
  },

  popup: {
    title: {
      breakStarted: "C'est la pause",
      focusStarted: "Place à la concentration",
      interrupted: "Session suspendue",
      launchersClosed: "Lanceurs de jeux fermés",
      meetingReminder: "Réunion pendant une pause",
      breakHeld: "Pause suspendue",
      workDigest: "Et ensuite ?",
    },
    workDigest: (n: number) =>
      `${n} ${plural(n, { one: "nouvelle tâche ou notification attend", other: "nouvelles tâches et notifications attendent" })} la pause`,
    workDigestGeneric: "De nouvelles tâches et notifications attendent la pause",
    meetingMessage: (range: string, subject: string) => `${range} · ${subject}`,
    meetingTitle: (n: number) => `Réunion pendant la pause ${n}`,
    meetingBreakAt: (range: string) => `Pause prévue : ${range}`,
    meetingChoices: (subject: string) => `Que faire de la réunion ${subject} ?`,
    breakHeld: (subject: string) =>
      `Réunion : ${subject}. Le minuteur de la pause attend — relancez-le après la réunion.`,
    breakHeldGeneric: "Une réunion est en cours. Le minuteur de la pause attend — relancez-le après la réunion.",
    breakHeldTitle: (time: string) => `Pause suspendue · ${time}`,
    choiceFailed: "Impossible d'enregistrer votre choix — choisissez dans la fenêtre FocusMode.",
    breakTitle: (time: string) => `C'est la pause · ${time}`,
    breakMusicOn: "La musique continue — éloignez-vous un peu de votre bureau.",
    breakMusicPaused: "La musique vous attend — éloignez-vous un peu de votre bureau.",
    breakNoMusic: "Éloignez-vous un peu de votre bureau.",
    reminderPrefix: "Votre rappel :",
    tipPrefix: (category: string) => `Suggestion (${category}) :`,
    focusMessage: (cycle: number, time: string) => `Cycle ${cycle} : ${time} de concentration. Boucliers activés.`,
    focusMessageNoShields: (cycle: number, time: string) => `Cycle ${cycle} : ${time} de concentration.`,
    interrupted: "L'ordinateur était en veille — reprenez quand vous serez de retour.",
    launchersClosed: (names: string) => `Fermés : ${names}`,
    showMain: "Afficher FocusMode",
    skip: "Passer",
    skipBreak: "Passer la pause",
    skipFocus: "Passer la concentration",
    dismiss: "Fermer",
  },

  blocklist: {
    title: "Liste des sites — websiteblacklist.txt",
    pathLabel: "Fichier de la liste",
    loading: "Chargement de la liste des sites…",
    summary: (domains: number, patterns: number) =>
      `${domains} ${plural(domains, { one: "domaine", other: "domaines" })} · ${patterns} ${plural(patterns, { one: "expression régulière", other: "expressions régulières" })}`,
    empty: "La liste est vide — aucun site ne sera bloqué pendant la concentration.",
    open: "Ouvrir websiteblacklist.txt",
    reload: "Recharger",
    reloaded: "La liste des sites a été rechargée.",
    autoReload: "Dès que vous enregistrez le fichier, la liste se recharge d'elle-même — pas besoin de redémarrer.",
    allValid: "Toutes les lignes sont valides",
    issuesTitle: (n: number) =>
      `${n} ${plural(n, { one: "ligne ignorée — corrigez-la", other: "lignes ignorées — corrigez-les" })} dans le fichier ou ajoutez # en début de ligne :`,
    issueLine: (line: number) => `Ligne ${line}`,
    issueText: {
      invalidDomain: "Domaine non valide",
      invalidRegex: "Expression régulière non valide",
      regexTooComplex: "L'expression régulière est trop complexe",
      matchesEverything: "L'expression correspond à toutes les adresses",
      pastedUrl:
        "Une adresse avec ? est une expression régulière qui ne correspond pas à cette adresse — pour bloquer uniquement cette adresse, remplacez ? par \\?",
      lineTooLong: "La ligne est trop longue (limite : 500 caractères)",
      tooManyDomains: "Trop de domaines (limite : 2000)",
      tooManyPatterns: "Trop d'expressions régulières (limite : 500)",
    },
    pastedUrl: (domain: string) =>
      `Une adresse avec ? est une expression régulière qui ne correspond pas à cette adresse — pour bloquer tout le site, saisissez ${domain} ; pour bloquer uniquement cette adresse, remplacez ? par \\?`,
    issueDetailLabel: "Détails (en anglais)",
    readError: "Impossible de lire le fichier — la liste précédente reste active :",
    helpTitle: "Comment écrire les entrées ?",
    helpLines:
      "Une entrée par ligne. Les lignes qui commencent par # sont des commentaires ; les lignes vides sont ignorées.",
    helpDomain: "Un domaine simple se bloque lui-même ainsi que tous ses sous-domaines (p. ex. www., m.).",
    helpRegex:
      "Une expression régulière est une ligne entre barres obliques /…/ ou qui contient l'un des caractères \\ ^ $ * + ? ( ) [ ] { } |. Elle est comparée à l'adresse complète de la page, sans tenir compte des majuscules. Un point signifie n'importe quel caractère : dans les domaines, écrivez donc \\.",
    examplesLabel: "Exemples :",
    examples: [
      { code: "reddit.com", note: "tout reddit.com, sous-domaines compris" },
      { code: "youtube\\.com/shorts", note: "uniquement YouTube Shorts ; le reste de YouTube fonctionne" },
      { code: "linkedin\\.com/feed", note: "uniquement le fil d'actualité LinkedIn" },
    ],
    extensionSkippedTitle:
      "Le navigateur ne peut pas utiliser ces expressions comme règles — je les vérifie au chargement d'un onglet",
  },

  mascot: {
    setting: "Afficher la mascotte (axolotl)",
    settingHint:
      "Un axolotl avec un casque audio à côté de l'horloge, pendant les pauses, dans le résumé et dans les fenêtres pop-up au changement de phase. Il partage de bonnes pratiques et des suggestions pour la pause.",
    practiceLabel: "Bonne pratique",
    nextUpLabel: "Et ensuite ?",
    startLines: [
      "Au travail !",
      "C'est parti. Bon courage !",
      "Casque sur les oreilles — on y va !",
      "Une tâche à la fois. Allons-y !",
    ],
    breakLines: [
      "Pause ! Éloignez-vous un peu de l'écran.",
      "C'est l'heure de la pause. Bien méritée !",
      "Pause — dégourdissez-vous les jambes.",
    ],
    cycleLines: [
      "Cycle terminé ! On se reconcentre.",
      "Bravo, encore un cycle de fait !",
      "Ça avance bien. Prochain tour !",
    ],
    conflictHint: (subject: string) =>
      `Une réunion tombe pendant une pause : ${subject}. Choisissez ci-dessous quoi en faire.`,
  },

  calendar: {
    title: "Calendriers",
    lead: "FocusMode consulte les réunions des prochaines heures et, avant même le début de la session, vous prévient si l'une d'elles tombe pendant une pause. Les calendriers sont uniquement lus — FocusMode n'y modifie jamais rien.",
    privacy:
      "Les réunions restent sur cet ordinateur. Les jetons de connexion et les liens des calendriers sont chiffrés, stockés hors du fichier de paramètres, et n'apparaissent jamais dans les journaux.",
    horizon: "Vérifier les réunions sur",
    horizonOption: (h: number) => `${h} ${plural(h, { one: "heure", other: "heures" })}`,
    horizonHint: "Pour une longue session, choisissez une période plus longue afin de couvrir toutes ses pauses.",
    lastSync: (time: string) => `Dernière synchronisation : ${time}`,
    neverSynced: "Les calendriers n'ont pas encore été lus.",
    refresh: "Actualiser",
    refreshing: "Actualisation…",
    sourceName: {
      microsoft: "Microsoft 365 / Outlook",
      google: "Google Agenda",
      ics: "Calendriers via un lien ICS",
    },
    sourceShort: {
      microsoft: "Outlook",
      google: "Google",
      ics: "ICS",
    },
    state: {
      off: "désactivé",
      on: "connecté",
      unavailable: "indisponible",
      error: "erreur",
    },
    statusLine: (state: string, account: string | null) => (account ? `${state} · ${account}` : state),
    errorLead: "Impossible de lire le calendrier :",

    microsoft: "Microsoft 365 / Outlook (le compte Microsoft de Teams)",
    microsoftHint:
      "Lit votre calendrier via le même compte Microsoft que l'intégration Teams (Microsoft Graph, lecture seule : Calendars.Read). Inclut les réunions Teams et tous les autres événements du calendrier.",
    microsoftReconnect:
      "FocusMode ne demande l'autorisation de lire votre calendrier (Calendars.Read) que lorsque vous vous connectez avec le calendrier activé. Un compte connecté auparavant uniquement pour Teams doit être reconnecté — inutile de le déconnecter, votre statut Teams continue de fonctionner. Dans l'inscription de l'application sur Azure, ajoutez l'autorisation Calendars.Read.",
    microsoftTeamsAbove:
      "Vous connectez le compte Microsoft plus haut, dans le bouclier « Microsoft Teams : Ne pas déranger ».",
    microsoftConnect: "Connecter un compte Microsoft",
    microsoftNeedsReconnect:
      "Cette connexion Microsoft n'a pas accès au calendrier. Reconnectez le compte et autorisez la lecture du calendrier.",
    microsoftReconnectButton: "Reconnecter",
    microsoftReconnecting: "Connexion…",
    microsoftReconnectCode: (code: string) =>
      `Saisissez le code ${code} sur la page de connexion Microsoft (elle est ouverte dans votre navigateur ; le lien se trouve aussi plus haut, dans le bouclier Teams).`,

    google: "Google Agenda",
    googleHint:
      "Vous vous connectez dans le navigateur, et FocusMode demande uniquement à lire votre agenda. Il vous faut votre propre ID client OAuth, gratuit, créé dans Google Cloud.",
    googleClientId: "ID client OAuth",
    googleClientIdPlaceholder: "123456789012-abc….apps.googleusercontent.com",
    googleClientIdInvalid:
      "Un ID client Google se termine par .apps.googleusercontent.com et ne contient que des lettres, des chiffres, des points, des tirets et des traits de soulignement.",
    googleClientSecret: "Code secret du client (Client secret)",
    googleClientSecretPlaceholder: "GOCSPX-…",
    googleClientSecretHint:
      "Issu du même client « Application de bureau » de Google Cloud que l'ID ci-dessus. Google l'exige pour la connexion mais ne le considère pas comme secret dans les applications installées, c'est pourquoi FocusMode l'enregistre dans le fichier de paramètres (jamais dans les journaux).",
    googleClientSecretInvalid: (max: number) =>
      `Le code secret du client ne contient que des lettres, des chiffres, des tirets et des traits de soulignement (${max} caractères au maximum).`,
    googleConnect: "Se connecter à Google",
    googleDisconnect: "Se déconnecter",
    googlePending: "Terminez la connexion dans la fenêtre du navigateur qui s'est ouverte…",
    googleConnected: (account: string | null) => (account ? `Connecté en tant que ${account}` : "Connecté"),
    googleNotConnected: "Non connecté",
    googleNeedsClientId: "Saisissez d'abord l'ID client.",
    googleErrorLead: "Impossible de se connecter à Google :",
    googleHelpTitle: "Comment obtenir un ID client Google ?",
    googleHelp: [
      "Ouvrez console.cloud.google.com et créez un projet (ou choisissez-en un existant).",
      "Sous « API et services » → « Bibliothèque », activez Google Calendar API.",
      "Configurez l'écran de consentement OAuth (Google Auth Platform) : audience « Externe » (ou « Interne » dans le Google Workspace d'une entreprise), champ d'application …/auth/calendar.readonly. Tant que l'application est en mode test, ajoutez votre adresse e-mail comme utilisateur test.",
      "Sous « Clients » (Identifiants), créez un ID client OAuth de type « Application de bureau ».",
      "Copiez l'« ID client » (il se termine par .apps.googleusercontent.com) et le « Code secret du client » (il commence par GOCSPX-) dans les champs ci-dessus, puis appuyez sur « Se connecter à Google ».",
    ],

    ics: "Autres calendriers (lien ICS / iCal)",
    icsHint:
      "Pour les calendriers dotés d'un lien d'abonnement privé : Outlook et Exchange en entreprise, Google, iCloud, Proton, Zoho, Zimbra, Nextcloud et d'autres. Les réunions périodiques sont aussi prises en compte.",
    icsLinks: "Liens des calendriers — un par ligne",
    icsPlaceholder: "https://outlook.office365.com/owa/calendar/…/calendar.ics\nwebcal://…",
    icsSave: "Enregistrer les liens",
    icsSaved: "Les liens des calendriers ont été enregistrés.",
    icsEdit: "Modifier les liens",
    icsAdd: "Ajouter des liens",
    icsCancel: "Annuler",
    icsRemoveAll: "Supprimer tous les liens",
    icsRemoved: "Les liens des calendriers ont été supprimés.",
    icsCount: (n: number) => (n === 0 ? "aucun lien" : `${n} ${plural(n, { one: "lien", other: "liens" })}`),
    icsLoading: "Chargement des liens enregistrés…",
    icsInvalidLines: (lines: readonly number[]) =>
      `${lines.length === 1 ? "Ligne" : "Lignes"} ${lines.join(", ")} : ceci ne ressemble pas à un lien de calendrier (https:// ou webcal://).`,
    icsSecret:
      "Toute personne qui connaît un tel lien peut voir votre calendrier. FocusMode le stocke chiffré et ne l'affiche jamais dans les messages d'erreur.",
    icsHelpTitle: "Où trouver le lien ICS ?",
    icsHelp: [
      {
        name: "Outlook sur le web (Microsoft 365, Exchange)",
        text: "Paramètres → Calendrier → Calendriers partagés → « Publier un calendrier » : choisissez le calendrier et « Peut afficher tous les détails », appuyez sur « Publier » et copiez le lien ICS. Sur un serveur Exchange d'entreprise, la publication doit être activée par l'administrateur.",
      },
      {
        name: "Google Agenda",
        text: "Paramètres → choisissez l'agenda à gauche → « Intégrer l'agenda » → « Adresse secrète au format iCal ».",
      },
      {
        name: "iCloud",
        text: "Dans Calendrier, partagez le calendrier en tant que « Calendrier public » et copiez son lien (webcal://).",
      },
      {
        name: "Proton, Zoho, Zimbra, Nextcloud et autres",
        text: "Dans les paramètres du calendrier, cherchez « Partager », « S'abonner », « Lien ICS » ou « iCal ».",
      },
    ],

    cardTitle: "Réunions pendant les pauses",
    leadPlanned: "Si vous démarrez maintenant, ces réunions tomberont pendant des pauses. Choisissez quoi faire :",
    leadRunning: "Ces réunions tombent pendant les pauses de cette session :",
    noConflictsPlanned: "Si vous démarrez maintenant, aucune réunion ne tombera pendant une pause.",
    noConflictsRunning: "Aucune réunion ne tombe pendant les pauses de cette session.",
    breakInfo: (n: number, range: string) => `Pause ${n} : ${range}`,
    choice: {
      extend: "Prolonger la concentration (musique en sourdine)",
      pauseAtBreak: "Suspendre à la pause",
      ignore: "Ignorer",
      remindLater: "Me le rappeler plus tard",
    },
    choiceExtend: (end: string) =>
      `La concentration dure jusqu'à la fin de la réunion (${end}), musique en sourdine. La pause commence après la réunion.`,
    choicePauseAtBreak:
      "À l'heure de la pause, le minuteur s'arrête et attend que vous appuyiez sur « Reprendre ».",
    choiceIgnore: "La session continue sans changement.",
    choiceRemindLater: (time: string | null) =>
      time
        ? `Je vous le rappellerai à ${time} — dans une petite fenêtre pop-up FocusMode.`
        : "Je vous le rappellerai peu avant la réunion.",
    choiceNone: "Choisissez quoi faire.",
    tooLong: "Réunion de plus de 4 h — la concentration ne sera pas prolongée",
    choiceGroup: (subject: string) => `Que faire de la réunion ${subject}`,
    upcomingTitle: "Prochaines réunions",
    upcomingEmpty: (hours: number) =>
      plural(hours, {
        one: "Aucune réunion dans l'heure qui vient.",
        other: `Aucune réunion dans les ${hours} prochaines heures.`,
      }),
    now: "maintenant",
    online: "en ligne",
    onlineAria: "réunion en ligne",
    tomorrow: "demain",
    untitled: "(sans titre)",
    connectHint:
      "Connectez un calendrier et, avant chaque session, FocusMode vous préviendra des réunions qui tombent pendant les pauses.",
    connectLink: "Paramètres des calendriers",
    sourceErrors: (names: string) => `Lecture impossible : ${names}.`,
    sourceErrorsLink: "Détails dans les Paramètres",
    loading: "Chargement du calendrier…",

    holdExtend: (time: string, subject: string) => `Concentration prolongée jusqu'à ${time} — réunion : ${subject}`,
    holdExtendHint: "La musique reste en sourdine jusqu'à la fin de la réunion ; ensuite, la pause commence.",
    holdPause: (subject: string) => `Pause suspendue — réunion : ${subject}`,
    holdPauseHint: (time: string) =>
      `Le minuteur de la pause attend. Appuyez sur « Reprendre » quand la réunion est terminée (fin prévue à ${time}).`,
  },

  work: {
    title: "Outils de travail (Et ensuite ?)",
    lead: "Connectez les outils avec lesquels vous travaillez et FocusMode vous suggérera par quoi commencer lors de votre prochaine session : d'abord les tâches qui vous sont attribuées, puis ce que d'autres attendent de vous (revues, corrections, mentions), enfin les autres notifications. Les outils sont uniquement lus — FocusMode n'y modifie jamais rien.",
    privacy:
      "Les jetons sont chiffrés, stockés hors du fichier de paramètres, et n'apparaissent jamais dans les journaux. FocusMode se connecte uniquement aux outils activés ci-dessus ; les tâches et notifications restent sur cet ordinateur et ne sont jamais envoyées à une IA ni à aucun autre service.",
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
        "Les tickets qui vous sont attribués, les pull requests qui attendent votre revue ou vos corrections et — avec un jeton classique — les notifications.",
      gitlab:
        "Les tickets qui vous sont attribués, les merge requests à relire et vos merge requests dont la CI a échoué. Fonctionne avec GitLab.com et avec les instances autogérées.",
      jira: "Les tickets qui vous sont attribués, avec le sprint actif, la priorité et l'échéance. Jira Cloud ou Data Center.",
      azureDevops:
        "Les éléments de travail qui vous sont attribués (avec l'itération en cours) et les pull requests qui attendent votre revue.",
      linear: "Les tickets qui vous sont attribués, avec le cycle en cours, la priorité et l'échéance.",
    },
    state: {
      off: "désactivé",
      on: "connecté",
      unavailable: "non connecté",
      error: "erreur",
    },
    connectedAs: (account: string | null) => (account ? `Connecté en tant que ${account}` : "Connecté"),
    syncing: "Jeton enregistré — première lecture en cours…",
    notConnected: "Non connecté — enregistrez un jeton ci-dessous.",
    noSecretStore: "Ce système ne peut pas stocker le jeton de façon sécurisée, cet outil ne fonctionne donc pas.",
    tokenInvalid: "Un jeton ne peut contenir que des caractères ASCII visibles, sans espaces (4 Ko au maximum).",
    needsUrl: {
      gitlab: "Saisissez d'abord l'adresse GitLab.",
      jira: "Saisissez d'abord l'adresse Jira.",
      azureDevops: "Saisissez d'abord le nom de l'organisation.",
    },
    errorLead: "Lecture impossible :",

    gitlabUrl: "Adresse GitLab",
    gitlabUrlPlaceholder: "https://gitlab.com",
    gitlabUrlHint: "Pour une instance autogérée, saisissez son adresse, p. ex. https://gitlab.example.com.",
    jiraUrl: "Adresse Jira",
    jiraUrlPlaceholder: "https://votre-entreprise.atlassian.net",
    jiraUrlHint:
      "L'adresse de votre site Jira Cloud ou de votre serveur Jira Data Center. Vous pouvez coller l'adresse depuis le navigateur, sans la partie qui suit le ?.",
    jiraEmail: "E-mail du compte Atlassian (Jira Cloud uniquement)",
    jiraEmailPlaceholder: "jean.dupont@example.com",
    jiraEmailHint: "Pour Jira Data Center avec un jeton d'accès personnel, laissez ce champ vide.",
    azureOrg: "Organisation Azure DevOps",
    azureOrgPlaceholder: "votre-entreprise",
    azureOrgHint: "Le nom figurant dans l'adresse https://dev.azure.com/<nom> — vous pouvez aussi coller l'adresse complète.",
    urlInvalid:
      "Saisissez une adresse qui commence par https://, p. ex. https://votre-entreprise.atlassian.net — sans identifiant, mot de passe, ? ni #.",
    emailInvalid:
      "Saisissez l'e-mail du compte, p. ex. jean@example.com (sans espaces ni deux-points), ou laissez le champ vide.",
    azureOrgInvalid:
      "Saisissez le nom de l'organisation : lettres, chiffres, tirets, points et traits de soulignement (64 caractères au maximum).",

    token: (name: string) => `Jeton d'accès ${name}`,
    tokenPlaceholder: "Collez le jeton",
    tokenPlaceholderReplace: "Collez un nouveau jeton pour remplacer celui qui est enregistré",
    tokenHint: "Le jeton est stocké chiffré et ne réapparaît jamais dans ce champ.",
    tokenSave: "Enregistrer le jeton",
    tokenSaving: "Enregistrement…",
    tokenRemove: "Supprimer le jeton",
    tokenSaved: (name: string) => `Le jeton ${name} a été enregistré.`,
    tokenRemoved: (name: string) => `Le jeton ${name} a été supprimé.`,
    helpTitle: "Comment créer un jeton en lecture seule ?",
    help: {
      github: [
        "Sur GitHub : Settings → Developer settings → Personal access tokens → Fine-grained tokens → Generate new token.",
        "Resource owner : votre compte ou l'organisation dans laquelle vous travaillez (un jeton fine-grained ne voit que les dépôts d'un seul propriétaire). Repository access : All repositories ou une sélection de dépôts.",
        "Repository permissions, chacune uniquement en « Read-only » : Issues, Pull requests et Metadata (GitHub ajoute Metadata de lui-même). Ne sélectionnez rien d'autre. Définissez une date d'expiration.",
      ],
      githubNotifications:
        "Via l'API, GitHub ne donne accès aux notifications (mentions, demandes de revue hors des dépôts du jeton) et aux résultats de GitHub Actions qu'aux jetons classiques — un jeton fine-grained ne peut pas les lire. Si vous souhaitez les voir, créez plutôt un jeton classique (Tokens (classic)) avec le scope notifications. Pour les dépôts privés, un jeton classique a aussi besoin du scope repo, qui autorise également l'écriture — FocusMode ne s'en sert pas, c'est pourquoi nous recommandons par défaut un jeton fine-grained.",
      gitlab: [
        "Dans GitLab : avatar → Modifier le profil (Edit profile) → Jetons d'accès (Personal access tokens) → Ajouter un nouveau jeton (Add new token ; dans les versions récentes : « Legacy token »).",
        "Sélectionnez uniquement le scope read_api (accès en lecture à l'API). Définissez une date d'expiration.",
        "Pour une instance GitLab autogérée, saisissez son adresse ci-dessus.",
      ],
      jira: [
        "Jira Cloud : allez sur id.atlassian.com/manage-profile/security/api-tokens et choisissez « Create API token with scopes », l'application Jira et uniquement les scopes read:jira-work et read:jira-user. Définissez une date d'expiration proche.",
        "Ci-dessus, saisissez l'adresse du site et l'e-mail de votre compte Atlassian. Un jeton avec scopes ne fonctionne que via la passerelle api.atlassian.com — FocusMode la choisit tout seul. Un jeton ordinaire sans scopes fonctionne aussi, mais il a toutes les autorisations de votre compte (FocusMode se contente de lire les tickets).",
        "Jira Data Center : profil → Personal Access Tokens → Create token, avec une date d'expiration. Laissez le champ e-mail vide.",
      ],
      azureDevops: [
        "Sur dev.azure.com/<organisation> : User settings (l'icône à côté de votre avatar) → Personal access tokens → New Token.",
        "Organization : votre organisation. Scopes : Custom defined → Work Items: Read, Code: Read et Project and Team: Read — rien de plus. Définissez une date d'expiration.",
      ],
      linear: [
        "Dans Linear : Settings → Security & access → Personal API keys → New API key.",
        "Permissions : « Only select permissions » → Read only. Team access : toutes les équipes ou une sélection.",
      ],
    },

    holdDuringFocus: "Retenir les notifications pendant la concentration",
    holdDuringFocusHint:
      "FocusMode n'affiche les nouvelles tâches et notifications arrivées pendant la concentration qu'à la pause ou avant la session suivante — rien n'interrompt votre concentration.",
    lastSync: (time: string) => `Dernière synchronisation : ${time}`,
    neverSynced: "Les outils n'ont pas encore été lus.",
    refresh: "Actualiser",
    refreshing: "Actualisation…",

    cardTitle: "Et ensuite ?",
    cardLead: "D'abord ce qui vous est attribué, puis ce que d'autres attendent.",
    loading: "Chargement des tâches…",
    connectHint:
      "Connectez GitHub, GitLab, Jira, Azure DevOps ou Linear, et FocusMode vous suggérera par quoi commencer dans cette session.",
    connectLink: "Paramètres des outils de travail",
    noTokenHint: "Enregistrez un jeton dans les Paramètres et vos tâches apparaîtront ici.",
    empty: "Rien ne vous attend — aucune tâche attribuée, demande de revue ni notification.",
    sourceErrors: (names: string) => `Lecture impossible : ${names}.`,
    sourceErrorsLink: "Détails dans les Paramètres",
    held: (n: number) => `${n} ${plural(n, { one: "nouveau", other: "nouveaux" })} pendant la concentration`,
    newCount: (n: number) => `${n} ${plural(n, { one: "nouveau", other: "nouveaux" })}`,
    unseen: "nouveau",
    showMore: (n: number) => `Afficher plus (${n})`,
    showLess: "Afficher moins",
    open: "Ouvrir",
    openAria: (name: string) => `Ouvrir ${name} dans le navigateur`,
    setGoal: "Définir comme objectif",
    setGoalAria: (name: string) => `Définir ${name} comme objectif de la session`,
    isGoal: "Objectif de la session",
    due: (date: string) => `échéance : ${date}`,
    reasonsLabel: "Pourquoi maintenant",
    reason: {
      assigned: "vous est attribué",
      inSprint: "sprint",
      highPriority: "priorité haute",
      dueToday: "échéance aujourd'hui",
      overdue: "en retard",
      inProgress: "en cours",
      blockingOthers: "attend votre revue",
      waitingLong: "attend depuis longtemps",
      mentioned: "mention",
    },
    kind: {
      assigned: "tâche",
      reviewRequested: "revue",
      changesRequested: "modifications demandées",
      ciFailed: "CI en échec",
      mention: "mention",
      notification: "notification",
    },
    itemName: (key: string, title: string) => (key && title ? `${key} ${title}` : key || title),

    goalLabel: "Objectif de la session :",
    goalOpenAria: (name: string) => `Objectif de la session : ${name}. Ouvrir dans le navigateur`,
    goalClear: "Effacer l'objectif de la session",
  },

  nudge: {
    title: "Retour au travail",
    body: (domain: string) => `${domain} attendra la pause.`,
    bodyPattern: "Ce site est sur la liste de blocage et attendra la pause.",
    dismiss: "Fermer",
  },

  toasts: {
    region: "Notifications",
    dismiss: "Fermer la notification",
  },

  errors: {
    generic: "Un problème est survenu.",
    fieldInvalid: (label: string) => `Valeur non valide : ${label}.`,
    fixConfigFirst: "Corrigez les paramètres signalés avant de démarrer une session.",
    unplayable: "Ce lien ne peut pas être lu dans l'application",
    allUnplayable: "Aucune des musiques choisies ne peut être lue.",
    autoplayBlocked: "La lecture a été bloquée — appuyez sur « Suivant » pour réessayer.",
    youtubeApi: "Impossible de charger le lecteur YouTube. Vérifiez votre connexion Internet.",
    clipboard: "Impossible de copier dans le presse-papiers.",
    codes: {
      "config.invalid": "Paramètre non valide.",
      "config.corrupt":
        "Le fichier de paramètres était endommagé — les paramètres par défaut ont été rétablis (sauvegarde : config.json.bak).",
      "config.read": "Impossible de lire le fichier de paramètres — les paramètres par défaut sont utilisés.",
      "config.save":
        "Impossible d'enregistrer les paramètres sur le disque — vos modifications restent actives jusqu'à la fermeture de l'application.",
      "audio.device": "Impossible de jouer la sonnerie — vérifiez votre périphérique audio.",
      "shields.notifications": "Impossible d'activer ou de désactiver « Ne pas déranger » de Windows.",
      "shields.teams": "Impossible de modifier votre statut Teams.",
      "shields.keepAwake": "Impossible d'activer ou de désactiver le maintien de l'écran allumé.",
      "bridge.port":
        "Le port du pont de l'extension est occupé — le blocage des sites ne fonctionne qu'en mode de secours.",
      "teams.auth": "La connexion à Teams a échoué.",
      "teams.network": "Impossible de joindre les services Microsoft — réessayez dans un instant.",
      "teams.notConfigured": "Saisissez d'abord l'Application (client) ID.",
      "teams.unsupported": "L'intégration Teams n'est pas disponible sur ce système.",
      "teams.disconnect": "Impossible de déconnecter le compte Teams.",
      "teams.unavailable": "L'intégration Teams ne répond pas — redémarrez l'application.",
      "music.folder": "Impossible de lire le dossier de musique.",
      "media.url":
        "Ce lien ne peut pas s'ouvrir dans la fenêtre YouTube — seules les adresses YouTube ou YouTube Music fonctionnent.",
      "media.window": "Impossible d'ouvrir ou de piloter la fenêtre YouTube.",
      "persistence.history": "Un problème avec l'historique des sessions (lecture ou enregistrement).",
      "extension.missing": "Le dossier de l'extension du navigateur est introuvable.",
      "extension.open": "Impossible d'ouvrir le dossier de l'extension.",
      "blocklist.read": "Impossible de lire websiteblacklist.txt — la liste précédente reste active.",
      "blocklist.open":
        "Impossible d'ouvrir websiteblacklist.txt dans un éditeur de texte — ouvrez le fichier vous-même (le chemin figure dans les Paramètres).",
      "shields.launchers": "Un des lanceurs de jeux n'a pas pu être fermé — la session continue.",
      "tint.apply": "Impossible de réchauffer les couleurs de l'écran — nouvel essai dans un instant.",
      "tint.restore": "Impossible de rétablir les couleurs habituelles de l'écran après la dernière exécution.",
      "chime.format": "Ce format n'est pas pris en charge — choisissez un fichier mp3, wav, ogg, flac ou m4a.",
      "chime.read": "Impossible de lire le fichier choisi.",
      "chime.tooLarge": "Le fichier est trop volumineux — une sonnerie ne peut pas dépasser 10 Mo.",
      "chime.save": "Impossible d'enregistrer le son dans le dossier de données de FocusMode.",
      "chime.decode": "Ce fichier ne peut pas être lu comme un son — choisissez-en un autre.",
      "music.library": "Impossible de charger la musique intégrée.",
      "popup.window": "Impossible d'afficher la fenêtre pop-up FocusMode.",
      "calendar.sync": "Impossible de lire le calendrier — détails dans Paramètres → Calendriers.",
      "calendar.unavailable": "Les calendriers ne répondent pas — redémarrez l'application.",
      "calendar.unknownMeeting": "Cette réunion ne figure plus dans le calendrier.",
      "calendar.remindLater":
        "Il est trop tard pour un rappel — la réunion commence dans 10 minutes ou moins. Choisissez une autre option.",
      "calendar.meetingTooLong":
        "Une réunion de plus de 4 h ne prolonge pas la concentration et ne suspend pas la pause. Choisissez une autre option.",
      "calendar.googleNotConfigured": "Saisissez d'abord l'ID client Google.",
      "calendar.googleAuth": "Impossible de lancer la connexion à Google.",
      "calendar.disconnect": "Impossible de supprimer les données de calendrier enregistrées.",
      "calendar.icsUrl": "Un des liens n'est pas un lien de calendrier valide (https:// ou webcal://).",
      "calendar.icsSave": "Impossible d'enregistrer les liens des calendriers de façon sécurisée.",
      "work.sync": "Impossible de lire un outil de travail — détails dans Paramètres → Outils de travail.",
      "work.unavailable": "Les outils de travail ne répondent pas — redémarrez l'application.",
      "work.token":
        "Un jeton ne peut contenir que des caractères ASCII visibles, sans espaces (4 Ko au maximum) — collez-le à nouveau.",
      "work.tokenSave": "Impossible d'enregistrer ou de supprimer le jeton de façon sécurisée.",
      autostart: "Impossible de modifier le lancement à l'ouverture de session.",
      "session.unavailable": "La session ne répond pas — redémarrez l'application.",
      "ipc.unavailable": "Cette fonction n'est disponible que dans l'application FocusMode.",
      "ipc.failed": "L'application n'a pas répondu correctement.",
    },
    areas: {
      config: "Un problème avec les paramètres.",
      audio: "Un problème avec le son.",
      shields: "Un des boucliers n'a pas fonctionné — la session continue.",
      teams: "Un problème de connexion à Teams.",
      bridge: "Un problème avec le pont de l'extension du navigateur.",
      blocklist: "Un problème avec la liste des sites bloqués (websiteblacklist.txt).",
      music: "Un problème avec la musique.",
      media: "Un problème avec la fenêtre YouTube.",
      persistence: "Impossible d'enregistrer les données.",
      session: "Un problème avec la session.",
      tracker: "Un problème avec le suivi des applications.",
      tint: "Un problème avec les couleurs chaudes de l'écran.",
      chime: "Un problème avec votre propre sonnerie.",
      popup: "Un problème avec la fenêtre pop-up FocusMode.",
      launchers: "Un problème lors de la fermeture des lanceurs de jeux.",
      window: "Un problème avec la fenêtre de l'application.",
      calendar: "Un problème avec le calendrier — la session continue.",
      google: "Un problème avec Google Agenda.",
      ics: "Un problème avec un calendrier via un lien ICS.",
      work: "Un problème avec les outils de travail (Et ensuite ?) — la session continue.",
    },
  },

  fields: {
    focusMinutes: "durée de concentration",
    breakMinutes: "durée de pause",
    chimeVolume: "volume de la sonnerie",
    "music.volume": "volume de la musique",
    "music.links": "liens",
    "music.streamUrl": "adresse du flux",
    "music.folder": "dossier de musique",
    "shields.teamsClientId": "Application (client) ID",
    "shields.teamsTenant": "locataire",
    "shields.launchers": "lanceurs de jeux",
    "tint.startMinutes": "début des couleurs chaudes",
    "tint.endMinutes": "fin des couleurs chaudes",
    "tint.strength": "intensité des couleurs chaudes",
    "tint.transitionMinutes": "transition progressive",
    "tint.mode": "méthode de réchauffement",
    language: "langue",
    theme: "thème",
    chimeSound: "sonnerie",
    "customChimes.breakFile": "son du début de la pause",
    "customChimes.focusFile": "son du début de la concentration",
    breakReminders: "rappels pendant les pauses",
    "calendar.horizonHours": "période de vérification du calendrier",
    "calendar.googleClientId": "ID client Google",
    "calendar.googleClientSecret": "code secret du client Google",
    "work.gitlabUrl": "adresse GitLab",
    "work.jiraUrl": "adresse Jira",
    "work.jiraEmail": "e-mail du compte Jira",
    "work.azureOrg": "organisation Azure DevOps",
  },

  dialog: {
    cancel: "Annuler",
  },

  launchers: {
    steam: "Steam",
    epic: "Epic Games Launcher",
    gog: "GOG Galaxy",
    ubisoft: "Ubisoft Connect",
    ea: "Application EA",
    battleNet: "Battle.net",
    riot: "Riot Client",
    xbox: "Application Xbox",
    amazon: "Amazon Games",
    itch: "itch",
  },

  themes: {
    legend: "Thème de l'application",
    hint: "Couleurs et arrière-plan de la fenêtre principale et des fenêtres pop-up de phase. Les modifications sont enregistrées immédiatement.",
    selected: "sélectionné",
    names: {
      midnight: "Minuit",
      vaporwave: "Vaporwave",
      lunar: "Phases de la Lune",
      autumn: "Automne douillet",
      dark: "Sombre",
      light: "Clair",
      pixel: "8 bits",
    },
    descriptions: {
      midnight: "Une nuit violette et chaleureuse, avec la pluie derrière la fenêtre. Thème par défaut.",
      vaporwave: "Rose, cyan et violet : un soleil rayé au-dessus d'une grille en perspective.",
      lunar: "Argent et indigo sous les étoiles.",
      autumn: "Des bruns et de l'ambre, des feuilles qui tombent doucement.",
      dark: "Gris neutre, sans décor en arrière-plan.",
      light: "Fond clair et texte sombre très contrasté, sans décor.",
      pixel: "Cadres anguleux, ombres franches et étoiles en pixels.",
    },
    moonToday: (phase: string, litPct: number) =>
      `La Lune en arrière-plan montre la phase du jour : ${phase} (éclairée à ${litPct} %).`,
    moonPhases: {
      new: "nouvelle lune",
      waxingCrescent: "premier croissant",
      firstQuarter: "premier quartier",
      waxingGibbous: "gibbeuse croissante",
      full: "pleine lune",
      waningGibbous: "gibbeuse décroissante",
      lastQuarter: "dernier quartier",
      waningCrescent: "dernier croissant",
    },
  },

  /** Break suggestions by id (category, length and source in `$lib/tips`). Suggestions only, never health promises. */
  tips: {
    "eyes-20-20-20":
      "Regardez par la fenêtre quelque chose situé à au moins 6 m et laissez votre regard s'y poser pendant 20 secondes.",
    "eyes-green-view":
      "Trouvez dehors un arbre, un peu d'herbe ou un coin de ciel et regardez-le pendant 40 secondes.",
    "eyes-slow-blinks": "Fermez lentement les yeux, comptez jusqu'à deux et rouvrez-les. Répétez 10 fois, tranquillement.",
    "eyes-look-around":
      "Regardez en haut, en bas, à gauche et à droite, puis faites lentement deux cercles avec les yeux dans chaque sens.",
    "eyes-near-far":
      "Regardez votre pouce, bras tendu, pendant 10 secondes, puis un point dehors pendant 10 secondes. Répétez 5 fois.",
    "eyes-screen-free": "Passez cette pause loin des écrans — posez aussi votre téléphone écran contre la table.",
    "stretch-shoulder-rolls": "Faites lentement 5 cercles des épaules vers l'arrière, puis 5 vers l'avant.",
    "stretch-shoulder-shrug":
      "Montez les épaules vers les oreilles, tenez 3 à 5 secondes et relâchez-les. Répétez 3 fois.",
    "stretch-neck-turn":
      "Asseyez-vous bien droit et tournez lentement la tête vers la gauche, aussi loin que c'est confortable. Tenez 5 secondes, puis à droite. 3 fois.",
    "stretch-neck-tilt":
      "Inclinez lentement la tête, l'oreille vers l'épaule, et tenez 10 secondes si c'est confortable. Puis de l'autre côté.",
    "stretch-chin-glide":
      "Assis bien droit, reculez la tête horizontalement sans lever le menton. Tenez 5 secondes, répétez 5 fois.",
    "stretch-chest-open":
      "Asseyez-vous droit sans vous appuyer sur le dossier, rapprochez les omoplates et ouvrez grand les bras. Tenez 10 s, 3 fois.",
    "stretch-upper-twist":
      "Croisez les bras sur la poitrine et tournez le buste vers la gauche, aussi loin que c'est confortable. Tenez 5 s, puis à droite. 5 fois.",
    "stretch-side-reach":
      "Entrecroisez les doigts au-dessus de la tête, étirez-vous vers le haut, puis penchez-vous à gauche et à droite, 10 secondes de chaque côté, si c'est confortable.",
    "stretch-wrists":
      "Tendez un bras et pliez doucement le poignet vers le haut, puis vers le bas, 5 secondes chaque fois. 3 fois, puis l'autre main.",
    "stretch-fingers": "Serrez les poings, puis écartez grand les doigts. Répétez 5 fois, sans vous presser.",
    "stretch-prayer":
      "Joignez les paumes devant vous et abaissez lentement les poignets jusqu'à un léger étirement, jamais jusqu'à la douleur. 3 fois.",
    "stretch-ankles":
      "Assis, tendez une jambe au-dessus du sol et dirigez les orteils tour à tour loin de vous et vers vous. 5 fois par jambe.",
    "stretch-standing-back":
      "Debout, genoux légèrement fléchis, mains au bas du dos, penchez-vous doucement en arrière, aussi loin que c'est confortable. 3 fois.",
    "stretch-routine":
      "Faites une série tranquille de 10 minutes : épaules, nuque, rotations du buste, poignets et chevilles — toujours dans une amplitude confortable.",
    "move-walk-5":
      "Levez-vous et marchez 5 minutes — dans votre logement, dans le couloir ou jusqu'à la cuisine et retour.",
    "move-stairs":
      "Montez un étage par l'escalier et redescendez, à votre rythme. Pas d'escalier ? Promenez-vous dans le couloir.",
    "move-march-seated":
      "Asseyez-vous droit et levez les genoux l'un après l'autre comme pour marcher, aussi haut que c'est confortable — 5 fois de chaque jambe.",
    "move-march-standing": "Marchez sur place pendant une minute en balançant librement les bras.",
    "move-calf-raises":
      "Debout devant votre bureau, mains posées dessus, montez lentement sur la pointe des pieds 10 fois.",
    "move-sit-to-stand": "Sur une chaise stable sans roulettes, levez-vous et rasseyez-vous 10 fois, tranquillement.",
    "move-one-song": "Mettez une chanson que vous aimez et bougez dessus comme vous voulez — debout ou assis.",
    "move-chores":
      "Faites quelque chose d'actif à la maison : étendez le linge, arrosez les plantes ou sortez les poubelles.",
    "hydrate-glass": "Buvez un verre d'eau — lentement, à petites gorgées.",
    "hydrate-refill": "Allez à la cuisine et remplissez d'eau une bouteille ou une tasse pour la prochaine session.",
    "hydrate-in-reach": "Avant de reprendre le travail, posez un verre d'eau à portée de main.",
    "breathe-count-5":
      "Installez-vous confortablement. Inspirez par le nez en comptant jusqu'à 5, expirez par la bouche en comptant jusqu'à 5. Continuez pendant 5 minutes.",
    "breathe-belly-hand":
      "Posez une main sur votre ventre et, pendant une minute, observez-le se soulever à l'inspiration et s'abaisser à l'expiration.",
    "breathe-long-exhale":
      "Inspirez par le nez, complétez par une deuxième petite inspiration, puis expirez lentement par la bouche. 5 fois.",
    "breathe-open-window":
      "Ouvrez la fenêtre et prenez 5 respirations calmes : inspirez sur quatre temps, expirez sur six.",
    "breathe-muscle-release":
      "Contractez les pieds pendant 5 secondes, puis relâchez. Remontez ainsi le long du corps jusqu'aux épaules et au visage.",
    "posture-reset":
      "Avant de reprendre : le dos contre le dossier, les pieds à plat sur le sol, les bras détendus le long du corps.",
    "posture-screen-check":
      "Vérifiez votre écran : le bord supérieur à hauteur des yeux ou un peu plus bas, à environ une longueur de bras.",
    "posture-glare-check":
      "Vérifiez qu'aucune fenêtre ni lampe ne se reflète dans votre écran. L'idéal est d'avoir la fenêtre sur le côté de l'écran.",
    "posture-shift":
      "Changez un moment de position : bougez sur votre chaise, déplacez les jambes, relâchez les épaules.",
    "outside-step-out":
      "Sortez sur un balcon, devant le bâtiment ou près d'une fenêtre ouverte et restez-y 2 minutes.",
    "outside-sky": "Sortez et regardez le ciel et les nuages pendant une minute.",
    "outside-walk-15":
      "Faites une promenade de 15 minutes, si possible là où il y a des arbres ou de l'herbe. Marchez d'un pas tranquille.",
    "outside-walk-noticing":
      "Allez vous promener sans téléphone à la main et comptez les arbres, les oiseaux ou les chiens en chemin.",
    "social-chat": "Discutez un moment avec quelqu'un d'un sujet qui n'a rien à voir avec le travail.",
    "social-message": "Envoyez un petit message à un proche, juste pour dire bonjour.",
    "social-walk-together": "Proposez à quelqu'un une courte promenade ensemble, même juste autour du bâtiment.",
    "social-pet": "Vous avez un animal ? Accordez-lui quelques minutes de toute votre attention.",
    "mind-do-nothing": "Installez-vous confortablement, fermez les yeux et ne faites simplement rien pendant 2 minutes.",
    "mind-one-song": "Écoutez attentivement une chanson les yeux fermés, sans rien faire d'autre.",
    "mind-5-4-3": "Nommez dans votre tête 5 choses que vous voyez, 4 que vous entendez et 3 que vous pouvez toucher.",
    "mind-wander": "Regardez par la fenêtre et laissez vos pensées vagabonder librement pendant 3 minutes.",
    "mind-park-the-task":
      "Écrivez une phrase sur papier : par quoi vous commencerez après la pause. Puis mettez le sujet de côté.",
    "mind-three-good": "Pensez à trois petites choses qui se sont bien passées aujourd'hui.",
    "mind-meal-away":
      "Prenez un repas loin de votre bureau et de l'écran — à table, sur un banc ou en compagnie.",
    "mind-your-choice": "Faites quelque chose dont vous avez vraiment envie — cette pause est à vous.",
    "mind-lie-down": "Si vous le pouvez, allongez-vous 10 à 15 minutes, les yeux fermés. Mettez un réveil.",
    "tidy-desk":
      "Rangez votre bureau : rapportez les tasses, rassemblez les papiers qui traînent et ne gardez que ce dont vous avez besoin maintenant.",
    "tidy-air": "Ouvrez grand la fenêtre quelques minutes et aérez la pièce.",
    "tidy-light":
      "Ajustez la lumière : baissez le store si elle éblouit, ou allumez une lampe s'il commence à faire sombre.",
  },

  /** Good working practices by id (one a day on the idle Focus page). */
  workingTips: {
    "gp-screen-position":
      "Placez l'écran bien en face de vous, à environ 50 à 70 cm des yeux, le bord supérieur à hauteur des yeux ou un peu plus bas.",
    "gp-light-and-glare":
      "Placez l'écran perpendiculairement à la fenêtre et adaptez sa luminosité à l'éclairage de la pièce.",
    "gp-sitting":
      "Le dos contre le dossier, les pieds à plat sur le sol, les coudes près du corps, les poignets dans le prolongement des avant-bras.",
    "gp-short-often":
      "Faites souvent de courtes pauses, avant de vous sentir fatigué : quelques minutes toutes les heures valent mieux qu'une longue pause toutes les deux heures.",
    "gp-20-20-20":
      "Pendant la concentration, détournez les yeux de l'écran environ toutes les 20 minutes et regardez au loin pendant 20 secondes. Pensez à cligner des yeux.",
    "gp-change-position":
      "Changez de position : levez-vous de votre bureau au moins une fois par heure, même brièvement.",
    "gp-batch-messages":
      "Consultez vos e-mails et messageries à heures fixes, par exemple pendant les pauses, plutôt qu'à chaque notification.",
    "gp-single-task":
      "Choisissez une seule tâche par session et fermez les onglets et fenêtres dont vous n'avez pas besoin pour elle.",
    "gp-phone-away": "Pendant la concentration, mettez votre téléphone en silencieux et rangez-le hors de vue.",
    "gp-water-in-reach": "Gardez de l'eau à portée de main et buvez-en un peu tout au long de la journée.",
  },

  /** Shown under every stretch suggestion. */
  stretchFootnote:
    "Bougez dans une amplitude confortable : vous devez sentir l'étirement, jamais de douleur. Si quelque chose fait mal, passez cet exercice.",

  time: {
    hoursMinutes: (h: number, m: number) => `${h} h ${m} min`,
    hours: (h: number) => `${h} h`,
    minutes: (m: number) => `${m} min`,
    seconds: (s: number) => `${s} s`,
    today: "aujourd'hui",
  },
};
