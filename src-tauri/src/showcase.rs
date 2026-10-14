//! Presentation showcase: the Developer page's "Presentation mode" and `--presentation` (ADR-019, beta rewards
//! ADR-022). For a short live demo the app looks like a busy, real install:
//!
//! * the UI shows in-memory sample layers (an 8-hour workday in Podsumowanie, three weeks of history, the weekly
//!   stats and heatmaps, "Co dalej?", the session goal) and treats the beta features as on;
//! * the shell runs the demo `TimingProfile` (40 s focus / 20 s break) from the next Start (the session actor's
//!   presentation mode) and serves a seeded preview ledger for the beta rewards ([`seeded_ledger`],
//!   `RewardsHandle::set_showcase`), primed so the first demo block and break show every live moment;
//! * the rewards and the small steps use [`beta_config`] while it is on (memory only, never saved);
//! * a demo session that ends during the showcase is shown, but never written to `history.jsonl` nor kept as the last
//!   summary ([`keeps_summary`]);
//! * the beta collection (ADR-024) of the showcase ledger: what level 8 and the best streak earned, and the season
//!   open today one goal day short of its theme ([`seed_unlocks_showcase`]); the core never unlocks in a preview, so
//!   the shell adds the showcase's own unlocks to its moments ([`apply_showcase_unlocks`], showcase ledger only).
//!
//! Nothing of it is written or sent: `config.json`, `history.jsonl` and `rewards.json` stay as they were. Switching it
//! off (or quitting: it lives in memory only) brings the real data back. `--presentation` starts the app in it, with
//! developer mode effectively on for that run only ([`launched`]; the stored `developerMode` is never changed).
//!
//! The UI learns about a showcase the shell started (`--presentation`, another launch, a reload of the page) from a
//! launch hint evaluated in the main window ([`on_page_load`], [`announce`]): `window.__FOCUSMODE_LAUNCH__` and a
//! `focusmode:launch` DOM event. It is not an IPC command, event or DTO, so it does not change the contract.
//! TODO(hackathon): move the hint into the IPC contract (e.g. a field of the session snapshot) in a contract round.

use std::sync::atomic::{AtomicBool, Ordering};

use focus_core::api::{
    AppError, Collection, Config, MomentKind, RewardMoment, RewardsConfig, StickerId,
};
use focus_core::rewards::{
    self, Counters, DAILY_GOAL_BONUS, DayRecord, Ledger, PITY_BLOCKS, REST_BANK_MAX, RewardDay,
    RewardsState, SeasonProgress, StreakLedger, SurpriseState,
};
use focus_core::unlocks::{self, SEASON_GOAL_DAYS, Unlockable};
use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::{AppHandle, Manager, Webview};

use crate::session_actor::ActorCmd;
use crate::{AppState, MAIN_WINDOW, read_config};

/// Starts the app in presentation mode (demo timings + showcase), developer mode on for this run only.
pub const PRESENTATION_ARG: &str = "--presentation";

/// This run was started (or asked by another launch) with [`PRESENTATION_ARG`].
static LAUNCHED: AtomicBool = AtomicBool::new(false);

// ---------------------------------------------------------------------------------------------------------------
// The seeded preview ledger (numbers only, like `rewards.json`; never written)
// ---------------------------------------------------------------------------------------------------------------

/// Level 8 with the bar 85 % full: 840 sparks reach level 8 (spans 60 + 80 + … + 180), its span is 200.
pub const SHOWCASE_TOTAL_SPARKS: u32 = 1010;
/// Today's sparks before the first demo Start (daily goal 30): Start (+12) and one step (+3) keep it below the goal
/// during the first demo focus, so the goal, the streak and the level up all arrive together in the first break.
pub const SHOWCASE_TODAY_SPARKS: u32 = 10;
/// Goal days in a row up to the last work day before today (today makes it 13).
pub const SHOWCASE_STREAK_DAYS: u32 = 12;
pub const SHOWCASE_BEST_STREAK: u32 = 21;
/// Rest days in the bank (of [`REST_BANK_MAX`]) and goal days towards the next one (today's count does not earn one).
pub const SHOWCASE_REST_BANK: u32 = 2;
const SHOWCASE_REST_PROGRESS: u32 = 2;
/// Surprises found so far.
pub const SHOWCASE_COLLECTION: Collection = Collection {
    bubbles: 9,
    pearls: 4,
    golden: 1,
};
/// 6 of the 11 stickers, consistent with the level, streak and collection. The rest stays for later live unlocks:
/// "A plan to start" (a Start with 3 steps), "Back again" (the first way back from a break, shown in the next break),
/// "Together" (the second full block), "A good break" and "Rest day".
pub const SHOWCASE_STICKERS: [StickerId; 6] = [
    StickerId::FirstStep,
    StickerId::DailyGoal,
    StickerId::ThreeDays,
    StickerId::Amazed,
    StickerId::GoldenPearl,
    StickerId::Level5,
];
/// Lifetime counts kept just below their sticker thresholds (10 full blocks, 5 breaks, 14 goal days), so the first demo
/// block and break unlock nothing beyond the four beats of the burst. Never shown in the UI.
const SHOWCASE_COUNTERS: Counters = Counters {
    full_blocks: 8,
    breaks_taken: 3,
    steps_done: 21,
    backs: 0,
};
/// Sparks of the goal days of the streak, newest first (the daily-goal bonus comes on top).
const STREAK_DAY_SPARKS: [u32; SHOWCASE_STREAK_DAYS as usize] =
    [64, 71, 48, 83, 57, 66, 92, 45, 74, 58, 69, 51];
/// Share of a goal day's sparks earned by focus minutes (the rest: blocks, breaks, steps, surprises).
const MINUTE_SHARE_PERCENT: u32 = 60;
const STEP_SPARKS_PER_DAY: u32 = 6;
const STARTS_PER_DAY: u8 = 2;
const PLANS_PER_DAY: u8 = 1;
/// Saturday and Sunday (`rewards::weekday`: 0 = Monday): days off in [`beta_config`], so never part of the streak.
const FIRST_WEEKEND_DAY: u8 = 5;
/// The seeded collection items were unlocked this many days apart, the newest this long before today.
const UNLOCK_SPACING_DAYS: RewardDay = 3;

/// The rewards config of the showcase: the beta defaults (sparks, small steps, surprises, full celebrations, the light
/// daily goal of 30, Saturday and Sunday off, streak shown and counting) with the user's sounds and focus companion.
/// Memory only: the caller's config is not changed and nothing is saved.
pub fn beta_config(cfg: &Config) -> Config {
    Config {
        beta_features: true,
        rewards: RewardsConfig {
            sounds: cfg.rewards.sounds,
            focus_companion: cfg.rewards.focus_companion,
            ..RewardsConfig::default()
        },
        ..cfg.clone()
    }
}

/// The config the beta features of the shell see now: [`beta_config`] while the showcase is on, else `cfg`.
pub fn effective(rewards: &crate::rewards::RewardsHandle, cfg: Config) -> Config {
    if rewards.showcase_on() {
        beta_config(&cfg)
    } else {
        cfg
    }
}

/// A finished session's summary is remembered (last summary) and stored (`history.jsonl`, with `keepHistory`): always,
/// except a demo session of the showcase (`showcase`: it ends while the showcase is on, or it started while it was on,
/// `RewardsHandle::showcase_session`), which is only shown.
pub fn keeps_summary(showcase: bool, demo: bool) -> bool {
    !(showcase && demo)
}

/// The seed of the showcase ledger's surprise rolls, from the activation time (stable while it is on).
pub fn activation_seed(now_ms: i64) -> u64 {
    let mut z = u64::from_ne_bytes(now_ms.to_ne_bytes()).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn is_weekend(day: RewardDay) -> bool {
    rewards::weekday(day) >= FIRST_WEEKEND_DAY
}

/// The seeded preview ledger for reward day `today` (see the constants above): level 8 at 85 %, a 12-day streak (best
/// 21) whose last goal day is the last work day before today, 2 of 3 rest days, goal days on the work days of this
/// week so far, 10 sparks today and nothing started yet. Primed for the live moments of the first demo block (40 s)
/// and break with small steps planned: Start +12 (start, first of the day, a plan), a step +3, then in the break the
/// daily goal (+10), streak 13, level 9 and a surprise of at least a pearl (the pity counter is full).
pub fn seeded_ledger(today: RewardDay, seed: u64) -> Ledger {
    let mut ledger = Ledger::new(seed);
    ledger.total_sparks = SHOWCASE_TOTAL_SPARKS;
    let goal_days: Vec<RewardDay> = (1..)
        .map(|back| today - back)
        .filter(|day| !is_weekend(*day))
        .take(SHOWCASE_STREAK_DAYS as usize)
        .collect();
    for (day, sparks) in goal_days.iter().zip(STREAK_DAY_SPARKS) {
        ledger.days.push(DayRecord {
            day: *day,
            sparks: sparks + DAILY_GOAL_BONUS,
            goal_bonus: DAILY_GOAL_BONUS,
            minute_sparks: sparks * MINUTE_SHARE_PERCENT / 100,
            step_sparks: STEP_SPARKS_PER_DAY,
            starts: STARTS_PER_DAY,
            plans: PLANS_PER_DAY,
            goal_met: true,
            rest: false,
            pond_full: false,
        });
    }
    ledger.days.push(DayRecord {
        day: today,
        sparks: SHOWCASE_TODAY_SPARKS,
        ..DayRecord::default()
    });
    let last_goal_day = goal_days.first().copied();
    ledger.last_day = last_goal_day.unwrap_or(today);
    ledger.streak = StreakLedger {
        current: SHOWCASE_STREAK_DAYS,
        best: SHOWCASE_BEST_STREAK,
        last_counted_day: last_goal_day,
        rest_bank: SHOWCASE_REST_BANK.min(REST_BANK_MAX),
        rest_progress: SHOWCASE_REST_PROGRESS,
        // Only shown while a streak waits; at 13 after today's goal it stays under the 14 of the "Rest day" sticker.
        active_days: SHOWCASE_STREAK_DAYS,
    };
    ledger.collection = SHOWCASE_COLLECTION;
    // The next full block rolls at least a pearl (showcase only: the real odds are never changed).
    ledger.surprises = SurpriseState {
        since_pearl: PITY_BLOCKS,
        first_done: true,
    };
    ledger.counters = SHOWCASE_COUNTERS;
    ledger.stickers = SHOWCASE_STICKERS.to_vec();
    seed_unlocks_showcase(&mut ledger, today);
    ledger.normalize();
    ledger
}

/// The beta collection (ADR-024) of the showcase ledger: every item the level and the best streak earned (unlocked on
/// earlier days, one every [`UNLOCK_SPACING_DAYS`]), and the season whose window is open today one goal day short of
/// its theme (in October: Halloween 2 of 3, from the goal days already in the window), so the daily goal of the first
/// demo break unlocks it on stage ([`apply_showcase_unlocks`]). Outside a seasonal window nothing seasonal is seeded.
pub fn seed_unlocks_showcase(ledger: &mut Ledger, today: RewardDay) {
    let earned = unlocks::evaluate(ledger, today).len();
    for (index, record) in ledger.unlocks.iter_mut().enumerate() {
        let back = RewardDay::try_from(earned - index).unwrap_or(1) * UNLOCK_SPACING_DAYS;
        record.day = today - back;
    }
    let Some((season, year)) = unlocks::season_on(today) else {
        return;
    };
    let (start, _) = unlocks::season_window(season, year);
    let short = usize::try_from(SEASON_GOAL_DAYS.saturating_sub(1)).unwrap_or(0);
    let mut days: Vec<RewardDay> = ledger
        .days
        .iter()
        .rev()
        .filter(|record| record.goal_met && start <= record.day && record.day < today)
        .map(|record| record.day)
        .take(short)
        .collect();
    days.sort_unstable();
    if !days.is_empty() {
        ledger.seasons.push(SeasonProgress {
            season: unlocks::season_key(season).to_owned(),
            year,
            days,
        });
    }
}

/// Showcase ledger only: the collection unlocks the core leaves out of a preview (ADR-024 records seasonal goal days and
/// unlocks on the real ledger only). A moment with the daily goal records today for the open season; a moment shown
/// outside focus (break recap, session end) carries what unlocks now, like every achievement ("focus is sacred").
pub fn apply_showcase_unlocks(
    state: &mut RewardsState,
    today: RewardDay,
    moment: &mut RewardMoment,
) {
    if moment.daily_goal {
        unlocks::record_goal_day(&mut state.ledger, today);
    }
    if !matches!(moment.kind, MomentKind::BreakStart | MomentKind::SessionEnd) {
        return;
    }
    for item in unlocks::evaluate(&mut state.ledger, today) {
        match item {
            Unlockable::Theme(theme) => moment.unlocked_themes.push(theme),
            Unlockable::Ambience(layer) => moment.unlocked_ambiences.push(layer),
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// On / off, `--presentation`, the launch hint
// ---------------------------------------------------------------------------------------------------------------

/// Marks this run as started with [`PRESENTATION_ARG`] (developer mode effectively on until the app quits).
pub fn mark_launched() {
    LAUNCHED.store(true, Ordering::SeqCst);
}

/// This run was started (or asked by another launch) with [`PRESENTATION_ARG`]: `dev_trigger` works whatever the
/// stored `developerMode` says, and the UI shows the Developer page (memory only, this run only).
pub fn launched() -> bool {
    LAUNCHED.load(Ordering::SeqCst)
}

/// Presentation mode on or off: the demo profile from the next Start (at once while idle) and the rewards showcase.
pub async fn set(state: &AppState, on: bool) -> Result<(), AppError> {
    state.actor.set_presentation_mode(on).await?;
    state.rewards.set_showcase(on, &read_config(&state.config));
    if on
        && let Ok(snapshot) = state.actor.snapshot().await
        && snapshot.demo
        && let Some(session_id) = &snapshot.session_id
    {
        // A demo session that runs now is on stage too: its summary is only shown, like the UI does.
        state.rewards.adopt_showcase_session(session_id);
    }
    if !on {
        // Back to the real config: with small steps off there, the actor drops a plan the showcase left (ADR-022).
        state.actor.send(ActorCmd::ConfigChanged)?;
    }
    log::info!(
        "presentation showcase {} (memory only)",
        if on { "on" } else { "off" }
    );
    Ok(())
}

/// The launch hint for the main window: whether the shell's showcase is on and whether developer mode is on for this
/// run only. Plain booleans: nothing else ever goes into the page this way.
fn hint_script(presentation: bool, developer: bool) -> String {
    format!(
        "window.__FOCUSMODE_LAUNCH__=Object.freeze({{presentation:{presentation},developer:{developer}}});\
         window.dispatchEvent(new CustomEvent('focusmode:launch'));"
    )
}

/// The app's own page: `tauri://localhost` / `http://tauri.localhost` in a build, the dev server on `localhost`.
fn is_app_page(url: &tauri::Url) -> bool {
    url.scheme() == "tauri" || matches!(url.host_str(), Some("tauri.localhost" | "localhost"))
}

/// `Builder::on_page_load`: every time the main window's page has loaded (start, a reload), it gets the launch hint
/// when the showcase is on or this run is a `--presentation` one. Other windows and other pages never get it.
pub fn on_page_load(webview: &Webview, payload: &PageLoadPayload<'_>) {
    if webview.label() != MAIN_WINDOW
        || payload.event() != PageLoadEvent::Finished
        || !is_app_page(payload.url())
    {
        return;
    }
    let presentation = webview
        .app_handle()
        .try_state::<AppState>()
        .is_some_and(|state| state.rewards.showcase_on());
    let wanted = presentation || launched();
    if wanted && let Err(err) = webview.eval(hint_script(presentation, launched())) {
        log::warn!("cannot tell the main window about presentation mode: {err}");
    }
}

/// Sends the launch hint to the main window now (another launch with [`PRESENTATION_ARG`]).
pub fn announce(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let presentation = app
        .try_state::<AppState>()
        .is_some_and(|state| state.rewards.showcase_on());
    if let Err(err) = window.eval(hint_script(presentation, launched())) {
        log::warn!("cannot tell the main window about presentation mode: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use focus_core::api::{
        AmbienceLayer, DailyGoal, DayMark, Rarity, RewardKind, RewardTheme, StreakState,
        UnlockProgress, UnlockState, Weekday,
    };
    use focus_core::rewards::{LedgerMode, RewardRules};
    use focus_core::session::{Effect, Input, Now, ProgressFact, Session, normalize_steps};
    use focus_core::timing::TimingProfile;

    /// Monday 2026-10-12, Tuesday 2026-10-13, Saturday 2026-10-17.
    const MON: RewardDay = 20_738;
    const TUE: RewardDay = MON + 1;
    const SAT: RewardDay = MON + 5;
    const NOW: i64 = 1_791_885_600_000;

    fn showcase_rules() -> RewardRules {
        RewardRules::from_config(&beta_config(&Config::default()))
    }

    #[test]
    fn beta_config_turns_the_beta_on_in_memory_only() {
        let user = Config {
            beta_features: false,
            rewards: RewardsConfig {
                enabled: false,
                micro_steps: false,
                sounds: false,
                daily_goal: DailyGoal::Strong,
                rest_weekdays: vec![Weekday::Wed],
                streak_paused: true,
                ..RewardsConfig::default()
            },
            ..Config::default()
        };
        let before = user.clone();
        let shown = beta_config(&user);
        assert_eq!(user, before, "the user's config is never changed");
        assert!(shown.rewards_active() && shown.steps_active() && shown.streak_counting());
        assert_eq!(shown.rewards.daily_goal, DailyGoal::Light);
        assert_eq!(shown.rewards.rest_weekdays, [Weekday::Sat, Weekday::Sun]);
        assert!(!shown.rewards.sounds, "the user's sound choice stays");
        // Everything outside the beta stays the user's.
        assert_eq!(
            Config {
                beta_features: user.beta_features,
                rewards: user.rewards.clone(),
                ..shown
            },
            user
        );
        assert!(keeps_summary(false, true) && keeps_summary(true, false));
        assert!(!keeps_summary(true, true));
    }

    #[test]
    fn the_seeded_ledger_has_the_showcase_numbers() {
        let state = RewardsState::new(seeded_ledger(TUE, 7));
        let status = rewards::view(&state, TUE, &showcase_rules());
        assert!(status.active);
        assert_eq!(status.level.level, 8);
        assert_eq!((status.level.into, status.level.span), (170, 200), "85 %");
        assert_eq!(status.total_sparks, SHOWCASE_TOTAL_SPARKS);
        assert_eq!(
            (
                status.today.sparks,
                status.today.goal,
                status.today.goal_met
            ),
            (10, 30, false)
        );
        assert_eq!(status.streak.state, StreakState::Alive);
        assert_eq!((status.streak.days, status.streak.best), (12, 21));
        assert_eq!(
            (status.streak.rest_bank, status.streak.rest_bank_max),
            (2, 3)
        );
        assert_eq!(status.streak.rest_reserved, 0);
        assert!(!status.streak.today_counted);
        assert_eq!(status.collection, SHOWCASE_COLLECTION);
        assert_eq!(status.stickers.len(), 6);
        let marks: Vec<DayMark> = status.week.iter().map(|cell| cell.mark).collect();
        assert_eq!(
            marks,
            [
                DayMark::Goal,
                DayMark::Today,
                DayMark::Future,
                DayMark::Future,
                DayMark::Future,
                DayMark::Future,
                DayMark::Future
            ]
        );
        // On a Saturday the week strip shows every work day as a goal day.
        let saturday = rewards::view(
            &RewardsState::new(seeded_ledger(SAT, 7)),
            SAT,
            &showcase_rules(),
        );
        let marks: Vec<DayMark> = saturday.week.iter().map(|cell| cell.mark).collect();
        assert_eq!(&marks[..5], &[DayMark::Goal; 5]);
        assert_eq!(marks[5], DayMark::Today);
        assert_eq!(
            (saturday.streak.state, saturday.streak.days),
            (StreakState::Alive, 12)
        );
        // The collection: what level 8 and the best streak earned, and Halloween (October) one goal day short.
        let theme = |theme: RewardTheme| {
            status
                .unlocks
                .themes
                .iter()
                .find(|item| item.theme == theme)
                .map(|item| (item.state, item.progress))
        };
        for earned in [
            RewardTheme::Underwater,
            RewardTheme::RainyWindow,
            RewardTheme::ForestCabin,
            RewardTheme::SakuraNight,
        ] {
            assert_eq!(
                theme(earned),
                Some((UnlockState::Unlocked, None)),
                "{earned:?}"
            );
        }
        assert_eq!(
            theme(RewardTheme::Halloween),
            Some((
                UnlockState::AvailableNow,
                Some(UnlockProgress { have: 2, need: 3 })
            ))
        );
        assert_eq!(
            theme(RewardTheme::Christmas).map(|item| item.0),
            Some(UnlockState::Locked)
        );
        assert!(
            status
                .unlocks
                .themes
                .iter()
                .filter_map(|item| item.unlocked_on.as_deref())
                .all(|date| date < "2026-10-13")
        );
        // Outside a seasonal window nothing seasonal is seeded (2026-11-17, a Tuesday).
        assert!(seeded_ledger(TUE + 35, 7).seasons.is_empty());
    }

    /// One demo block with two planned steps (one ticked), the break, the way back and a Stop, like the shell runs it.
    fn demo_run(today: RewardDay, seed: u64) -> Vec<focus_core::api::RewardMoment> {
        let mut state = RewardsState::new(seeded_ledger(today, seed));
        let rules = showcase_rules();
        // The shell's demo profile as it is (sleep-gap detection on), ticked every second like the session actor.
        let profile = TimingProfile::demo();
        let mut session = Session::new();
        let at = |secs: u64| Now {
            mono_ms: secs * 1000,
            wall_ms: NOW + i64::try_from(secs).unwrap_or(0) * 1000,
        };
        let drafts = ["Open the demo build", "Record the tour"]
            .iter()
            .map(|text| focus_core::api::StepDraft {
                id: None,
                text: (*text).to_owned(),
            })
            .collect();
        session.handle(
            at(0),
            Input::SetSteps(normalize_steps(drafts).expect("steps")),
        );
        let focus = u64::from(profile.focus_secs);
        let pause = u64::from(profile.break_secs);
        let mut effects = session.handle(
            at(0),
            Input::Start {
                session_id: "1791885600000-4242-1".into(),
                profile,
            },
        );
        let back = focus + pause + 10;
        for secs in 1..=back {
            effects.extend(session.handle(at(secs), Input::Tick));
            if secs == 5 {
                effects.extend(session.handle(at(secs), Input::SetStepDone { id: 1, done: true }));
            }
        }
        effects.extend(session.handle(at(back), Input::Stop));
        effects
            .into_iter()
            .filter_map(|effect| match effect {
                Effect::Progress(fact) => {
                    assert!(fact.profile().demo);
                    let mut moment =
                        rewards::apply(&mut state, today, &rules, LedgerMode::Preview, &fact, NOW);
                    if let Some(moment) = moment.as_mut() {
                        apply_showcase_unlocks(&mut state, today, moment);
                    }
                    moment
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_first_demo_block_and_break_play_every_live_moment() {
        for today in [MON, TUE, SAT, SAT + 1] {
            for seed in [1, 7, 42, 0xDEAD_BEEF] {
                let moments = demo_run(today, seed);
                let kinds: Vec<MomentKind> = moments.iter().map(|m| m.kind).collect();
                assert_eq!(
                    &kinds[..4],
                    &[
                        MomentKind::Start,
                        MomentKind::Step,
                        MomentKind::BreakStart,
                        MomentKind::BackFromBreak
                    ],
                    "day {today}, seed {seed}"
                );
                let (start, step, burst) = (&moments[0], &moments[1], &moments[2]);
                // Start float: start + first of the day + a plan.
                assert_eq!(start.sparks, 12);
                assert!(start.level_up.is_none() && !start.daily_goal);
                // Step tick.
                assert_eq!(step.sparks, 3);
                assert!(step.stickers.is_empty() && !step.daily_goal);
                // The break burst: daily goal, streak 13, level 9 and at least a pearl, and nothing else.
                assert!(burst.daily_goal, "day {today}, seed {seed}");
                assert_eq!(burst.streak, Some(13));
                assert!(!burst.new_streak && burst.milestone.is_none());
                assert_eq!((burst.level_up, burst.levels_gained), (Some(9), 1));
                assert!(matches!(
                    burst.surprise,
                    Some(Rarity::Pearl | Rarity::GoldenPearl)
                ));
                assert!(burst.stickers.is_empty(), "{:?}", burst.stickers);
                // October: the third Halloween goal day unlocks the theme and its ambience on stage.
                assert_eq!(burst.unlocked_themes, [RewardTheme::Halloween]);
                assert_eq!(burst.unlocked_ambiences, [AmbienceLayer::SpookyNight]);
                assert!(start.unlocked_themes.is_empty() && step.unlocked_themes.is_empty());
                assert!(!burst.rest_day_earned && burst.rest_days_used == 0 && !burst.pond_full);
                assert!(burst.lines.iter().any(|l| l.kind == RewardKind::DailyGoal));
                assert!(burst.preview, "a preview moment: never written");
                // 40 focus seconds + a full block + the daily goal + a pearl (15) or a golden pearl (40).
                let golden = burst.surprise == Some(Rarity::GoldenPearl);
                assert_eq!(burst.sparks, if golden { 100 } else { 75 });
                // The way back from the break: break taken + back.
                assert_eq!(moments[3].sparks, 10);
                assert!(moments.iter().all(|moment| moment.preview));
            }
        }
    }

    #[test]
    fn outside_a_season_the_burst_is_the_same_without_a_theme() {
        // 2026-11-17, a Tuesday: no seasonal window is open.
        let moments = demo_run(TUE + 35, 7);
        let burst = &moments[2];
        assert_eq!((moments[0].sparks, moments[1].sparks), (12, 3));
        assert!(burst.daily_goal && burst.surprise.is_some());
        assert_eq!((burst.streak, burst.level_up), (Some(13), Some(9)));
        assert!(burst.unlocked_themes.is_empty() && burst.unlocked_ambiences.is_empty());
    }

    #[test]
    fn real_facts_never_reach_the_showcase_rules() {
        // A real (non-demo) fact on the showcase ledger with the real rules of a config without the beta: nothing.
        let mut state = RewardsState::new(seeded_ledger(TUE, 1));
        let real_rules = RewardRules::from_config(&Config::default());
        let fact = ProgressFact::Started {
            session_id: "1".into(),
            steps: 1,
            profile: TimingProfile::normal(),
        };
        assert!(
            rewards::apply(&mut state, TUE, &real_rules, LedgerMode::Real, &fact, NOW).is_none()
        );
        assert_eq!(state.ledger, seeded_ledger(TUE, 1));
    }

    #[test]
    fn the_hint_goes_to_the_app_page_only() {
        let url = |text: &str| tauri::Url::parse(text).expect("url");
        assert!(is_app_page(&url("http://tauri.localhost/")));
        assert!(is_app_page(&url("tauri://localhost/summary")));
        assert!(is_app_page(&url("http://localhost:1420/")));
        assert!(!is_app_page(&url("https://www.youtube.com/watch?v=x")));
        assert!(!is_app_page(&url("https://localhost.example.com/")));
    }

    #[test]
    fn the_hint_carries_two_booleans_only() {
        let script = hint_script(true, false);
        assert!(script.contains("presentation:true,developer:false"));
        assert!(script.contains("focusmode:launch"));
        assert!(!hint_script(false, true).contains("presentation:true"));
    }
}
