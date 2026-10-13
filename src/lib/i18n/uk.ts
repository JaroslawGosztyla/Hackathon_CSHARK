// Ukrainian UI texts (translated from English `en.ts`, checked against the Polish source `pl.ts`; same shape).
// Tone: friendly, short and calm; the user is addressed as "ви" (lowercase), as is usual in Ukrainian software.
// No health promises: the app suggests, it never claims effects. Terms: docs/i18n-glossary.md (column UK).
// "FocusMode" is a name and is never translated; neither is the file name websiteblacklist.txt.
import type { Strings } from "./pl";
import { pluralizer } from "./plural";

const pluralUk = pluralizer("uk");

/** Ukrainian plural form: 1 хвилина, 2–4 хвилини, 5+ хвилин (21 хвилина, 11–14 хвилин). */
function plural(n: number, one: string, few: string, many: string): string {
  return pluralUk(Math.trunc(n), { one, few, many, other: many });
}

export const uk: Strings = {
  appName: "FocusMode",
  tagline: "lofi-сесії фокусу",

  nav: {
    label: "Головна навігація",
    focus: "Фокус",
    summary: "Підсумок",
    settings: "Налаштування",
    developer: "Розробник",
  },

  phase: {
    idle: "Готово до старту",
    focus: "Фокус",
    break: "Перерва",
  },

  phaseAnnouncement: {
    focusStarted: (cycle: number) => `Починається фокус, цикл ${cycle}.`,
    breakStarted: "Починається перерва.",
    stopped: "Сесію завершено.",
  },

  timer: {
    paused: "На паузі",
    cycle: (n: number) => `Цикл ${n}`,
    demo: "коротке демо",
    remainingLabel: (phase: string, time: string) => `${phase}: залишилося ${time}`,
    idleHint: "Налаштуйте час і музику, а потім натисніть «Старт».",
    focusedSoFar: (time: string) => `Фокус за цю сесію: ${time}`,
  },

  controls: {
    start: "Старт",
    pause: "Пауза",
    resume: "Продовжити",
    skipToBreak: "Перерва зараз",
    skipToFocus: "Повернутися до фокусу",
    skip: "Пропустити фазу",
    stop: "Стоп",
    sessionControls: "Керування сесією",
  },

  timing: {
    title: "Час",
    focus: "Фокус (хв)",
    break: "Перерва (хв)",
    presets: "Готові налаштування",
    preset: (focus: number, brk: number) => `${focus} / ${brk}`,
    presetAria: (focus: number, brk: number) =>
      `${focus} ${plural(focus, "хвилина", "хвилини", "хвилин")} фокусу, ${brk} ${plural(brk, "хвилина", "хвилини", "хвилин")} перерви`,
    rangeFocus: "Введіть кількість хвилин від 1 до 240.",
    rangeBreak: "Введіть кількість хвилин від 1 до 120.",
  },

  music: {
    title: "Музика",
    sourceLabel: "Джерело музики",
    source: {
      library: "Open Lo-Fi (вбудована)",
      stream: "Трансляція",
      links: "Посилання",
      folder: "Папка",
      noise: "Шум",
      none: "Тиша",
    },
    libraryName: "Open Lo-Fi",
    libraryHint: "Спокійні lo-fi треки, що входять до FocusMode, — грають без інтернету.",
    libraryCredit: "Музика: Open Lo-Fi, автор btahir, ліцензія CC0 (суспільне надбання).",
    libraryCreditLink: "github.com/btahir/open-lofi",
    libraryCreditAria: "Open Lo-Fi на GitHub (відкриває браузер)",
    libraryEmpty: "Вбудовану музику не знайдено — виберіть інше джерело.",
    streamUrl: "Адреса трансляції (YouTube наживо або інтернет-радіо)",
    streamPlaceholder: "https://www.youtube.com/watch?v=…",
    lofiPreset: "Lofi Girl — радіо наживо",
    lofiPresetHint:
      "Поточна пряма трансляція каналу Lofi Girl. Грає у фоні через приховане вікно YouTube (власник не дозволяє вбудовувати її в застосунки); паузою та гучністю ви керуєте з FocusMode. Першого разу YouTube покаже вікно вибору файлів cookie — після вибору воно сховається саме.",
    presetsLabel: "Варіанти",
    linksLabel: "Посилання — по одному в рядку (YouTube, YouTube Music, плейлисти, аудіофайли)",
    linksPlaceholder: "https://music.youtube.com/watch?v=…\nhttps://youtu.be/…\nhttps://example.com/track.mp3",
    linksSummary: (ok: number, bad: number) =>
      bad > 0
        ? `${ok} ${plural(ok, "посилання", "посилання", "посилань")} для відтворення, ${bad} ${plural(bad, "непідтримуване", "непідтримувані", "непідтримуваних")}`
        : `${ok} ${plural(ok, "посилання", "посилання", "посилань")} для відтворення`,
    shuffle: "Випадковий порядок",
    pickFolder: "Вибрати папку",
    changeFolder: "Змінити папку",
    pickFolderTitle: "Виберіть папку з музикою",
    noFolder: "Папку не вибрано.",
    loadingTracks: "Шукаю аудіофайли…",
    trackCount: (n: number) => `${n} ${plural(n, "трек", "треки", "треків")}`,
    noTracks: "У цій папці немає підтримуваних аудіофайлів (mp3, m4a, flac, wav, ogg, opus…).",
    noise: {
      brown: "Коричневий",
      pink: "Рожевий",
      white: "Білий",
    },
    noiseLabel: "Тип шуму",
    noiseHint: "Генерується на цьому комп’ютері — працює без інтернету.",
    noneHint: "Без музики — ви почуєте лише сигнали на початку й наприкінці перерви.",
    detected: {
      youtubeVideo: "YouTube — відео або трансляція",
      youtubePlaylist: "YouTube — плейлист",
      youtubeWindow:
        "YouTube — гратиме в маленькому окремому вікні YouTube (це посилання не можна відтворити в застосунку).",
      audio: "Аудіопотік або аудіофайл",
      unsupported:
        "Цей сервіс не підтримується (наприклад, Spotify) — використайте YouTube або пряме посилання на аудіо.",
      invalid: "Це не схоже на адресу http(s).",
    },
    pauseDuringBreak: "Ставити музику на паузу під час перерви",
    pauseDuringBreakHint: "За замовчуванням музика грає далі, зокрема й під час перерви.",
  },

  player: {
    region: "Музичний програвач",
    idle: "Музика заграє, коли почнеться сесія",
    pausedBySession: "На паузі",
    pausedForBreak: "На паузі на час перерви",
    pausedForMeeting: "Приглушено на час зустрічі",
    loading: "Завантаження…",
    silence: "Тиша",
    next: "Наступний",
    volume: "Гучність музики",
    expand: "Показати відео",
    collapse: "Сховати відео",
    noiseTitle: (color: string) => `${color} шум`,
    youtubeFallbackTitle: "YouTube",
    trackOf: (i: number, n: number) => `${i} з ${n}`,
    sourceNotReady: "Виберіть джерело музики на екрані «Фокус».",
    windowMode: "Грає у фоні через YouTube — власник не дозволяє вбудовувати це відео в застосунки.",
    windowModeShort: "Грає у фоні через YouTube",
    showWindow: "Показати вікно",
    showWindowAria: "Показати вікно YouTube",
    windowClosed: "Вікно YouTube закрито",
    reopenWindow: "Відкрити знову",
    reopenWindowAria: "Знову відкрити вікно YouTube",
  },

  focusView: {
    nowUsing: "Зараз використовуєте",
    nowUsingEmpty: "Збираю дані про застосунки…",
    trackingOff: "Відстеження застосунків вимкнено в налаштуваннях.",
    idle: "Неактивність",
    unknown: "Невідомо",
    shields: "Щити",
    shieldsNone: "Жоден щит не ввімкнено — їх можна ввімкнути в налаштуваннях.",
  },

  shieldChip: {
    notifications: "Сповіщення приглушено",
    teams: "Teams: Не турбувати",
    sites: "Блокування сайтів",
    sitesFallback: "Блокування сайтів (резервний режим)",
    keepAwake: "Екран не вимикається",
    launchers: "Лаунчери ігор",
    launchersClosed: (names: string) => `Закрито: ${names}`,
    launchersNothing: "Жоден із вибраних лаунчерів не був запущений.",
    state: {
      off: "вимкнено",
      on: "увімкнено",
      unavailable: "недоступно",
      error: "помилка",
    },
    chipAria: (name: string, state: string) => `${name}: ${state}`,
  },

  breakView: {
    title: "Час зробити перерву",
    lead: "Відійдіть ненадовго від столу. Музика може грати далі.",
    reminderLabel: (n: number) => `Ваше нагадування · перерва ${n}`,
    suggestionLabel: "Ідея для цієї перерви",
    another: "Інша ідея",
    source: "Джерело: ",
    sourceAria: (host: string) => `Джерело ідеї: ${host} (відкриває браузер)`,
    categories: {
      eyes: "Очі",
      stretch: "Розтяжка",
      move: "Рух",
      hydrate: "Вода",
      breathe: "Дихання",
      posture: "Робоче місце",
      outside: "Надворі",
      social: "Люди",
      mind: "Хвилинка для голови",
      tidy: "Оточення",
    },
  },

  idleTips: {
    practiceTitle: "Корисна звичка",
    lawTitle: "Варто знати",
    source: "Джерело: ",
    sourceAria: (host: string) => `Джерело: ${host} (відкриває браузер)`,
  },

  summary: {
    title: "Підсумок",
    lastSession: "Остання сесія",
    selectedSession: "Вибрана сесія",
    empty: "Підсумку ще немає. Завершіть сесію кнопкою «Стоп» — і тут з’явиться діаграма застосунків.",
    focusTime: "Час фокусу",
    cycles: "Завершені цикли",
    cyclesCount: (n: number) => `${n} ${plural(n, "цикл", "цикли", "циклів")}`,
    breaks: "Час перерв",
    span: "Початок – кінець",
    chartTitle: "Час фокусу за застосунками",
    chartSubtitle: (time: string) => `Відстежений час: ${time}`,
    chartEmpty: "Немає даних про застосунки (відстеження було вимкнено або сесія була дуже короткою).",
    other: (n: number) => `Інше (${n})`,
    otherPlain: "Інше",
    idle: "Неактивність",
    unknown: "Невідомо",
    barAria: (name: string, time: string, pct: string) => `${name}: ${time}, ${pct}`,
    titlesHeading: "Найчастіші вікна",
    titlesHint: "Наведіть курсор на застосунок або перейдіть до нього клавішею Tab, щоб побачити його найчастіші вікна.",
    notSaved: "Цю сесію не додано до історії — збереження історії вимкнено в налаштуваннях.",
    history: "Історія",
    historyEmpty: "Історія порожня.",
    historyLegend: "Кольори застосунків в історії",
    demo: "демо",
    showSession: (date: string) => `Показати сесію від ${date}`,
    backToLast: "Назад до останньої сесії",
    privacy: "Дані про застосунки залишаються на цьому комп’ютері.",
    goalLabel: "Ціль сесії",
    goalQuestion: "Виконано?",
    goalYes: "Так",
    goalNo: "Ні",
    goalGroup: "Чи виконано ціль сесії?",
    goalDone: "Виконано — чудова робота!",
    goalNotDone: "Цього разу ні — її можна вибрати знову в «Що далі?».",
    goalOpenAria: (name: string) => `Відкрити ${name} у браузері`,
    goalMark: (done: boolean | null) => (done === true ? "ціль ✓" : done === false ? "ціль ✗" : "ціль"),
    goalMarkTitle: (name: string, done: boolean | null) =>
      `Ціль: ${name}${done === true ? " — виконано" : done === false ? " — не виконано" : ""}`,
    goalAria: (done: boolean | null) =>
      done === true ? "ціль виконано" : done === false ? "ціль не виконано" : "ціль без відповіді",
  },

  settings: {
    title: "Налаштування",
    saving: "Збереження…",
    saved: "Збережено",
    shieldsTitle: "Щити під час фокусу",
    shieldsLead: "Вмикаються на час фокусу й самі вимикаються під час перерви та після «Стоп».",

    notifications: "Приглушувати сповіщення Windows",
    notificationsHint:
      "Вмикає в Windows режим «Не турбувати» на час фокусу й вимикає його під час перерви, після «Стоп» і під час закриття застосунку — навіть якщо він був увімкнений раніше.",

    teams: "Microsoft Teams: «Не турбувати»",
    teamsHint:
      "Встановлює ваш статус у Teams на час фокусу й скидає його під час перерви. Якщо в Teams ви відображаєтеся як не в мережі або самі встановили «Не турбувати», FocusMode не змінює ваш статус. Потрібно один раз підключити обліковий запис.",
    teamsClientId: "Application (client) ID",
    teamsTenant: "Tenant (ідентифікатор каталогу або «organizations»)",
    teamsConnect: "Підключитися до Teams",
    teamsDisconnect: "Відключити",
    teamsConnected: (account: string | null) => (account ? `Підключено як ${account}` : "Підключено"),
    teamsNotConnected: "Не підключено",
    teamsNotConfigured: "Спочатку введіть Application (client) ID.",
    teamsErrorLead: "Не вдалося підключитися до Teams:",
    teamsPendingLead: "Відкрийте сторінку входу Microsoft і введіть код:",
    teamsOpenLink: "Відкрити сторінку входу",
    teamsCopyCode: "Копіювати код",
    teamsCopied: "Скопійовано",
    teamsCodeExpires: (min: number) =>
      `Код дійсний ще приблизно ${min} ${plural(min, "хвилину", "хвилини", "хвилин")}.`,
    teamsHelpTitle: "Як отримати Application ID?",
    teamsHelp: [
      "Відкрийте портал Azure → Microsoft Entra ID → «Реєстрації програм» (App registrations) → «Нова реєстрація» (New registration).",
      "Тип облікового запису: облікові записи в будь-якому каталозі організації. Платформа: «Мобільні та настільні програми» (Mobile and desktop applications).",
      "У розділі «Автентифікація» (Authentication) увімкніть «Дозволити загальнодоступні клієнтські потоки» (Allow public client flows).",
      "У розділі «Дозволи API» (API permissions) додайте делеговані дозволи Microsoft Graph: Presence.ReadWrite і User.Read, а для календаря — ще Calendars.Read.",
      "Скопіюйте сюди «Application (client) ID». Якщо потрібна згода адміністратора, попросіть його її надати.",
    ],

    sites: "Блокувати сайти, що відволікають",
    sitesHint:
      "Під час фокусу розширення браузера замінює сайти зі списку нижче на сторінку «Повертаймося до роботи». Нічого не надсилається в інтернет.",
    extensionTitle: "Розширення браузера (Chrome, Edge, Brave, Opera, Vivaldi)",
    extensionStep1: "Відкрийте папку розширення:",
    extensionOpenDir: "Відкрити папку розширення",
    extensionStep2:
      "У браузері перейдіть на chrome://extensions (Edge: edge://extensions, Opera: opera://extensions) і ввімкніть «Режим розробника».",
    extensionStep3: "Натисніть «Завантажити розпаковане розширення» й виберіть відкриту папку.",
    extensionConnected: "Розширення підключено",
    extensionDisconnected: "Розширення не підключено — використовую резервне виявлення за заголовком вікна",
    extensionSeen: (ago: string) => `останній зв’язок ${ago} тому`,

    keepAwake: "Не вимикати екран",
    keepAwakeHint: "Комп’ютер і екран не перейдуть у режим сну під час фокусу.",

    historyTitle: "Історія",
    trackApps: "Записувати, якими застосунками ви користуєтеся під час фокусу",
    trackAppsHint:
      "Щосекунди FocusMode перевіряє назву процесу активного вікна. Панель завдань, меню «Пуск» та інші елементи робочого столу Windows пропускаються. Дані залишаються на цьому комп’ютері.",
    trackTitles: "Записувати заголовки вікон",
    trackTitlesHint:
      "Для кожного застосунку FocusMode запам’ятовує кілька найчастіших заголовків вікон (наприклад, вкладку браузера або папку) і показує їх, коли ви наводите курсор на діаграму в «Підсумку». Вони залишаються тільки на цьому комп’ютері й ніколи не потрапляють у журнали.",
    trackTitlesNeedsApps: "Працює, коли записування застосунків увімкнено.",
    keepHistory: "Зберігати історію сесій",
    keepHistoryHint:
      "Якщо вимкнути, підсумок усе одно з’явиться після «Стоп», але сесію не буде збережено. Раніше збережені сесії залишаться, доки ви їх не видалите.",
    clearHistory: "Видалити всю історію",
    clearHistoryHint: "Видаляє з цього комп’ютера всі збережені сесії та останній підсумок.",
    clearHistoryTitle: "Видалити всю історію?",
    clearHistoryBody:
      "Усі збережені сесії, діаграми застосунків і заголовки вікон буде остаточно видалено з цього комп’ютера. Цю дію не можна скасувати.",
    clearHistoryConfirm: "Так, видалити історію",
    historyCleared: "Історію видалено.",

    appearanceTitle: "Вигляд",

    soundTitle: "Звук",
    chimeSound: "Звук сигналу",
    chimeSounds: {
      midnight: "Midnight — м’який дзвін",
      vaporwave: "Vaporwave — синтезаторний акорд",
      lunar: "Lunar — скляні тони",
      autumn: "Autumn — дерев’яна марімба",
      classic: "Класичний — дінь-дон",
      soft: "Ніжний — м’який тон",
      chiptune: "8-bit — арпеджіо",
      custom: "Власний…",
    },
    chimeSoundHint: "Назви збігаються з темами оформлення, але звук можна вибрати незалежно від теми.",
    customChimes: "Власні звуки",
    customBreak: "Початок перерви",
    customFocus: "Початок фокусу (необов’язково)",
    customNone: "Файл не вибрано",
    customDefaultFallback: "Без власного файлу звучатиме Midnight.",
    customBreakUsesFocus: "Без окремого файлу звучатиме той самий звук, що й на початку фокусу.",
    customFocusUsesBreak: "Без окремого файлу звучатиме той самий звук, що й на початку перерви.",
    customPick: "Вибрати файл…",
    customChange: "Змінити файл…",
    customPickTitle: (label: string) => `Виберіть звук: ${label}`,
    customFilter: "Звук (mp3, wav, ogg, flac, m4a)",
    customRemove: "Видалити",
    customRemoveAria: (label: string) => `Видалити власний звук: ${label}`,
    customHint: "Файли mp3, wav, ogg, flac або m4a до 10 МБ. FocusMode копіює їх у свою папку даних.",
    customImported: "Власний звук додано.",
    chimeVolume: "Гучність сигналу",
    testChime: "Перевірити сигнал:",
    testBreak: "перерва",
    testFocus: "фокус",
    musicVolume: "Гучність музики",

    remindersTitle: "Нагадування на перервах",
    remindersLead:
      "Власний список для перерв у сесії, наприклад: перерва 1 — розтяжка, 2 — вода й прогулянка з собакою, 3 — обід. Нагадування з’явиться на екрані перерви та у спливному вікні FocusMode.",
    reminderLabel: (n: number) => `Перерва ${n}`,
    reminderPlaceholders: ["напр., розтяжка", "напр., вода й прогулянка з собакою", "напр., обід"],
    reminderPlaceholderOther: "Без нагадування",
    reminderAdd: "Додати перерву",
    reminderRemove: (n: number) => `Видалити нагадування для перерви ${n}`,
    reminderUp: (n: number) => `Перемістити нагадування для перерви ${n} вище`,
    reminderDown: (n: number) => `Перемістити нагадування для перерви ${n} нижче`,
    reminderChars: (used: number, max: number) => `${used}/${max}`,
    reminderCharsAria: (used: number, max: number) => `Символів: ${used} з ${max}`,
    reminderMoved: (n: number) => `Переміщено на позицію ${n}.`,
    reminderRemoved: "Нагадування видалено.",
    remindersEmpty: "Нагадувань немає — під час перерв ви побачите лише ідеї.",
    remindersFull: (max: number) =>
      `У списку вже ${max} ${plural(max, "пункт", "пункти", "пунктів")} — це максимум.`,
    remindersRepeat: "Повторювати список спочатку",
    remindersRepeatHint:
      "Після останнього пункту список починається знову. Без цього наступні перерви будуть без нагадувань.",
    remindersInvalid: (max: number, chars: number) =>
      `Не більше ніж ${max} ${plural(max, "нагадування", "нагадування", "нагадувань")}, кожне — до ${chars} ${plural(chars, "символу", "символів", "символів")}.`,

    systemTitle: "Система",
    launchOnLogin: "Запускати FocusMode під час входу в систему",
    launchOnLoginHint:
      "Після входу в систему FocusMode запускається згорнутим в область сповіщень (значок біля годинника).",

    developerTitle: "Розробник",
    developerMode: "Увімкнути режим розробника",
    developerModeHint:
      "Додає вкладку «Розробник» із кнопками, які викликають спливні вікна, сповіщення та реакції талісмана, — для тестів і знімків екрана. Тестові дані ніколи не зберігаються.",

    launchersTitle: "Лаунчери ігор",
    launchers: "Закривати лаунчери ігор на початку фокусу",
    launchersHint:
      "На початку кожного фокусу FocusMode закриває вибрані лаунчери. Закриваються лише лаунчери, а не самі ігри, — але гра, запущена через лаунчер, може втратити незбережений прогрес, тож спершу збережіть гру.",
    launchersPick: "Які лаунчери закривати",
    launchersNone: "Виберіть принаймні один лаунчер.",
    launchersLastClosed: (names: string) => `На початку останнього фокусу FocusMode закрив: ${names}.`,
    launchersLastNone: "На початку останнього фокусу жоден із вибраних лаунчерів не був запущений.",
    launchersUnavailable: "Закриття лаунчерів працює лише у Windows.",
    launchersError: "Не вдалося закрити один із лаунчерів.",

    windowTitle: "Вікно та спливні вікна",
    closeButton: "Кнопка X (закрити вікно)",
    closeAsk: "Запитати першого разу",
    closeTray: "Ховати в область сповіщень",
    closeQuit: "Закривати застосунок",
    closeHint:
      "Схований в області сповіщень, FocusMode далі відлічує час і грає музику. Відкрийте його значком біля годинника.",
    phasePopups: "Спливні вікна FocusMode під час зміни фази",
    phasePopupsHint: "Маленьке вікно в кутку екрана, коли починається фокус або перерва. Замінює сповіщення Windows.",

    tintTitle: "Теплі кольори екрана",
    tint: "Робити кольори екрана теплішими залежно від часу доби",
    tintHint:
      "Увечері кольори екрана поступово теплішають, а вранці повертаються звичайні. Теплий відтінок накладається вже після формування зображення, тож не потрапляє на знімки екрана чи записи.",
    tintMode: "Спосіб",
    tintModes: {
      nightLight: "Нічник Windows (рекомендовано)",
      gamma: "Безпосередньо на відеокарті (гамма)",
    },
    tintModeShort: {
      nightLight: "Нічник Windows",
      gamma: "гамма відеокарти",
    },
    tintModeHints: {
      nightLight:
        "Вбудований у Windows «Нічник». FocusMode вмикає його за розкладом і задає його інтенсивність — без прав адміністратора. У Windows 11 24H2 і новіших ним не можна керувати ззовні, тому FocusMode тоді робить кольори теплішими через гамму.",
      gamma:
        "Змінює криву кольорів кожного монітора, тож діє на весь екран, зокрема на панель завдань та повноекранні ігри. Не працює на екранах у режимі HDR, а Windows може не дозволити дуже теплі налаштування.",
    },
    tintFrom: "З",
    tintTo: "До",
    tintScheduleHint: "Проміжок може переходити через північ, наприклад з 18:00 до 9:00.",
    tintStrength: "Інтенсивність",
    tintStrengthHint: "Перетягніть повзунок, щоб побачити, як це виглядатиме на екрані.",
    tintTransition: "Плавний перехід",
    tintTransitionNone: "Без переходу",
    tintTransitionMinutes: (m: number) => `${m} хв`,
    tintStatusOff: "Зараз: вимкнено",
    tintStatusLevel: (pct: number) => `Зараз: ${pct}%`,
    tintStatusPreview: (pct: number) => `Попередній перегляд: ${pct}%`,
    tintStatusFallback: (mode: string) => `Вибраний спосіб недоступний — використовую: ${mode}.`,
    tintStatusCapped: "Windows не дозволяє повну інтенсивність — застосовано слабший теплий відтінок.",
    tintStatusError: "Не вдалося зробити кольори екрана теплішими — спробую ще раз трохи згодом.",
    tintStatusRestoreError: "Не вдалося повернути звичайні кольори екрана — спробую ще раз трохи згодом.",
    tintTimeInvalid: "Введіть час, наприклад 18:00.",
    tintSameTimes: "Час «До» має відрізнятися від «З».",
    tintStrengthRange: "Введіть інтенсивність від 0 до 100.",
    tintTransitionRange: "Плавний перехід може тривати від 0 до 180 хвилин.",

    languageTitle: "Мова",
    language: "Мова застосунку",
    languageSystem: (name: string | null) => (name ? `Системна (${name})` : "Системна"),

    demoNote: "Демо-режим: фокус триває 40 с, перерва — 20 с.",
  },

  closePrompt: {
    title: "Згорнути FocusMode в область сповіщень?",
    body: "Таймер і музика працюватимуть у фоні, а FocusMode покаже маленьке спливне вікно, коли почнеться фокус або перерва.",
    note: "Це можна змінити в налаштуваннях.",
    toTray: "Так, в область сповіщень",
    quit: "Ні, закрити застосунок",
  },

  popup: {
    title: {
      breakStarted: "Час на перерву",
      focusStarted: "Час для фокусу",
      interrupted: "Сесію призупинено",
      launchersClosed: "Лаунчери ігор закрито",
      meetingReminder: "Зустріч під час перерви",
      breakHeld: "Перерва чекає",
      workDigest: "Що далі?",
    },
    workDigest: (n: number) =>
      `${n} ${plural(n, "нове завдання чи сповіщення чекає", "нові завдання й сповіщення чекають", "нових завдань і сповіщень чекають")} на перерву`,
    workDigestGeneric: "Нові завдання й сповіщення чекають на перерву",
    meetingMessage: (range: string, subject: string) => `${range} · ${subject}`,
    meetingTitle: (n: number) => `Зустріч під час перерви ${n}`,
    meetingBreakAt: (range: string) => `Запланована перерва: ${range}`,
    meetingChoices: (subject: string) => `Що робити із зустріччю ${subject}?`,
    breakHeld: (subject: string) => `Зустріч: ${subject}. Таймер перерви чекає — продовжте його після зустрічі.`,
    breakHeldGeneric: "Триває зустріч. Таймер перерви чекає — продовжте його після зустрічі.",
    breakHeldTitle: (time: string) => `Перерва чекає · ${time}`,
    choiceFailed: "Не вдалося зберегти вибір — виберіть у вікні FocusMode.",
    breakTitle: (time: string) => `Час на перерву · ${time}`,
    breakMusicOn: "Музика грає далі — відійдіть ненадовго від столу.",
    breakMusicPaused: "Музика зачекає — відійдіть ненадовго від столу.",
    breakNoMusic: "Відійдіть ненадовго від столу.",
    reminderPrefix: "Ваше нагадування:",
    tipPrefix: (category: string) => `Ідея (${category}):`,
    focusMessage: (cycle: number, time: string) => `Цикл ${cycle}: ${time} фокусу. Щити ввімкнено.`,
    focusMessageNoShields: (cycle: number, time: string) => `Цикл ${cycle}: ${time} фокусу.`,
    interrupted: "Комп’ютер був у режимі сну — продовжте, коли повернетеся.",
    launchersClosed: (names: string) => `Закрито: ${names}`,
    showMain: "Показати вікно",
    skip: "Пропустити",
    skipBreak: "Пропустити перерву",
    skipFocus: "Пропустити фокус",
    dismiss: "Закрити",
  },

  blocklist: {
    title: "Список сайтів — websiteblacklist.txt",
    pathLabel: "Файл списку",
    loading: "Завантаження списку сайтів…",
    summary: (domains: number, patterns: number) =>
      `${domains} ${plural(domains, "домен", "домени", "доменів")} · ${patterns} ${plural(patterns, "регулярний вираз", "регулярні вирази", "регулярних виразів")}`,
    empty: "Список порожній — під час фокусу жоден сайт не блокуватиметься.",
    open: "Відкрити websiteblacklist.txt",
    reload: "Перезавантажити",
    reloaded: "Список сайтів перезавантажено.",
    autoReload: "Щойно ви збережете файл, список оновиться сам — нічого не треба перезапускати.",
    allValid: "Усі рядки правильні",
    issuesTitle: (n: number) =>
      `${n} ${plural(n, "рядок пропущено", "рядки пропущено", "рядків пропущено")} — виправте це у файлі або додайте # на початку рядка:`,
    issueLine: (line: number) => `Рядок ${line}`,
    issueText: {
      invalidDomain: "Неправильний домен",
      invalidRegex: "Неправильний регулярний вираз",
      regexTooComplex: "Регулярний вираз надто складний",
      matchesEverything: "Вираз збігається з будь-якою адресою",
      pastedUrl:
        "Адреса зі знаком ? — це регулярний вираз, який не збігається з цією адресою. Щоб заблокувати лише цю адресу, замініть ? на \\?",
      lineTooLong: "Рядок задовгий (ліміт: 500 символів)",
      tooManyDomains: "Забагато доменів (ліміт: 2000)",
      tooManyPatterns: "Забагато регулярних виразів (ліміт: 500)",
    },
    pastedUrl: (domain: string) =>
      `Адреса зі знаком ? — це регулярний вираз, який не збігається з цією адресою. Щоб заблокувати весь сайт, введіть ${domain}; щоб лише цю адресу — замініть ? на \\?`,
    issueDetailLabel: "Подробиці (англійською)",
    readError: "Не вдалося прочитати файл — діє попередній список:",
    helpTitle: "Як записувати рядки?",
    helpLines: "Один запис у рядку. Рядки, що починаються з #, — це коментарі; порожні рядки пропускаються.",
    helpDomain: "Звичайний домен блокує себе та всі свої піддомени (наприклад, www., m.).",
    helpRegex:
      "Регулярний вираз — це рядок у скісних рисках /…/ або рядок, що містить будь-який із символів \\ ^ $ * + ? ( ) [ ] { } |. Він порівнюється з усією адресою сторінки без урахування регістру. Крапка означає будь-який символ, тому в доменах пишіть \\.",
    examplesLabel: "Приклади:",
    examples: [
      { code: "reddit.com", note: "увесь reddit.com разом із піддоменами" },
      { code: "youtube\\.com/shorts", note: "лише YouTube Shorts; решта YouTube працює" },
      { code: "linkedin\\.com/feed", note: "лише стрічка LinkedIn" },
    ],
    extensionSkippedTitle:
      "Браузер не може використати ці вирази як правила — я перевіряю їх під час завантаження вкладки",
  },

  mascot: {
    setting: "Показувати талісман (аксолотля)",
    settingHint:
      "Аксолотль у навушниках біля годинника, під час перерви, у підсумку та в спливних вікнах під час зміни фази. Ділиться корисними звичками та ідеями для перерви.",
    practiceLabel: "Корисна звичка",
    nextUpLabel: "Що далі?",
    startLines: [
      "До роботи!",
      "Починаємо. Успіхів!",
      "Навушники вдягнено — працюємо!",
      "Одне завдання за раз. Поїхали!",
    ],
    breakLines: [
      "Перерва! Відійдіть ненадовго від екрана.",
      "Час на перерву. Заслужену!",
      "Перерва — розімніть ноги.",
    ],
    cycleLines: [
      "Цикл завершено! Повертаємося до зосередження.",
      "Чудово, ще один цикл позаду!",
      "Усе йде добре. Наступний раунд!",
    ],
    conflictHint: (subject: string) =>
      `Зустріч припадає на перерву: ${subject}. Вирішіть нижче, що з нею робити.`,
    tickle: "Полоскотати аксолотля",
    tickleLines: [
      "Ой, лоскотно!",
      "Хі-хі, перестаньте!",
      "Буп! Повертаймося до роботи?",
      "У мене лоскітні зябра!",
      "Погладжування зараховано. Фокус +1!",
      "Обережно, я м’якенький!",
      "Ще раз? Гаразд, востаннє!",
      "Ви знайшли, де я боюся лоскоту!",
    ],
  },

  calendar: {
    title: "Календарі",
    lead: "FocusMode перевіряє зустрічі на найближчі години й ще до початку сесії попереджає, якщо якась припадає на перерву. Календарі лише читаються — FocusMode нічого в них не змінює.",
    privacy:
      "Зустрічі залишаються на цьому комп’ютері. Токени входу й посилання на календарі зберігаються в зашифрованому вигляді, окремо від файлу налаштувань, і ніколи не потрапляють у журнали.",
    horizon: "Перевіряти зустрічі на найближчі",
    horizonOption: (h: number) => `${h} ${plural(h, "годину", "години", "годин")}`,
    horizonHint: "Для довгої сесії виберіть довший проміжок, щоб охопити всі її перерви.",
    lastSync: (time: string) => `Остання синхронізація: ${time}`,
    neverSynced: "Календарі ще не читалися.",
    refresh: "Оновити зараз",
    refreshing: "Оновлення…",
    sourceName: {
      microsoft: "Microsoft 365 / Outlook",
      google: "Google Календар",
      ics: "Календарі з посилання ICS",
    },
    sourceShort: {
      microsoft: "Outlook",
      google: "Google",
      ics: "ICS",
    },
    state: {
      off: "вимкнено",
      on: "підключено",
      unavailable: "недоступно",
      error: "помилка",
    },
    statusLine: (state: string, account: string | null) => (account ? `${state} · ${account}` : state),
    errorLead: "Не вдалося прочитати календар:",

    microsoft: "Microsoft 365 / Outlook (обліковий запис Microsoft із Teams)",
    microsoftHint:
      "Читає ваш календар через той самий обліковий запис Microsoft, що й інтеграція з Teams (Microsoft Graph, лише читання: Calendars.Read). Охоплює зустрічі Teams і всі інші події календаря.",
    microsoftReconnect:
      "FocusMode просить дозвіл на читання календаря (Calendars.Read) лише під час входу з увімкненим календарем. Обліковий запис, підключений раніше тільки для Teams, треба підключити ще раз — відключати не потрібно, статус у Teams і далі працюватиме. У реєстрації застосунку в Azure додайте дозвіл Calendars.Read.",
    microsoftTeamsAbove: "Обліковий запис Microsoft підключається вище, у щиті «Microsoft Teams: Не турбувати».",
    microsoftConnect: "Підключити обліковий запис Microsoft",
    microsoftNeedsReconnect:
      "Цей вхід Microsoft не має доступу до календаря. Підключіть обліковий запис ще раз і дозвольте читання календаря.",
    microsoftReconnectButton: "Підключити знову",
    microsoftReconnecting: "Підключення…",
    microsoftReconnectCode: (code: string) =>
      `Введіть код ${code} на сторінці входу Microsoft (вона відкрита в браузері; посилання є й вище, у щиті Teams).`,

    google: "Google Календар",
    googleHint:
      "Вхід відбувається в браузері, а FocusMode просить лише дозвіл на читання календаря. Потрібен власний безкоштовний ідентифікатор клієнта OAuth із Google Cloud.",
    googleClientId: "Ідентифікатор клієнта OAuth (Client ID)",
    googleClientIdPlaceholder: "123456789012-abc….apps.googleusercontent.com",
    googleClientIdInvalid:
      "Ідентифікатор клієнта Google закінчується на .apps.googleusercontent.com і містить лише літери, цифри, крапки, дефіси та підкреслення.",
    googleClientSecret: "Секрет клієнта (Client secret)",
    googleClientSecretPlaceholder: "GOCSPX-…",
    googleClientSecretHint:
      "З того самого клієнта типу «Застосунок для комп’ютера» (Desktop app) у Google Cloud, що й ідентифікатор вище. Google вимагає його для входу, але у встановлених застосунках не вважає секретним, тому FocusMode зберігає його у файлі налаштувань (але ніколи — у журналах).",
    googleClientSecretInvalid: (max: number) =>
      `Секрет клієнта містить лише літери, цифри, дефіси та підкреслення (не більше ніж ${max} ${plural(max, "символ", "символи", "символів")}).`,
    googleConnect: "Підключитися до Google",
    googleDisconnect: "Відключити",
    googlePending: "Завершіть вхід у вікні браузера, що відкрилося…",
    googleConnected: (account: string | null) => (account ? `Підключено як ${account}` : "Підключено"),
    googleNotConnected: "Не підключено",
    googleNeedsClientId: "Спочатку введіть ідентифікатор клієнта.",
    googleErrorLead: "Не вдалося підключитися до Google:",
    googleHelpTitle: "Як отримати ідентифікатор клієнта Google?",
    googleHelp: [
      "Відкрийте console.cloud.google.com і створіть проєкт (або виберіть наявний).",
      "У розділі «API та сервіси» → «Бібліотека» увімкніть Google Calendar API.",
      "Налаштуйте екран згоди OAuth (Google Auth Platform): аудиторія «Зовнішня» (External; або «Внутрішня» (Internal) у корпоративному Google Workspace), область доступу …/auth/calendar.readonly. Поки застосунок у режимі тестування, додайте свою адресу електронної пошти як тестового користувача.",
      "У розділі «Клієнти» (Clients, «Облікові дані») створіть ідентифікатор клієнта OAuth типу «Застосунок для комп’ютера» (Desktop app).",
      "Скопіюйте «Ідентифікатор клієнта» (закінчується на .apps.googleusercontent.com) і «Секрет клієнта» (починається з GOCSPX-) у поля вище й натисніть «Підключитися до Google».",
    ],

    ics: "Інші календарі (посилання ICS / iCal)",
    icsHint:
      "Для календарів із приватним посиланням для підписки: Outlook і Exchange на роботі, Google, iCloud, Proton, Zoho, Zimbra, Nextcloud та інші. Повторювані зустрічі теж враховуються.",
    icsLinks: "Посилання на календарі — по одному в рядку",
    icsPlaceholder: "https://outlook.office365.com/owa/calendar/…/calendar.ics\nwebcal://…",
    icsSave: "Зберегти посилання",
    icsSaved: "Посилання на календарі збережено.",
    icsEdit: "Змінити посилання",
    icsAdd: "Додати посилання",
    icsCancel: "Скасувати",
    icsRemoveAll: "Видалити всі посилання",
    icsRemoved: "Посилання на календарі видалено.",
    icsCount: (n: number) =>
      n === 0 ? "немає посилань" : `${n} ${plural(n, "посилання", "посилання", "посилань")}`,
    icsLoading: "Завантаження збережених посилань…",
    icsInvalidLines: (lines: readonly number[]) =>
      `${lines.length === 1 ? "Рядок" : "Рядки"} ${lines.join(", ")}: це не схоже на посилання на календар (https:// або webcal://).`,
    icsSecret:
      "Будь-хто, хто знає таке посилання, може бачити ваш календар. FocusMode зберігає його в зашифрованому вигляді й ніколи не показує в повідомленнях про помилки.",
    icsHelpTitle: "Де знайти посилання ICS?",
    icsHelp: [
      {
        name: "Outlook в інтернеті (Microsoft 365, Exchange)",
        text: "Параметри → Календар → Спільні календарі → «Опублікувати календар»: виберіть календар і «Може переглядати всі відомості», натисніть «Опублікувати» й скопіюйте посилання ICS. На корпоративному сервері Exchange публікування має ввімкнути адміністратор.",
      },
      {
        name: "Google Календар",
        text: "Налаштування → виберіть календар ліворуч → «Інтеграція календаря» → «Секретна адреса у форматі iCal».",
      },
      {
        name: "iCloud",
        text: "У Календарі відкрийте спільний доступ до календаря як до «Загальнодоступного календаря» й скопіюйте його посилання (webcal://).",
      },
      {
        name: "Proton, Zoho, Zimbra, Nextcloud та інші",
        text: "У налаштуваннях календаря шукайте «Поділитися», «Підписатися», «Посилання ICS» або «iCal».",
      },
    ],

    cardTitle: "Зустрічі під час перерв",
    leadPlanned: "Якщо почати зараз, ці зустрічі припадуть на перерви. Вирішіть, що робити:",
    leadRunning: "Ці зустрічі припадають на перерви цієї сесії:",
    noConflictsPlanned: "Якщо почати зараз, жодна зустріч не припаде на перерву.",
    noConflictsRunning: "Жодна зустріч не припадає на перерви цієї сесії.",
    breakInfo: (n: number, range: string) => `Перерва ${n}: ${range}`,
    choice: {
      extend: "Подовжити фокус (музику приглушено)",
      pauseAtBreak: "Призупинити на перерві",
      ignore: "Ігнорувати",
      remindLater: "Нагадати пізніше",
    },
    choiceExtend: (end: string) =>
      `Фокус триватиме до кінця зустрічі (${end}), музику буде приглушено. Перерва почнеться після зустрічі.`,
    choicePauseAtBreak: "Коли настане час перерви, таймер зупиниться й чекатиме, доки ви не натиснете «Продовжити».",
    choiceIgnore: "Сесія йде без змін.",
    choiceRemindLater: (time: string | null) =>
      time ? `Я нагадаю о ${time} — у маленькому спливному вікні FocusMode.` : "Я нагадаю ближче до зустрічі.",
    choiceNone: "Виберіть, що робити.",
    tooLong: "Зустріч довша за 4 год — фокус не буде подовжено",
    choiceGroup: (subject: string) => `Що робити із зустріччю ${subject}`,
    upcomingTitle: "Найближчі зустрічі",
    upcomingEmpty: (hours: number) =>
      `Протягом ${hours} ${plural(hours, "години", "годин", "годин")} зустрічей немає.`,
    now: "зараз",
    online: "онлайн",
    onlineAria: "онлайн-зустріч",
    tomorrow: "завтра",
    untitled: "(без назви)",
    connectHint: "Підключіть календар, і FocusMode перед сесією попередить вас про зустрічі під час перерв.",
    connectLink: "Налаштування календарів",
    sourceErrors: (names: string) => `Не вдалося прочитати: ${names}.`,
    sourceErrorsLink: "Подробиці в налаштуваннях",
    loading: "Завантаження календаря…",

    holdExtend: (time: string, subject: string) => `Фокус подовжено до ${time} — зустріч: ${subject}`,
    holdExtendHint: "Музику приглушено до кінця зустрічі, потім почнеться перерва.",
    holdPause: (subject: string) => `Перерву призупинено — зустріч: ${subject}`,
    holdPauseHint: (time: string) =>
      `Таймер перерви чекає. Натисніть «Продовжити», коли зустріч закінчиться (за планом о ${time}).`,
  },

  work: {
    title: "Робочі інструменти («Що далі?»)",
    lead: "Підключіть інструменти, у яких ви працюєте, і FocusMode підкаже, чим зайнятися в наступній сесії: спершу завдання, призначені вам, потім те, на що чекають інші (рев’ю, виправлення, згадки), а наостанок — решту сповіщень. Інструменти лише читаються — FocusMode нічого в них не змінює.",
    privacy:
      "Токени зберігаються в зашифрованому вигляді, окремо від файлу налаштувань, і ніколи не потрапляють у журнали. FocusMode підключається лише до ввімкнених вище інструментів; завдання та сповіщення залишаються на цьому комп’ютері й ніколи не надсилаються жодному ШІ чи іншому сервісу.",
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
        "Задачі (issues), призначені вам, пул-реквести, що чекають на ваше рев’ю або ваші виправлення, і — з класичним токеном — сповіщення.",
      gitlab:
        "Задачі, призначені вам, мерж-реквести на рев’ю та ваші мерж-реквести з невдалим CI. Працює з GitLab.com і з власним сервером GitLab.",
      jira: "Задачі, призначені вам, з активним спринтом, пріоритетом і терміном. Jira Cloud або Data Center.",
      azureDevops:
        "Робочі елементи (work items), призначені вам (з поточною ітерацією), і пул-реквести, що чекають на ваше рев’ю.",
      linear: "Задачі, призначені вам, з поточним циклом, пріоритетом і терміном.",
    },
    state: {
      off: "вимкнено",
      on: "підключено",
      unavailable: "не підключено",
      error: "помилка",
    },
    connectedAs: (account: string | null) => (account ? `Підключено як ${account}` : "Підключено"),
    syncing: "Токен збережено — перше читання…",
    notConnected: "Не підключено — збережіть токен нижче.",
    noSecretStore: "Ця система не може безпечно зберегти токен, тому цей інструмент не працює.",
    tokenInvalid: "Токен може містити лише видимі символи ASCII, без пробілів (не більше ніж 4 КБ).",
    needsUrl: {
      gitlab: "Спочатку введіть адресу GitLab.",
      jira: "Спочатку введіть адресу Jira.",
      azureDevops: "Спочатку введіть назву організації.",
    },
    errorLead: "Не вдалося прочитати:",

    gitlabUrl: "Адреса GitLab",
    gitlabUrlPlaceholder: "https://gitlab.com",
    gitlabUrlHint: "Для власного сервера введіть його адресу, наприклад https://gitlab.example.com.",
    jiraUrl: "Адреса Jira",
    jiraUrlPlaceholder: "https://your-company.atlassian.net",
    jiraUrlHint:
      "Адреса вашого сайту Jira Cloud або сервера Jira Data Center. Можна вставити адресу з браузера без частини після ?.",
    jiraEmail: "Електронна пошта облікового запису Atlassian (лише Jira Cloud)",
    jiraEmailPlaceholder: "olena.kovalenko@example.com",
    jiraEmailHint: "Для Jira Data Center з особистим токеном доступу залиште порожнім.",
    azureOrg: "Організація Azure DevOps",
    azureOrgPlaceholder: "your-company",
    azureOrgHint: "Назва з адреси https://dev.azure.com/<назва> — можна вставити й усю адресу.",
    urlInvalid:
      "Введіть адресу, що починається з https://, наприклад https://your-company.atlassian.net, — без логіна, пароля, ? і #.",
    emailInvalid:
      "Введіть адресу електронної пошти облікового запису, наприклад olena@example.com (без пробілів і двокрапок), або залиште порожнім.",
    azureOrgInvalid: "Введіть назву організації: літери, цифри, дефіси, крапки й підкреслення (до 64 символів).",

    token: (name: string) => `Токен доступу ${name}`,
    tokenPlaceholder: "Вставте токен",
    tokenPlaceholderReplace: "Вставте новий токен, щоб замінити збережений",
    tokenHint: "Токен зберігається в зашифрованому вигляді й ніколи не повертається в це поле.",
    tokenSave: "Зберегти токен",
    tokenSaving: "Збереження…",
    tokenRemove: "Видалити токен",
    tokenSaved: (name: string) => `Токен ${name} збережено.`,
    tokenRemoved: (name: string) => `Токен ${name} видалено.`,
    helpTitle: "Як створити токен лише для читання?",
    help: {
      github: [
        "На GitHub: Settings → Developer settings → Personal access tokens → Fine-grained tokens → Generate new token.",
        "Resource owner: ваш обліковий запис або організація, у якій ви працюєте (fine-grained токен бачить репозиторії лише одного власника). Repository access: All repositories або вибрані репозиторії.",
        "Repository permissions, кожен лише «Read-only»: Issues, Pull requests і Metadata (Metadata GitHub додає сам). Більше нічого не вибирайте. Установіть термін дії.",
      ],
      githubNotifications:
        "Сповіщення GitHub (згадки, запити на рев’ю поза репозиторіями токена) і результати GitHub Actions доступні через API лише класичним токенам — fine-grained токен їх не прочитає. Якщо хочете їх бачити, створіть натомість класичний токен (Tokens (classic)) з областю notifications. Для приватних репозиторіїв класичному токену потрібна ще область repo, яка дає й право запису, — FocusMode ним не користується, тому за замовчуванням ми радимо fine-grained токен.",
      gitlab: [
        "У GitLab: аватар → Редагувати профіль → Токени доступу (Personal access tokens) → Додати новий токен (у новіших версіях: «Legacy token»).",
        "Виберіть лише область read_api (доступ до API на читання). Установіть термін дії.",
        "Для власного сервера GitLab введіть його адресу вище.",
      ],
      jira: [
        "Jira Cloud: перейдіть на id.atlassian.com/manage-profile/security/api-tokens і виберіть «Create API token with scopes», застосунок Jira та лише області read:jira-work і read:jira-user. Установіть короткий термін дії.",
        "Вище введіть адресу сайту й електронну пошту облікового запису Atlassian. Токен з областями працює лише через шлюз api.atlassian.com — FocusMode вибере його сам. Звичайний токен без областей теж підійде, але він має всі дозволи вашого облікового запису (FocusMode лише читає задачі).",
        "Jira Data Center: профіль → Personal Access Tokens → Create token, з терміном дії. Поле електронної пошти залиште порожнім.",
      ],
      azureDevops: [
        "На dev.azure.com/<організація>: User settings (значок біля аватара) → Personal access tokens → New Token.",
        "Organization: ваша організація. Scopes: Custom defined → Work Items: Read, Code: Read і Project and Team: Read — нічого більше. Установіть термін дії.",
      ],
      linear: [
        "У Linear: Settings → Security & access → Personal API keys → New API key.",
        "Permissions: «Only select permissions» → Read only. Team access: усі команди або вибрані.",
      ],
    },

    holdDuringFocus: "Притримувати сповіщення під час фокусу",
    holdDuringFocusHint:
      "Нові завдання й сповіщення, що з’явилися під час фокусу, FocusMode покаже лише на перерві або перед наступною сесією — ніщо не перериває фокус.",
    lastSync: (time: string) => `Остання синхронізація: ${time}`,
    neverSynced: "Інструменти ще не читалися.",
    refresh: "Оновити зараз",
    refreshing: "Оновлення…",

    cardTitle: "Що далі?",
    cardLead: "Спершу те, що призначено вам, потім те, на що чекають інші.",
    loading: "Завантаження завдань…",
    connectHint:
      "Підключіть GitHub, GitLab, Jira, Azure DevOps або Linear, і FocusMode підкаже, чим зайнятися в цій сесії.",
    connectLink: "Налаштування робочих інструментів",
    noTokenHint: "Збережіть токен у налаштуваннях, і тут з’являться ваші завдання.",
    empty: "На вас нічого не чекає — немає призначених завдань, запитів на рев’ю чи сповіщень.",
    sourceErrors: (names: string) => `Не вдалося прочитати: ${names}.`,
    sourceErrorsLink: "Подробиці в налаштуваннях",
    held: (n: number) => `${n} ${plural(n, "нове", "нові", "нових")} за час фокусу`,
    newCount: (n: number) => `${n} ${plural(n, "нове", "нові", "нових")}`,
    unseen: "нове",
    showMore: (n: number) => `Показати більше (${n})`,
    showLess: "Показати менше",
    open: "Відкрити",
    openAria: (name: string) => `Відкрити ${name} у браузері`,
    setGoal: "Зробити ціллю",
    setGoalAria: (name: string) => `Зробити ${name} ціллю сесії`,
    isGoal: "Ціль сесії",
    due: (date: string) => `термін: ${date}`,
    reasonsLabel: "Чому зараз",
    reason: {
      assigned: "призначено вам",
      inSprint: "спринт",
      highPriority: "високий пріоритет",
      dueToday: "термін сьогодні",
      overdue: "прострочено",
      inProgress: "у роботі",
      blockingOthers: "чекає на ваше рев’ю",
      waitingLong: "давно чекає",
      mentioned: "згадка",
    },
    kind: {
      assigned: "завдання",
      reviewRequested: "рев’ю",
      changesRequested: "потрібні зміни",
      ciFailed: "CI не проходить",
      mention: "згадка",
      notification: "сповіщення",
    },
    itemName: (key: string, title: string) => (key && title ? `${key} ${title}` : key || title),

    goalLabel: "Ціль сесії:",
    goalOpenAria: (name: string) => `Ціль сесії: ${name}. Відкрити в браузері`,
    goalClear: "Очистити ціль сесії",
  },

  nudge: {
    title: "Повертаймося до роботи",
    body: (domain: string) => `${domain} зачекає до перерви.`,
    bodyPattern: "Цей сайт є в списку блокування й зачекає до перерви.",
    dismiss: "Закрити",
  },

  toasts: {
    region: "Сповіщення",
    dismiss: "Закрити сповіщення",
  },

  errors: {
    generic: "Щось пішло не так.",
    fieldInvalid: (label: string) => `Неправильне значення: ${label}.`,
    fixConfigFirst: "Виправте позначені налаштування, перш ніж почати сесію.",
    unplayable: "Це посилання не можна відтворити в застосунку",
    allUnplayable: "Нічого з вибраної музики не вдається відтворити.",
    autoplayBlocked: "Відтворення заблоковано — натисніть «Наступний», щоб спробувати ще раз.",
    youtubeApi: "Не вдалося завантажити програвач YouTube. Перевірте підключення до інтернету.",
    clipboard: "Не вдалося скопіювати в буфер обміну.",
    codes: {
      "config.invalid": "Неправильне налаштування.",
      "config.corrupt":
        "Файл налаштувань був пошкоджений — відновлено налаштування за замовчуванням (резервна копія: config.json.bak).",
      "config.read": "Не вдалося прочитати файл налаштувань — використовуються налаштування за замовчуванням.",
      "config.save": "Не вдалося зберегти налаштування на диск — зміни діятимуть до закриття застосунку.",
      "audio.device": "Не вдається відтворити сигнал — перевірте аудіопристрій.",
      "shields.notifications": "Не вдалося перемкнути режим «Не турбувати» у Windows.",
      "shields.teams": "Не вдалося змінити ваш статус у Teams.",
      "shields.keepAwake": "Не вдалося перемкнути «Не вимикати екран».",
      "bridge.port": "Порт мосту розширення зайнятий — блокування сайтів працює лише в резервному режимі.",
      "teams.auth": "Не вдалося ввійти в Teams.",
      "teams.network": "Немає зв’язку зі службами Microsoft — спробуйте ще раз трохи згодом.",
      "teams.notConfigured": "Спочатку введіть Application (client) ID.",
      "teams.unsupported": "Інтеграція з Teams недоступна в цій системі.",
      "teams.disconnect": "Не вдалося відключити обліковий запис Teams.",
      "teams.unavailable": "Інтеграція з Teams не відповідає — перезапустіть застосунок.",
      "music.folder": "Не вдається прочитати папку з музикою.",
      "media.url": "Це посилання не відкривається у вікні YouTube — підходять лише адреси YouTube або YouTube Music.",
      "media.window": "Не вдалося відкрити вікно YouTube або керувати ним.",
      "persistence.history": "Проблема з історією сесій (читання або збереження).",
      "extension.missing": "Папку розширення браузера не знайдено.",
      "extension.open": "Не вдалося відкрити папку розширення.",
      "blocklist.read": "Не вдалося прочитати websiteblacklist.txt — діє попередній список сайтів.",
      "blocklist.open":
        "Не вдалося відкрити websiteblacklist.txt у текстовому редакторі — відкрийте файл самостійно (шлях є в налаштуваннях).",
      "shields.launchers": "Не вдалося закрити один із лаунчерів ігор — сесія триває.",
      "tint.apply": "Не вдалося зробити кольори екрана теплішими — спробую ще раз трохи згодом.",
      "tint.restore": "Не вдалося повернути звичайні кольори екрана після попереднього запуску.",
      "chime.format": "Цей формат не підтримується — виберіть файл mp3, wav, ogg, flac або m4a.",
      "chime.read": "Не вдалося прочитати вибраний файл.",
      "chime.tooLarge": "Файл завеликий — сигнал може мати щонайбільше 10 МБ.",
      "chime.save": "Не вдалося зберегти звук у папці даних FocusMode.",
      "chime.decode": "Цей файл не вдається відтворити як звук — виберіть інший.",
      "music.library": "Не вдалося завантажити вбудовану музику.",
      "popup.window": "Не вдалося показати спливне вікно FocusMode.",
      "calendar.sync": "Не вдалося прочитати календар — подробиці в «Налаштування» → «Календарі».",
      "calendar.unavailable": "Календарі не відповідають — перезапустіть застосунок.",
      "calendar.unknownMeeting": "Цієї зустрічі вже немає в календарі.",
      "calendar.remindLater":
        "Для нагадування вже запізно — зустріч почнеться через 10 хвилин або раніше. Виберіть інший варіант.",
      "calendar.meetingTooLong":
        "Зустріч, довша за 4 год, не подовжує фокус і не призупиняє перерву. Виберіть інший варіант.",
      "calendar.googleNotConfigured": "Спочатку введіть ідентифікатор клієнта Google.",
      "calendar.googleAuth": "Не вдалося почати вхід у Google.",
      "calendar.disconnect": "Не вдалося видалити збережені дані календаря.",
      "calendar.icsUrl": "Одне з посилань не є правильним посиланням на календар (https:// або webcal://).",
      "calendar.icsSave": "Не вдалося безпечно зберегти посилання на календарі.",
      "work.sync": "Не вдалося прочитати робочий інструмент — подробиці в «Налаштування» → «Робочі інструменти».",
      "work.unavailable": "Робочі інструменти не відповідають — перезапустіть застосунок.",
      "work.token":
        "Токен може містити лише видимі символи ASCII, без пробілів (не більше ніж 4 КБ), — вставте його ще раз.",
      "work.tokenSave": "Не вдалося безпечно зберегти або видалити токен.",
      autostart: "Не вдалося змінити запуск під час входу в систему.",
      "session.unavailable": "Сесія не відповідає — перезапустіть застосунок.",
      "ipc.unavailable": "Ця функція працює лише в застосунку FocusMode.",
      "ipc.failed": "Застосунок відповів некоректно.",
    },
    areas: {
      config: "Проблема з налаштуваннями.",
      audio: "Проблема зі звуком.",
      shields: "Один із щитів не спрацював — сесія триває.",
      teams: "Проблема з підключенням до Teams.",
      bridge: "Проблема з мостом розширення браузера.",
      blocklist: "Проблема зі списком заблокованих сайтів (websiteblacklist.txt).",
      music: "Проблема з музикою.",
      media: "Проблема з вікном YouTube.",
      persistence: "Не вдалося зберегти дані.",
      session: "Проблема із сесією.",
      tracker: "Проблема з відстеженням застосунків.",
      tint: "Проблема з теплими кольорами екрана.",
      chime: "Проблема з власним звуком сигналу.",
      popup: "Проблема зі спливним вікном FocusMode.",
      launchers: "Проблема із закриттям лаунчерів ігор.",
      window: "Проблема з вікном застосунку.",
      calendar: "Проблема з календарем — сесія триває.",
      google: "Проблема з Google Календарем.",
      ics: "Проблема з календарем із посилання ICS.",
      work: "Проблема з робочими інструментами («Що далі?») — сесія триває.",
    },
  },

  fields: {
    focusMinutes: "тривалість фокусу",
    breakMinutes: "тривалість перерви",
    chimeVolume: "гучність сигналу",
    "music.volume": "гучність музики",
    "music.links": "посилання",
    "music.streamUrl": "адреса трансляції",
    "music.folder": "папка з музикою",
    "shields.teamsClientId": "Application (client) ID",
    "shields.teamsTenant": "tenant",
    "shields.launchers": "лаунчери ігор",
    "tint.startMinutes": "початок теплих кольорів",
    "tint.endMinutes": "кінець теплих кольорів",
    "tint.strength": "інтенсивність теплих кольорів",
    "tint.transitionMinutes": "плавний перехід",
    "tint.mode": "спосіб зміни кольорів",
    language: "мова",
    theme: "тема",
    chimeSound: "звук сигналу",
    "customChimes.breakFile": "звук початку перерви",
    "customChimes.focusFile": "звук початку фокусу",
    breakReminders: "нагадування на перервах",
    "calendar.horizonHours": "проміжок перевірки календаря",
    "calendar.googleClientId": "ідентифікатор клієнта Google",
    "calendar.googleClientSecret": "секрет клієнта Google",
    "work.gitlabUrl": "адреса GitLab",
    "work.jiraUrl": "адреса Jira",
    "work.jiraEmail": "електронна пошта облікового запису Jira",
    "work.azureOrg": "організація Azure DevOps",
  },

  dialog: {
    cancel: "Скасувати",
  },

  launchers: {
    steam: "Steam",
    epic: "Epic Games Launcher",
    gog: "GOG Galaxy",
    ubisoft: "Ubisoft Connect",
    ea: "Застосунок EA",
    battleNet: "Battle.net",
    riot: "Riot Client",
    xbox: "Застосунок Xbox",
    amazon: "Amazon Games",
    itch: "itch",
  },

  themes: {
    legend: "Тема застосунку",
    hint: "Кольори й тло головного вікна та спливних вікон фаз. Зміни зберігаються одразу.",
    selected: "вибрана",
    names: {
      midnight: "Опівніч",
      vaporwave: "Vaporwave",
      lunar: "Фази Місяця",
      autumn: "Затишна осінь",
      dark: "Темна",
      light: "Світла",
      pixel: "8-bit",
    },
    descriptions: {
      midnight: "Тепла фіолетова ніч із дощем за вікном. За замовчуванням.",
      vaporwave: "Рожевий, бірюзовий і фіолетовий: смугасте сонце над сіткою в перспективі.",
      lunar: "Срібло й індиго під зорями.",
      autumn: "Коричневі й бурштинові відтінки, листя повільно падає.",
      dark: "Нейтральний сірий, без декорацій у тлі.",
      light: "Світле тло й темний висококонтрастний текст, без декорацій.",
      pixel: "Кутасті рамки, чіткі тіні й піксельні зорі.",
    },
    moonToday: (phase: string, litPct: number) =>
      `Місяць у тлі показує сьогоднішню фазу: ${phase} (освітлено ${litPct}%).`,
    moonPhases: {
      new: "молодик",
      waxingCrescent: "зростаючий серп",
      firstQuarter: "перша чверть",
      waxingGibbous: "зростаючий місяць",
      full: "повня",
      waningGibbous: "спадний місяць",
      lastQuarter: "остання чверть",
      waningCrescent: "спадний серп",
    },
  },

  /** Break suggestions by id (category, length and source in `$lib/tips`). Suggestions only, never health promises. */
  tips: {
    "eyes-20-20-20": "Подивіться у вікно на щось щонайменше за 6 м і затримайте там погляд на 20 секунд.",
    "eyes-green-view": "Знайдіть за вікном дерево, траву або клаптик неба й подивіться на них 40 секунд.",
    "eyes-slow-blinks": "Повільно заплющте очі, порахуйте до двох і розплющте. Повторіть 10 разів у спокійному темпі.",
    "eyes-look-around":
      "Подивіться вгору, вниз, ліворуч і праворуч, а потім повільно зробіть очима два кола в кожен бік.",
    "eyes-near-far":
      "10 секунд дивіться на великий палець витягнутої руки, потім 10 секунд — на точку за вікном. Повторіть 5 разів.",
    "eyes-screen-free": "Проведіть цю перерву без екранів — покладіть і телефон екраном донизу.",
    "stretch-shoulder-rolls": "Повільно зробіть 5 кругових рухів плечима назад, а потім 5 — вперед.",
    "stretch-shoulder-shrug":
      "Підніміть плечі до вух, затримайтеся на 3–5 секунд і вільно опустіть. Повторіть 3 рази.",
    "stretch-neck-turn":
      "Сядьте рівно й повільно поверніть голову ліворуч, наскільки зручно. Затримайтеся на 5 секунд, потім праворуч. 3 рази.",
    "stretch-neck-tilt":
      "Повільно нахиліть голову вухом до плеча й затримайтеся на 10 секунд, якщо зручно. Потім в інший бік.",
    "stretch-chin-glide":
      "Сидячи рівно, посуньте голову прямо назад, не піднімаючи підборіддя. Затримайтеся на 5 секунд, повторіть 5 разів.",
    "stretch-chest-open":
      "Сядьте рівно, не спираючись на спинку, зведіть лопатки й широко розведіть руки. Затримайтеся на 10 с, 3 рази.",
    "stretch-upper-twist":
      "Схрестіть руки на грудях і поверніть тулуб ліворуч, наскільки зручно. Затримайтеся на 5 с, потім праворуч. 5 разів.",
    "stretch-side-reach":
      "Зчепіть пальці над головою, потягніться вгору, а потім нахиліться ліворуч і праворуч — по 10 секунд, якщо зручно.",
    "stretch-wrists":
      "Витягніть руку й м’яко зігніть зап’ястя вгору, потім униз, по 5 секунд. 3 рази, потім інша рука.",
    "stretch-fingers": "Стисніть кулаки, а потім широко розчепірте пальці. Повторіть 5 разів, не поспішаючи.",
    "stretch-prayer":
      "Складіть долоні перед собою й повільно опускайте зап’ястя до легкого розтягнення, не до болю. 3 рази.",
    "stretch-ankles":
      "Сидячи, випряміть ногу над підлогою й тягніть пальці стопи то від себе, то до себе. По 5 разів кожною ногою.",
    "stretch-standing-back":
      "Станьте, злегка зігнувши коліна, покладіть долоні на поперек і м’яко відхиліться назад, наскільки зручно. 3 рази.",
    "stretch-routine":
      "Зробіть спокійний 10-хвилинний комплекс: плечі, шия, повороти тулуба, зап’ястя й щиколотки — у зручному діапазоні.",
    "move-walk-5": "Встаньте й пройдіться 5 хвилин — квартирою, коридором або до кухні й назад.",
    "move-stairs":
      "Підніміться сходами на один поверх і спустіться у своєму темпі. Немає сходів? Пройдіться коридором.",
    "move-march-seated":
      "Сядьте рівно й по черзі піднімайте коліна, ніби крокуєте на місці, наскільки зручно, — по 5 разів кожною ногою.",
    "move-march-standing": "Помаршируйте на місці хвилину, вільно розмахуючи руками.",
    "move-calf-raises": "Станьте біля столу, обіпріться на нього долонями й 10 разів повільно підніміться на носки.",
    "move-sit-to-stand": "На стійкому стільці без коліщат встаньте й сядьте 10 разів у спокійному темпі.",
    "move-one-song": "Увімкніть одну улюблену пісню й рухайтеся під неї як завгодно — стоячи чи сидячи.",
    "move-chores": "Зробіть щось активне вдома: розвісьте білизну, полийте квіти або винесіть сміття.",
    "hydrate-glass": "Випийте склянку води — спокійно, маленькими ковтками.",
    "hydrate-refill": "Пройдіться до кухні й наповніть пляшку або чашку водою на наступну сесію.",
    "hydrate-in-reach": "Перш ніж повернутися до роботи, поставте склянку води так, щоб легко дотягнутися.",
    "breathe-count-5":
      "Сядьте зручно. Вдихайте носом на рахунок 5, видихайте ротом на рахунок 5. Продовжуйте 5 хвилин.",
    "breathe-belly-hand":
      "Покладіть долоню на живіт і протягом хвилини стежте, як він піднімається на вдиху й опускається на видиху.",
    "breathe-long-exhale":
      "Вдихніть носом, наприкінці додайте ще короткий ковток повітря, а потім повільно видихніть ротом. 5 разів.",
    "breathe-open-window": "Відчиніть вікно й зробіть 5 спокійних вдихів: вдих на рахунок чотири, видих — на шість.",
    "breathe-muscle-release":
      "Напружте м’язи стоп на 5 секунд, потім розслабте. Так поступово рухайтеся вгору тілом — до плечей і обличчя.",
    "posture-reset":
      "Перш ніж повернутися: спина на спинці стільця, стопи рівно на підлозі, руки вільно вздовж тіла.",
    "posture-screen-check":
      "Перевірте монітор: верхній край на рівні очей або трохи нижче, екран — приблизно на відстані витягнутої руки.",
    "posture-glare-check":
      "Перевірте, чи не відбивається в екрані вікно або лампа. Найкраще, коли вікно збоку від монітора.",
    "posture-shift": "Сядьте на хвилинку інакше: посуньтеся на стільці, змініть положення ніг, опустіть плечі.",
    "outside-step-out":
      "Вийдіть на балкон, перед будинок або станьте біля відчиненого вікна й побудьте там 2 хвилини.",
    "outside-sky": "Вийдіть надвір і хвилину подивіться на небо й хмари.",
    "outside-walk-15":
      "Вирушайте на 15-хвилинну прогулянку, бажано туди, де є дерева чи трава. Ідіть спокійним кроком.",
    "outside-walk-noticing":
      "Вийдіть на прогулянку без телефона в руці й рахуйте дорогою дерева, птахів або собак.",
    "social-chat": "Поговоріть трохи з кимось про щось, що зовсім не стосується роботи.",
    "social-message": "Напишіть коротке повідомлення близькій людині — просто щоб привітатися.",
    "social-walk-together": "Запросіть когось на коротку спільну прогулянку, хоча б навколо будинку.",
    "social-pet": "Маєте домашнього улюбленця? Приділіть йому кілька хвилин повної уваги.",
    "mind-do-nothing": "Сядьте зручно, заплющте очі й 2 хвилини просто нічого не робіть.",
    "mind-one-song": "Уважно послухайте одну пісню із заплющеними очима, нічого більше не роблячи.",
    "mind-5-4-3": "Назвіть подумки 5 речей, які бачите, 4 — які чуєте, і 3 — яких можете торкнутися.",
    "mind-wander": "Подивіться у вікно й дозвольте думкам вільно блукати 3 хвилини.",
    "mind-park-the-task":
      "Запишіть на папері одне речення: з чого почнете після перерви. Потім відкладіть цю тему.",
    "mind-three-good": "Згадайте три дрібниці, які сьогодні вдалися.",
    "mind-meal-away":
      "Поїжте подалі від робочого місця й екрана — за обіднім столом, на лавці або в компанії.",
    "mind-your-choice": "Зробіть те, чого вам справді хочеться, — ця перерва ваша.",
    "mind-lie-down": "Якщо можете, полежте 10–15 хвилин із заплющеними очима. Поставте будильник.",
    "tidy-desk":
      "Приберіть на столі: віднесіть чашки, зберіть розкидані папери й залиште лише те, що потрібно зараз.",
    "tidy-air": "Відчиніть вікно навстіж на кілька хвилин і провітріть кімнату.",
    "tidy-light": "Налаштуйте світло: засуньте штору, якщо сліпить, або увімкніть лампу, якщо стемніло.",
  },

  /** Good working practices by id (one a day on the idle Focus page). */
  workingTips: {
    "gp-screen-position":
      "Поставте монітор прямо перед собою, приблизно за 50–70 см від очей, верхнім краєм на рівні очей або трохи нижче.",
    "gp-light-and-glare": "Ставте екран боком до вікна й підбирайте його яскравість до освітлення в кімнаті.",
    "gp-sitting":
      "Спина на спинці стільця, стопи рівно на підлозі, лікті біля тулуба, зап’ястя на одній лінії з передпліччями.",
    "gp-short-often":
      "Робіть короткі перерви часто, ще до того, як відчуєте втому: кілька хвилин щогодини краще, ніж одна довга перерва раз на дві години.",
    "gp-20-20-20":
      "Під час фокусу приблизно кожні 20 хвилин відводьте погляд від екрана й 20 секунд дивіться вдалину. Не забувайте кліпати.",
    "gp-change-position": "Змінюйте положення: хоча б раз на годину вставайте з-за столу, навіть ненадовго.",
    "gp-batch-messages":
      "Перевіряйте пошту й месенджери у визначений час, наприклад на перервах, а не після кожного сповіщення.",
    "gp-single-task": "Виберіть на сесію одне завдання й закрийте вкладки та вікна, які для нього не потрібні.",
    "gp-phone-away": "На час фокусу вимкніть звук на телефоні й покладіть його так, щоб не бачити.",
    "gp-water-in-reach": "Тримайте воду під рукою й потроху пийте її протягом дня.",
  },

  /** Shown under every stretch suggestion. */
  stretchFootnote:
    "Рухайтеся в зручному діапазоні: розтягнення має відчуватися, але без болю. Якщо щось болить, пропустіть цю вправу.",

  time: {
    hoursMinutes: (h: number, m: number) => `${h} год ${m} хв`,
    hours: (h: number) => `${h} год`,
    minutes: (m: number) => `${m} хв`,
    seconds: (s: number) => `${s} с`,
    today: "сьогодні",
  },
};
