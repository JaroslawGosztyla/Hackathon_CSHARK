// Russian UI texts (translated from English `en.ts`, checked against the Polish source `pl.ts`; same shape).
// Tone: friendly, short and calm; the user is addressed as "вы" (lowercase), as is usual in Russian software.
// Written without "ё", as Windows does. No health promises: the app suggests, it never claims effects.
// Terms: docs/i18n-glossary.md (column RU). "FocusMode" and websiteblacklist.txt are never translated.
import type { Strings } from "./pl";
import { pluralizer } from "./plural";

const pluralRu = pluralizer("ru");

/** Russian plural form: 1 минута, 2–4 минуты, 5+ минут (21 минута, 11–14 минут). */
function plural(n: number, one: string, few: string, many: string): string {
  return pluralRu(Math.trunc(n), { one, few, many, other: many });
}

export const ru: Strings = {
  appName: "FocusMode",
  tagline: "lofi-сессии фокуса",

  nav: {
    label: "Основная навигация",
    focus: "Фокус",
    summary: "Итоги",
    settings: "Настройки",
    developer: "Разработчик",
  },

  phase: {
    idle: "Готово к старту",
    focus: "Фокус",
    break: "Перерыв",
  },

  phaseAnnouncement: {
    focusStarted: (cycle: number) => `Начинается фокус, цикл ${cycle}.`,
    breakStarted: "Начинается перерыв.",
    stopped: "Сессия завершена.",
  },

  timer: {
    paused: "На паузе",
    cycle: (n: number) => `Цикл ${n}`,
    demo: "короткое демо",
    remainingLabel: (phase: string, time: string) => `${phase}: осталось ${time}`,
    idleHint: "Задайте время и музыку, а затем нажмите «Старт».",
    focusedSoFar: (time: string) => `Фокус за эту сессию: ${time}`,
  },

  controls: {
    start: "Старт",
    pause: "Пауза",
    resume: "Продолжить",
    skipToBreak: "Перерыв сейчас",
    skipToFocus: "Вернуться к фокусу",
    skip: "Пропустить фазу",
    stop: "Стоп",
    sessionControls: "Управление сессией",
  },

  timing: {
    title: "Время",
    focus: "Фокус (мин)",
    break: "Перерыв (мин)",
    presets: "Готовые настройки",
    preset: (focus: number, brk: number) => `${focus} / ${brk}`,
    presetAria: (focus: number, brk: number) =>
      `${focus} ${plural(focus, "минута", "минуты", "минут")} фокуса, ${brk} ${plural(brk, "минута", "минуты", "минут")} перерыва`,
    rangeFocus: "Введите число минут от 1 до 240.",
    rangeBreak: "Введите число минут от 1 до 120.",
  },

  music: {
    title: "Музыка",
    sourceLabel: "Источник музыки",
    source: {
      library: "Open Lo-Fi (встроенная)",
      stream: "Трансляция",
      links: "Ссылки",
      folder: "Папка",
      noise: "Шум",
      none: "Тишина",
    },
    libraryName: "Open Lo-Fi",
    libraryHint: "Спокойные lo-fi-треки, которые идут вместе с FocusMode, — играют без интернета.",
    libraryCredit: "Музыка: Open Lo-Fi, автор btahir, лицензия CC0 (общественное достояние).",
    libraryCreditLink: "github.com/btahir/open-lofi",
    libraryCreditAria: "Open Lo-Fi на GitHub (откроется браузер)",
    libraryEmpty: "Встроенная музыка не найдена — выберите другой источник.",
    streamUrl: "Адрес трансляции (YouTube в прямом эфире или интернет-радио)",
    streamPlaceholder: "https://www.youtube.com/watch?v=…",
    lofiPreset: "Lofi Girl — радио в прямом эфире",
    lofiPresetHint:
      "Текущая прямая трансляция канала Lofi Girl. Играет в фоне через скрытое окно YouTube (владелец не разрешает встраивать ее в приложения); паузой и громкостью вы управляете из FocusMode. В первый раз YouTube покажет окно выбора файлов cookie — после выбора оно скроется само.",
    presetsLabel: "Варианты",
    linksLabel: "Ссылки — по одной в строке (YouTube, YouTube Music, плейлисты, аудиофайлы)",
    linksPlaceholder: "https://music.youtube.com/watch?v=…\nhttps://youtu.be/…\nhttps://example.com/track.mp3",
    linksSummary: (ok: number, bad: number) =>
      bad > 0
        ? `${ok} ${plural(ok, "ссылка", "ссылки", "ссылок")} для воспроизведения, ${bad} ${plural(bad, "неподдерживаемая", "неподдерживаемые", "неподдерживаемых")}`
        : `${ok} ${plural(ok, "ссылка", "ссылки", "ссылок")} для воспроизведения`,
    shuffle: "Случайный порядок",
    pickFolder: "Выбрать папку",
    changeFolder: "Сменить папку",
    pickFolderTitle: "Выберите папку с музыкой",
    noFolder: "Папка не выбрана.",
    loadingTracks: "Ищу аудиофайлы…",
    trackCount: (n: number) => `${n} ${plural(n, "трек", "трека", "треков")}`,
    noTracks: "В этой папке нет поддерживаемых аудиофайлов (mp3, m4a, flac, wav, ogg, opus…).",
    noise: {
      brown: "Коричневый",
      pink: "Розовый",
      white: "Белый",
    },
    noiseLabel: "Тип шума",
    noiseHint: "Генерируется на этом компьютере — работает без интернета.",
    noneHint: "Без музыки — вы услышите только сигналы в начале и в конце перерыва.",
    detected: {
      youtubeVideo: "YouTube — видео или трансляция",
      youtubePlaylist: "YouTube — плейлист",
      youtubeWindow:
        "YouTube — будет играть в маленьком отдельном окне YouTube (эту ссылку нельзя воспроизвести в приложении).",
      audio: "Аудиопоток или аудиофайл",
      unsupported:
        "Этот сервис не поддерживается (например, Spotify) — используйте YouTube или прямую ссылку на аудио.",
      invalid: "Это не похоже на адрес http(s).",
    },
    pauseDuringBreak: "Ставить музыку на паузу во время перерыва",
    pauseDuringBreakHint: "По умолчанию музыка продолжает играть, в том числе во время перерыва.",
  },

  player: {
    region: "Музыкальный плеер",
    idle: "Музыка заиграет, когда начнется сессия",
    pausedBySession: "На паузе",
    pausedForBreak: "На паузе на время перерыва",
    pausedForMeeting: "Приглушено на время встречи",
    loading: "Загрузка…",
    silence: "Тишина",
    next: "Следующий",
    volume: "Громкость музыки",
    expand: "Показать видео",
    collapse: "Скрыть видео",
    noiseTitle: (color: string) => `${color} шум`,
    youtubeFallbackTitle: "YouTube",
    trackOf: (i: number, n: number) => `${i} из ${n}`,
    sourceNotReady: "Выберите источник музыки на экране «Фокус».",
    windowMode: "Играет в фоне через YouTube — владелец не разрешает встраивать это видео в приложения.",
    windowModeShort: "Играет в фоне через YouTube",
    showWindow: "Показать окно",
    showWindowAria: "Показать окно YouTube",
    windowClosed: "Окно YouTube закрыто",
    reopenWindow: "Открыть снова",
    reopenWindowAria: "Снова открыть окно YouTube",
  },

  youtube: {
    title: "Аккаунт YouTube",
    checking: "Проверяю вход в YouTube…",
    signedIn: "Вы вошли в YouTube",
    signedInHint: "Музыка с YouTube играет в фоне через окно YouTube, в вашем аккаунте — с Premium без рекламы.",
    notSignedIn: "Вы не вошли в YouTube",
    notSignedInHint: "Есть YouTube Premium? Войдите, и музыка с YouTube будет играть без рекламы.",
    signIn: "Войти в YouTube",
    signInOpen: "Войдите в открывшемся окне Google — оно закроется само после входа.",
    showSignIn: "Показать окно входа",
    signOut: "Выйти",
    rejected:
      "Google не разрешил войти в окне приложения («Этот браузер или приложение могут быть небезопасными») — музыка играет дальше без входа.",
    privacy:
      "Вы входите на странице Google — FocusMode никогда не видит ваш пароль. Файлы cookie входа остаются только в браузере приложения на этом компьютере; YouTube сохраняет воспроизведенное в истории вашего аккаунта.",
    playerWindowHint: "Играет в фоне через окно YouTube, в вашем аккаунте YouTube.",
    pickerHint: "Есть YouTube Premium?",
    pickerLink: "Войдите в Настройках",
    pickerSignedIn: "Вы вошли в YouTube — музыка с YouTube играет в фоне через окно YouTube.",
  },

  focusView: {
    nowUsing: "Сейчас используете",
    nowUsingEmpty: "Собираю данные о приложениях…",
    trackingOff: "Отслеживание приложений выключено в настройках.",
    idle: "Неактивность",
    unknown: "Неизвестно",
    shields: "Щиты",
    shieldsNone: "Ни один щит не включен — их можно включить в настройках.",
  },

  shieldChip: {
    notifications: "Уведомления приглушены",
    teams: "Teams: Не беспокоить",
    sites: "Блокировка сайтов",
    sitesFallback: "Блокировка сайтов (резервный режим)",
    keepAwake: "Экран не отключается",
    launchers: "Игровые лаунчеры",
    launchersClosed: (names: string) => `Закрыто: ${names}`,
    launchersNothing: "Ни один из выбранных лаунчеров не был запущен.",
    state: {
      off: "выключено",
      on: "включено",
      unavailable: "недоступно",
      error: "ошибка",
    },
    chipAria: (name: string, state: string) => `${name}: ${state}`,
  },

  breakView: {
    title: "Пора сделать перерыв",
    lead: "Отойдите ненадолго от стола. Музыка может играть дальше.",
    reminderLabel: (n: number) => `Ваше напоминание · перерыв ${n}`,
    suggestionLabel: "Идея для этого перерыва",
    another: "Другая идея",
    source: "Источник: ",
    sourceAria: (host: string) => `Источник идеи: ${host} (откроется браузер)`,
    categories: {
      eyes: "Глаза",
      stretch: "Растяжка",
      move: "Движение",
      hydrate: "Вода",
      breathe: "Дыхание",
      posture: "Рабочее место",
      outside: "На улице",
      social: "Люди",
      mind: "Минутка для головы",
      tidy: "Обстановка",
    },
  },

  idleTips: {
    practiceTitle: "Полезная привычка",
    lawTitle: "Полезно знать",
    source: "Источник: ",
    sourceAria: (host: string) => `Источник: ${host} (откроется браузер)`,
  },

  summary: {
    title: "Итоги",
    lastSession: "Последняя сессия",
    selectedSession: "Выбранная сессия",
    empty: "Итогов пока нет. Завершите сессию кнопкой «Стоп» — и здесь появится диаграмма приложений.",
    focusTime: "Время фокуса",
    cycles: "Завершенные циклы",
    cyclesCount: (n: number) => `${n} ${plural(n, "цикл", "цикла", "циклов")}`,
    breaks: "Время перерывов",
    span: "Начало – конец",
    chartTitle: "Время фокуса по приложениям",
    chartSubtitle: (time: string) => `Отслеженное время: ${time}`,
    chartEmpty: "Нет данных о приложениях (отслеживание было выключено или сессия была очень короткой).",
    other: (n: number) => `Другое (${n})`,
    otherPlain: "Другое",
    idle: "Неактивность",
    unknown: "Неизвестно",
    barAria: (name: string, time: string, pct: string) => `${name}: ${time}, ${pct}`,
    titlesHeading: "Самые частые окна",
    titlesHint:
      "Наведите указатель на приложение или перейдите к нему клавишей Tab, чтобы увидеть его самые частые окна.",
    notSaved: "Эта сессия не добавлена в историю — сохранение истории выключено в настройках.",
    history: "История",
    historyEmpty: "История пуста.",
    historyLegend: "Цвета приложений в истории",
    demo: "демо",
    showSession: (date: string) => `Показать сессию от ${date}`,
    backToLast: "Назад к последней сессии",
    privacy: "Данные о приложениях остаются на этом компьютере.",
    goalLabel: "Цель сессии",
    goalQuestion: "Выполнено?",
    goalYes: "Да",
    goalNo: "Нет",
    goalGroup: "Цель сессии выполнена?",
    goalDone: "Выполнено — отличная работа!",
    goalNotDone: "Не в этот раз — ее можно выбрать снова в «Что дальше?».",
    goalOpenAria: (name: string) => `Открыть ${name} в браузере`,
    goalMark: (done: boolean | null) => (done === true ? "цель ✓" : done === false ? "цель ✗" : "цель"),
    goalMarkTitle: (name: string, done: boolean | null) =>
      `Цель: ${name}${done === true ? " — выполнена" : done === false ? " — не выполнена" : ""}`,
    goalAria: (done: boolean | null) =>
      done === true ? "цель выполнена" : done === false ? "цель не выполнена" : "цель без ответа",
  },

  settings: {
    title: "Настройки",
    saving: "Сохранение…",
    saved: "Сохранено",
    shieldsTitle: "Щиты во время фокуса",
    shieldsLead: "Включаются на время фокуса и сами выключаются во время перерыва и после «Стоп».",

    notifications: "Приглушать уведомления Windows",
    notificationsHint:
      "Включает в Windows режим «Не беспокоить» на время фокуса и выключает его во время перерыва, после «Стоп» и при закрытии приложения — даже если он был включен раньше.",

    teams: "Microsoft Teams: «Не беспокоить»",
    teamsHint:
      "Устанавливает ваш статус в Teams на время фокуса и сбрасывает его во время перерыва. Если в Teams вы отображаетесь как «Не в сети» или сами установили «Не беспокоить», FocusMode не меняет ваш статус. Нужно один раз подключить учетную запись.",
    teamsClientId: "Application (client) ID",
    teamsTenant: "Tenant (идентификатор каталога или «organizations»)",
    teamsConnect: "Подключиться к Teams",
    teamsDisconnect: "Отключить",
    teamsConnected: (account: string | null) => (account ? `Подключено как ${account}` : "Подключено"),
    teamsNotConnected: "Не подключено",
    teamsNotConfigured: "Сначала введите Application (client) ID.",
    teamsErrorLead: "Не удалось подключиться к Teams:",
    teamsPendingLead: "Откройте страницу входа Microsoft и введите код:",
    teamsOpenLink: "Открыть страницу входа",
    teamsCopyCode: "Копировать код",
    teamsCopied: "Скопировано",
    teamsCodeExpires: (min: number) =>
      `Код действует еще примерно ${min} ${plural(min, "минуту", "минуты", "минут")}.`,
    teamsHelpTitle: "Как получить Application ID?",
    teamsHelp: [
      "Откройте портал Azure → Microsoft Entra ID → «Регистрация приложений» (App registrations) → «Новая регистрация» (New registration).",
      "Тип учетной записи: учетные записи в любом каталоге организации. Платформа: «Мобильные и классические приложения» (Mobile and desktop applications).",
      "В разделе «Проверка подлинности» (Authentication) включите «Разрешить общедоступные потоки клиента» (Allow public client flows).",
      "В разделе «Разрешения API» (API permissions) добавьте делегированные разрешения Microsoft Graph: Presence.ReadWrite и User.Read, а для календаря — еще Calendars.Read.",
      "Скопируйте сюда «Application (client) ID». Если требуется согласие администратора, попросите его об этом.",
    ],

    sites: "Блокировать отвлекающие сайты",
    sitesHint:
      "Во время фокуса расширение браузера заменяет сайты из списка ниже страницей «Вернемся к работе». Ничего не отправляется в интернет.",
    extensionTitle: "Расширение браузера (Chrome, Edge, Brave, Opera, Vivaldi)",
    extensionStep1: "Откройте папку расширения:",
    extensionOpenDir: "Открыть папку расширения",
    extensionStep2:
      "В браузере перейдите на chrome://extensions (Edge: edge://extensions, Opera: opera://extensions) и включите «Режим разработчика».",
    extensionStep3: "Нажмите «Загрузить распакованное расширение» и выберите открытую папку.",
    extensionConnected: "Расширение подключено",
    extensionDisconnected: "Расширение не подключено — использую резервное определение по заголовку окна",
    extensionSeen: (ago: string) => `последняя связь ${ago} назад`,

    keepAwake: "Не отключать экран",
    keepAwakeHint: "Компьютер и экран не перейдут в спящий режим во время фокуса.",

    historyTitle: "История",
    trackApps: "Записывать, какими приложениями вы пользуетесь во время фокуса",
    trackAppsHint:
      "Каждую секунду FocusMode проверяет имя процесса активного окна. Панель задач, меню «Пуск» и другие элементы рабочего стола Windows пропускаются. Данные остаются на этом компьютере.",
    trackTitles: "Записывать заголовки окон",
    trackTitlesHint:
      "Для каждого приложения FocusMode запоминает несколько самых частых заголовков окон (например, вкладку браузера или папку) и показывает их, когда вы наводите указатель на диаграмму в «Итогах». Они остаются только на этом компьютере и никогда не попадают в журналы.",
    trackTitlesNeedsApps: "Работает, когда запись приложений включена.",
    keepHistory: "Сохранять историю сессий",
    keepHistoryHint:
      "Если выключить, итоги все равно появятся после «Стоп», но сессия не сохранится. Ранее сохраненные сессии останутся, пока вы их не удалите.",
    clearHistory: "Удалить всю историю",
    clearHistoryHint: "Удаляет с этого компьютера все сохраненные сессии и последние итоги.",
    clearHistoryTitle: "Удалить всю историю?",
    clearHistoryBody:
      "Все сохраненные сессии, диаграммы приложений и заголовки окон будут безвозвратно удалены с этого компьютера. Это действие нельзя отменить.",
    clearHistoryConfirm: "Да, удалить историю",
    historyCleared: "История удалена.",

    appearanceTitle: "Внешний вид",

    soundTitle: "Звук",
    chimeSound: "Звук сигнала",
    chimeSounds: {
      midnight: "Midnight — мягкий колокол",
      vaporwave: "Vaporwave — синтезаторный аккорд",
      lunar: "Lunar — стеклянные тона",
      autumn: "Autumn — деревянная маримба",
      classic: "Классический — динь-дон",
      soft: "Нежный — мягкий тон",
      chiptune: "8-bit — арпеджио",
      custom: "Свой…",
    },
    chimeSoundHint: "Названия совпадают с темами оформления, но звук можно выбрать независимо от темы.",
    customChimes: "Свои звуки",
    customBreak: "Начало перерыва",
    customFocus: "Начало фокуса (необязательно)",
    customNone: "Файл не выбран",
    customDefaultFallback: "Без своего файла прозвучит Midnight.",
    customBreakUsesFocus: "Без отдельного файла прозвучит тот же звук, что и в начале фокуса.",
    customFocusUsesBreak: "Без отдельного файла прозвучит тот же звук, что и в начале перерыва.",
    customPick: "Выбрать файл…",
    customChange: "Сменить файл…",
    customPickTitle: (label: string) => `Выберите звук: ${label}`,
    customFilter: "Звук (mp3, wav, ogg, flac, m4a)",
    customRemove: "Удалить",
    customRemoveAria: (label: string) => `Удалить свой звук: ${label}`,
    customHint: "Файлы mp3, wav, ogg, flac или m4a до 10 МБ. FocusMode копирует их в свою папку данных.",
    customImported: "Свой звук добавлен.",
    chimeVolume: "Громкость сигнала",
    testChime: "Проверить сигнал:",
    testBreak: "перерыв",
    testFocus: "фокус",
    musicVolume: "Громкость музыки",

    remindersTitle: "Напоминания на перерывах",
    remindersLead:
      "Свой список для перерывов в сессии, например: перерыв 1 — растяжка, 2 — вода и прогулка с собакой, 3 — обед. Напоминание появится на экране перерыва и во всплывающем окне FocusMode.",
    reminderLabel: (n: number) => `Перерыв ${n}`,
    reminderPlaceholders: ["например, растяжка", "например, вода и прогулка с собакой", "например, обед"],
    reminderPlaceholderOther: "Без напоминания",
    reminderAdd: "Добавить перерыв",
    reminderRemove: (n: number) => `Удалить напоминание для перерыва ${n}`,
    reminderUp: (n: number) => `Переместить напоминание для перерыва ${n} выше`,
    reminderDown: (n: number) => `Переместить напоминание для перерыва ${n} ниже`,
    reminderChars: (used: number, max: number) => `${used}/${max}`,
    reminderCharsAria: (used: number, max: number) => `Символов: ${used} из ${max}`,
    reminderMoved: (n: number) => `Перемещено на позицию ${n}.`,
    reminderRemoved: "Напоминание удалено.",
    remindersEmpty: "Напоминаний нет — во время перерывов вы увидите только идеи.",
    remindersFull: (max: number) =>
      `В списке уже ${max} ${plural(max, "пункт", "пункта", "пунктов")} — это максимум.`,
    remindersRepeat: "Повторять список с начала",
    remindersRepeatHint:
      "После последнего пункта список начинается заново. Без этого следующие перерывы останутся без напоминаний.",
    remindersInvalid: (max: number, chars: number) =>
      `Не больше ${max} ${plural(max, "напоминания", "напоминаний", "напоминаний")}, каждое — до ${chars} ${plural(chars, "символа", "символов", "символов")}.`,

    systemTitle: "Система",
    launchOnLogin: "Запускать FocusMode при входе в систему",
    launchOnLoginHint:
      "После входа в систему FocusMode запускается свернутым в область уведомлений (значок рядом с часами).",

    developerTitle: "Разработчик",
    developerMode: "Включить режим разработчика",
    developerModeHint:
      "Добавляет вкладку «Разработчик» с кнопками, которые вызывают всплывающие окна, уведомления и реакции талисмана, — для тестов и снимков экрана. Тестовые данные никогда не сохраняются.",

    launchersTitle: "Игровые лаунчеры",
    launchers: "Закрывать игровые лаунчеры в начале фокуса",
    launchersHint:
      "В начале каждого фокуса FocusMode закрывает выбранные лаунчеры. Закрываются только лаунчеры, а не сами игры, — но игра, запущенная через лаунчер, может потерять несохраненный прогресс, поэтому сначала сохранитесь.",
    launchersPick: "Какие лаунчеры закрывать",
    launchersNone: "Выберите хотя бы один лаунчер.",
    launchersLastClosed: (names: string) => `В начале последнего фокуса FocusMode закрыл: ${names}.`,
    launchersLastNone: "В начале последнего фокуса ни один из выбранных лаунчеров не был запущен.",
    launchersUnavailable: "Закрытие лаунчеров работает только в Windows.",
    launchersError: "Не удалось закрыть один из лаунчеров.",

    windowTitle: "Окно и всплывающие окна",
    closeButton: "Кнопка X (закрыть окно)",
    closeAsk: "Спросить в первый раз",
    closeTray: "Сворачивать в область уведомлений",
    closeQuit: "Закрывать приложение",
    closeHint:
      "Свернутый в область уведомлений, FocusMode продолжает отсчитывать время и играть музыку. Открыть его можно значком рядом с часами.",
    phasePopups: "Всплывающие окна FocusMode при смене фазы",
    phasePopupsHint:
      "Маленькое окно в углу экрана, когда начинается фокус или перерыв. Заменяет уведомления Windows.",

    tintTitle: "Теплые цвета экрана",
    tint: "Делать цвета экрана теплее в зависимости от времени суток",
    tintHint:
      "Вечером цвета экрана постепенно становятся теплее, а утром возвращаются обычные. Теплый оттенок накладывается уже после формирования изображения, поэтому не попадает на снимки экрана и записи.",
    tintMode: "Способ",
    tintModes: {
      nightLight: "Ночной свет Windows (рекомендуется)",
      gamma: "Напрямую на видеокарте (гамма)",
    },
    tintModeShort: {
      nightLight: "Ночной свет Windows",
      gamma: "гамма видеокарты",
    },
    tintModeHints: {
      nightLight:
        "Встроенный в Windows «Ночной свет». FocusMode включает его по расписанию и задает его интенсивность — без прав администратора. В Windows 11 24H2 и новее им нельзя управлять извне, поэтому FocusMode тогда делает цвета теплее через гамму.",
      gamma:
        "Меняет цветовую кривую каждого монитора, поэтому действует на весь экран, включая панель задач и полноэкранные игры. Не работает на экранах в режиме HDR, а Windows может не разрешить очень теплые настройки.",
    },
    tintFrom: "С",
    tintTo: "До",
    tintScheduleHint: "Интервал может переходить через полночь, например с 18:00 до 9:00.",
    tintStrength: "Интенсивность",
    tintStrengthHint: "Передвиньте ползунок, чтобы увидеть, как это будет выглядеть на экране.",
    tintTransition: "Плавный переход",
    tintTransitionNone: "Без перехода",
    tintTransitionMinutes: (m: number) => `${m} мин`,
    tintStatusOff: "Сейчас: выключено",
    tintStatusLevel: (pct: number) => `Сейчас: ${pct}%`,
    tintStatusPreview: (pct: number) => `Предпросмотр: ${pct}%`,
    tintStatusFallback: (mode: string) => `Выбранный способ недоступен — использую: ${mode}.`,
    tintStatusCapped: "Windows не разрешает полную интенсивность — применен более слабый теплый оттенок.",
    tintStatusError: "Не удалось сделать цвета экрана теплее — попробую снова чуть позже.",
    tintStatusRestoreError: "Не удалось вернуть обычные цвета экрана — попробую снова чуть позже.",
    tintTimeInvalid: "Введите время, например 18:00.",
    tintSameTimes: "Время «До» должно отличаться от «С».",
    tintStrengthRange: "Введите интенсивность от 0 до 100.",
    tintTransitionRange: "Плавный переход может длиться от 0 до 180 минут.",

    languageTitle: "Язык",
    language: "Язык приложения",
    languageSystem: (name: string | null) => (name ? `Системный (${name})` : "Системный"),

    demoNote: "Демо-режим: фокус длится 40 с, перерыв — 20 с.",
  },

  closePrompt: {
    title: "Свернуть FocusMode в область уведомлений?",
    body: "Таймер и музыка продолжат работать в фоне, а FocusMode покажет маленькое всплывающее окно, когда начнется фокус или перерыв.",
    note: "Это можно изменить в настройках.",
    toTray: "Да, в область уведомлений",
    quit: "Нет, закрыть приложение",
  },

  popup: {
    title: {
      breakStarted: "Время перерыва",
      focusStarted: "Время фокуса",
      interrupted: "Сессия приостановлена",
      launchersClosed: "Игровые лаунчеры закрыты",
      meetingReminder: "Встреча во время перерыва",
      breakHeld: "Перерыв ждет",
      workDigest: "Что дальше?",
    },
    workDigest: (n: number) =>
      `${n} ${plural(n, "новая задача или уведомление ждет", "новые задачи и уведомления ждут", "новых задач и уведомлений ждут")} перерыва`,
    workDigestGeneric: "Новые задачи и уведомления ждут перерыва",
    meetingMessage: (range: string, subject: string) => `${range} · ${subject}`,
    meetingTitle: (n: number) => `Встреча во время перерыва ${n}`,
    meetingBreakAt: (range: string) => `Запланированный перерыв: ${range}`,
    meetingChoices: (subject: string) => `Что делать со встречей ${subject}?`,
    breakHeld: (subject: string) => `Встреча: ${subject}. Таймер перерыва ждет — продолжите его после встречи.`,
    breakHeldGeneric: "Идет встреча. Таймер перерыва ждет — продолжите его после встречи.",
    breakHeldTitle: (time: string) => `Перерыв ждет · ${time}`,
    choiceFailed: "Не удалось сохранить выбор — выберите в окне FocusMode.",
    breakTitle: (time: string) => `Время перерыва · ${time}`,
    breakMusicOn: "Музыка играет дальше — отойдите ненадолго от стола.",
    breakMusicPaused: "Музыка подождет — отойдите ненадолго от стола.",
    breakNoMusic: "Отойдите ненадолго от стола.",
    reminderPrefix: "Ваше напоминание:",
    tipPrefix: (category: string) => `Идея (${category}):`,
    focusMessage: (cycle: number, time: string) => `Цикл ${cycle}: ${time} фокуса. Щиты включены.`,
    focusMessageNoShields: (cycle: number, time: string) => `Цикл ${cycle}: ${time} фокуса.`,
    interrupted: "Компьютер был в спящем режиме — продолжите, когда вернетесь.",
    launchersClosed: (names: string) => `Закрыто: ${names}`,
    showMain: "Показать окно",
    skip: "Пропустить",
    skipBreak: "Пропустить перерыв",
    skipFocus: "Пропустить фокус",
    dismiss: "Закрыть",
  },

  blocklist: {
    title: "Список сайтов — websiteblacklist.txt",
    pathLabel: "Файл списка",
    loading: "Загрузка списка сайтов…",
    summary: (domains: number, patterns: number) =>
      `${domains} ${plural(domains, "домен", "домена", "доменов")} · ${patterns} ${plural(patterns, "регулярное выражение", "регулярных выражения", "регулярных выражений")}`,
    empty: "Список пуст — во время фокуса ни один сайт не будет заблокирован.",
    open: "Открыть websiteblacklist.txt",
    reload: "Перезагрузить",
    reloaded: "Список сайтов перезагружен.",
    autoReload: "Как только вы сохраните файл, список обновится сам — ничего перезапускать не нужно.",
    allValid: "Все строки корректны",
    issuesTitle: (n: number) =>
      `${n} ${plural(n, "строка пропущена", "строки пропущены", "строк пропущено")} — исправьте это в файле или поставьте # в начале строки:`,
    issueLine: (line: number) => `Строка ${line}`,
    issueText: {
      invalidDomain: "Некорректный домен",
      invalidRegex: "Некорректное регулярное выражение",
      regexTooComplex: "Регулярное выражение слишком сложное",
      matchesEverything: "Выражение совпадает с любым адресом",
      pastedUrl:
        "Адрес со знаком ? — это регулярное выражение, которое не совпадает с этим адресом. Чтобы заблокировать только этот адрес, замените ? на \\?",
      lineTooLong: "Строка слишком длинная (лимит: 500 символов)",
      tooManyDomains: "Слишком много доменов (лимит: 2000)",
      tooManyPatterns: "Слишком много регулярных выражений (лимит: 500)",
    },
    pastedUrl: (domain: string) =>
      `Адрес со знаком ? — это регулярное выражение, которое не совпадает с этим адресом. Чтобы заблокировать весь сайт, введите ${domain}; чтобы только этот адрес — замените ? на \\?`,
    issueDetailLabel: "Подробности (на английском)",
    readError: "Не удалось прочитать файл — действует предыдущий список:",
    helpTitle: "Как записывать строки?",
    helpLines: "Одна запись в строке. Строки, начинающиеся с #, — комментарии; пустые строки пропускаются.",
    helpDomain: "Обычный домен блокирует себя и все свои поддомены (например, www., m.).",
    helpRegex:
      "Регулярное выражение — это строка в косых чертах /…/ или строка, содержащая любой из символов \\ ^ $ * + ? ( ) [ ] { } |. Оно сравнивается со всем адресом страницы без учета регистра. Точка означает любой символ, поэтому в доменах пишите \\.",
    examplesLabel: "Примеры:",
    examples: [
      { code: "reddit.com", note: "весь reddit.com вместе с поддоменами" },
      { code: "youtube\\.com/shorts", note: "только YouTube Shorts; остальной YouTube работает" },
      { code: "linkedin\\.com/feed", note: "только лента LinkedIn" },
    ],
    extensionSkippedTitle:
      "Браузер не может использовать эти выражения как правила — я проверяю их при загрузке вкладки",
  },

  mascot: {
    setting: "Показывать талисман (аксолотля)",
    settingHint:
      "Аксолотль в наушниках рядом с часами, во время перерыва, в итогах и во всплывающих окнах при смене фазы. Делится полезными привычками и идеями для перерыва.",
    practiceLabel: "Полезная привычка",
    nextUpLabel: "Что дальше?",
    startLines: [
      "За дело!",
      "Начинаем. Удачи!",
      "Наушники надеты — работаем!",
      "Одна задача за раз. Поехали!",
    ],
    breakLines: [
      "Перерыв! Отойдите ненадолго от экрана.",
      "Время для перерыва. Заслуженного!",
      "Перерыв — разомните ноги.",
    ],
    cycleLines: [
      "Цикл пройден! Возвращаемся к фокусу.",
      "Отлично, еще один цикл позади!",
      "Все идет хорошо. Следующий раунд!",
    ],
    conflictHint: (subject: string) =>
      `Встреча выпадает на перерыв: ${subject}. Решите ниже, что с ней делать.`,
    tickle: "Пощекотать аксолотля",
    tickleLines: [
      "Ой, щекотно!",
      "Хи-хи, перестаньте!",
      "Буп! Возвращаемся к работе?",
      "У меня жабры боятся щекотки!",
      "Поглаживание засчитано. Фокус +1!",
      "Осторожно, я мягонький!",
      "Еще раз? Ладно, последний!",
      "Вы нашли, где я боюсь щекотки!",
    ],
  },

  calendar: {
    title: "Календари",
    lead: "FocusMode проверяет встречи на ближайшие часы и еще до начала сессии предупреждает, если какая-то из них выпадает на перерыв. Календари только читаются — FocusMode ничего в них не меняет.",
    privacy:
      "Встречи остаются на этом компьютере. Токены входа и ссылки на календари хранятся в зашифрованном виде, отдельно от файла настроек, и никогда не попадают в журналы.",
    horizon: "Проверять встречи на ближайшие",
    horizonOption: (h: number) => `${h} ${plural(h, "час", "часа", "часов")}`,
    horizonHint: "Для длинной сессии выберите больший интервал, чтобы охватить все ее перерывы.",
    lastSync: (time: string) => `Последняя синхронизация: ${time}`,
    neverSynced: "Календари еще не читались.",
    refresh: "Обновить сейчас",
    refreshing: "Обновление…",
    sourceName: {
      microsoft: "Microsoft 365 / Outlook",
      google: "Google Календарь",
      ics: "Календари по ссылке ICS",
    },
    sourceShort: {
      microsoft: "Outlook",
      google: "Google",
      ics: "ICS",
    },
    state: {
      off: "выключено",
      on: "подключено",
      unavailable: "недоступно",
      error: "ошибка",
    },
    statusLine: (state: string, account: string | null) => (account ? `${state} · ${account}` : state),
    errorLead: "Не удалось прочитать календарь:",

    microsoft: "Microsoft 365 / Outlook (учетная запись Microsoft из Teams)",
    microsoftHint:
      "Читает ваш календарь через ту же учетную запись Microsoft, что и интеграция с Teams (Microsoft Graph, только чтение: Calendars.Read). Охватывает встречи Teams и все остальные события календаря.",
    microsoftReconnect:
      "FocusMode запрашивает разрешение на чтение календаря (Calendars.Read) только при входе с включенным календарем. Учетную запись, подключенную раньше только для Teams, нужно подключить еще раз — отключать ее не нужно, статус в Teams продолжит работать. В регистрации приложения в Azure добавьте разрешение Calendars.Read.",
    microsoftTeamsAbove: "Учетная запись Microsoft подключается выше, в щите «Microsoft Teams: Не беспокоить».",
    microsoftConnect: "Подключить учетную запись Microsoft",
    microsoftNeedsReconnect:
      "У этого входа Microsoft нет доступа к календарю. Подключите учетную запись еще раз и разрешите чтение календаря.",
    microsoftReconnectButton: "Подключить снова",
    microsoftReconnecting: "Подключение…",
    microsoftReconnectCode: (code: string) =>
      `Введите код ${code} на странице входа Microsoft (она открыта в браузере; ссылка есть и выше, в щите Teams).`,

    google: "Google Календарь",
    googleHint:
      "Вход выполняется в браузере, а FocusMode запрашивает только чтение календаря. Нужен собственный бесплатный идентификатор клиента OAuth из Google Cloud.",
    googleClientId: "Идентификатор клиента OAuth (Client ID)",
    googleClientIdPlaceholder: "123456789012-abc….apps.googleusercontent.com",
    googleClientIdInvalid:
      "Идентификатор клиента Google заканчивается на .apps.googleusercontent.com и содержит только буквы, цифры, точки, дефисы и подчеркивания.",
    googleClientSecret: "Секрет клиента (Client secret)",
    googleClientSecretPlaceholder: "GOCSPX-…",
    googleClientSecretHint:
      "Из того же клиента типа «Приложение для ПК» (Desktop app) в Google Cloud, что и идентификатор выше. Google требует его для входа, но в устанавливаемых приложениях не считает секретным, поэтому FocusMode хранит его в файле настроек (но никогда — в журналах).",
    googleClientSecretInvalid: (max: number) =>
      `Секрет клиента содержит только буквы, цифры, дефисы и подчеркивания (не больше ${max} ${plural(max, "символа", "символов", "символов")}).`,
    googleConnect: "Подключиться к Google",
    googleDisconnect: "Отключить",
    googlePending: "Завершите вход в открывшемся окне браузера…",
    googleConnected: (account: string | null) => (account ? `Подключено как ${account}` : "Подключено"),
    googleNotConnected: "Не подключено",
    googleNeedsClientId: "Сначала введите идентификатор клиента.",
    googleErrorLead: "Не удалось подключиться к Google:",
    googleHelpTitle: "Как получить идентификатор клиента Google?",
    googleHelp: [
      "Откройте console.cloud.google.com и создайте проект (или выберите существующий).",
      "В разделе «API и сервисы» → «Библиотека» включите Google Calendar API.",
      "Настройте экран согласия OAuth (Google Auth Platform): аудитория «Внешние» (External; или «Внутренние» (Internal) в корпоративном Google Workspace), область доступа …/auth/calendar.readonly. Пока приложение в режиме тестирования, добавьте свой адрес электронной почты как тестового пользователя.",
      "В разделе «Клиенты» (Clients, «Учетные данные») создайте идентификатор клиента OAuth типа «Приложение для ПК» (Desktop app).",
      "Скопируйте «Идентификатор клиента» (заканчивается на .apps.googleusercontent.com) и «Секрет клиента» (начинается с GOCSPX-) в поля выше и нажмите «Подключиться к Google».",
    ],

    ics: "Другие календари (ссылка ICS / iCal)",
    icsHint:
      "Для календарей с закрытой ссылкой на подписку: Outlook и Exchange на работе, Google, iCloud, Proton, Zoho, Zimbra, Nextcloud и другие. Повторяющиеся встречи тоже учитываются.",
    icsLinks: "Ссылки на календари — по одной в строке",
    icsPlaceholder: "https://outlook.office365.com/owa/calendar/…/calendar.ics\nwebcal://…",
    icsSave: "Сохранить ссылки",
    icsSaved: "Ссылки на календари сохранены.",
    icsEdit: "Изменить ссылки",
    icsAdd: "Добавить ссылки",
    icsCancel: "Отмена",
    icsRemoveAll: "Удалить все ссылки",
    icsRemoved: "Ссылки на календари удалены.",
    icsCount: (n: number) => (n === 0 ? "нет ссылок" : `${n} ${plural(n, "ссылка", "ссылки", "ссылок")}`),
    icsLoading: "Загрузка сохраненных ссылок…",
    icsInvalidLines: (lines: readonly number[]) =>
      `${lines.length === 1 ? "Строка" : "Строки"} ${lines.join(", ")}: это не похоже на ссылку на календарь (https:// или webcal://).`,
    icsSecret:
      "Любой, кто знает такую ссылку, может видеть ваш календарь. FocusMode хранит ее в зашифрованном виде и никогда не показывает в сообщениях об ошибках.",
    icsHelpTitle: "Где найти ссылку ICS?",
    icsHelp: [
      {
        name: "Outlook в интернете (Microsoft 365, Exchange)",
        text: "Параметры → Календарь → Общие календари → «Опубликовать календарь»: выберите календарь и «Может просматривать все сведения», нажмите «Опубликовать» и скопируйте ссылку ICS. На корпоративном сервере Exchange публикацию должен включить администратор.",
      },
      {
        name: "Google Календарь",
        text: "Настройки → выберите календарь слева → «Интеграция календаря» → «Закрытый адрес в формате iCal».",
      },
      {
        name: "iCloud",
        text: "В Календаре откройте общий доступ к календарю как к «Публичному календарю» и скопируйте его ссылку (webcal://).",
      },
      {
        name: "Proton, Zoho, Zimbra, Nextcloud и другие",
        text: "В настройках календаря ищите «Поделиться», «Подписаться», «Ссылка ICS» или «iCal».",
      },
    ],

    cardTitle: "Встречи во время перерывов",
    leadPlanned: "Если начать сейчас, эти встречи выпадут на перерывы. Решите, что делать:",
    leadRunning: "Эти встречи выпадают на перерывы этой сессии:",
    noConflictsPlanned: "Если начать сейчас, ни одна встреча не выпадет на перерыв.",
    noConflictsRunning: "Ни одна встреча не выпадает на перерывы этой сессии.",
    breakInfo: (n: number, range: string) => `Перерыв ${n}: ${range}`,
    choice: {
      extend: "Продлить фокус (музыка приглушена)",
      pauseAtBreak: "Приостановить на перерыве",
      ignore: "Игнорировать",
      remindLater: "Напомнить позже",
    },
    choiceExtend: (end: string) =>
      `Фокус продлится до конца встречи (${end}), музыка будет приглушена. Перерыв начнется после встречи.`,
    choicePauseAtBreak: "Когда наступит время перерыва, таймер остановится и будет ждать, пока вы не нажмете «Продолжить».",
    choiceIgnore: "Сессия идет без изменений.",
    choiceRemindLater: (time: string | null) =>
      time ? `Я напомню в ${time} — в маленьком всплывающем окне FocusMode.` : "Я напомню ближе к встрече.",
    choiceNone: "Выберите, что делать.",
    tooLong: "Встреча длиннее 4 ч — фокус не будет продлен",
    choiceGroup: (subject: string) => `Что делать со встречей ${subject}`,
    upcomingTitle: "Ближайшие встречи",
    upcomingEmpty: (hours: number) =>
      `В течение ${hours} ${plural(hours, "часа", "часов", "часов")} встреч нет.`,
    now: "сейчас",
    online: "онлайн",
    onlineAria: "онлайн-встреча",
    tomorrow: "завтра",
    untitled: "(без названия)",
    connectHint: "Подключите календарь, и FocusMode перед сессией предупредит вас о встречах во время перерывов.",
    connectLink: "Настройки календарей",
    sourceErrors: (names: string) => `Не удалось прочитать: ${names}.`,
    sourceErrorsLink: "Подробности в настройках",
    loading: "Загрузка календаря…",

    holdExtend: (time: string, subject: string) => `Фокус продлен до ${time} — встреча: ${subject}`,
    holdExtendHint: "Музыка приглушена до конца встречи, затем начнется перерыв.",
    holdPause: (subject: string) => `Перерыв приостановлен — встреча: ${subject}`,
    holdPauseHint: (time: string) =>
      `Таймер перерыва ждет. Нажмите «Продолжить», когда встреча закончится (по плану в ${time}).`,
  },

  work: {
    title: "Рабочие инструменты («Что дальше?»)",
    lead: "Подключите инструменты, в которых вы работаете, и FocusMode подскажет, чем заняться в следующей сессии: сначала задачи, назначенные вам, затем то, чего от вас ждут другие (ревью, исправления, упоминания), и в конце остальные уведомления. Инструменты только читаются — FocusMode ничего в них не меняет.",
    privacy:
      "Токены хранятся в зашифрованном виде, отдельно от файла настроек, и никогда не попадают в журналы. FocusMode подключается только к инструментам, включенным выше; задачи и уведомления остаются на этом компьютере и никогда не отправляются ни в какой ИИ или другой сервис.",
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
        "Задачи (issues), назначенные вам, пул-реквесты, ожидающие вашего ревью или ваших исправлений, и — с классическим токеном — уведомления.",
      gitlab:
        "Задачи, назначенные вам, мерж-реквесты на ревью и ваши мерж-реквесты с упавшим CI. Работает с GitLab.com и с собственным сервером GitLab.",
      jira: "Задачи, назначенные вам, с активным спринтом, приоритетом и сроком. Jira Cloud или Data Center.",
      azureDevops:
        "Рабочие элементы (work items), назначенные вам (с текущей итерацией), и пул-реквесты, ожидающие вашего ревью.",
      linear: "Задачи, назначенные вам, с текущим циклом, приоритетом и сроком.",
    },
    state: {
      off: "выключено",
      on: "подключено",
      unavailable: "не подключено",
      error: "ошибка",
    },
    connectedAs: (account: string | null) => (account ? `Подключено как ${account}` : "Подключено"),
    syncing: "Токен сохранен — первое чтение…",
    notConnected: "Не подключено — сохраните токен ниже.",
    noSecretStore: "В этой системе нельзя безопасно сохранить токен, поэтому этот инструмент не работает.",
    tokenInvalid: "Токен может содержать только видимые символы ASCII, без пробелов (не больше 4 КБ).",
    needsUrl: {
      gitlab: "Сначала введите адрес GitLab.",
      jira: "Сначала введите адрес Jira.",
      azureDevops: "Сначала введите название организации.",
    },
    errorLead: "Не удалось прочитать:",

    gitlabUrl: "Адрес GitLab",
    gitlabUrlPlaceholder: "https://gitlab.com",
    gitlabUrlHint: "Для собственного сервера введите его адрес, например https://gitlab.example.com.",
    jiraUrl: "Адрес Jira",
    jiraUrlPlaceholder: "https://your-company.atlassian.net",
    jiraUrlHint:
      "Адрес вашего сайта Jira Cloud или сервера Jira Data Center. Можно вставить адрес из браузера без части после ?.",
    jiraEmail: "Электронная почта учетной записи Atlassian (только Jira Cloud)",
    jiraEmailPlaceholder: "ivan.petrov@example.com",
    jiraEmailHint: "Для Jira Data Center с персональным токеном доступа оставьте пустым.",
    azureOrg: "Организация Azure DevOps",
    azureOrgPlaceholder: "your-company",
    azureOrgHint: "Название из адреса https://dev.azure.com/<название> — можно вставить и весь адрес.",
    urlInvalid:
      "Введите адрес, который начинается с https://, например https://your-company.atlassian.net, — без логина, пароля, ? и #.",
    emailInvalid:
      "Введите адрес электронной почты учетной записи, например ivan@example.com (без пробелов и двоеточий), или оставьте пустым.",
    azureOrgInvalid:
      "Введите название организации: буквы, цифры, дефисы, точки и подчеркивания (до 64 символов).",

    token: (name: string) => `Токен доступа ${name}`,
    tokenPlaceholder: "Вставьте токен",
    tokenPlaceholderReplace: "Вставьте новый токен, чтобы заменить сохраненный",
    tokenHint: "Токен хранится в зашифрованном виде и никогда не возвращается в это поле.",
    tokenSave: "Сохранить токен",
    tokenSaving: "Сохранение…",
    tokenRemove: "Удалить токен",
    tokenSaved: (name: string) => `Токен ${name} сохранен.`,
    tokenRemoved: (name: string) => `Токен ${name} удален.`,
    helpTitle: "Как создать токен только для чтения?",
    help: {
      github: [
        "На GitHub: Settings → Developer settings → Personal access tokens → Fine-grained tokens → Generate new token.",
        "Resource owner: ваша учетная запись или организация, в которой вы работаете (fine-grained-токен видит репозитории только одного владельца). Repository access: All repositories или выбранные репозитории.",
        "Repository permissions, каждое только «Read-only»: Issues, Pull requests и Metadata (Metadata GitHub добавляет сам). Больше ничего не выбирайте. Задайте срок действия.",
      ],
      githubNotifications:
        "Уведомления GitHub (упоминания, запросы на ревью вне репозиториев токена) и результаты GitHub Actions доступны через API только классическим токенам — fine-grained-токен их не прочитает. Если хотите их видеть, создайте вместо него классический токен (Tokens (classic)) с областью notifications. Для приватных репозиториев классическому токену нужна еще область repo, которая дает и право записи, — FocusMode ею не пользуется, поэтому по умолчанию мы советуем fine-grained-токен.",
      gitlab: [
        "В GitLab: аватар → Редактировать профиль → Токены доступа (Personal access tokens) → Добавить новый токен (в новых версиях: «Legacy token»).",
        "Выберите только область read_api (доступ к API на чтение). Задайте срок действия.",
        "Для собственного сервера GitLab введите его адрес выше.",
      ],
      jira: [
        "Jira Cloud: откройте id.atlassian.com/manage-profile/security/api-tokens и выберите «Create API token with scopes», приложение Jira и только области read:jira-work и read:jira-user. Задайте короткий срок действия.",
        "Выше введите адрес сайта и электронную почту учетной записи Atlassian. Токен с областями работает только через шлюз api.atlassian.com — FocusMode выберет его сам. Обычный токен без областей тоже подойдет, но у него есть все права вашей учетной записи (FocusMode только читает задачи).",
        "Jira Data Center: профиль → Personal Access Tokens → Create token, со сроком действия. Поле электронной почты оставьте пустым.",
      ],
      azureDevops: [
        "На dev.azure.com/<организация>: User settings (значок рядом с аватаром) → Personal access tokens → New Token.",
        "Organization: ваша организация. Scopes: Custom defined → Work Items: Read, Code: Read и Project and Team: Read — больше ничего. Задайте срок действия.",
      ],
      linear: [
        "В Linear: Settings → Security & access → Personal API keys → New API key.",
        "Permissions: «Only select permissions» → Read only. Team access: все команды или выбранные.",
      ],
    },

    holdDuringFocus: "Откладывать уведомления во время фокуса",
    holdDuringFocusHint:
      "Новые задачи и уведомления, появившиеся во время фокуса, FocusMode покажет только на перерыве или перед следующей сессией — ничто не прерывает фокус.",
    lastSync: (time: string) => `Последняя синхронизация: ${time}`,
    neverSynced: "Инструменты еще не читались.",
    refresh: "Обновить сейчас",
    refreshing: "Обновление…",

    cardTitle: "Что дальше?",
    cardLead: "Сначала то, что назначено вам, затем то, чего ждут другие.",
    loading: "Загрузка задач…",
    connectHint:
      "Подключите GitHub, GitLab, Jira, Azure DevOps или Linear, и FocusMode подскажет, чем заняться в этой сессии.",
    connectLink: "Настройки рабочих инструментов",
    noTokenHint: "Сохраните токен в настройках, и здесь появятся ваши задачи.",
    empty: "Вас ничего не ждет — нет назначенных задач, запросов на ревью и уведомлений.",
    sourceErrors: (names: string) => `Не удалось прочитать: ${names}.`,
    sourceErrorsLink: "Подробности в настройках",
    held: (n: number) => `${n} ${plural(n, "новое", "новых", "новых")} за время фокуса`,
    newCount: (n: number) => `${n} ${plural(n, "новое", "новых", "новых")}`,
    unseen: "новое",
    showMore: (n: number) => `Показать еще (${n})`,
    showLess: "Показать меньше",
    open: "Открыть",
    openAria: (name: string) => `Открыть ${name} в браузере`,
    setGoal: "Сделать целью",
    setGoalAria: (name: string) => `Сделать ${name} целью сессии`,
    isGoal: "Цель сессии",
    due: (date: string) => `срок: ${date}`,
    reasonsLabel: "Почему сейчас",
    reason: {
      assigned: "назначено вам",
      inSprint: "спринт",
      highPriority: "высокий приоритет",
      dueToday: "срок сегодня",
      overdue: "просрочено",
      inProgress: "в работе",
      blockingOthers: "ждет вашего ревью",
      waitingLong: "долго ждет",
      mentioned: "упоминание",
    },
    kind: {
      assigned: "задача",
      reviewRequested: "ревью",
      changesRequested: "нужны изменения",
      ciFailed: "CI не проходит",
      mention: "упоминание",
      notification: "уведомление",
    },
    itemName: (key: string, title: string) => (key && title ? `${key} ${title}` : key || title),

    goalLabel: "Цель сессии:",
    goalOpenAria: (name: string) => `Цель сессии: ${name}. Открыть в браузере`,
    goalClear: "Очистить цель сессии",
  },

  // Настройки → Конфиденциальность и данные (ADR-020).
  privacy: {
    title: "Конфиденциальность и данные",
    lead: "Все, что сохраняет FocusMode, остается на этом компьютере. Здесь видно, что это и сколько места занимает, — и это можно забрать с собой или удалить.",
    openFolder: "Открыть папку данных",
    export: "Экспортировать мои данные…",
    exportHint:
      "Один файл JSON: настройки, история сессий, ответы на встречи и просмотренные пункты «Что дальше?». Никогда не содержит токенов, ссылок ICS, cookie или других секретов.",
    exportDialogTitle: "Сохранить экспорт данных FocusMode",
    exportFilterName: "Файл JSON",
    exported: "Ваши данные экспортированы.",
    folders: "Папки данных:",
    busy: "Выполняется…",
    loading: "Проверяю сохраненные данные…",
    groups: {
      app: "Настройки и история",
      connections: "Подключения (зашифрованы Windows)",
      browser: "Браузер и журналы",
      restore: "Файлы восстановления",
    },
    restoreHint:
      "Они существуют, только пока работают щиты или теплые цвета (или после сбоя), и исчезают сами, как только FocusMode восстановит настройки Windows. Поэтому здесь их не удаляют.",
    items: {
      settings: "Настройки",
      history: "История сессий",
      meetingChoices: "Ответы на встречи",
      workSeen: "Просмотренные пункты «Что дальше?»",
      microsoftToken: "Аккаунт Microsoft (Teams, календарь)",
      googleToken: "Аккаунт Google (календарь)",
      icsLinks: "Ссылки ICS на календари",
      githubToken: "Токен GitHub",
      gitlabToken: "Токен GitLab",
      jiraToken: "Токен Jira",
      azureDevopsToken: "Токен Azure DevOps",
      linearToken: "Токен Linear",
      customChimes: "Свои звуки сигнала",
      blocklist: "Список сайтов (websiteblacklist.txt)",
      shieldsRestore: "Восстановление уведомлений",
      tintRestore: "Восстановление цветов экрана",
      browserData: "Данные встроенного браузера",
      logs: "Журналы приложения",
    },
    explain: {
      settings: "Ваш выбор на этой странице (config.json). Возвращается к значениям по умолчанию только через «Удалить все».",
      history: "Завершенные сессии: время, циклы и приложения, в которых вы работали.",
      meetingChoices: "Как вы ответили на встречи, совпадающие с перерывом. Только идентификаторы, без тем.",
      workSeen: "Какие пункты «Что дальше?» вы уже видели. Только идентификаторы, без названий.",
      microsoftToken: "Вход для Teams и календаря Microsoft. Зашифрован, никогда не показывается.",
      googleToken: "Вход в Google Календарь. Зашифрован, никогда не показывается.",
      icsLinks: "Личные ссылки на календари. Зашифрованы, никогда не показываются.",
      githubToken: "Токен только для чтения для «Что дальше?». Зашифрован.",
      gitlabToken: "Токен только для чтения для «Что дальше?». Зашифрован.",
      jiraToken: "Токен только для чтения для «Что дальше?». Зашифрован.",
      azureDevopsToken: "Токен только для чтения для «Что дальше?». Зашифрован.",
      linearToken: "Ключ только для чтения для «Что дальше?». Зашифрован.",
      customChimes: "Копии звуков, которые вы импортировали как сигнал (оригиналы остаются на месте).",
      blocklist: "Сайты, которые блокируются во время фокуса. Удаление возвращает список по умолчанию.",
      shieldsRestore: "Состояние уведомлений Windows до фокуса, чтобы оно вернулось даже после сбоя.",
      tintRestore: "Цвета экрана до потепления, чтобы они вернулись даже после сбоя.",
      browserData: "Cookie (в том числе вход и согласие YouTube), кэш и данные сайтов плеера.",
      logs: "Журнал действий приложения для диагностики проблем. Без токенов, названий окон и тем встреч.",
    },
    withTitles: "с названиями окон",
    present: "Сохранено",
    absent: "Нет",
    counts: {
      sessions: (n: number) => `${n} ${plural(n, "сессия", "сессии", "сессий")}`,
      answers: (n: number) => `${n} ${plural(n, "ответ", "ответа", "ответов")}`,
      ids: (n: number) => `${n} ${plural(n, "пункт", "пункта", "пунктов")}`,
      links: (n: number) => `${n} ${plural(n, "ссылка", "ссылки", "ссылок")}`,
      files: (n: number) => `${n} ${plural(n, "файл", "файла", "файлов")}`,
      entries: (n: number) => `${n} ${plural(n, "запись", "записи", "записей")}`,
    },
    delete: "Удалить",
    clear: "Очистить",
    restoreDefault: "Вернуть по умолчанию",
    deleteTitle: (name: string) => `Удалить: ${name}?`,
    resetTitle: "Вернуть список сайтов по умолчанию?",
    consequences: {
      history: "Все сохраненные сессии исчезнут из «Итогов». Это нельзя отменить.",
      meetingChoices: "FocusMode забудет ваши ответы и снова спросит, когда встреча совпадет с перерывом.",
      workSeen: "Текущие пункты «Что дальше?» будут отмечены как новые, пока вы их не посмотрите.",
      microsoftToken:
        "Отключает аккаунт Microsoft: статус Teams и календарь Microsoft не будут работать, пока вы не подключите его снова.",
      googleToken: "Отключает Google Календарь (токен также отзывается в Google).",
      icsLinks: "Эти календари больше не будут проверяться, пока вы снова не добавите ссылки.",
      githubToken: "Отключает GitHub: «Что дальше?» больше не показывает его пункты.",
      gitlabToken: "Отключает GitLab: «Что дальше?» больше не показывает его пункты.",
      jiraToken: "Отключает Jira: «Что дальше?» больше не показывает ее пункты.",
      azureDevopsToken: "Отключает Azure DevOps: «Что дальше?» больше не показывает его пункты.",
      linearToken: "Отключает Linear: «Что дальше?» больше не показывает его пункты.",
      customChimes: "Импортированные звуки исчезнут, а сигнал вернется к звуку по умолчанию.",
      blocklist: "Ваш список сайтов будет заменен списком по умолчанию. Ваши изменения в файле пропадут.",
      browserData:
        "Удаляет cookie, кэш и данные сайтов. Выходит из YouTube, и YouTube снова спросит согласие на cookie.",
      logs: "Старые журналы исчезнут, а текущий будет очищен.",
    },
    sessionStops: "Сначала будет остановлена текущая сессия.",
    deleted: (name: string) => `Удалено: ${name}.`,
    blocklistReset: "Список сайтов по умолчанию восстановлен.",
    deleteAll: "Удалить все данные",
    deleteAllHint: "Удаляет все перечисленное выше и возвращает настройки по умолчанию. Приложение остается установленным.",
    deleteAllButton: "Удалить все…",
    deleteAllTitle: "Удалить все данные FocusMode?",
    deleteAllBody:
      "Приложение останется установленным, а расширение браузера удаляется в самом браузере. С этого компьютера исчезнет:",
    deleteAllSettings: "Настройки (вернутся к значениям по умолчанию, включая язык, тему и запуск при входе)",
    deleteAllConfirm: "Да, удалить все",
    deleteAllDone: "Все данные удалены. Настройки вернулись к значениям по умолчанию.",
  },

  nudge: {
    title: "Вернемся к работе",
    body: (domain: string) => `${domain} подождет до перерыва.`,
    bodyPattern: "Этот сайт в списке блокировки и подождет до перерыва.",
    dismiss: "Закрыть",
  },

  toasts: {
    region: "Уведомления",
    dismiss: "Закрыть уведомление",
  },

  errors: {
    generic: "Что-то пошло не так.",
    fieldInvalid: (label: string) => `Недопустимое значение: ${label}.`,
    fixConfigFirst: "Исправьте отмеченные настройки, прежде чем начать сессию.",
    unplayable: "Эту ссылку нельзя воспроизвести в приложении",
    allUnplayable: "Ничего из выбранной музыки нельзя воспроизвести.",
    autoplayBlocked: "Воспроизведение заблокировано — нажмите «Следующий», чтобы попробовать еще раз.",
    youtubeApi: "Не удалось загрузить плеер YouTube. Проверьте подключение к интернету.",
    clipboard: "Не удалось скопировать в буфер обмена.",
    codes: {
      "config.invalid": "Недопустимая настройка.",
      "config.corrupt":
        "Файл настроек был поврежден — восстановлены настройки по умолчанию (резервная копия: config.json.bak).",
      "config.read": "Не удалось прочитать файл настроек — используются настройки по умолчанию.",
      "config.save": "Не удалось сохранить настройки на диск — изменения действуют до закрытия приложения.",
      "audio.device": "Не удается воспроизвести сигнал — проверьте аудиоустройство.",
      "shields.notifications": "Не удалось переключить режим «Не беспокоить» в Windows.",
      "shields.teams": "Не удалось изменить ваш статус в Teams.",
      "shields.keepAwake": "Не удалось переключить «Не отключать экран».",
      "bridge.port": "Порт моста расширения занят — блокировка сайтов работает только в резервном режиме.",
      "teams.auth": "Не удалось войти в Teams.",
      "teams.network": "Нет связи со службами Microsoft — попробуйте еще раз чуть позже.",
      "teams.notConfigured": "Сначала введите Application (client) ID.",
      "teams.unsupported": "Интеграция с Teams недоступна в этой системе.",
      "teams.disconnect": "Не удалось отключить учетную запись Teams.",
      "teams.unavailable": "Интеграция с Teams не отвечает — перезапустите приложение.",
      "music.folder": "Не удается прочитать папку с музыкой.",
      "media.url": "Эта ссылка не открывается в окне YouTube — подходят только адреса YouTube или YouTube Music.",
      "media.window": "Не удалось открыть окно YouTube или управлять им.",
      "youtube.signIn": "Не удалось открыть окно входа в YouTube.",
      "youtube.signOut": "Не удалось выйти из YouTube — попробуйте еще раз.",
      "persistence.history": "Проблема с историей сессий (чтение или сохранение).",
      "extension.missing": "Папка расширения браузера не найдена.",
      "extension.open": "Не удалось открыть папку расширения.",
      "blocklist.read": "Не удалось прочитать websiteblacklist.txt — действует предыдущий список сайтов.",
      "blocklist.open":
        "Не удалось открыть websiteblacklist.txt в текстовом редакторе — откройте файл вручную (путь указан в настройках).",
      "shields.launchers": "Не удалось закрыть один из игровых лаунчеров — сессия продолжается.",
      "tint.apply": "Не удалось сделать цвета экрана теплее — попробую снова чуть позже.",
      "tint.restore": "Не удалось вернуть обычные цвета экрана после предыдущего запуска.",
      "chime.format": "Этот формат не поддерживается — выберите файл mp3, wav, ogg, flac или m4a.",
      "chime.read": "Не удалось прочитать выбранный файл.",
      "chime.tooLarge": "Файл слишком большой — сигнал может весить не больше 10 МБ.",
      "chime.save": "Не удалось сохранить звук в папке данных FocusMode.",
      "chime.decode": "Этот файл нельзя воспроизвести как звук — выберите другой.",
      "music.library": "Не удалось загрузить встроенную музыку.",
      "popup.window": "Не удалось показать всплывающее окно FocusMode.",
      "calendar.sync": "Не удалось прочитать календарь — подробности в разделе «Настройки» → «Календари».",
      "calendar.unavailable": "Календари не отвечают — перезапустите приложение.",
      "calendar.unknownMeeting": "Этой встречи уже нет в календаре.",
      "calendar.remindLater":
        "Для напоминания уже слишком поздно — встреча начнется через 10 минут или раньше. Выберите другой вариант.",
      "calendar.meetingTooLong":
        "Встреча длиннее 4 ч не продлевает фокус и не приостанавливает перерыв. Выберите другой вариант.",
      "calendar.googleNotConfigured": "Сначала введите идентификатор клиента Google.",
      "calendar.googleAuth": "Не удалось начать вход в Google.",
      "calendar.disconnect": "Не удалось удалить сохраненные данные календаря.",
      "calendar.icsUrl": "Одна из ссылок не является корректной ссылкой на календарь (https:// или webcal://).",
      "calendar.icsSave": "Не удалось безопасно сохранить ссылки на календари.",
      "work.sync":
        "Не удалось прочитать рабочий инструмент — подробности в разделе «Настройки» → «Рабочие инструменты».",
      "work.unavailable": "Рабочие инструменты не отвечают — перезапустите приложение.",
      "work.token":
        "Токен может содержать только видимые символы ASCII, без пробелов (не больше 4 КБ), — вставьте его еще раз.",
      "work.tokenSave": "Не удалось безопасно сохранить или удалить токен.",
      // Codes of src-tauri/src/privacy.rs (+ focus-core privacy.rs).
      "privacy.export": "Не удалось сохранить экспорт — выберите другую папку (вне папки данных FocusMode).",
      "privacy.delete": "Часть данных не удалось удалить (остальное удалено) — попробуйте еще раз.",
      "privacy.notDeletable": "Эти данные не удаляются отдельно.",
      "privacy.openFolder": "Не удалось открыть папку данных.",
      "privacy.report": "Не удалось проверить сохраненные данные.",
      autostart: "Не удалось изменить запуск при входе в систему.",
      "session.unavailable": "Сессия не отвечает — перезапустите приложение.",
      "ipc.unavailable": "Эта функция работает только в приложении FocusMode.",
      "ipc.failed": "Приложение ответило некорректно.",
    },
    areas: {
      config: "Проблема с настройками.",
      audio: "Проблема со звуком.",
      shields: "Один из щитов не сработал — сессия продолжается.",
      teams: "Проблема с подключением к Teams.",
      bridge: "Проблема с мостом расширения браузера.",
      blocklist: "Проблема со списком заблокированных сайтов (websiteblacklist.txt).",
      music: "Проблема с музыкой.",
      media: "Проблема с окном YouTube.",
      youtube: "Проблема с аккаунтом YouTube.",
      persistence: "Не удалось сохранить данные.",
      session: "Проблема с сессией.",
      tracker: "Проблема с отслеживанием приложений.",
      tint: "Проблема с теплыми цветами экрана.",
      chime: "Проблема со своим звуком сигнала.",
      popup: "Проблема со всплывающим окном FocusMode.",
      launchers: "Проблема с закрытием игровых лаунчеров.",
      window: "Проблема с окном приложения.",
      calendar: "Проблема с календарем — сессия продолжается.",
      google: "Проблема с Google Календарем.",
      ics: "Проблема с календарем по ссылке ICS.",
      work: "Проблема с рабочими инструментами («Что дальше?») — сессия продолжается.",
      privacy: "Проблема с данными, которые сохраняет FocusMode.",
    },
  },

  fields: {
    focusMinutes: "длительность фокуса",
    breakMinutes: "длительность перерыва",
    chimeVolume: "громкость сигнала",
    "music.volume": "громкость музыки",
    "music.links": "ссылки",
    "music.streamUrl": "адрес трансляции",
    "music.folder": "папка с музыкой",
    "shields.teamsClientId": "Application (client) ID",
    "shields.teamsTenant": "tenant",
    "shields.launchers": "игровые лаунчеры",
    "tint.startMinutes": "начало теплых цветов",
    "tint.endMinutes": "конец теплых цветов",
    "tint.strength": "интенсивность теплых цветов",
    "tint.transitionMinutes": "плавный переход",
    "tint.mode": "способ смены цветов",
    language: "язык",
    theme: "тема",
    chimeSound: "звук сигнала",
    "customChimes.breakFile": "звук начала перерыва",
    "customChimes.focusFile": "звук начала фокуса",
    breakReminders: "напоминания на перерывах",
    "calendar.horizonHours": "интервал проверки календаря",
    "calendar.googleClientId": "идентификатор клиента Google",
    "calendar.googleClientSecret": "секрет клиента Google",
    "work.gitlabUrl": "адрес GitLab",
    "work.jiraUrl": "адрес Jira",
    "work.jiraEmail": "электронная почта учетной записи Jira",
    "work.azureOrg": "организация Azure DevOps",
  },

  dialog: {
    cancel: "Отмена",
  },

  launchers: {
    steam: "Steam",
    epic: "Epic Games Launcher",
    gog: "GOG Galaxy",
    ubisoft: "Ubisoft Connect",
    ea: "Приложение EA",
    battleNet: "Battle.net",
    riot: "Riot Client",
    xbox: "Приложение Xbox",
    amazon: "Amazon Games",
    itch: "itch",
  },

  themes: {
    legend: "Тема приложения",
    hint: "Цвета и фон главного окна и всплывающих окон фаз. Изменения сохраняются сразу.",
    selected: "выбрана",
    names: {
      midnight: "Полночь",
      vaporwave: "Vaporwave",
      lunar: "Фазы Луны",
      autumn: "Уютная осень",
      dark: "Темная",
      light: "Светлая",
      pixel: "8-bit",
    },
    descriptions: {
      midnight: "Теплая фиолетовая ночь с дождем за окном. По умолчанию.",
      vaporwave: "Розовый, бирюзовый и фиолетовый: полосатое солнце над сеткой в перспективе.",
      lunar: "Серебро и индиго под звездами.",
      autumn: "Коричневые и янтарные тона, медленно падающие листья.",
      dark: "Нейтральный серый, без украшений на фоне.",
      light: "Светлый фон и темный высококонтрастный текст, без украшений.",
      pixel: "Угловатые рамки, жесткие тени и пиксельные звезды.",
    },
    moonToday: (phase: string, litPct: number) =>
      `Луна на фоне показывает сегодняшнюю фазу: ${phase} (освещено ${litPct}%).`,
    moonPhases: {
      new: "новолуние",
      waxingCrescent: "растущий серп",
      firstQuarter: "первая четверть",
      waxingGibbous: "растущая луна",
      full: "полнолуние",
      waningGibbous: "убывающая луна",
      lastQuarter: "последняя четверть",
      waningCrescent: "убывающий серп",
    },
  },

  /** Break suggestions by id (category, length and source in `$lib/tips`). Suggestions only, never health promises. */
  tips: {
    "eyes-20-20-20":
      "Посмотрите в окно на что-нибудь на расстоянии не меньше 6 м и задержите там взгляд на 20 секунд.",
    "eyes-green-view": "Найдите за окном дерево, траву или кусочек неба и посмотрите на них 40 секунд.",
    "eyes-slow-blinks": "Медленно закройте глаза, сосчитайте до двух и откройте. Повторите 10 раз в спокойном темпе.",
    "eyes-look-around":
      "Посмотрите вверх, вниз, влево и вправо, а затем медленно опишите глазами два круга в каждую сторону.",
    "eyes-near-far":
      "10 секунд смотрите на большой палец вытянутой руки, затем 10 секунд — на точку за окном. Повторите 5 раз.",
    "eyes-screen-free": "Проведите этот перерыв без экранов — положите и телефон экраном вниз.",
    "stretch-shoulder-rolls": "Медленно сделайте 5 круговых движений плечами назад, а затем 5 — вперед.",
    "stretch-shoulder-shrug":
      "Поднимите плечи к ушам, задержитесь на 3–5 секунд и свободно опустите. Повторите 3 раза.",
    "stretch-neck-turn":
      "Сядьте ровно и медленно поверните голову влево, насколько удобно. Задержитесь на 5 секунд, затем вправо. 3 раза.",
    "stretch-neck-tilt":
      "Медленно наклоните голову ухом к плечу и задержитесь на 10 секунд, если удобно. Затем в другую сторону.",
    "stretch-chin-glide":
      "Сидя ровно, сдвиньте голову прямо назад, не поднимая подбородок. Задержитесь на 5 секунд, повторите 5 раз.",
    "stretch-chest-open":
      "Сядьте ровно, не опираясь на спинку, сведите лопатки и широко разведите руки. Задержитесь на 10 с, 3 раза.",
    "stretch-upper-twist":
      "Скрестите руки на груди и поверните корпус влево, насколько удобно. Задержитесь на 5 с, затем вправо. 5 раз.",
    "stretch-side-reach":
      "Сцепите пальцы над головой, потянитесь вверх, а затем наклонитесь влево и вправо — по 10 секунд, если удобно.",
    "stretch-wrists":
      "Вытяните руку и мягко согните запястье вверх, затем вниз, по 5 секунд. 3 раза, затем другая рука.",
    "stretch-fingers": "Сожмите кулаки, а затем широко растопырьте пальцы. Повторите 5 раз, не спеша.",
    "stretch-prayer":
      "Сложите ладони перед собой и медленно опускайте запястья до легкого растяжения, не до боли. 3 раза.",
    "stretch-ankles":
      "Сидя, выпрямите ногу над полом и тяните носок то от себя, то к себе. По 5 раз каждой ногой.",
    "stretch-standing-back":
      "Встаньте, слегка согнув колени, положите ладони на поясницу и мягко прогнитесь назад, насколько удобно. 3 раза.",
    "stretch-routine":
      "Сделайте спокойный 10-минутный комплекс: плечи, шея, повороты корпуса, запястья и лодыжки — в удобной амплитуде.",
    "move-walk-5": "Встаньте и пройдитесь 5 минут — по квартире, по коридору или до кухни и обратно.",
    "move-stairs":
      "Поднимитесь по лестнице на один этаж и спуститесь в своем темпе. Нет лестницы? Пройдитесь по коридору.",
    "move-march-seated":
      "Сядьте ровно и по очереди поднимайте колени, как при ходьбе на месте, насколько удобно, — по 5 раз каждой ногой.",
    "move-march-standing": "Помаршируйте на месте минуту, свободно размахивая руками.",
    "move-calf-raises": "Встаньте у стола, обопритесь на него ладонями и 10 раз медленно поднимитесь на носки.",
    "move-sit-to-stand": "На устойчивом стуле без колесиков встаньте и сядьте 10 раз в спокойном темпе.",
    "move-one-song": "Включите одну любимую песню и двигайтесь под нее как хочется — стоя или сидя.",
    "move-chores": "Сделайте что-нибудь активное по дому: развесьте белье, полейте цветы или вынесите мусор.",
    "hydrate-glass": "Выпейте стакан воды — спокойно, маленькими глотками.",
    "hydrate-refill": "Сходите на кухню и наполните бутылку или кружку водой на следующую сессию.",
    "hydrate-in-reach": "Прежде чем вернуться к работе, поставьте стакан воды так, чтобы легко дотянуться.",
    "breathe-count-5":
      "Сядьте удобно. Вдыхайте носом на счет 5, выдыхайте ртом на счет 5. Продолжайте 5 минут.",
    "breathe-belly-hand":
      "Положите ладонь на живот и в течение минуты наблюдайте, как он поднимается на вдохе и опускается на выдохе.",
    "breathe-long-exhale":
      "Вдохните носом, в конце добавьте еще короткий глоток воздуха, а затем медленно выдохните ртом. 5 раз.",
    "breathe-open-window": "Откройте окно и сделайте 5 спокойных вдохов: вдох на счет четыре, выдох — на шесть.",
    "breathe-muscle-release":
      "Напрягите мышцы стоп на 5 секунд, затем расслабьте. Так постепенно двигайтесь вверх по телу — до плеч и лица.",
    "posture-reset": "Прежде чем вернуться: спина на спинке стула, стопы ровно на полу, руки свободно вдоль тела.",
    "posture-screen-check":
      "Проверьте монитор: верхний край на уровне глаз или чуть ниже, экран — примерно на расстоянии вытянутой руки.",
    "posture-glare-check":
      "Проверьте, не отражается ли в экране окно или лампа. Лучше всего, когда окно сбоку от монитора.",
    "posture-shift": "Сядьте ненадолго по-другому: подвиньтесь на стуле, смените положение ног, опустите плечи.",
    "outside-step-out":
      "Выйдите на балкон, на улицу перед домом или встаньте у открытого окна и побудьте там 2 минуты.",
    "outside-sky": "Выйдите на улицу и минуту посмотрите на небо и облака.",
    "outside-walk-15":
      "Сходите на 15-минутную прогулку, лучше туда, где есть деревья или трава. Идите спокойным шагом.",
    "outside-walk-noticing":
      "Выйдите на прогулку без телефона в руке и считайте по дороге деревья, птиц или собак.",
    "social-chat": "Поговорите немного с кем-нибудь о чем-то, что совсем не связано с работой.",
    "social-message": "Напишите короткое сообщение близкому человеку — просто чтобы поздороваться.",
    "social-walk-together": "Пригласите кого-нибудь на короткую совместную прогулку, хотя бы вокруг здания.",
    "social-pet": "Есть домашний питомец? Уделите ему несколько минут полного внимания.",
    "mind-do-nothing": "Сядьте удобно, закройте глаза и 2 минуты просто ничего не делайте.",
    "mind-one-song": "Внимательно послушайте одну песню с закрытыми глазами, больше ничего не делая.",
    "mind-5-4-3": "Назовите про себя 5 вещей, которые видите, 4 — которые слышите, и 3 — которых можете коснуться.",
    "mind-wander": "Посмотрите в окно и позвольте мыслям свободно блуждать 3 минуты.",
    "mind-park-the-task":
      "Запишите на бумаге одно предложение: с чего вы начнете после перерыва. Затем отложите эту тему.",
    "mind-three-good": "Вспомните три мелочи, которые сегодня получились.",
    "mind-meal-away":
      "Поешьте вдали от рабочего места и экрана — за обеденным столом, на скамейке или в компании.",
    "mind-your-choice": "Сделайте то, чего вам действительно хочется, — этот перерыв ваш.",
    "mind-lie-down": "Если можете, полежите 10–15 минут с закрытыми глазами. Поставьте будильник.",
    "tidy-desk":
      "Наведите порядок на столе: унесите кружки, соберите бумаги и оставьте только то, что нужно сейчас.",
    "tidy-air": "Откройте окно настежь на несколько минут и проветрите комнату.",
    "tidy-light": "Настройте свет: задерните штору, если слепит, или включите лампу, если стемнело.",
  },

  /** Good working practices by id (one a day on the idle Focus page). */
  workingTips: {
    "gp-screen-position":
      "Поставьте монитор прямо перед собой, примерно в 50–70 см от глаз, верхним краем на уровне глаз или чуть ниже.",
    "gp-light-and-glare": "Ставьте экран боком к окну и подбирайте его яркость под освещение в комнате.",
    "gp-sitting":
      "Спина на спинке стула, стопы ровно на полу, локти у корпуса, запястья на одной линии с предплечьями.",
    "gp-short-often":
      "Делайте короткие перерывы часто, еще до того, как почувствуете усталость: несколько минут каждый час лучше, чем один длинный перерыв раз в два часа.",
    "gp-20-20-20":
      "Во время фокуса примерно каждые 20 минут отводите взгляд от экрана и 20 секунд смотрите вдаль. Не забывайте моргать.",
    "gp-change-position": "Меняйте положение: хотя бы раз в час вставайте из-за стола, пусть даже ненадолго.",
    "gp-batch-messages":
      "Проверяйте почту и мессенджеры в определенное время, например на перерывах, а не после каждого уведомления.",
    "gp-single-task": "Выберите на сессию одну задачу и закройте вкладки и окна, которые для нее не нужны.",
    "gp-phone-away": "На время фокуса выключите звук на телефоне и уберите его из поля зрения.",
    "gp-water-in-reach": "Держите воду под рукой и понемногу пейте ее в течение дня.",
  },

  /** Shown under every stretch suggestion. */
  stretchFootnote:
    "Двигайтесь в удобной амплитуде: растяжение должно ощущаться, но без боли. Если что-то болит, пропустите это упражнение.",

  time: {
    hoursMinutes: (h: number, m: number) => `${h} ч ${m} мин`,
    hours: (h: number) => `${h} ч`,
    minutes: (m: number) => `${m} мин`,
    seconds: (s: number) => `${s} с`,
    today: "сегодня",
  },
};
