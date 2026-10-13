// Spanish UI texts (translated from `en.ts`, checked against the Polish source `pl.ts`; same shape).
// Tone: friendly, short and calm; informal "tú", neutral Spanish (no "vosotros", no strongly regional words),
// gender-neutral wording where Spanish allows it. "Descanso" is the break phase, "Pausar" / "En pausa" pauses the timer.
// No health promises: the app suggests, it never claims effects.
// "FocusMode" is a name and is never translated; neither is the file name websiteblacklist.txt.
import type { Strings } from "./pl";
import { pluralizer } from "./plural";

const plural = pluralizer("es");

export const es: Strings = {
  appName: "FocusMode",
  tagline: "sesiones de concentración lofi",

  nav: {
    label: "Navegación principal",
    focus: "Concentración",
    summary: "Resumen",
    settings: "Configuración",
    developer: "Desarrollador",
  },

  phase: {
    idle: "Listo para empezar",
    focus: "Concentración",
    break: "Descanso",
  },

  phaseAnnouncement: {
    focusStarted: (cycle: number) => `Empieza la concentración, ciclo ${cycle}.`,
    breakStarted: "Empieza el descanso.",
    stopped: "Sesión terminada.",
  },

  timer: {
    paused: "En pausa",
    cycle: (n: number) => `Ciclo ${n}`,
    demo: "demo corta",
    remainingLabel: (phase: string, time: string) => `${phase}: quedan ${time}`,
    idleHint: "Ajusta los tiempos y la música, y pulsa Iniciar.",
    focusedSoFar: (time: string) => `Concentración en esta sesión: ${time}`,
  },

  controls: {
    start: "Iniciar",
    pause: "Pausar",
    resume: "Reanudar",
    skipToBreak: "Descansar ahora",
    skipToFocus: "Volver a concentrarme",
    skip: "Saltar fase",
    stop: "Detener",
    sessionControls: "Controles de la sesión",
  },

  timing: {
    title: "Tiempo",
    focus: "Concentración (min)",
    break: "Descanso (min)",
    presets: "Predefinidos",
    preset: (focus: number, brk: number) => `${focus} / ${brk}`,
    presetAria: (focus: number, brk: number) =>
      `${focus} ${plural(focus, { one: "minuto", other: "minutos" })} de concentración, ${brk} ${plural(brk, { one: "minuto", other: "minutos" })} de descanso`,
    rangeFocus: "Escribe un número de minutos entre 1 y 240.",
    rangeBreak: "Escribe un número de minutos entre 1 y 120.",
  },

  music: {
    title: "Música",
    sourceLabel: "Fuente de música",
    source: {
      library: "Open Lo-Fi (integrada)",
      stream: "Stream",
      links: "Enlaces",
      folder: "Carpeta",
      noise: "Ruido",
      none: "Silencio",
    },
    libraryName: "Open Lo-Fi",
    libraryHint: "Temas lo-fi tranquilos que vienen con FocusMode — suenan sin internet.",
    libraryCredit: "Música: Open Lo-Fi, de btahir, con licencia CC0 (dominio público).",
    libraryCreditLink: "github.com/btahir/open-lofi",
    libraryCreditAria: "Open Lo-Fi en GitHub (abre el navegador)",
    libraryEmpty: "No se encontró la música integrada — elige otra fuente.",
    streamUrl: "Dirección del stream (YouTube en directo o radio por internet)",
    streamPlaceholder: "https://www.youtube.com/watch?v=…",
    lofiPreset: "Lofi Girl — radio en directo",
    lofiPresetHint:
      "La emisión en directo actual del canal Lofi Girl. Suena de fondo a través de una ventana oculta de YouTube (el propietario no permite insertarla en aplicaciones); la pausa y el volumen los controlas desde FocusMode. La primera vez, YouTube muestra su ventana para elegir las cookies — cuando eliges, se oculta sola.",
    presetsLabel: "Propuestas",
    linksLabel: "Enlaces — uno por línea (YouTube, YouTube Music, listas de reproducción, archivos de audio)",
    linksPlaceholder: "https://music.youtube.com/watch?v=…\nhttps://youtu.be/…\nhttps://example.com/cancion.mp3",
    linksSummary: (ok: number, bad: number) =>
      bad > 0
        ? `${ok} ${plural(ok, { one: "enlace reproducible", other: "enlaces reproducibles" })}, ${bad} ${plural(bad, { one: "no compatible", other: "no compatibles" })}`
        : `${ok} ${plural(ok, { one: "enlace reproducible", other: "enlaces reproducibles" })}`,
    shuffle: "Orden aleatorio",
    pickFolder: "Elegir carpeta",
    changeFolder: "Cambiar carpeta",
    pickFolderTitle: "Elige una carpeta de música",
    noFolder: "No has elegido ninguna carpeta.",
    loadingTracks: "Buscando archivos de audio…",
    trackCount: (n: number) => `${n} ${plural(n, { one: "pista", other: "pistas" })}`,
    noTracks: "Esta carpeta no tiene archivos de audio compatibles (mp3, m4a, flac, wav, ogg, opus…).",
    noise: {
      brown: "Marrón",
      pink: "Rosa",
      white: "Blanco",
    },
    noiseLabel: "Tipo de ruido",
    noiseHint: "Se genera aquí mismo — funciona sin internet.",
    noneHint: "Sin música — solo oirás la campanilla al empezar y al terminar el descanso.",
    detected: {
      youtubeVideo: "YouTube — vídeo o directo",
      youtubePlaylist: "YouTube — lista de reproducción",
      youtubeWindow:
        "YouTube — suena en una pequeña ventana aparte de YouTube (este enlace no se puede reproducir dentro de la aplicación).",
      audio: "Stream o archivo de audio",
      unsupported: "Este servicio no es compatible (p. ej., Spotify) — usa YouTube o un enlace de audio directo.",
      invalid: "Esto no parece una dirección http(s).",
    },
    pauseDuringBreak: "Pausar la música durante los descansos",
    pauseDuringBreakHint: "De forma predeterminada, la música sigue sonando, también en los descansos.",
  },

  player: {
    region: "Reproductor de música",
    idle: "La música empieza con la sesión",
    pausedBySession: "En pausa",
    pausedForBreak: "En pausa durante el descanso",
    pausedForMeeting: "Silenciada durante la reunión",
    loading: "Cargando…",
    silence: "Silencio",
    next: "Siguiente",
    volume: "Volumen de la música",
    expand: "Mostrar vídeo",
    collapse: "Ocultar vídeo",
    noiseTitle: (color: string) => `Ruido ${color.toLowerCase()}`,
    youtubeFallbackTitle: "YouTube",
    trackOf: (i: number, n: number) => `${i} de ${n}`,
    sourceNotReady: "Elige una fuente de música en la pantalla Concentración.",
    windowMode: "Suena de fondo a través de YouTube — el propietario no permite insertarlo en aplicaciones.",
    windowModeShort: "Suena de fondo a través de YouTube",
    showWindow: "Mostrar ventana",
    showWindowAria: "Mostrar la ventana de YouTube",
    windowClosed: "Se cerró la ventana de YouTube",
    reopenWindow: "Volver a abrir",
    reopenWindowAria: "Volver a abrir la ventana de YouTube",
  },

  focusView: {
    nowUsing: "Ahora usas",
    nowUsingEmpty: "Recopilando datos de aplicaciones…",
    trackingOff: "El seguimiento de aplicaciones está desactivado en Configuración.",
    idle: "Inactividad",
    unknown: "Desconocida",
    shields: "Protecciones",
    shieldsNone: "No hay ninguna protección activada — puedes activarlas en Configuración.",
  },

  shieldChip: {
    notifications: "Notificaciones silenciadas",
    teams: "Teams: No molestar",
    sites: "Bloqueo de sitios",
    sitesFallback: "Bloqueo de sitios (modo alternativo)",
    keepAwake: "La pantalla no se apaga",
    launchers: "Lanzadores de juegos",
    launchersClosed: (names: string) => `Cerrados: ${names}`,
    launchersNothing: "Ninguno de los lanzadores seleccionados estaba abierto.",
    state: {
      off: "desactivada",
      on: "activa",
      unavailable: "no disponible",
      error: "error",
    },
    chipAria: (name: string, state: string) => `${name}: ${state}`,
  },

  breakView: {
    title: "Hora de descansar",
    lead: "Aléjate un poco del escritorio. La música puede seguir sonando.",
    reminderLabel: (n: number) => `Tu recordatorio · descanso ${n}`,
    suggestionLabel: "Una sugerencia para este descanso",
    another: "Otra sugerencia",
    source: "Fuente: ",
    sourceAria: (host: string) => `Fuente de la sugerencia: ${host} (abre el navegador)`,
    categories: {
      eyes: "Ojos",
      stretch: "Estiramientos",
      move: "Movimiento",
      hydrate: "Agua",
      breathe: "Respiración",
      posture: "Puesto de trabajo",
      outside: "Al aire libre",
      social: "Personas",
      mind: "Un momento para la mente",
      tidy: "Entorno",
    },
  },

  idleTips: {
    practiceTitle: "Buena práctica",
    lawTitle: "Conviene saberlo",
    source: "Fuente: ",
    sourceAria: (host: string) => `Fuente: ${host} (abre el navegador)`,
  },

  summary: {
    title: "Resumen",
    lastSession: "Última sesión",
    selectedSession: "Sesión seleccionada",
    empty: "Todavía no hay resumen. Termina una sesión con Detener y aquí verás un gráfico de tus aplicaciones.",
    focusTime: "Tiempo de concentración",
    cycles: "Ciclos completados",
    cyclesCount: (n: number) => `${n} ${plural(n, { one: "ciclo", other: "ciclos" })}`,
    breaks: "Tiempo de descanso",
    span: "Inicio – fin",
    chartTitle: "Tiempo de concentración por aplicación",
    chartSubtitle: (time: string) => `Tiempo registrado: ${time}`,
    chartEmpty: "No hay datos de aplicaciones (el seguimiento estaba desactivado o la sesión fue muy corta).",
    other: (n: number) => `Otras (${n})`,
    otherPlain: "Otras",
    idle: "Inactividad",
    unknown: "Desconocida",
    barAria: (name: string, time: string, pct: string) => `${name}: ${time}, ${pct}`,
    titlesHeading: "Ventanas más frecuentes",
    titlesHint: "Pasa el puntero sobre una aplicación, o llega a ella con Tab, para ver sus ventanas más frecuentes.",
    notSaved: "Esta sesión no se añadió al historial — guardar el historial está desactivado en Configuración.",
    history: "Historial",
    historyEmpty: "El historial está vacío.",
    historyLegend: "Colores de las aplicaciones en el historial",
    demo: "demo",
    showSession: (date: string) => `Mostrar la sesión del ${date}`,
    backToLast: "Volver a la última sesión",
    privacy: "Los datos de aplicaciones se quedan en este equipo.",
    goalLabel: "Objetivo de la sesión",
    goalQuestion: "¿Hecho?",
    goalYes: "Sí",
    goalNo: "No",
    goalGroup: "¿Se cumplió el objetivo de la sesión?",
    goalDone: "Hecho — ¡buen trabajo!",
    goalNotDone: "Esta vez no — puedes volver a elegirlo en «¿Qué sigue?».",
    goalOpenAria: (name: string) => `Abrir ${name} en el navegador`,
    goalMark: (done: boolean | null) => (done === true ? "objetivo ✓" : done === false ? "objetivo ✗" : "objetivo"),
    goalMarkTitle: (name: string, done: boolean | null) =>
      `Objetivo: ${name}${done === true ? " — hecho" : done === false ? " — no hecho" : ""}`,
    goalAria: (done: boolean | null) =>
      done === true ? "objetivo cumplido" : done === false ? "objetivo no cumplido" : "objetivo sin respuesta",
  },

  settings: {
    title: "Configuración",
    saving: "Guardando…",
    saved: "Guardado",
    shieldsTitle: "Protecciones durante la concentración",
    shieldsLead: "Se activan durante la concentración y se desactivan solas en los descansos y al detener la sesión.",

    notifications: "Silenciar las notificaciones de Windows",
    notificationsHint:
      "Activa «No molestar» de Windows durante la concentración y lo desactiva en los descansos, al detener la sesión y al cerrar la aplicación — aunque ya estuviera activado antes.",

    teams: "Microsoft Teams: «No molestar»",
    teamsHint:
      "Pone tu estado de Teams durante la concentración y lo quita en los descansos. Si en Teams apareces como desconectado, o pones «No molestar» por tu cuenta, FocusMode no toca tu estado. Requiere conectar la cuenta una sola vez.",
    teamsClientId: "Application (client) ID",
    teamsTenant: "Tenant (Id. de directorio u «organizations»)",
    teamsConnect: "Conectar con Teams",
    teamsDisconnect: "Desconectar",
    teamsConnected: (account: string | null) => (account ? `Conectado como ${account}` : "Conectado"),
    teamsNotConnected: "No conectado",
    teamsNotConfigured: "Primero escribe el Application (client) ID.",
    teamsErrorLead: "No se pudo conectar con Teams:",
    teamsPendingLead: "Abre la página de inicio de sesión de Microsoft y escribe el código:",
    teamsOpenLink: "Abrir la página de inicio de sesión",
    teamsCopyCode: "Copiar código",
    teamsCopied: "Copiado",
    teamsCodeExpires: (min: number) =>
      `El código caduca en aproximadamente ${min} ${plural(min, { one: "minuto", other: "minutos" })}.`,
    teamsHelpTitle: "¿Cómo consigo un Application ID?",
    teamsHelp: [
      "Ve al portal de Azure → Microsoft Entra ID → Registros de aplicaciones → Nuevo registro.",
      "Tipo de cuenta: cuentas de cualquier directorio organizativo. Plataforma: «Aplicaciones móviles y de escritorio» (Mobile and desktop applications).",
      "En «Autenticación», activa «Permitir flujos de clientes públicos» (Allow public client flows).",
      "En «Permisos de API», añade los permisos delegados de Microsoft Graph Presence.ReadWrite y User.Read, y también Calendars.Read para el calendario.",
      "Copia aquí el «Id. de aplicación (cliente)» (Application (client) ID). Si tu administrador tiene que dar su consentimiento, pídeselo.",
    ],

    sites: "Bloquear los sitios que distraen",
    sitesHint:
      "Durante la concentración, la extensión del navegador cambia los sitios de la lista de abajo por una página «Vuelve al trabajo». No se envía nada a internet.",
    extensionTitle: "Extensión del navegador (Chrome, Edge, Brave, Opera, Vivaldi)",
    extensionStep1: "Abre la carpeta de la extensión:",
    extensionOpenDir: "Abrir la carpeta de la extensión",
    extensionStep2:
      "En el navegador, ve a chrome://extensions (Edge: edge://extensions, Opera: opera://extensions) y activa el «Modo de desarrollador».",
    extensionStep3: "Haz clic en «Cargar descomprimida» (Load unpacked) y elige la carpeta que abriste.",
    extensionConnected: "Extensión conectada",
    extensionDisconnected: "Extensión no conectada — uso la detección alternativa por el título de la ventana",
    extensionSeen: (ago: string) => `último contacto hace ${ago}`,

    keepAwake: "Mantener la pantalla encendida",
    keepAwakeHint: "El equipo y la pantalla no entran en suspensión durante la concentración.",

    historyTitle: "Historial",
    trackApps: "Registrar qué aplicaciones usas durante la concentración",
    trackAppsHint:
      "Cada segundo, FocusMode comprueba el nombre del proceso de la ventana activa. Se omiten la barra de tareas, el menú Inicio y otras partes del escritorio de Windows. Los datos se quedan en este equipo.",
    trackTitles: "Registrar los títulos de las ventanas",
    trackTitlesHint:
      "Para cada aplicación, FocusMode recuerda algunos de sus títulos de ventana más frecuentes (p. ej., una pestaña del navegador o una carpeta) y los muestra al pasar el puntero por el gráfico en Resumen. Se quedan solo en este equipo y nunca van a los registros.",
    trackTitlesNeedsApps: "Funciona cuando el registro de aplicaciones está activado.",
    keepHistory: "Guardar el historial de sesiones",
    keepHistoryHint:
      "Si está desactivado, el resumen sigue apareciendo al detener la sesión, pero la sesión no se guarda. Las sesiones guardadas antes se mantienen hasta que las borres.",
    clearHistory: "Borrar todo el historial",
    clearHistoryHint: "Borra de este equipo todas las sesiones guardadas y el último resumen.",
    clearHistoryTitle: "¿Borrar todo el historial?",
    clearHistoryBody:
      "Todas las sesiones guardadas, los gráficos de aplicaciones y los títulos de ventanas se borrarán de forma permanente de este equipo. No se puede deshacer.",
    clearHistoryConfirm: "Sí, borrar el historial",
    historyCleared: "Se borró el historial.",

    appearanceTitle: "Apariencia",

    soundTitle: "Sonido",
    chimeSound: "Sonido de la campanilla",
    chimeSounds: {
      midnight: "Medianoche — campana suave",
      vaporwave: "Vaporwave — acorde de sintetizador",
      lunar: "Fases lunares — tonos cristalinos",
      autumn: "Otoño — marimba de madera",
      classic: "Clásico — din-don",
      soft: "Suave — tono dulce",
      chiptune: "8-bit — arpegio",
      custom: "Propio…",
    },
    chimeSoundHint: "Los nombres coinciden con los temas de apariencia, pero puedes elegir el sonido sin importar el tema.",
    customChimes: "Sonidos propios",
    customBreak: "Empieza el descanso",
    customFocus: "Empieza la concentración (opcional)",
    customNone: "Ningún archivo elegido",
    customDefaultFallback: "Sin un archivo propio, suena Medianoche.",
    customBreakUsesFocus: "Sin un archivo aparte, suena lo mismo que al empezar la concentración.",
    customFocusUsesBreak: "Sin un archivo aparte, suena lo mismo que al empezar el descanso.",
    customPick: "Elegir archivo…",
    customChange: "Cambiar archivo…",
    customPickTitle: (label: string) => `Elige un sonido: ${label}`,
    customFilter: "Sonido (mp3, wav, ogg, flac, m4a)",
    customRemove: "Quitar",
    customRemoveAria: (label: string) => `Quitar el sonido propio: ${label}`,
    customHint: "Archivos mp3, wav, ogg, flac o m4a de hasta 10 MB. FocusMode los copia en su carpeta de datos.",
    customImported: "Se añadió tu sonido.",
    chimeVolume: "Volumen de la campanilla",
    testChime: "Probar la campanilla:",
    testBreak: "descanso",
    testFocus: "concentración",
    musicVolume: "Volumen de la música",

    remindersTitle: "Recordatorios para los descansos",
    remindersLead:
      "Tu propia lista para los descansos de una sesión, p. ej., descanso 1: estiramientos; 2: agua y paseo con el perro; 3: comida. El recordatorio aparece en la pantalla de descanso y en la ventana emergente de FocusMode.",
    reminderLabel: (n: number) => `Descanso ${n}`,
    reminderPlaceholders: ["p. ej., estiramientos", "p. ej., agua y paseo con el perro", "p. ej., comida"],
    reminderPlaceholderOther: "Sin recordatorio",
    reminderAdd: "Añadir un descanso",
    reminderRemove: (n: number) => `Quitar el recordatorio del descanso ${n}`,
    reminderUp: (n: number) => `Subir el recordatorio del descanso ${n}`,
    reminderDown: (n: number) => `Bajar el recordatorio del descanso ${n}`,
    reminderChars: (used: number, max: number) => `${used}/${max}`,
    reminderCharsAria: (used: number, max: number) => `${used} de ${max} caracteres escritos`,
    reminderMoved: (n: number) => `Movido a la posición ${n}.`,
    reminderRemoved: "Recordatorio quitado.",
    remindersEmpty: "Sin recordatorios — en los descansos solo verás sugerencias.",
    remindersFull: (max: number) =>
      `La lista ya tiene ${max} ${plural(max, { one: "elemento", other: "elementos" })} — es el máximo.`,
    remindersRepeat: "Repetir la lista desde el principio",
    remindersRepeatHint:
      "Tras el último elemento, la lista vuelve a empezar. Sin esta opción, los descansos siguientes no tienen recordatorio.",
    remindersInvalid: (max: number, chars: number) =>
      `Como máximo ${max} recordatorios, cada uno de hasta ${chars} ${plural(chars, { one: "carácter", other: "caracteres" })}.`,

    systemTitle: "Sistema",
    launchOnLogin: "Abrir FocusMode al iniciar sesión",
    launchOnLoginHint:
      "Al iniciar sesión, FocusMode se abre minimizado en el área de notificación (el icono junto al reloj).",

    developerTitle: "Desarrollador",
    developerMode: "Activar el modo desarrollador",
    developerModeHint:
      "Añade una pestaña Desarrollador con botones que muestran las ventanas emergentes, las notificaciones y las reacciones de la mascota, para pruebas y capturas de pantalla. Los datos de ejemplo nunca se guardan.",

    launchersTitle: "Lanzadores de juegos",
    launchers: "Cerrar los lanzadores de juegos al empezar la concentración",
    launchersHint:
      "Cada vez que empieza la concentración, FocusMode cierra los lanzadores seleccionados. Solo se cierran los lanzadores, no los juegos — pero un juego abierto desde un lanzador podría perder el progreso no guardado, así que guarda la partida antes.",
    launchersPick: "Qué lanzadores cerrar",
    launchersNone: "Selecciona al menos un lanzador.",
    launchersLastClosed: (names: string) => `La última vez que empezó la concentración, FocusMode cerró: ${names}.`,
    launchersLastNone:
      "La última vez que empezó la concentración, ninguno de los lanzadores seleccionados estaba abierto.",
    launchersUnavailable: "Cerrar lanzadores solo funciona en Windows.",
    launchersError: "No se pudo cerrar uno de los lanzadores.",

    windowTitle: "Ventana y ventanas emergentes",
    closeButton: "El botón X (cerrar ventana)",
    closeAsk: "Preguntar la primera vez",
    closeTray: "Ocultar en el área de notificación",
    closeQuit: "Salir de la aplicación",
    closeHint:
      "Oculto en el área de notificación, FocusMode sigue contando el tiempo y reproduciendo música. Ábrelo desde el icono junto al reloj.",
    phasePopups: "Ventanas emergentes de FocusMode al cambiar de fase",
    phasePopupsHint:
      "Una pequeña ventana en la esquina de la pantalla cuando empieza la concentración o un descanso. Sustituye a las notificaciones de Windows.",

    tintTitle: "Colores cálidos de pantalla",
    tint: "Calentar los colores de la pantalla según la hora del día",
    tintHint:
      "Por la noche, los colores de la pantalla se vuelven más cálidos poco a poco, y por la mañana vuelven los habituales. El calentamiento se aplica después de componer la imagen, así que no aparece en capturas de pantalla ni en grabaciones.",
    tintMode: "Cómo calentar",
    tintModes: {
      nightLight: "Luz nocturna de Windows (recomendado)",
      gamma: "Directamente en la tarjeta gráfica (gamma)",
    },
    tintModeShort: {
      nightLight: "Luz nocturna de Windows",
      gamma: "gamma de la tarjeta gráfica",
    },
    tintModeHints: {
      nightLight:
        "La «Luz nocturna» integrada en Windows. FocusMode la activa según el horario y ajusta su intensidad — sin permisos de administrador. En Windows 11 24H2 y versiones posteriores no se puede controlar desde fuera, así que FocusMode calienta la pantalla mediante la gamma.",
      gamma:
        "Cambia la curva de color de cada monitor, así que cubre toda la pantalla, incluidas la barra de tareas y los juegos a pantalla completa. No funciona en pantallas en modo HDR, y es posible que Windows no permita ajustes muy cálidos.",
    },
    tintFrom: "Desde",
    tintTo: "Hasta",
    tintScheduleHint: "El horario puede pasar de la medianoche, p. ej., de 18:00 a 9:00.",
    tintStrength: "Intensidad",
    tintStrengthHint: "Mueve el control deslizante para verlo en la pantalla.",
    tintTransition: "Transición suave",
    tintTransitionNone: "Sin transición",
    tintTransitionMinutes: (m: number) => `${m} min`,
    tintStatusOff: "Ahora: desactivado",
    tintStatusLevel: (pct: number) => `Ahora: ${pct}%`,
    tintStatusPreview: (pct: number) => `Vista previa: ${pct}%`,
    tintStatusFallback: (mode: string) => `El método elegido no está disponible — uso: ${mode}.`,
    tintStatusCapped: "Windows no permitió la intensidad completa — se aplica un calentamiento más suave.",
    tintStatusError: "No se pudieron calentar los colores de la pantalla — lo intentaré de nuevo en un momento.",
    tintStatusRestoreError:
      "No se pudieron recuperar los colores habituales de la pantalla — lo intentaré de nuevo en un momento.",
    tintTimeInvalid: "Escribe una hora, p. ej., 18:00.",
    tintSameTimes: "«Hasta» tiene que ser distinto de «Desde».",
    tintStrengthRange: "Escribe una intensidad entre 0 y 100.",
    tintTransitionRange: "La transición suave puede durar entre 0 y 180 minutos.",

    languageTitle: "Idioma",
    language: "Idioma de la aplicación",
    languageSystem: (name: string | null) => (name ? `Sistema (${name})` : "Sistema"),

    demoNote: "Modo demo: la concentración dura 40 s y el descanso, 20 s.",
  },

  closePrompt: {
    title: "¿Minimizar FocusMode en el área de notificación?",
    body: "El temporizador y la música siguen en segundo plano, y FocusMode muestra una pequeña ventana emergente cuando empieza la concentración o un descanso.",
    note: "Puedes cambiarlo en Configuración.",
    toTray: "Sí, al área de notificación",
    quit: "No, salir de la aplicación",
  },

  popup: {
    title: {
      breakStarted: "Hora de descansar",
      focusStarted: "Hora de concentrarse",
      interrupted: "Sesión en pausa",
      launchersClosed: "Lanzadores de juegos cerrados",
      meetingReminder: "Reunión durante un descanso",
      breakHeld: "Descanso pausado",
      workDigest: "¿Qué sigue?",
    },
    workDigest: (n: number) =>
      `${n} ${plural(n, { one: "tarea o notificación nueva espera", other: "tareas y notificaciones nuevas esperan" })} al descanso`,
    workDigestGeneric: "Hay tareas y notificaciones nuevas esperando al descanso",
    meetingMessage: (range: string, subject: string) => `${range} · ${subject}`,
    meetingTitle: (n: number) => `Reunión durante el descanso ${n}`,
    meetingBreakAt: (range: string) => `Descanso previsto: ${range}`,
    meetingChoices: (subject: string) => `¿Qué hacer con la reunión ${subject}?`,
    breakHeld: (subject: string) =>
      `Reunión: ${subject}. El temporizador del descanso espera — reanúdalo después de la reunión.`,
    breakHeldGeneric: "Hay una reunión en curso. El temporizador del descanso espera — reanúdalo después de la reunión.",
    breakHeldTitle: (time: string) => `Descanso pausado · ${time}`,
    choiceFailed: "No se pudo guardar tu elección — elige en la ventana de FocusMode.",
    breakTitle: (time: string) => `Hora de descansar · ${time}`,
    breakMusicOn: "La música sigue sonando — aléjate un poco del escritorio.",
    breakMusicPaused: "La música te espera — aléjate un poco del escritorio.",
    breakNoMusic: "Aléjate un poco del escritorio.",
    reminderPrefix: "Tu recordatorio:",
    tipPrefix: (category: string) => `Sugerencia (${category}):`,
    focusMessage: (cycle: number, time: string) =>
      `Ciclo ${cycle}: ${time} de concentración. Protecciones activadas.`,
    focusMessageNoShields: (cycle: number, time: string) => `Ciclo ${cycle}: ${time} de concentración.`,
    interrupted: "El equipo estaba en suspensión — reanuda cuando vuelvas.",
    launchersClosed: (names: string) => `Cerrados: ${names}`,
    showMain: "Mostrar FocusMode",
    skip: "Saltar",
    skipBreak: "Saltar el descanso",
    skipFocus: "Saltar la concentración",
    dismiss: "Cerrar",
  },

  blocklist: {
    title: "Lista de sitios — websiteblacklist.txt",
    pathLabel: "Archivo de la lista",
    loading: "Cargando la lista de sitios…",
    summary: (domains: number, patterns: number) =>
      `${domains} ${plural(domains, { one: "dominio", other: "dominios" })} · ${patterns} ${plural(patterns, { one: "expresión regular", other: "expresiones regulares" })}`,
    empty: "La lista está vacía — durante la concentración no se bloqueará ningún sitio.",
    open: "Abrir websiteblacklist.txt",
    reload: "Volver a cargar",
    reloaded: "Se volvió a cargar la lista de sitios.",
    autoReload: "Cuando guardas el archivo, la lista se vuelve a cargar sola — no hace falta reiniciar nada.",
    allValid: "Todas las líneas son válidas",
    issuesTitle: (n: number) =>
      `${n} ${plural(n, { one: "línea omitida", other: "líneas omitidas" })} — corrige el archivo o pon # al principio de la línea:`,
    issueLine: (line: number) => `Línea ${line}`,
    issueText: {
      invalidDomain: "Dominio no válido",
      invalidRegex: "Expresión regular no válida",
      regexTooComplex: "La expresión regular es demasiado compleja",
      matchesEverything: "La expresión coincide con cualquier dirección",
      pastedUrl:
        "Una dirección con ? es una expresión regular que no coincide con esa dirección — para bloquear solo esta dirección, cambia ? por \\?",
      lineTooLong: "La línea es demasiado larga (límite: 500 caracteres)",
      tooManyDomains: "Demasiados dominios (límite: 2000)",
      tooManyPatterns: "Demasiadas expresiones regulares (límite: 500)",
    },
    pastedUrl: (domain: string) =>
      `Una dirección con ? es una expresión regular que no coincide con esa dirección — para bloquear todo el sitio, escribe ${domain}; para bloquear solo esta dirección, cambia ? por \\?`,
    issueDetailLabel: "Detalles (en inglés)",
    readError: "No se pudo leer el archivo — sigue en uso la lista anterior:",
    helpTitle: "¿Cómo escribo las entradas?",
    helpLines: "Una entrada por línea. Las líneas que empiezan por # son comentarios; las líneas vacías se omiten.",
    helpDomain: "Un dominio simple se bloquea a sí mismo y a todos sus subdominios (p. ej., www., m.).",
    helpRegex:
      "Una expresión regular es una línea entre barras /…/ o una línea que contiene alguno de los caracteres \\ ^ $ * + ? ( ) [ ] { } |. Se compara con la dirección completa de la página, sin distinguir mayúsculas de minúsculas. Un punto significa cualquier carácter, así que en los dominios escribe \\.",
    examplesLabel: "Ejemplos:",
    examples: [
      { code: "reddit.com", note: "todo reddit.com, subdominios incluidos" },
      { code: "youtube\\.com/shorts", note: "solo YouTube Shorts; el resto de YouTube funciona" },
      { code: "linkedin\\.com/feed", note: "solo el feed de LinkedIn" },
    ],
    extensionSkippedTitle:
      "El navegador no puede usar estas expresiones como reglas — las compruebo cuando se carga una pestaña",
  },

  mascot: {
    setting: "Mostrar la mascota (ajolote)",
    settingHint:
      "Un ajolote con auriculares junto al reloj, en los descansos, en el resumen y en las ventanas emergentes al cambiar de fase. Comparte buenas prácticas y sugerencias para el descanso.",
    practiceLabel: "Buena práctica",
    nextUpLabel: "¿Qué sigue?",
    startLines: [
      "¡Manos a la obra!",
      "Empezamos. ¡Suerte!",
      "Auriculares puestos — ¡vamos!",
      "Una tarea a la vez. ¡Adelante!",
    ],
    breakLines: [
      "¡Descanso! Aléjate un rato de la pantalla.",
      "Hora de descansar. ¡Te lo has ganado!",
      "Descanso — estira las piernas.",
    ],
    cycleLines: [
      "¡Ciclo completado! Volvemos a concentrarnos.",
      "¡Genial, otro ciclo hecho!",
      "Vas muy bien. ¡Siguiente ronda!",
    ],
    conflictHint: (subject: string) =>
      `Una reunión cae en un descanso: ${subject}. Decide abajo qué hacer con ella.`,
    tickle: "Hacer cosquillas al ajolote",
    tickleLines: [
      "¡Oye, eso hace cosquillas!",
      "¡Jiji, para ya!",
      "¡Bup! ¿Volvemos al trabajo?",
      "¡Tengo cosquillas en las branquias!",
      "Caricia recibida. ¡Concentración +1!",
      "¡Cuidado, que soy blandito!",
      "¿Otra vez? ¡Bueno, una más!",
      "¡Me descubriste: ahí tengo cosquillas!",
    ],
  },

  calendar: {
    title: "Calendarios",
    lead: "FocusMode revisa las reuniones de las próximas horas y, antes de empezar la sesión, te avisa si alguna cae durante un descanso. Los calendarios solo se leen — FocusMode nunca cambia nada en ellos.",
    privacy:
      "Las reuniones se quedan en este equipo. Los tokens de inicio de sesión y los enlaces de calendario se guardan cifrados, fuera del archivo de configuración, y nunca van a los registros.",
    horizon: "Revisar las reuniones de las próximas",
    horizonOption: (h: number) => `${h} ${plural(h, { one: "hora", other: "horas" })}`,
    horizonHint: "Para una sesión larga, elige un intervalo más amplio para que cubra todos sus descansos.",
    lastSync: (time: string) => `Última sincronización: ${time}`,
    neverSynced: "Todavía no se han leído los calendarios.",
    refresh: "Actualizar ahora",
    refreshing: "Actualizando…",
    sourceName: {
      microsoft: "Microsoft 365 / Outlook",
      google: "Google Calendar",
      ics: "Calendarios de un enlace ICS",
    },
    sourceShort: {
      microsoft: "Outlook",
      google: "Google",
      ics: "ICS",
    },
    state: {
      off: "desactivado",
      on: "conectado",
      unavailable: "no disponible",
      error: "error",
    },
    statusLine: (state: string, account: string | null) => (account ? `${state} · ${account}` : state),
    errorLead: "No se pudo leer el calendario:",

    microsoft: "Microsoft 365 / Outlook (la cuenta Microsoft de Teams)",
    microsoftHint:
      "Lee tu calendario con la misma cuenta Microsoft que usa la integración con Teams (Microsoft Graph, solo lectura: Calendars.Read). Incluye las reuniones de Teams y todos los demás eventos del calendario.",
    microsoftReconnect:
      "FocusMode pide permiso para leer tu calendario (Calendars.Read) solo cuando inicias sesión con el calendario activado. Una cuenta conectada antes solo para Teams hay que conectarla de nuevo — no hace falta desconectarla, y tu estado de Teams sigue funcionando. En el registro de la aplicación en Azure, añade el permiso Calendars.Read.",
    microsoftTeamsAbove: "La cuenta Microsoft se conecta más arriba, en la protección «Microsoft Teams: No molestar».",
    microsoftConnect: "Conectar una cuenta Microsoft",
    microsoftNeedsReconnect:
      "Este inicio de sesión de Microsoft no tiene acceso al calendario. Conecta la cuenta de nuevo y permite leer el calendario.",
    microsoftReconnectButton: "Conectar de nuevo",
    microsoftReconnecting: "Conectando…",
    microsoftReconnectCode: (code: string) =>
      `Escribe el código ${code} en la página de inicio de sesión de Microsoft (está abierta en el navegador; el enlace también está más arriba, en la protección de Teams).`,

    google: "Google Calendar",
    googleHint:
      "Inicias sesión en el navegador, y FocusMode solo pide leer tu calendario. Necesitas tu propio ID de cliente de OAuth gratuito de Google Cloud.",
    googleClientId: "ID de cliente de OAuth",
    googleClientIdPlaceholder: "123456789012-abc….apps.googleusercontent.com",
    googleClientIdInvalid:
      "Un ID de cliente de Google termina en .apps.googleusercontent.com y solo contiene letras, números, puntos, guiones y guiones bajos.",
    googleClientSecret: "Secreto del cliente",
    googleClientSecretPlaceholder: "GOCSPX-…",
    googleClientSecretHint:
      "Del mismo cliente «App de escritorio» de Google Cloud que el ID de arriba. Google lo exige para iniciar sesión, pero en las aplicaciones instaladas no lo considera secreto, así que FocusMode lo guarda en el archivo de configuración (nunca en los registros).",
    googleClientSecretInvalid: (max: number) =>
      `El secreto del cliente solo contiene letras, números, guiones y guiones bajos (como máximo ${max} caracteres).`,
    googleConnect: "Conectar con Google",
    googleDisconnect: "Desconectar",
    googlePending: "Termina de iniciar sesión en la ventana del navegador que se abrió…",
    googleConnected: (account: string | null) => (account ? `Conectado como ${account}` : "Conectado"),
    googleNotConnected: "No conectado",
    googleNeedsClientId: "Primero escribe el ID de cliente.",
    googleErrorLead: "No se pudo conectar con Google:",
    googleHelpTitle: "¿Cómo consigo un ID de cliente de Google?",
    googleHelp: [
      "Abre console.cloud.google.com y crea un proyecto (o elige uno existente).",
      "En «APIs y servicios» → «Biblioteca», habilita la Google Calendar API.",
      "Configura la pantalla de consentimiento de OAuth (Google Auth Platform): público «Externo» (o «Interno» en un Google Workspace de empresa), alcance …/auth/calendar.readonly. Mientras la aplicación esté en modo de prueba, añade tu dirección de correo como usuario de prueba.",
      "En «Clientes» (Credenciales), crea un ID de cliente de OAuth de tipo «App de escritorio» (Desktop app).",
      "Copia el «ID de cliente» (termina en .apps.googleusercontent.com) y el «Secreto del cliente» (empieza por GOCSPX-) en los campos de arriba y pulsa «Conectar con Google».",
    ],

    ics: "Otros calendarios (enlace ICS / iCal)",
    icsHint:
      "Para calendarios con un enlace de suscripción privado: Outlook y Exchange del trabajo, Google, iCloud, Proton, Zoho, Zimbra, Nextcloud y otros. También se incluyen las reuniones periódicas.",
    icsLinks: "Enlaces de calendario — uno por línea",
    icsPlaceholder: "https://outlook.office365.com/owa/calendar/…/calendar.ics\nwebcal://…",
    icsSave: "Guardar enlaces",
    icsSaved: "Se guardaron los enlaces de calendario.",
    icsEdit: "Editar enlaces",
    icsAdd: "Añadir enlaces",
    icsCancel: "Cancelar",
    icsRemoveAll: "Quitar todos los enlaces",
    icsRemoved: "Se quitaron los enlaces de calendario.",
    icsCount: (n: number) =>
      n === 0 ? "ningún enlace" : `${n} ${plural(n, { one: "enlace", other: "enlaces" })}`,
    icsLoading: "Cargando los enlaces guardados…",
    icsInvalidLines: (lines: readonly number[]) =>
      `${lines.length === 1 ? "Línea" : "Líneas"} ${lines.join(", ")}: no parece un enlace de calendario (https:// o webcal://).`,
    icsSecret:
      "Cualquiera que conozca un enlace así puede ver tu calendario. FocusMode lo guarda cifrado y nunca lo muestra en los mensajes de error.",
    icsHelpTitle: "¿Dónde encuentro el enlace ICS?",
    icsHelp: [
      {
        name: "Outlook en la web (Microsoft 365, Exchange)",
        text: "Configuración → Calendario → Calendarios compartidos → «Publicar un calendario»: elige el calendario y «Puede ver todos los detalles», pulsa «Publicar» y copia el enlace ICS. En un servidor Exchange de empresa, la publicación tiene que activarla el administrador.",
      },
      {
        name: "Google Calendar",
        text: "Configuración → elige el calendario a la izquierda → «Integrar el calendario» → «Dirección secreta en formato iCal».",
      },
      {
        name: "iCloud",
        text: "En Calendario, comparte el calendario como «Calendario público» y copia su enlace (webcal://).",
      },
      {
        name: "Proton, Zoho, Zimbra, Nextcloud y otros",
        text: "En la configuración del calendario, busca «Compartir», «Suscribirse», «Enlace ICS» o «iCal».",
      },
    ],

    cardTitle: "Reuniones durante los descansos",
    leadPlanned: "Si empiezas ahora, estas reuniones caerán durante los descansos. Decide qué hacer:",
    leadRunning: "Estas reuniones caen durante los descansos de esta sesión:",
    noConflictsPlanned: "Si empiezas ahora, ninguna reunión caerá durante un descanso.",
    noConflictsRunning: "Ninguna reunión cae durante los descansos de esta sesión.",
    breakInfo: (n: number, range: string) => `Descanso ${n}: ${range}`,
    choice: {
      extend: "Alargar la concentración (música silenciada)",
      pauseAtBreak: "Dejar el descanso en espera",
      ignore: "Ignorar",
      remindLater: "Recordármelo más tarde",
    },
    choiceExtend: (end: string) =>
      `La concentración dura hasta que termine la reunión (${end}), con la música silenciada. El descanso empieza después de la reunión.`,
    choicePauseAtBreak:
      "Cuando llegue la hora del descanso, el temporizador se detiene y espera hasta que pulses «Reanudar».",
    choiceIgnore: "La sesión sigue sin cambios.",
    choiceRemindLater: (time: string | null) =>
      time
        ? `Te lo recordaré en una pequeña ventana emergente de FocusMode (${time}).`
        : "Te lo recordaré más cerca de la reunión.",
    choiceNone: "Elige qué hacer.",
    tooLong: "Reunión de más de 4 h — no se alargará la concentración",
    choiceGroup: (subject: string) => `Qué hacer con la reunión ${subject}`,
    upcomingTitle: "Próximas reuniones",
    upcomingEmpty: (hours: number) =>
      hours === 1 ? "No hay reuniones en la próxima hora." : `No hay reuniones en las próximas ${hours} horas.`,
    now: "ahora",
    online: "en línea",
    onlineAria: "reunión en línea",
    tomorrow: "mañana",
    untitled: "(sin título)",
    connectHint:
      "Conecta un calendario y, antes de una sesión, FocusMode te avisará de las reuniones que caigan en los descansos.",
    connectLink: "Configuración de calendarios",
    sourceErrors: (names: string) => `No se pudo leer: ${names}.`,
    sourceErrorsLink: "Detalles en Configuración",
    loading: "Cargando el calendario…",

    holdExtend: (time: string, subject: string) => `Concentración alargada hasta las ${time} — reunión: ${subject}`,
    holdExtendHint: "La música está silenciada hasta que termine la reunión; después empieza el descanso.",
    holdPause: (subject: string) => `Descanso en espera — reunión: ${subject}`,
    holdPauseHint: (time: string) =>
      `El temporizador del descanso espera. Pulsa «Reanudar» cuando termine la reunión (fin previsto: ${time}).`,
  },

  work: {
    title: "Herramientas de trabajo (¿Qué sigue?)",
    lead: "Conecta las herramientas con las que trabajas y FocusMode te sugerirá qué abordar en tu próxima sesión: primero las tareas asignadas a ti, luego lo que otros esperan de ti (revisiones, correcciones, menciones) y después el resto de notificaciones. Las herramientas solo se leen — FocusMode nunca cambia nada en ellas.",
    privacy:
      "Los tokens se guardan cifrados, fuera del archivo de configuración, y nunca van a los registros. FocusMode solo se conecta a las herramientas activadas arriba; las tareas y notificaciones se quedan en este equipo y nunca se envían a ninguna IA ni a otro servicio.",
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
        "Issues asignados a ti, pull requests que esperan tu revisión o tus correcciones y — con un token clásico — notificaciones.",
      gitlab:
        "Issues asignados a ti, merge requests para revisar y tus merge requests con CI fallida. Funciona con GitLab.com y con instancias autogestionadas.",
      jira: "Incidencias asignadas a ti, con el sprint activo, la prioridad y la fecha de vencimiento. Jira Cloud o Data Center.",
      azureDevops:
        "Elementos de trabajo asignados a ti (con la iteración actual) y pull requests que esperan tu revisión.",
      linear: "Issues asignados a ti, con el ciclo actual, la prioridad y la fecha de vencimiento.",
    },
    state: {
      off: "desactivada",
      on: "conectada",
      unavailable: "no conectada",
      error: "error",
    },
    connectedAs: (account: string | null) => (account ? `Conectado como ${account}` : "Conectado"),
    syncing: "Token guardado — leyendo por primera vez…",
    notConnected: "No conectado — guarda un token abajo.",
    noSecretStore: "Este sistema no puede guardar el token de forma segura, así que esta herramienta no funciona.",
    tokenInvalid: "Un token solo puede contener caracteres ASCII visibles, sin espacios (como máximo 4 KB).",
    needsUrl: {
      gitlab: "Primero escribe la dirección de GitLab.",
      jira: "Primero escribe la dirección de Jira.",
      azureDevops: "Primero escribe el nombre de la organización.",
    },
    errorLead: "No se pudo leer:",

    gitlabUrl: "Dirección de GitLab",
    gitlabUrlPlaceholder: "https://gitlab.com",
    gitlabUrlHint: "Para una instancia autogestionada, escribe su dirección, p. ej., https://gitlab.example.com.",
    jiraUrl: "Dirección de Jira",
    jiraUrlPlaceholder: "https://tu-empresa.atlassian.net",
    jiraUrlHint:
      "La dirección de tu sitio de Jira Cloud o de tu servidor Jira Data Center. Puedes pegar la dirección del navegador sin la parte que va después del ?.",
    jiraEmail: "Correo de la cuenta de Atlassian (solo Jira Cloud)",
    jiraEmailPlaceholder: "ana.garcia@example.com",
    jiraEmailHint: "Para Jira Data Center con un token de acceso personal, déjalo vacío.",
    azureOrg: "Organización de Azure DevOps",
    azureOrgPlaceholder: "tu-empresa",
    azureOrgHint: "El nombre de la dirección https://dev.azure.com/<nombre> — también puedes pegar la dirección completa.",
    urlInvalid:
      "Escribe una dirección que empiece por https://, p. ej., https://tu-empresa.atlassian.net — sin usuario, contraseña, ? ni #.",
    emailInvalid: "Escribe el correo de la cuenta, p. ej., ana@example.com (sin espacios ni dos puntos), o déjalo vacío.",
    azureOrgInvalid:
      "Escribe el nombre de la organización: letras, números, guiones, puntos y guiones bajos (hasta 64 caracteres).",

    token: (name: string) => `Token de acceso de ${name}`,
    tokenPlaceholder: "Pega el token",
    tokenPlaceholderReplace: "Pega un token nuevo para sustituir el guardado",
    tokenHint: "El token se guarda cifrado y nunca vuelve a este campo.",
    tokenSave: "Guardar token",
    tokenSaving: "Guardando…",
    tokenRemove: "Quitar token",
    tokenSaved: (name: string) => `Se guardó el token de ${name}.`,
    tokenRemoved: (name: string) => `Se quitó el token de ${name}.`,
    helpTitle: "¿Cómo creo un token de solo lectura?",
    help: {
      github: [
        "En GitHub: Settings → Developer settings → Personal access tokens → Fine-grained tokens → Generate new token.",
        "Resource owner: tu cuenta o la organización en la que trabajas (un token fine-grained solo ve los repositorios de un propietario). Repository access: All repositories o los repositorios que elijas.",
        "Repository permissions, cada uno solo «Read-only»: Issues, Pull requests y Metadata (GitHub añade Metadata por su cuenta). No marques nada más. Pon una fecha de caducidad.",
      ],
      githubNotifications:
        "GitHub solo ofrece por API las notificaciones (menciones, solicitudes de revisión fuera de los repositorios del token) y los resultados de GitHub Actions a los tokens clásicos — un token fine-grained no puede leerlos. Si quieres verlos, crea en su lugar un token clásico (Tokens (classic)) con el scope notifications. Para repositorios privados, un token clásico también necesita el scope repo, que además permite escribir — FocusMode no lo usa, y por eso recomendamos por defecto un token fine-grained.",
      gitlab: [
        "En GitLab: avatar → Edit profile → Access tokens (Personal access tokens) → Add new token (en versiones más recientes: «Legacy token»).",
        "Marca solo el scope read_api (acceso de lectura a la API). Pon una fecha de caducidad.",
        "Para una instancia de GitLab autogestionada, escribe su dirección arriba.",
      ],
      jira: [
        "Jira Cloud: ve a id.atlassian.com/manage-profile/security/api-tokens y elige «Create API token with scopes», la aplicación Jira y solo los scopes read:jira-work y read:jira-user. Pon una fecha de caducidad corta.",
        "Arriba, escribe la dirección del sitio y el correo de tu cuenta de Atlassian. Un token con scopes solo funciona a través de la pasarela api.atlassian.com — FocusMode la elige por su cuenta. Un token normal sin scopes también funciona, pero tiene todos los permisos de tu cuenta (FocusMode solo lee incidencias).",
        "Jira Data Center: perfil → Personal Access Tokens → Create token, con fecha de caducidad. Deja vacío el campo de correo.",
      ],
      azureDevops: [
        "En dev.azure.com/<organización>: User settings (el icono junto a tu avatar) → Personal access tokens → New Token.",
        "Organization: tu organización. Scopes: Custom defined → Work Items: Read, Code: Read y Project and Team: Read — nada más. Pon una fecha de caducidad.",
      ],
      linear: [
        "En Linear: Settings → Security & access → Personal API keys → New API key.",
        "Permissions: «Only select permissions» → Read only. Team access: todos los equipos o los que elijas.",
      ],
    },

    holdDuringFocus: "Retener las notificaciones durante la concentración",
    holdDuringFocusHint:
      "FocusMode muestra las tareas y notificaciones nuevas del tiempo de concentración solo en el descanso o antes de la siguiente sesión — nada interrumpe tu concentración.",
    lastSync: (time: string) => `Última sincronización: ${time}`,
    neverSynced: "Todavía no se han leído las herramientas.",
    refresh: "Actualizar ahora",
    refreshing: "Actualizando…",

    cardTitle: "¿Qué sigue?",
    cardLead: "Primero lo que tienes asignado, luego lo que otros esperan de ti.",
    loading: "Cargando tareas…",
    connectHint:
      "Conecta GitHub, GitLab, Jira, Azure DevOps o Linear, y FocusMode te sugerirá qué abordar en esta sesión.",
    connectLink: "Configuración de herramientas de trabajo",
    noTokenHint: "Guarda un token en Configuración y tus tareas aparecerán aquí.",
    empty: "No tienes nada pendiente — ni tareas asignadas, ni solicitudes de revisión, ni notificaciones.",
    sourceErrors: (names: string) => `No se pudo leer: ${names}.`,
    sourceErrorsLink: "Detalles en Configuración",
    held: (n: number) =>
      `${n} ${plural(n, { one: "novedad", other: "novedades" })} del tiempo de concentración`,
    newCount: (n: number) => `${n} ${plural(n, { one: "novedad", other: "novedades" })}`,
    unseen: "nuevo",
    showMore: (n: number) => `Mostrar más (${n})`,
    showLess: "Mostrar menos",
    open: "Abrir",
    openAria: (name: string) => `Abrir ${name} en el navegador`,
    setGoal: "Fijar como objetivo",
    setGoalAria: (name: string) => `Fijar ${name} como objetivo de la sesión`,
    isGoal: "Objetivo de la sesión",
    due: (date: string) => `vence: ${date}`,
    reasonsLabel: "Por qué ahora",
    reason: {
      assigned: "asignado a ti",
      inSprint: "sprint",
      highPriority: "prioridad alta",
      dueToday: "vence hoy",
      overdue: "vencido",
      inProgress: "en curso",
      blockingOthers: "espera tu revisión",
      waitingLong: "lleva tiempo esperando",
      mentioned: "mención",
    },
    kind: {
      assigned: "tarea",
      reviewRequested: "revisión",
      changesRequested: "cambios solicitados",
      ciFailed: "CI fallida",
      mention: "mención",
      notification: "notificación",
    },
    itemName: (key: string, title: string) => (key && title ? `${key} ${title}` : key || title),

    goalLabel: "Objetivo de la sesión:",
    goalOpenAria: (name: string) => `Objetivo de la sesión: ${name}. Abrir en el navegador`,
    goalClear: "Quitar el objetivo de la sesión",
  },

  nudge: {
    title: "Vuelve al trabajo",
    body: (domain: string) => `${domain} puede esperar hasta el descanso.`,
    bodyPattern: "Este sitio está en la lista de bloqueo y puede esperar hasta el descanso.",
    dismiss: "Cerrar",
  },

  toasts: {
    region: "Notificaciones",
    dismiss: "Cerrar la notificación",
  },

  errors: {
    generic: "Algo salió mal.",
    fieldInvalid: (label: string) => `Valor no válido: ${label}.`,
    fixConfigFirst: "Corrige los ajustes marcados antes de empezar una sesión.",
    unplayable: "Este enlace no se puede reproducir en la aplicación",
    allUnplayable: "No se puede reproducir nada de la música elegida.",
    autoplayBlocked: "Se bloqueó la reproducción — pulsa «Siguiente» para volver a intentarlo.",
    youtubeApi: "No se pudo cargar el reproductor de YouTube. Comprueba tu conexión a internet.",
    clipboard: "No se pudo copiar al portapapeles.",
    codes: {
      "config.invalid": "Ajuste no válido.",
      "config.corrupt":
        "El archivo de configuración estaba dañado — se restauró la configuración predeterminada (copia: config.json.bak).",
      "config.read": "No se pudo leer el archivo de configuración — uso la configuración predeterminada.",
      "config.save":
        "No se pudo guardar la configuración en el disco — tus cambios funcionan hasta que cierres la aplicación.",
      "audio.device": "No se puede reproducir la campanilla — comprueba tu dispositivo de audio.",
      "shields.notifications": "No se pudo cambiar «No molestar» de Windows.",
      "shields.teams": "No se pudo cambiar tu estado de Teams.",
      "shields.keepAwake": "No se pudo cambiar el ajuste de mantener la pantalla encendida.",
      "bridge.port":
        "El puerto del puente de la extensión está ocupado — el bloqueo de sitios solo funciona en modo alternativo.",
      "teams.auth": "No se pudo iniciar sesión en Teams.",
      "teams.network": "No se puede contactar con los servicios de Microsoft — inténtalo de nuevo en un momento.",
      "teams.notConfigured": "Primero escribe el Application (client) ID.",
      "teams.unsupported": "La integración con Teams no está disponible en este sistema.",
      "teams.disconnect": "No se pudo desconectar la cuenta de Teams.",
      "teams.unavailable": "La integración con Teams no responde — reinicia la aplicación.",
      "music.folder": "No se puede leer la carpeta de música.",
      "media.url":
        "Este enlace no se puede abrir en la ventana de YouTube — solo funcionan direcciones de YouTube o YouTube Music.",
      "media.window": "No se pudo abrir ni controlar la ventana de YouTube.",
      "persistence.history": "Un problema con el historial de sesiones (lectura o guardado).",
      "extension.missing": "No se encontró la carpeta de la extensión del navegador.",
      "extension.open": "No se pudo abrir la carpeta de la extensión.",
      "blocklist.read": "No se pudo leer websiteblacklist.txt — sigue en uso la lista de sitios anterior.",
      "blocklist.open":
        "No se pudo abrir websiteblacklist.txt en un editor de texto — abre el archivo manualmente (la ruta está en Configuración).",
      "shields.launchers": "No se pudo cerrar uno de los lanzadores de juegos — la sesión continúa.",
      "tint.apply": "No se pudieron calentar los colores de la pantalla — lo intentaré de nuevo en un momento.",
      "tint.restore": "No se pudieron recuperar los colores habituales de la pantalla tras la ejecución anterior.",
      "chime.format": "Este formato no es compatible — elige un archivo mp3, wav, ogg, flac o m4a.",
      "chime.read": "No se pudo leer el archivo elegido.",
      "chime.tooLarge": "El archivo es demasiado grande — una campanilla puede tener como máximo 10 MB.",
      "chime.save": "No se pudo guardar el sonido en la carpeta de datos de FocusMode.",
      "chime.decode": "Este archivo no se puede reproducir como sonido — elige otro.",
      "music.library": "No se pudo cargar la música integrada.",
      "popup.window": "No se pudo mostrar la ventana emergente de FocusMode.",
      "calendar.sync": "No se pudo leer el calendario — detalles en Configuración → Calendarios.",
      "calendar.unavailable": "Los calendarios no responden — reinicia la aplicación.",
      "calendar.unknownMeeting": "Esta reunión ya no está en el calendario.",
      "calendar.remindLater":
        "Es demasiado tarde para un recordatorio — la reunión empieza en 10 minutos o menos. Elige otra opción.",
      "calendar.meetingTooLong":
        "Una reunión de más de 4 h no alarga la concentración ni deja el descanso en espera. Elige otra opción.",
      "calendar.googleNotConfigured": "Primero escribe el ID de cliente de Google.",
      "calendar.googleAuth": "No se pudo empezar a iniciar sesión en Google.",
      "calendar.disconnect": "No se pudieron quitar los datos guardados del calendario.",
      "calendar.icsUrl": "Uno de los enlaces no es un enlace de calendario válido (https:// o webcal://).",
      "calendar.icsSave": "No se pudieron guardar de forma segura los enlaces de calendario.",
      "work.sync": "No se pudo leer una herramienta de trabajo — detalles en Configuración → Herramientas de trabajo.",
      "work.unavailable": "Las herramientas de trabajo no responden — reinicia la aplicación.",
      "work.token":
        "Un token solo puede contener caracteres ASCII visibles, sin espacios (como máximo 4 KB) — vuelve a pegarlo.",
      "work.tokenSave": "No se pudo guardar ni quitar el token de forma segura.",
      autostart: "No se pudo cambiar la apertura automática al iniciar sesión.",
      "session.unavailable": "La sesión no responde — reinicia la aplicación.",
      "ipc.unavailable": "Esta función solo está disponible en la aplicación FocusMode.",
      "ipc.failed": "La aplicación no respondió correctamente.",
    },
    areas: {
      config: "Un problema con la configuración.",
      audio: "Un problema con el sonido.",
      shields: "Una de las protecciones no funcionó — la sesión continúa.",
      teams: "Un problema con la conexión a Teams.",
      bridge: "Un problema con el puente de la extensión del navegador.",
      blocklist: "Un problema con la lista de bloqueo (websiteblacklist.txt).",
      music: "Un problema con la música.",
      media: "Un problema con la ventana de YouTube.",
      persistence: "No se pudieron guardar los datos.",
      session: "Un problema con la sesión.",
      tracker: "Un problema con el seguimiento de aplicaciones.",
      tint: "Un problema con los colores cálidos de pantalla.",
      chime: "Un problema con tu sonido de campanilla propio.",
      popup: "Un problema con la ventana emergente de FocusMode.",
      launchers: "Un problema al cerrar los lanzadores de juegos.",
      window: "Un problema con la ventana de la aplicación.",
      calendar: "Un problema con el calendario — la sesión continúa.",
      google: "Un problema con Google Calendar.",
      ics: "Un problema con un calendario de un enlace ICS.",
      work: "Un problema con las herramientas de trabajo (¿Qué sigue?) — la sesión continúa.",
    },
  },

  fields: {
    focusMinutes: "tiempo de concentración",
    breakMinutes: "tiempo de descanso",
    chimeVolume: "volumen de la campanilla",
    "music.volume": "volumen de la música",
    "music.links": "enlaces",
    "music.streamUrl": "dirección del stream",
    "music.folder": "carpeta de música",
    "shields.teamsClientId": "Application (client) ID",
    "shields.teamsTenant": "tenant",
    "shields.launchers": "lanzadores de juegos",
    "tint.startMinutes": "inicio de los colores cálidos",
    "tint.endMinutes": "fin de los colores cálidos",
    "tint.strength": "intensidad de los colores cálidos",
    "tint.transitionMinutes": "transición suave",
    "tint.mode": "método de calentamiento",
    language: "idioma",
    theme: "tema",
    chimeSound: "sonido de la campanilla",
    "customChimes.breakFile": "sonido al empezar el descanso",
    "customChimes.focusFile": "sonido al empezar la concentración",
    breakReminders: "recordatorios para los descansos",
    "calendar.horizonHours": "intervalo de revisión del calendario",
    "calendar.googleClientId": "ID de cliente de Google",
    "calendar.googleClientSecret": "secreto del cliente de Google",
    "work.gitlabUrl": "dirección de GitLab",
    "work.jiraUrl": "dirección de Jira",
    "work.jiraEmail": "correo de la cuenta de Jira",
    "work.azureOrg": "organización de Azure DevOps",
  },

  dialog: {
    cancel: "Cancelar",
  },

  launchers: {
    steam: "Steam",
    epic: "Epic Games Launcher",
    gog: "GOG Galaxy",
    ubisoft: "Ubisoft Connect",
    ea: "Aplicación EA",
    battleNet: "Battle.net",
    riot: "Riot Client",
    xbox: "Aplicación Xbox",
    amazon: "Amazon Games",
    itch: "itch",
  },

  themes: {
    legend: "Tema de la aplicación",
    hint: "Colores y fondo de la ventana principal y de las ventanas emergentes de fase. Los cambios se guardan al momento.",
    selected: "seleccionado",
    names: {
      midnight: "Medianoche",
      vaporwave: "Vaporwave",
      lunar: "Fases lunares",
      autumn: "Otoño acogedor",
      dark: "Oscuro",
      light: "Claro",
      pixel: "8-bit",
    },
    descriptions: {
      midnight: "Una cálida noche violeta con lluvia tras la ventana. El predeterminado.",
      vaporwave: "Rosa, cian y violeta: un sol a rayas sobre una cuadrícula en perspectiva.",
      lunar: "Plata e índigo bajo las estrellas.",
      autumn: "Marrones y ámbar, hojas que caen despacio.",
      dark: "Gris neutro, sin decoraciones de fondo.",
      light: "Fondo claro y texto oscuro de alto contraste, sin decoraciones.",
      pixel: "Marcos cuadrados, sombras duras y estrellas pixeladas.",
    },
    moonToday: (phase: string, litPct: number) =>
      `La luna del fondo muestra la fase de hoy: ${phase} (${litPct}% iluminada).`,
    moonPhases: {
      new: "luna nueva",
      waxingCrescent: "luna creciente",
      firstQuarter: "cuarto creciente",
      waxingGibbous: "gibosa creciente",
      full: "luna llena",
      waningGibbous: "gibosa menguante",
      lastQuarter: "cuarto menguante",
      waningCrescent: "luna menguante",
    },
  },

  /** Break suggestions by id (category, length and source in `$lib/tips`). Suggestions only, never health promises. */
  tips: {
    "eyes-20-20-20": "Mira por la ventana algo que esté a 6 metros o más y mantén la vista allí durante 20 segundos.",
    "eyes-green-view": "Busca por la ventana un árbol, un poco de césped o un trozo de cielo y míralo durante 40 segundos.",
    "eyes-slow-blinks": "Cierra los ojos despacio, cuenta hasta dos y ábrelos. Repite 10 veces con calma.",
    "eyes-look-around":
      "Mira hacia arriba, hacia abajo, a la izquierda y a la derecha; luego haz despacio dos círculos con los ojos en cada sentido.",
    "eyes-near-far":
      "Mira tu pulgar con el brazo extendido durante 10 segundos y luego un punto por la ventana otros 10 segundos. Repite 5 veces.",
    "eyes-screen-free": "Pasa este descanso lejos de las pantallas — pon también el teléfono bocabajo.",
    "stretch-shoulder-rolls": "Gira los hombros despacio hacia atrás 5 veces y luego hacia delante otras 5.",
    "stretch-shoulder-shrug": "Sube los hombros hacia las orejas, mantén 3–5 segundos y déjalos caer. Repite 3 veces.",
    "stretch-neck-turn":
      "Con la espalda recta, gira despacio la cabeza a la izquierda, hasta donde te resulte cómodo. Mantén 5 segundos y luego a la derecha. 3 veces.",
    "stretch-neck-tilt":
      "Inclina despacio la cabeza, llevando la oreja hacia el hombro, y mantén 10 segundos si te resulta cómodo. Luego hacia el otro lado.",
    "stretch-chin-glide":
      "Con la espalda recta, lleva la cabeza hacia atrás en horizontal sin levantar la barbilla. Mantén 5 segundos y repite 5 veces.",
    "stretch-chest-open":
      "Siéntate con la espalda recta, sin apoyarte en el respaldo, junta los omóplatos y abre bien los brazos. Mantén 10 s, 3 veces.",
    "stretch-upper-twist":
      "Cruza los brazos sobre el pecho y gira el tronco a la izquierda, hasta donde te resulte cómodo. Mantén 5 s y luego a la derecha. 5 veces.",
    "stretch-side-reach":
      "Entrelaza los dedos sobre la cabeza, estírate hacia arriba y luego inclínate a la izquierda y a la derecha, 10 segundos a cada lado, si te resulta cómodo.",
    "stretch-wrists":
      "Extiende un brazo y dobla con suavidad la muñeca hacia arriba y luego hacia abajo, 5 segundos cada vez. 3 veces y luego con la otra mano.",
    "stretch-fingers": "Cierra las manos en puño y luego abre bien los dedos. Repite 5 veces, sin prisa.",
    "stretch-prayer":
      "Junta las palmas delante de ti y baja despacio las muñecas hasta notar un estiramiento suave, no dolor. 3 veces.",
    "stretch-ankles":
      "En la silla, estira una pierna separándola del suelo y lleva la punta del pie hacia delante y hacia atrás. 5 veces con cada pierna.",
    "stretch-standing-back":
      "De pie, con las rodillas un poco flexionadas y las manos en la parte baja de la espalda, inclínate con suavidad hacia atrás, hasta donde te resulte cómodo. 3 veces.",
    "stretch-routine":
      "Haz una tanda tranquila de 10 minutos: hombros, cuello, giros del tronco, muñecas y tobillos — siempre hasta donde te resulte cómodo.",
    "move-walk-5": "Levántate y camina 5 minutos — por casa, por el pasillo o hasta la cocina y vuelta.",
    "move-stairs": "Sube un tramo de escaleras y bájalo a tu ritmo. ¿No hay escaleras? Da un paseo por el pasillo.",
    "move-march-seated":
      "Con la espalda recta, levanta las rodillas por turnos como si marcharas, hasta donde te resulte cómodo — 5 veces con cada pierna.",
    "move-march-standing": "Marcha en el sitio durante un minuto, balanceando los brazos sueltos.",
    "move-calf-raises": "De pie junto al escritorio, apoya las manos en él y ponte de puntillas despacio 10 veces.",
    "move-sit-to-stand": "En una silla estable sin ruedas, levántate y siéntate 10 veces con calma.",
    "move-one-song": "Pon una canción que te guste y muévete como te apetezca — de pie o en la silla.",
    "move-chores": "Haz algo activo en casa: tiende la ropa, riega las plantas o saca la basura.",
    "hydrate-glass": "Bebe un vaso de agua — despacio, a sorbos pequeños.",
    "hydrate-refill": "Ve a la cocina y llena de agua una botella o una taza para la próxima sesión.",
    "hydrate-in-reach": "Antes de volver al trabajo, deja un vaso de agua al alcance de la mano.",
    "breathe-count-5":
      "Siéntate cómodamente. Inspira por la nariz contando hasta 5 y espira por la boca contando hasta 5. Sigue así 5 minutos.",
    "breathe-belly-hand":
      "Apoya una mano en el abdomen y, durante un minuto, nota cómo sube al inspirar y baja al espirar.",
    "breathe-long-exhale":
      "Inspira por la nariz, añade un segundo sorbo corto de aire y luego espira despacio por la boca. 5 veces.",
    "breathe-open-window":
      "Abre la ventana y haz 5 respiraciones tranquilas: inspira contando hasta cuatro y espira contando hasta seis.",
    "breathe-muscle-release":
      "Tensa los pies durante 5 segundos y luego suéltalos. Sube así por el cuerpo hasta los hombros y la cara.",
    "posture-reset": "Antes de volver: espalda apoyada en el respaldo, pies planos en el suelo, brazos relajados a los lados.",
    "posture-screen-check":
      "Revisa el monitor: borde superior a la altura de los ojos o un poco por debajo, pantalla a la distancia de un brazo, más o menos.",
    "posture-glare-check":
      "Comprueba que ninguna ventana ni lámpara se refleje en la pantalla. Lo ideal es tener la ventana a un lado del monitor.",
    "posture-shift": "Cambia un rato de postura: muévete en la silla, cambia la posición de las piernas, relaja los hombros.",
    "outside-step-out":
      "Sal al balcón o a la puerta del edificio, o asómate a una ventana abierta, y quédate ahí 2 minutos.",
    "outside-sky": "Sal al aire libre y mira el cielo y las nubes durante un minuto.",
    "outside-walk-15": "Da un paseo de 15 minutos, mejor si hay árboles o césped. Ve a un ritmo tranquilo.",
    "outside-walk-noticing": "Sal a pasear sin el teléfono en la mano y cuenta árboles, pájaros o perros por el camino.",
    "social-chat": "Charla un rato con alguien sobre algo que no tenga nada que ver con el trabajo.",
    "social-message": "Manda un mensaje corto a alguien cercano, solo para saludar.",
    "social-walk-together": "Invita a alguien a dar un paseo corto juntos, aunque sea alrededor del edificio.",
    "social-pet": "¿Tienes un animal en casa? Dedícale unos minutos de toda tu atención.",
    "mind-do-nothing": "Siéntate cómodamente, cierra los ojos y, durante 2 minutos, simplemente no hagas nada.",
    "mind-one-song": "Escucha con atención una canción con los ojos cerrados, sin hacer nada más.",
    "mind-5-4-3": "Nombra mentalmente 5 cosas que ves, 4 que oyes y 3 que puedes tocar.",
    "mind-wander": "Mira por la ventana y deja que tus pensamientos divaguen libremente durante 3 minutos.",
    "mind-park-the-task":
      "Escribe una frase en un papel: con qué empezarás después del descanso. Luego deja ese tema a un lado.",
    "mind-three-good": "Piensa en tres pequeñas cosas buenas de hoy.",
    "mind-meal-away": "Come lejos del escritorio y de la pantalla — en una mesa, en un banco o en compañía.",
    "mind-your-choice": "Haz algo que de verdad te apetezca — este descanso es tuyo.",
    "mind-lie-down": "Si puedes, acuéstate 10–15 minutos con los ojos cerrados. Pon una alarma.",
    "tidy-desk":
      "Despeja el escritorio: llévate las tazas, reúne los papeles sueltos y deja solo lo que necesitas ahora.",
    "tidy-air": "Abre bien la ventana unos minutos y ventila la habitación.",
    "tidy-light": "Ajusta la luz: baja la persiana si deslumbra, o enciende una lámpara si está oscureciendo.",
  },

  /** Good working practices by id (one a day on the idle Focus page). */
  workingTips: {
    "gp-screen-position":
      "Coloca el monitor justo delante de ti, a unos 50–70 cm de los ojos, con el borde superior a la altura de los ojos o un poco por debajo.",
    "gp-light-and-glare": "Pon la pantalla de lado respecto a la ventana y ajusta su brillo a la luz de la habitación.",
    "gp-sitting":
      "Espalda apoyada en el respaldo, pies planos en el suelo, codos cerca del cuerpo, muñecas alineadas con los antebrazos.",
    "gp-short-often":
      "Haz descansos cortos y frecuentes, antes de notar cansancio: unos minutos cada hora es mejor que un descanso largo cada dos horas.",
    "gp-20-20-20":
      "Mientras te concentras, aparta la vista de la pantalla cada 20 minutos más o menos y mira a lo lejos durante 20 segundos. Acuérdate de parpadear.",
    "gp-change-position": "Cambia de postura: levántate del escritorio al menos una vez por hora, aunque sea un momento.",
    "gp-batch-messages":
      "Revisa el correo y el chat a horas fijas, por ejemplo en los descansos, en lugar de después de cada notificación.",
    "gp-single-task": "Elige una tarea por sesión y cierra las pestañas y ventanas que no necesites para ella.",
    "gp-phone-away": "Durante la concentración, silencia el teléfono y déjalo fuera de tu vista.",
    "gp-water-in-reach": "Ten agua al alcance de la mano y bebe a sorbos a lo largo del día.",
  },

  /** Shown under every stretch suggestion. */
  stretchFootnote:
    "Muévete hasta donde te resulte cómodo: debes notar el estiramiento, nunca dolor. Si algo duele, sáltate este ejercicio.",

  time: {
    hoursMinutes: (h: number, m: number) => `${h} h ${m} min`,
    hours: (h: number) => `${h} h`,
    minutes: (m: number) => `${m} min`,
    seconds: (s: number) => `${s} s`,
    today: "hoy",
  },
};
