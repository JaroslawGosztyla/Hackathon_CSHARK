//! Game launchers that can be closed when a focus phase starts (`ShieldsConfig.close_launchers`):
//! display names and the Windows executables of each launcher's own client processes.
//!
//! Only the launcher/client processes are listed: never games, never Windows services that need
//! administrator rights to stop (Steam Client Service, EABackgroundService, GalaxyCommunication,
//! UplayService, Gaming Services, Riot Vanguard), and never generic names (`Agent.exe`,
//! `Launcher.exe`, `Helper.exe`, `QtWebEngineProcess.exe`) that other software also uses.
//! The shell finds and closes the processes (`focus-platform`); this module is the pure catalog.

use crate::api::GameLauncher;

/// Every launcher, in the canonical (settings) order.
pub const ALL: [GameLauncher; 10] = [
    GameLauncher::Steam,
    GameLauncher::Epic,
    GameLauncher::Gog,
    GameLauncher::Ubisoft,
    GameLauncher::Ea,
    GameLauncher::BattleNet,
    GameLauncher::Riot,
    GameLauncher::Xbox,
    GameLauncher::Amazon,
    GameLauncher::Itch,
];

/// Launchers selected by default (the stores most likely to pull someone into a game).
const DEFAULT: [GameLauncher; 7] = [
    GameLauncher::Steam,
    GameLauncher::Epic,
    GameLauncher::Gog,
    GameLauncher::Ubisoft,
    GameLauncher::Ea,
    GameLauncher::BattleNet,
    GameLauncher::Riot,
];

/// Product name shown in the UI and in popups (brand names are not translated).
pub fn display_name(launcher: GameLauncher) -> &'static str {
    match launcher {
        GameLauncher::Steam => "Steam",
        GameLauncher::Epic => "Epic Games Launcher",
        GameLauncher::Gog => "GOG Galaxy",
        GameLauncher::Ubisoft => "Ubisoft Connect",
        GameLauncher::Ea => "EA app",
        GameLauncher::BattleNet => "Battle.net",
        GameLauncher::Riot => "Riot Client",
        GameLauncher::Xbox => "Xbox app",
        GameLauncher::Amazon => "Amazon Games",
        GameLauncher::Itch => "itch",
    }
}

/// Executable names (as in Task Manager's Details tab) of the launcher's client processes.
/// Compare them case-insensitively ([`launcher_for_process`]).
pub fn process_names(launcher: GameLauncher) -> &'static [&'static str] {
    match launcher {
        // Client and its Chromium UI helper (`steamservice.exe` is a service, `GameOverlayUI.exe` runs with games).
        GameLauncher::Steam => &["steam.exe", "steamwebhelper.exe"],
        // Launcher and its web helper (`EpicOnlineServices*` belong to games and a service).
        GameLauncher::Epic => &["EpicGamesLauncher.exe", "EpicWebHelper.exe"],
        // Galaxy 2.0 client, its UI helper and notification renderer (`GalaxyCommunication.exe` is a service).
        GameLauncher::Gog => &[
            "GalaxyClient.exe",
            "GalaxyClient Helper.exe",
            "GOG Galaxy Notifications Renderer.exe",
        ],
        // Ubisoft Connect stub, the PC client and its web core (`UbisoftGameLauncher*.exe` start games,
        // `UplayService.exe` is a service).
        GameLauncher::Ubisoft => &["UbisoftConnect.exe", "upc.exe", "UplayWebCore.exe"],
        // EA app and its Chromium subprocess (`EABackgroundService.exe` is a service). The retired
        // Origin client (shut down in April 2025) is left out: `Origin.exe` is also an unrelated app.
        GameLauncher::Ea => &["EADesktop.exe", "EACefSubProcess.exe"],
        // Desktop app and its start stub (the update agent is the generic `Agent.exe`, left out).
        GameLauncher::BattleNet => &["Battle.net.exe", "Battle.net Launcher.exe"],
        // Riot Client services and UI (Vanguard `vgc.exe` is a service; game clients are left out).
        GameLauncher::Riot => &[
            "RiotClientServices.exe",
            "RiotClientUx.exe",
            "RiotClientUxRender.exe",
        ],
        // Xbox app (Microsoft.GamingApp package); Gaming Services and Game Bar are left out.
        GameLauncher::Xbox => &["XboxPcApp.exe", "XboxPcAppFT.exe"],
        // Amazon Games: main process, its UI and its library service (all per-user, in %LOCALAPPDATA%).
        GameLauncher::Amazon => &[
            "Amazon Games.exe",
            "Amazon Games UI.exe",
            "Amazon Games Services.exe",
        ],
        // itch app (Electron; its `butler.exe` daemon exits with it and has a too generic name).
        GameLauncher::Itch => &["itch.exe"],
    }
}

/// Default selection for `ShieldsConfig.launchers`: Steam, Epic, GOG, Ubisoft, EA, Battle.net, Riot.
pub fn default_launchers() -> Vec<GameLauncher> {
    DEFAULT.to_vec()
}

/// `launchers` without duplicates, in the canonical [`ALL`] order.
pub fn normalized(launchers: &[GameLauncher]) -> Vec<GameLauncher> {
    ALL.into_iter()
        .filter(|launcher| launchers.contains(launcher))
        .collect()
}

/// The launcher a process belongs to: `process` is an executable name or a full path, compared
/// case-insensitively (`STEAM.EXE`, `C:\Program Files (x86)\Steam\steam.exe`).
pub fn launcher_for_process(process: &str) -> Option<GameLauncher> {
    let trimmed = process.trim();
    let file_name = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    ALL.into_iter().find(|&launcher| {
        process_names(launcher)
            .iter()
            .any(|name| name.eq_ignore_ascii_case(file_name))
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn all_lists_every_launcher_once() {
        for (index, launcher) in ALL.iter().enumerate() {
            assert!(!ALL.iter().skip(index + 1).any(|other| other == launcher));
        }
        // Every variant appears (the match below stops compiling when one is added).
        for launcher in ALL {
            match launcher {
                GameLauncher::Steam
                | GameLauncher::Epic
                | GameLauncher::Gog
                | GameLauncher::Ubisoft
                | GameLauncher::Ea
                | GameLauncher::BattleNet
                | GameLauncher::Riot
                | GameLauncher::Xbox
                | GameLauncher::Amazon
                | GameLauncher::Itch => {}
            }
        }
    }

    #[test]
    fn process_names_are_specific_executables() {
        const GENERIC: &[&str] = &[
            "agent.exe",
            "launcher.exe",
            "helper.exe",
            "updater.exe",
            "update.exe",
            "service.exe",
            "client.exe",
            "setup.exe",
            "installer.exe",
            "butler.exe",
            "origin.exe",
            "qtwebengineprocess.exe",
            "unrealcefsubprocess.exe",
            "crashreporter.exe",
        ];
        let mut seen = HashSet::new();
        for launcher in ALL {
            let names = process_names(launcher);
            assert!(!names.is_empty(), "{launcher:?}");
            for name in names {
                let lower = name.to_ascii_lowercase();
                assert!(lower.ends_with(".exe"), "{name}");
                assert!(lower.len() > ".exe".len() + 2, "{name}");
                assert_eq!(name.trim(), *name, "{name}");
                assert!(!name.contains(['/', '\\']), "{name}");
                assert!(!GENERIC.contains(&lower.as_str()), "generic name {name}");
                assert!(seen.insert(lower), "{name} listed twice");
            }
        }
    }

    #[test]
    fn display_names_are_unique_and_non_empty() {
        let names: HashSet<_> = ALL.into_iter().map(display_name).collect();
        assert_eq!(names.len(), ALL.len());
        assert!(names.iter().all(|name| !name.trim().is_empty()));
        assert_eq!(display_name(GameLauncher::Gog), "GOG Galaxy");
        assert_eq!(display_name(GameLauncher::BattleNet), "Battle.net");
    }

    #[test]
    fn default_selection() {
        assert_eq!(
            default_launchers(),
            [
                GameLauncher::Steam,
                GameLauncher::Epic,
                GameLauncher::Gog,
                GameLauncher::Ubisoft,
                GameLauncher::Ea,
                GameLauncher::BattleNet,
                GameLauncher::Riot,
            ]
        );
        assert_eq!(normalized(&default_launchers()), default_launchers());
    }

    #[test]
    fn normalizes_to_canonical_order_without_duplicates() {
        assert_eq!(
            normalized(&[
                GameLauncher::Riot,
                GameLauncher::Steam,
                GameLauncher::Riot,
                GameLauncher::Itch,
                GameLauncher::Steam,
            ]),
            [GameLauncher::Steam, GameLauncher::Riot, GameLauncher::Itch]
        );
        assert!(normalized(&[]).is_empty());
        assert_eq!(normalized(&ALL), ALL);
    }

    #[test]
    fn finds_the_launcher_of_a_process() {
        for (process, expected) in [
            ("steam.exe", Some(GameLauncher::Steam)),
            ("STEAMWEBHELPER.EXE", Some(GameLauncher::Steam)),
            (
                r"C:\Program Files (x86)\Steam\steam.exe",
                Some(GameLauncher::Steam),
            ),
            ("EpicGamesLauncher.exe", Some(GameLauncher::Epic)),
            ("galaxyclient helper.exe", Some(GameLauncher::Gog)),
            ("upc.exe", Some(GameLauncher::Ubisoft)),
            ("EADesktop.exe", Some(GameLauncher::Ea)),
            ("Battle.net.exe", Some(GameLauncher::BattleNet)),
            ("RiotClientServices.exe", Some(GameLauncher::Riot)),
            ("XboxPcApp.exe", Some(GameLauncher::Xbox)),
            ("Amazon Games UI.exe", Some(GameLauncher::Amazon)),
            ("  itch.exe ", Some(GameLauncher::Itch)),
            ("steam", None),
            ("Agent.exe", None),
            ("chrome.exe", None),
            ("cs2.exe", None),
            ("", None),
        ] {
            assert_eq!(launcher_for_process(process), expected, "{process}");
        }
    }
}
