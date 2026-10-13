# Słownik tłumaczeń (i18n)

Wspólne terminy FocusMode dla 10 języków UI (ADR-014 pkt 5): angielski, polski, niemiecki, francuski, włoski, ukraiński,
rosyjski, hiszpański, chiński (uproszczony) i japoński. **Język źródłowy to polski**; angielski jest pełnym tłumaczeniem
i wzorem dla pozostałych. Tłumacz wypełnia kolumnę swojego języka **przed** tłumaczeniem plików i trzyma się jej wszędzie
(UI, zasobnik, okienka, toasty, rozszerzenie), żeby ten sam element nazywał się wszędzie tak samo.

## Gdzie są teksty

| Co | Plik źródłowy (PL) | Angielski | Pozostałe języki (dziś tymczasowo = angielski) |
| --- | --- | --- | --- |
| UI (okno główne, okienka faz, propozycje na przerwę) | `src/lib/i18n/pl.ts` | `src/lib/i18n/en.ts` | `src/lib/i18n/{de,fr,it,uk,ru,es,zh,ja}.ts` |
| Powłoka (zasobnik, toasty, tytuły okien, błędy pliku listy) | `src-tauri/src/texts/pl.rs` | `src-tauri/src/texts/en.rs` | `src-tauri/src/texts/{de,fr,it,uk,ru,es,zh,ja}.rs` |
| Rozszerzenie przeglądarki | `browser-extension/_locales/pl/messages.json` | `…/_locales/en/messages.json` | `…/_locales/{de,fr,it,uk,ru,es,zh_CN,ja}/messages.json` |

- Plik TS ma dokładnie kształt `Strings` z `pl.ts` (`export const de: Strings = { … }`); TypeScript pilnuje kompletu kluczy.
  Liczba mnoga: `pluralizer("de")` z `src/lib/i18n/plural.ts` (kategorie CLDR: `one`, `few`, `many`, `other`).
- Plik Rust wypełnia `Texts` (`pub(super) static TEXTS: Texts = Texts { … }`, wzór: `en.rs`); liczba mnoga: `plural(Language::De, n)`.
- W `messages.json` zmieniasz tylko `message` (`description` to wskazówka); `lang` = kod języka tekstu (`de`, …, `zh-Hans`, `ja`).
  Znacznik `{{file}}` i placeholdery `$HOST$`, `$TIME$`… zostają bez zmian.
- Język wybiera się w **Ustawienia → Język**; domyślnie „System” (język wyświetlania Windows, a gdy go nie ma na liście — angielski).
  Rozszerzenie zawsze mówi w języku interfejsu przeglądarki.

## Zasady

- **Ton**: przyjazny, krótki, spokojny — to przytulna aplikacja lofi dla osób z ADHD. Krótkie zdania, bez wykrzykników i presji,
  bez żargonu. Zwracamy się do użytkownika bezpośrednio (PL: „Ty”, wielką literą w zaimkach dzierżawczych: „Twoje”; EN: „you”;
  w językach z formą grzecznościową wybierz jedną i trzymaj się jej, np. DE „du”, FR „vous”, RU/UK „ты/ти” albo „вы/ви” — zapisz wybór niżej).
- **Bez obietnic zdrowotnych**: propozycje na przerwę to propozycje („spróbuj”, „jeśli to wygodne”), nigdy terapia ani efekt.
- **Nie tłumaczymy**: `FocusMode`, `websiteblacklist.txt`, nazw produktów i marek (Teams, Outlook, GitHub, Lofi Girl, Steam…),
  identyfikatorów i zakresów (Application (client) ID, `Calendars.Read`, `read_api`), adresów i ścieżek plików.
- **Elementy cudzych aplikacji** (Windows, Teams, Azure, Google Cloud, Jira…) nazywamy tak, jak nazywa je ich interfejs w danym języku;
  gdy nie wiadomo, zostaw angielską nazwę w nawiasie, jak w `pl.ts`.
- **Cudzysłowy**: PL „…”, EN “…”, DE „…“, FR « … », IT/ES «…» albo “…”, UK/RU «…», ZH “…”, JA 「…」.
- **Jednostki czasu**: `min`, `h`, `s` — albo lokalny skrót, jeśli tak pisze system (np. JA 分/時間/秒, ZH 分钟/小时/秒).
- Daty, godziny i procenty formatuje `Intl` według wybranego języka — nie wpisuj ich formatu w tekstach.

## Terminy

Kolumny DE…JA wypełnia tłumacz. Wyjątek: oficjalne nazwy funkcji Windows (wiersze oznaczone **Windows**) — wpisane według
interfejsu Windows; „(sprawdzić)” = potwierdź w ustawieniach Windows w tym języku (strony pomocy Microsoft bywają tłumaczone maszynowo).

| # | PL (źródło) | EN | DE | FR | IT | UK | RU | ES | ZH | JA | Uwagi |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | FocusMode | FocusMode | FocusMode | FocusMode | FocusMode | FocusMode | FocusMode | FocusMode | FocusMode | FocusMode | Nazwa, nigdy nie tłumaczymy ani nie odmieniamy. |
| 2 | Skupienie | Focus | Fokus | Concentration | Concentrazione | Фокус | Фокус | Concentración | 专注 | 集中 | Faza pracy, nazwa zakładki i ekranu. Rzeczownik, nie „koncentracja”. |
| 3 | Przerwa | Break | Pause | Pause | Pausa | Перерва | Перерыв | Descanso | 休息 | 休憩 | Faza odpoczynku. |
| 4 | cykl | cycle | Zyklus | cycle | ciclo | цикл | цикл | ciclo | 轮（第 2 轮） | サイクル | Skupienie + przerwa; „Cykl 2”. |
| 5 | sesja (skupienia) | (focus) session | (Fokus-)Sitzung | session (de concentration) | sessione (di concentrazione) | сесія (фокусу) | сессия (фокуса) | sesión (de concentración) | 专注时段 / 时段 | セッション | Od Start do Stop. |
| 6 | Start | Start | Start | Démarrer | Avvia | Старт | Старт | Iniciar | 开始 | 開始 | Przycisk. |
| 7 | Pauza (stan) / Wstrzymaj | Pause / Paused | Anhalten / Angehalten | Suspendre / Suspendu | Sospendi / Sospeso | Пауза / На паузі | Пауза / На паузе | Pausar / En pausa | 暂停 / 已暂停 | 一時停止 / 一時停止中 | Przycisk i stan „wstrzymane”. |
| 8 | Wznów | Resume | Fortsetzen | Reprendre | Riprendi | Продовжити | Продолжить | Reanudar | 继续 | 再開 | |
| 9 | Pomiń fazę / Pomiń | Skip phase / Skip | Phase überspringen / Überspringen | Passer la phase / Passer | Salta fase / Salta | Пропустити фазу / Пропустити | Пропустить фазу / Пропустить | Saltar fase / Saltar | 跳过此阶段 / 跳过 | フェーズをスキップ / スキップ | Zasobnik i okienka. |
| 10 | Przerwa teraz / Wracam teraz | Break now / Back to focus now | Jetzt Pause / Zurück zum Fokus | Pause maintenant / Je m'y remets | Pausa adesso / Torno a concentrarmi | Перерва зараз / Повернутися до фокусу | Перерыв сейчас / Вернуться к фокусу | Descansar ahora / Volver a concentrarme | 现在休息 / 现在回到专注 | 今すぐ休憩 / 今すぐ集中に戻る | Pominięcie fazy z ekranu głównego. |
| 11 | Stop | Stop | Stopp | Arrêter | Stop | Стоп | Стоп | Detener | 停止 | 停止 | Kończy sesję. |
| 12 | Osłony | Shields | Schutzschilde | Boucliers | Protezioni | Щити | Щиты | Protecciones | 防护 | シールド | Wszystko, co działa tylko w czasie skupienia. |
| 13 | Co dalej? | What's next? | Was steht an? | Et ensuite ? | Cosa fare dopo? | Що далі? | Что дальше? | ¿Qué sigue? | 接下来做什么？ | 次にやること | Karta z zadaniami z narzędzi pracy; także nagłówek okienka. |
| 14 | cel sesji | session goal | Sitzungsziel | objectif de la session | obiettivo della sessione | ціль сесії | цель сессии | objetivo de la sesión | 时段目标 | セッションの目標 | „Ustaw jako cel”, „Zrobione?”. |
| 15 | przypomnienie (w przerwie) | (break) reminder | Erinnerung | rappel | promemoria (per la pausa) | нагадування (на перерві) | напоминание (на перерыве) | recordatorio (para el descanso) | （休息）提醒 | リマインダー | Własny tekst użytkownika na n-tą przerwę. |
| 16 | propozycja (na przerwę) | suggestion | Vorschlag | suggestion | suggerimento | ідея (для перерви) | идея (для перерыва) | sugerencia | 建议 | おすすめ | Porada ruchu/odpoczynku; nie „ćwiczenie”, nie „zalecenie”. |
| 17 | Dobra praktyka | Good practice | Gute Gewohnheit | Bonne pratique | Buona pratica | Корисна звичка | Полезная привычка | Buena práctica | 好习惯 | よい習慣 | Porada dnia na ekranie Skupienie. |
| 18 | maskotka (aksolotl) | mascot (axolotl) | Maskottchen (Axolotl) | mascotte (axolotl) | mascotte (assolotto) | талісман (аксолотль) | талисман (аксолотль) | mascota (ajolote) | 吉祥物（美西螈） | マスコット（ウーパールーパー） | Aksolotl w słuchawkach. |
| 19 | zasobnik | tray | Infobereich | zone de notification | area di notifica | область сповіщень | область уведомлений | área de notificación | 系统托盘 | システムトレイ | Obszar powiadomień obok zegara; użyj nazwy z Windows. |
| 20 | okienko (FocusMode) | pop-up | Pop-up | fenêtre pop-up | pop-up (di FocusMode) | спливне вікно (FocusMode) | всплывающее окно (FocusMode) | ventana emergente (de FocusMode) | 弹窗 | ポップアップ | Małe okno w rogu przy zmianie fazy; nie „powiadomienie Windows”. |
| 21 | toast / powiadomienie Windows | Windows notification | Windows-Benachrichtigung | notification Windows | notifica di Windows | сповіщення Windows | уведомление Windows | notificación de Windows | Windows 通知 | Windows の通知 | Zapasowa forma okienka. |
| 22 | motyw | theme | Design | thème | tema | тема | тема | tema | 主题 | テーマ | Wygląd aplikacji. |
| 23 | dzwonek | chime | Signalton | sonnerie | campanella | сигнал | сигнал | campanilla | 提示音 | チャイム | Dźwięk na zmianę fazy. |
| 24 | źródło muzyki | music source | Musikquelle | source musicale | sorgente musicale | джерело музики | источник музыки | fuente de música | 音乐来源 | 音楽ソース | Wbudowana / Stream / Linki / Folder / Szum / Cisza. |
| 25 | szum (brązowy, różowy, biały) | (brown, pink, white) noise | (braunes, rosa, weißes) Rauschen | bruit (brun, rose, blanc) | rumore (marrone, rosa, bianco) | шум (коричневий, рожевий, білий) | шум (коричневый, розовый, белый) | ruido (marrón, rosa, blanco) | 布朗噪音、粉红噪音、白噪音 | ブラウンノイズ、ピンクノイズ、ホワイトノイズ | Kolory szumu jak w akustyce. |
| 26 | ciepłe kolory ekranu | warm screen colours | warme Bildschirmfarben | couleurs chaudes de l'écran | colori caldi dello schermo | теплі кольори екрана | теплые цвета экрана | colores cálidos de pantalla | 屏幕暖色 | 画面の暖色化 | Nazwa funkcji FocusMode (nie „filtr światła niebieskiego”, bez obietnic zdrowotnych). |
| 27 | **Windows**: Oświetlenie nocne | Night light | Nachtmodus | Éclairage nocturne | Luce notturna | Нічник | Ночной свет | Luz nocturna | 夜间模式 (UI Windows 10/11; strony pomocy MS: 夜灯 — sprawdzić) | 夜間モード | Ustawienia → System → Ekran. |
| 28 | gamma (karty graficznej) | (graphics card) gamma | Gamma (der Grafikkarte) | gamma (de la carte graphique) | gamma (della scheda grafica) | гамма (відеокарти) | гамма (видеокарты) | gamma (de la tarjeta gráfica) | （显卡）gamma | （グラフィックスカードの）ガンマ | Termin techniczny. |
| 29 | **Windows**: Nie przeszkadzać | Do not disturb | Nicht stören | Ne pas déranger | Non disturbare | Не турбувати | Не беспокоить | No molestar | 请勿打扰 | 応答不可 | Ustawienia → System → Powiadomienia (Windows 11). |
| 30 | **Teams**: Nie przeszkadzać | Do not disturb | Nicht stören | Ne pas déranger | Non disturbare | Не турбувати | Не беспокоить | No molestar | 请勿打扰 | 応答不可 | Status w Teams; jak w interfejsie Teams. |
| 31 | blokada stron | site blocking | Seitensperre | blocage des sites | blocco dei siti | блокування сайтів | блокировка сайтов | bloqueo de sitios | 网站屏蔽 | サイトのブロック | Osłona z rozszerzeniem przeglądarki. |
| 32 | lista blokowanych (stron) | block list | Sperrliste | liste de blocage | elenco dei siti bloccati | список блокування | список блокировки | lista de bloqueo | 屏蔽列表 | ブロックリスト | Zawartość `websiteblacklist.txt`. |
| 33 | websiteblacklist.txt | websiteblacklist.txt | websiteblacklist.txt | websiteblacklist.txt | websiteblacklist.txt | websiteblacklist.txt | websiteblacklist.txt | websiteblacklist.txt | websiteblacklist.txt | websiteblacklist.txt | Nazwa pliku, nigdy nie tłumaczymy. |
| 34 | wyrażenie regularne / wzorzec | regular expression / pattern | regulärer Ausdruck / Muster | expression régulière / motif | espressione regolare / pattern | регулярний вираз / шаблон | регулярное выражение / шаблон | expresión regular / patrón | 正则表达式 / 表达式 | 正規表現 / パターン | W rozszerzeniu „Wzorce (regex)”. |
| 35 | rozszerzenie (przeglądarki) | (browser) extension | (Browser-)Erweiterung; „Entwicklermodus“, „Entpackte Erweiterung laden“ | extension (du navigateur) ; « Mode développeur », « Charger l'extension non empaquetée » | estensione (del browser); Chrome: „Modalità sviluppatore”, „Carica estensione non pacchettizzata” | розширення (браузера); «Режим розробника», «Завантажити розпаковане розширення» | расширение (браузера); «Режим разработчика», «Загрузить распакованное расширение» | extensión (del navegador); Chrome: „Modo de desarrollador”, „Cargar descomprimida” | （浏览器）扩展；Chrome：“开发者模式”“加载已解压的扩展程序” | （ブラウザーの）拡張機能；Chrome：「デベロッパー モード」「パッケージ化されていない拡張機能を読み込む」 | „Załaduj rozpakowane”, „Tryb dewelopera” — nazwy z przeglądarki w danym języku. |
| 36 | tryb awaryjny | fallback mode | Ersatzmodus | mode de secours | modalità di riserva | резервний режим | резервный режим | modo alternativo | 备用模式 | 簡易モード | Wykrywanie po tytule okna, gdy rozszerzenie milczy. |
| 37 | Wracaj do pracy | Back to work | Zurück an die Arbeit | Retour au travail | Torna al lavoro | Повертаймося до роботи | Вернемся к работе | Vuelve al trabajo | 回到工作吧 | 作業に戻りましょう | Strona zamiast zablokowanej strony i toast. |
| 38 | launcher (gier) | (game) launcher | (Spiele-)Launcher; EA app, Xbox-App | lanceur (de jeux) ; Application EA, Application Xbox | launcher (di gioco); App EA, App Xbox | лаунчер (ігор); «Застосунок EA», «Застосунок Xbox» | игровой лаунчер; «Приложение EA», «Приложение Xbox» | lanzador (de juegos); Aplicación EA, Aplicación Xbox | 游戏启动器；EA app、Xbox 应用 | ゲームランチャー；EA app、Xbox アプリ | Steam, Epic…; „Aplikacja EA”, „Aplikacja Xbox” jak w sklepie danego kraju. |
| 39 | Podsumowanie | Summary | Zusammenfassung | Résumé | Riepilogo | Підсумок | Итоги | Resumen | 总结 | まとめ | Zakładka z wykresem po Stop. |
| 40 | Historia | History | Verlauf | Historique | Cronologia | Історія | История | Historial | 历史记录 | 履歴 | Zapisane sesje. |
| 41 | Ustawienia | Settings | Einstellungen | Paramètres | Impostazioni | Налаштування | Настройки | Configuración | 设置 | 設定 | Zakładka. |
| 42 | bezczynność | idle | Inaktiv | Inactivité | inattività | неактивність | неактивность | inactividad | 空闲 | アイドル | Brak aktywności w statystykach. |
| 43 | Narzędzia pracy | Work tools | Arbeitstools | Outils de travail | Strumenti di lavoro | Робочі інструменти | Рабочие инструменты | Herramientas de trabajo | 工作工具 | 仕事ツール | GitHub, GitLab, Jira, Azure DevOps, Linear. |
| 44 | Kalendarze / spotkanie | Calendars / meeting | Kalender / Besprechung | Calendriers / réunion | Calendari / riunione | Календарі / зустріч | Календари / встреча | Calendarios / reunión | 日历 / 会议 | カレンダー / 会議 | |
| 45 | Przedłuż skupienie / Wstrzymaj przy przerwie / Ignoruj / Przypomnij później | Extend focus / Hold at the break / Ignore / Remind me later | Fokus verlängern / An der Pause anhalten / Ignorieren / Später erinnern | Prolonger la concentration / Suspendre à la pause / Ignorer / Me le rappeler plus tard | Prolunga la concentrazione / Metti in attesa la pausa / Ignora / Ricordamelo più tardi | Подовжити фокус / Призупинити на перерві / Ігнорувати / Нагадати пізніше | Продлить фокус / Приостановить на перерыве / Игнорировать / Напомнить позже | Alargar la concentración / Dejar el descanso en espera / Ignorar / Recordármelo más tarde | 延长专注 / 休息时暂停 / 忽略 / 稍后提醒 | 集中を延長 / 休憩で一時停止 / 無視 / あとで通知 | Wybór przy spotkaniu w czasie przerwy. |
| 46 | przerwa wstrzymana | break on hold | Pause angehalten | pause suspendue | pausa in attesa | перерву призупинено | перерыв приостановлен | descanso en espera | 休息已暂停 | 休憩を保留中 | Minutnik przerwy czeka na „Wznów”. W tytule okienka fazy (380 px) krócej: RU „Перерыв ждет”, UK „Перерва чекає”, ES „Descanso pausado”. |
| 47 | tryb prezentacji | demo mode | Demomodus | mode démo | modalità demo | демо-режим | демо-режим | modo demo | 演示模式 | デモモード | Skrócone czasy (40 s / 20 s). |
| 48 | token (dostępu) | (access) token | (Zugriffs-)Token | jeton (d'accès) | token (di accesso) | токен (доступу) | токен (доступа) | token (de acceso) | （访问）令牌 | （アクセス）トークン | Personal access token itd. — nazwy z danego serwisu. |
| 49 | Język / Systemowy | Language / System | Sprache / System | Langue / Système | Lingua / Sistema | Мова / Системна | Язык / Системный | Idioma / Sistema | 语言 / 跟随系统 | 言語 / システムに合わせる | Nazwy języków na liście zawsze w ich własnym języku (English, Polski, Deutsch…). |

## Wybory tłumaczy

Krótko, jeden wiersz na język: forma zwracania się, cudzysłowy, nietypowe decyzje.

| Język | Forma | Uwagi |
| --- | --- | --- |
| EN | „you” | Pisownia brytyjska (colours, favourite) — tak jak w poradach na przerwę. |
| DE | „du” (małą literą) | Cudzysłowy „…“. „Pause” to faza, więc przycisk pauzy to „Anhalten”/„Angehalten”, wznowienie „Fortsetzen”. Jednostki min/h/s jak w EN. „Google Kalender” (oficjalna nazwa niemiecka). Nazwy dzwonków = nazwy motywów (Mitternacht, Mond, Herbst). Tytuł szumu odmieniony (Braunes/Rosa/Weißes Rauschen) przez mapę w `de.ts`. Tenant = „Mandant” (jak w Azure). |
| FR | „vous” | Cudzysłowy « … » i niełamliwa spacja (U+00A0) przed : ; ? ! %, przed jednostkami i wewnątrz « ». „Pause” to faza, więc przycisk pauzy to „Suspendre”/„Suspendu”, wznowienie „Reprendre”. MB/KB → Mo/Ko. „Google Agenda” (oficjalna nazwa francuska). Nazwy dzwonków = nazwy motywów (Minuit, Lune, Automne). Źródło „Stream” → „Flux”. Tenant = „locataire” (jak w Azure). |
| IT | „tu” (nieformalnie, jak Windows po włosku) | Cudzysłowy “…”. „Pausa” to faza przerwy, więc Pauza/wstrzymanie = „Sospendi” / „Sospeso” (nigdy „Pausa” w sensie wstrzymania). Stop zostaje „Stop”. Propozycje na przerwę bez form zależnych od płci (bez „seduto”, „stanco”). Nazwy z Azure/Google/Outlook po włosku, angielska w nawiasie, gdy niepewna. |
| UK | „ви” (małą literą) | Cudzysłowy «…», apostrof ’. „Фокус” jako nazwa fazy (krótko, jak w aplikacjach Pomodoro; nie „зосередження”); „застосунок” = aplikacja; „ідея” (dla przerwy) zamiast „пропозиція”; „сигнал” = dzwonek; „Щити” = osłony; jednostki год/хв/с. „Нічник” potwierdzony: strona pomocy Microsoft (uk-UA) i instrukcja Lenovo każą wpisać „Нічник” w wyszukiwarce Windows. Nazwy z Azure/Google Cloud po ukraińsku z angielskim w nawiasie, GitHub/Linear/Azure DevOps po angielsku (nie mają UI po ukraińsku). |
| RU | „вы” (małą literą) | Cudzysłowy «…»; bez „ё” (jak w Windows). „Фокус” jako nazwa fazy; „приложение” = aplikacja; „идея” (для перерыва), bo „предложение” to też „zdanie”; „сигнал” = dzwonek; „Щиты” = osłony; „ревью”, „пул-реквесты” jak w żargonie programistów; jednostki ч/мин/с. Nazwy z Azure/Google Cloud po rosyjsku z angielskim w nawiasie, GitHub/Linear/Azure DevOps po angielsku. |
| ES | „tú” (neutralny hiszpański, bez „vosotros”; jak Microsoft) | Cudzysłowy «…». Przerwa = „Descanso”, Pauza = „Pausar” / „En pausa” (żeby się nie myliły). Słownictwo wspólne dla Hiszpanii i Ameryki Łacińskiej: „equipo” zamiast ordenador/computadora, „teléfono” zamiast móvil/celular, „puntero” zamiast ratón/mouse; „vídeo” jak w es-ES. „mascota” tylko dla maskotki (zwierzak w propozycji to „animal”). Propozycje bez form zależnych od płci. |
| ZH | „你” (jak Microsoft i Apple po chińsku) | Chiński uproszczony (`zh-Hans`; rozszerzenie: `zh_CN`). Interpunkcja pełnej szerokości（，。：“”）, spacja między znakami chińskimi a słowami łacińskimi i cyframi (styl Microsoft). Sesja = 专注时段 (jak „Focus sessions” w Zegarze Windows 11). Night light = 夜间模式 (tak w UI Windows; strony pomocy MS piszą 夜灯). Nazwy szumu zawierają 噪音, więc `noiseTitle` zwraca samą nazwę. Przykład domeny na liście: weibo.com. |
| JA | です/ます, bez „あなた”, gdzie się da | Interpunkcja 、。「」（）：. Spacja między słowami łacińskimi a japońskimi (styl Microsoft), bez spacji między liczbą a jednostką (15分, 2時間). Zasobnik = システムトレイ (Windows 11: „その他のシステム トレイ アイコン”), aksolotl = ウーパールーパー, nazwy z Chrome jak w Chrome po japońsku. Nazwy szumu zawierają ノイズ, więc `noiseTitle` zwraca samą nazwę. Przykład domeny na liście: x.com. |
