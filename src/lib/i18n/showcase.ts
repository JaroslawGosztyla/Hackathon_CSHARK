// Presentation showcase (Developer page "Presentation mode", `--presentation`; ADR-019): the sample content it shows
// outside the Developer page, in the UI language — the goal and small steps of the sample workday and the two steps
// planned for the live demo session. Plain data (like a user's own texts), read when the showcase is built; the
// app names and window titles of the sample stay as the apps write them. Never saved, sent or logged.
import { language, type UiLanguage } from "./index.svelte";

export interface ShowcaseTexts {
  /** The goal of the sample workday (marked done). */
  goal: string;
  /** The small steps of the sample workday (all ticked). */
  daySteps: readonly [string, string, string, string];
  /** Steps of older sample sessions. */
  moreSteps: readonly [string, string, string, string];
  /** The two steps planned for the live demo session (so its Start pays the plan bonus). */
  demoSteps: readonly [string, string];
}

const SHOWCASE_TEXTS: Record<UiLanguage, ShowcaseTexts> = {
  en: {
    goal: "Finish the demo build",
    daySteps: ["Fix the last failing test", "Polish the rewards screen", "Record the demo tour", "Tag the release build"],
    moreSteps: ["Review the pull request", "Write the release notes", "Answer the design feedback", "Plan tomorrow"],
    demoSteps: ["Show the live rewards", "Open the weekly stats"],
  },
  pl: {
    goal: "Dokończyć build demo",
    daySteps: ["Naprawić ostatni czerwony test", "Dopracować ekran nagród", "Nagrać przejście demo", "Otagować wydanie"],
    moreSteps: ["Przejrzeć pull request", "Napisać notatki do wydania", "Odpowiedzieć na uwagi do designu", "Zaplanować jutro"],
    demoSteps: ["Pokazać nagrody na żywo", "Otworzyć statystyki tygodnia"],
  },
  de: {
    goal: "Demo-Build fertigstellen",
    daySteps: ["Den letzten roten Test reparieren", "Belohnungsansicht polieren", "Demo-Rundgang aufnehmen", "Release-Build taggen"],
    moreSteps: ["Pull-Request prüfen", "Release Notes schreiben", "Auf Design-Feedback antworten", "Morgen planen"],
    demoSteps: ["Belohnungen live zeigen", "Wochenstatistik öffnen"],
  },
  fr: {
    goal: "Finir la version de démo",
    daySteps: [
      "Corriger le dernier test en échec",
      "Peaufiner l'écran des récompenses",
      "Enregistrer la visite de démo",
      "Taguer la version finale",
    ],
    moreSteps: ["Relire la pull request", "Rédiger les notes de version", "Répondre aux retours design", "Planifier demain"],
    demoSteps: ["Montrer les récompenses en direct", "Ouvrir les stats de la semaine"],
  },
  it: {
    goal: "Finire la build della demo",
    daySteps: [
      "Sistemare l'ultimo test che fallisce",
      "Rifinire la schermata dei premi",
      "Registrare il tour della demo",
      "Taggare la build di rilascio",
    ],
    moreSteps: [
      "Rivedere la pull request",
      "Scrivere le note di rilascio",
      "Rispondere ai feedback sul design",
      "Pianificare domani",
    ],
    demoSteps: ["Mostrare i premi dal vivo", "Aprire le statistiche settimanali"],
  },
  uk: {
    goal: "Завершити демо-збірку",
    daySteps: ["Виправити останній тест, що падає", "Доопрацювати екран нагород", "Записати демо-тур", "Позначити реліз тегом"],
    moreSteps: [
      "Переглянути pull request",
      "Написати нотатки до релізу",
      "Відповісти на відгуки щодо дизайну",
      "Спланувати завтра",
    ],
    demoSteps: ["Показати нагороди наживо", "Відкрити статистику тижня"],
  },
  ru: {
    goal: "Доделать демо-сборку",
    daySteps: ["Починить последний падающий тест", "Доработать экран наград", "Записать демо-тур", "Поставить тег релиза"],
    moreSteps: [
      "Посмотреть pull request",
      "Написать заметки к релизу",
      "Ответить на замечания по дизайну",
      "Спланировать завтра",
    ],
    demoSteps: ["Показать награды вживую", "Открыть статистику недели"],
  },
  es: {
    goal: "Terminar la build de la demo",
    daySteps: [
      "Arreglar el último test que falla",
      "Pulir la pantalla de recompensas",
      "Grabar el recorrido de la demo",
      "Etiquetar la build de lanzamiento",
    ],
    moreSteps: [
      "Revisar la pull request",
      "Escribir las notas de la versión",
      "Responder al feedback de diseño",
      "Planificar mañana",
    ],
    demoSteps: ["Mostrar las recompensas en vivo", "Abrir las estadísticas semanales"],
  },
  zh: {
    goal: "完成演示版本",
    daySteps: ["修复最后一个失败的测试", "打磨奖励界面", "录制演示导览", "给发布版本打标签"],
    moreSteps: ["审查拉取请求", "撰写发布说明", "回复设计反馈", "规划明天"],
    demoSteps: ["现场展示奖励", "打开每周统计"],
  },
  ja: {
    goal: "デモビルドを仕上げる",
    daySteps: ["最後の失敗テストを直す", "ごほうび画面を仕上げる", "デモツアーを録画する", "リリースビルドにタグを付ける"],
    moreSteps: ["プルリクエストをレビューする", "リリースノートを書く", "デザインのフィードバックに返信する", "明日の計画を立てる"],
    demoSteps: ["ごほうびをライブで見せる", "週間の統計を開く"],
  },
};

/** The showcase content in the current UI language (read when the showcase is built). */
export function showcaseTexts(lang: UiLanguage = language()): ShowcaseTexts {
  return SHOWCASE_TEXTS[lang];
}
