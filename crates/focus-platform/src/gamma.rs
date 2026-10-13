//! Warm screen through each display's gamma ramp (colour temperature for the whole screen: taskbar, every app and
//! full-screen games; it acts after composition, so screenshots keep their true colours).
//!
//! Windows implementation (GDI, per user, no admin rights):
//! * `EnumDisplayDevicesW` → the displays attached to the desktop (mirroring drivers skipped),
//! * `CreateDCW("DISPLAY", \\.\DISPLAYn)` → `GetDeviceGammaRamp` / `SetDeviceGammaRamp` → `DeleteDC`.
//!
//! [`GammaRamp::apply`] never builds on the ramp currently on the screen: it scales the ramp recorded by
//! [`GammaRamp::snapshot`] (the original) per channel, so repeated applies do not compound and an identity gain
//! gives the original back. A display that appears later is recorded the first time it is seen.
//!
//! Limits:
//! * Windows only accepts ramps close enough to the identity (unless the machine-wide gamma range was unlocked
//!   by an administrator): on a Windows 11 build 26200 test machine every channel gain had to stay at 0.5 or
//!   above, so about 3300 K is the warmest; 3400 K went through in full, 1900 K was capped. When the full warmth
//!   is refused, `apply` retries with gains closer to 1.0 (5 % steps) and reports [`GammaResult::capped`].
//! * HDR displays ignore GDI gamma ramps (Windows accepts the call, nothing changes).
//! * A display mode change, driver reset or sign-out resets the ramps; nothing outlives the session, so a crash
//!   leaves the screen warm only until FocusMode's next start restores the persisted [`GammaSnapshot`].

use serde::{Deserialize, Serialize};

use crate::PlatformError;

/// Entries per channel of a GDI gamma ramp.
pub const RAMP_LEN: usize = 256;

/// Fractions of the requested warmth tried in turn when Windows refuses a ramp (1.0, 0.95, ..., 0.05). A refused
/// ramp changes nothing on screen, so only the accepted one is ever visible.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
const STRENGTHS: [f32; 20] = {
    let mut steps = [0.0f32; 20];
    let mut index = 0;
    while index < steps.len() {
        steps[index] = (20 - index) as f32 / 20.0;
        index += 1;
    }
    steps
};

/// One display's gamma ramp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayGamma {
    /// GDI device name, e.g. `\\.\DISPLAY1`.
    pub device: String,
    /// [`RAMP_LEN`] entries each.
    pub red: Vec<u16>,
    pub green: Vec<u16>,
    pub blue: Vec<u16>,
}

impl DisplayGamma {
    /// All three channels have [`RAMP_LEN`] entries (a restore file may have been edited).
    pub fn is_valid(&self) -> bool {
        self.red.len() == RAMP_LEN && self.green.len() == RAMP_LEN && self.blue.len() == RAMP_LEN
    }

    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    fn from_ramp(device: String, ramp: &Ramp) -> Self {
        Self {
            device,
            red: ramp[0].to_vec(),
            green: ramp[1].to_vec(),
            blue: ramp[2].to_vec(),
        }
    }

    /// The ramp with each channel multiplied by its gain (rounded, saturating). `None` when invalid.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    fn scaled(&self, gain: [f32; 3]) -> Option<Ramp> {
        if !self.is_valid() {
            return None;
        }
        let mut ramp = [[0u16; RAMP_LEN]; 3];
        for (out, (channel, gain)) in ramp
            .iter_mut()
            .zip([&self.red, &self.green, &self.blue].into_iter().zip(gain))
        {
            for (slot, &value) in out.iter_mut().zip(channel.iter()) {
                *slot = (f32::from(value) * gain).round().clamp(0.0, 65_535.0) as u16;
            }
        }
        Some(ramp)
    }
}

/// The gamma ramps of all displays before FocusMode changed them; persisted by the shell for crash-restore.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GammaSnapshot {
    pub displays: Vec<DisplayGamma>,
}

/// Outcome of [`GammaRamp::apply`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GammaResult {
    /// Windows refused the full warmth on at least one display and a weaker one is applied there (or none).
    pub capped: bool,
}

/// Red, green and blue tables in the layout `GetDeviceGammaRamp` / `SetDeviceGammaRamp` use.
type Ramp = [[u16; RAMP_LEN]; 3];

/// Scales display gamma ramps (warm screen) and puts the originals back.
pub trait GammaRamp: Send {
    /// Reads the ramps of all active displays and remembers them as the originals for [`apply`](Self::apply).
    /// Changes nothing. Persist the result before the first `apply`.
    fn snapshot(&mut self) -> Result<GammaSnapshot, PlatformError>;
    /// Sets every display to its original ramp multiplied by `gain` (`[r, g, b]`, each 0..=1; `[1, 1, 1]` =
    /// original). Without an earlier [`snapshot`](Self::snapshot) the current ramps count as the originals.
    /// Fails only when no display accepted a ramp.
    fn apply(&mut self, gain: [f32; 3]) -> Result<GammaResult, PlatformError>;
    /// Puts back the ramps in `snap` on the displays that are still attached (and makes them the originals).
    /// A display that [`apply`](Self::apply) first saw after the snapshot (connected later) goes back to the ramp
    /// recorded then, so no display is left warm.
    fn restore(&mut self, snap: &GammaSnapshot) -> Result<(), PlatformError>;
    /// The originals recorded so far: the snapshot plus displays first seen by [`apply`](Self::apply) afterwards.
    /// Persist it again when it grows, so a crash-restore also covers a display connected after the snapshot.
    fn originals(&self) -> GammaSnapshot;
}

/// Platform implementation: GDI gamma ramps on Windows, or [`NoopGamma`] elsewhere.
pub fn gamma() -> Box<dyn GammaRamp> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows_impl::WindowsGamma::new())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Box::new(NoopGamma)
    }
}

/// Gamma control for unsupported platforms: `snapshot`/`apply` fail with [`PlatformError::Unsupported`],
/// `restore` is a successful no-op (nothing was changed).
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopGamma;

impl GammaRamp for NoopGamma {
    fn snapshot(&mut self) -> Result<GammaSnapshot, PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn apply(&mut self, _gain: [f32; 3]) -> Result<GammaResult, PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn restore(&mut self, _snap: &GammaSnapshot) -> Result<(), PlatformError> {
        Ok(())
    }

    fn originals(&self) -> GammaSnapshot {
        GammaSnapshot::default()
    }
}

impl GammaSnapshot {
    /// The valid displays of this snapshot, then the valid displays of `recorded` that it does not cover
    /// (first seen after it, e.g. [`GammaRamp::originals`]). This snapshot wins for a display in both: its ramp
    /// was read before any change.
    pub fn merged_with(&self, recorded: &[DisplayGamma]) -> GammaSnapshot {
        let mut displays: Vec<DisplayGamma> = self
            .displays
            .iter()
            .filter(|display| display.is_valid())
            .cloned()
            .collect();
        for display in recorded {
            if display.is_valid() && !displays.iter().any(|known| known.device == display.device) {
                displays.push(display.clone());
            }
        }
        GammaSnapshot { displays }
    }
}

/// Each gain limited to 0..=1 (NaN and infinities become 1: no change).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn sanitize_gain(gain: [f32; 3]) -> [f32; 3] {
    gain.map(|value| {
        if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            1.0
        }
    })
}

/// The gain moved towards 1.0: `strength` 1 keeps it, 0 gives `[1, 1, 1]`.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn weaken(gain: [f32; 3], strength: f32) -> [f32; 3] {
    gain.map(|value| 1.0 - (1.0 - value) * strength)
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::ffi::c_void;

    use windows::Win32::Graphics::Gdi::{
        CreateDCW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_MIRRORING_DRIVER,
        DISPLAY_DEVICEW, DeleteDC, EnumDisplayDevicesW, HDC,
    };
    use windows::Win32::UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp};
    use windows::core::{PCWSTR, w};

    use super::{
        DisplayGamma, GammaRamp, GammaResult, GammaSnapshot, RAMP_LEN, Ramp, STRENGTHS,
        sanitize_gain, weaken,
    };
    use crate::PlatformError;

    /// Upper bound of display devices enumerated.
    const MAX_DEVICES: u32 = 64;

    /// Deletes a device context on drop.
    struct OwnedDc(HDC);

    impl Drop for OwnedDc {
        fn drop(&mut self) {
            // SAFETY: the DC came from a successful CreateDCW and is deleted exactly once.
            let _ = unsafe { DeleteDC(self.0) };
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn until_nul(units: &[u16]) -> String {
        let len = units
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(units.len());
        String::from_utf16_lossy(&units[..len])
    }

    /// GDI device names of the displays attached to the desktop.
    pub(super) fn active_displays() -> Vec<String> {
        let mut names = Vec::new();
        for index in 0..MAX_DEVICES {
            let mut device = DISPLAY_DEVICEW {
                cb: size_of::<DISPLAY_DEVICEW>() as u32,
                ..Default::default()
            };
            // SAFETY: `device` is a valid DISPLAY_DEVICEW with `cb` set; a null device name enumerates adapters.
            let found = unsafe { EnumDisplayDevicesW(PCWSTR::null(), index, &mut device, 0) };
            if !found.as_bool() {
                break;
            }
            let flags = device.StateFlags;
            if flags.0 & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP.0 != 0
                && flags.0 & DISPLAY_DEVICE_MIRRORING_DRIVER.0 == 0
            {
                let name = until_nul(&device.DeviceName);
                if !name.is_empty() {
                    names.push(name);
                }
            }
        }
        names
    }

    fn open_dc(device: &str) -> Result<OwnedDc, PlatformError> {
        let name = wide(device);
        // SAFETY: both strings are NUL-terminated and outlive the call; no DEVMODE (current settings).
        let dc = unsafe { CreateDCW(w!("DISPLAY"), PCWSTR(name.as_ptr()), PCWSTR::null(), None) };
        if dc.is_invalid() {
            return Err(PlatformError::Os(format!("CreateDC failed for {device}")));
        }
        Ok(OwnedDc(dc))
    }

    pub(super) fn read_ramp(device: &str) -> Result<Ramp, PlatformError> {
        let dc = open_dc(device)?;
        let mut ramp: Ramp = [[0u16; RAMP_LEN]; 3];
        // SAFETY: `ramp` is 3 × 256 contiguous WORDs, the buffer layout GetDeviceGammaRamp writes.
        let ok = unsafe { GetDeviceGammaRamp(dc.0, ramp.as_mut_ptr().cast::<c_void>()) };
        if !ok.as_bool() {
            return Err(PlatformError::Os(format!(
                "GetDeviceGammaRamp failed for {device}"
            )));
        }
        Ok(ramp)
    }

    /// `Err` when Windows refuses the ramp (outside the allowed range) or the display is gone.
    fn write_ramp(dc: &OwnedDc, device: &str, ramp: &Ramp) -> Result<(), PlatformError> {
        // SAFETY: `ramp` is 3 × 256 contiguous WORDs, the layout SetDeviceGammaRamp reads; it is only read.
        let ok = unsafe { SetDeviceGammaRamp(dc.0, ramp.as_ptr().cast::<c_void>()) };
        if !ok.as_bool() {
            return Err(PlatformError::Os(format!(
                "SetDeviceGammaRamp refused the ramp for {device}"
            )));
        }
        Ok(())
    }

    /// The step Windows accepted last time for a display and gain.
    struct Accepted {
        device: String,
        gain: [f32; 3],
        step: usize,
    }

    pub(super) struct WindowsGamma {
        /// Original ramps (from `snapshot`, `restore`, or the first sight of a display).
        base: Vec<DisplayGamma>,
        /// Last accepted step per display, so re-applying a capped gain (every tick) costs one call, not twenty.
        accepted: Vec<Accepted>,
    }

    impl WindowsGamma {
        pub(super) fn new() -> Self {
            Self {
                base: Vec::new(),
                accepted: Vec::new(),
            }
        }

        /// First step to try: the one accepted last time for the same display and gain, otherwise full strength.
        fn first_step(&self, device: &str, gain: [f32; 3]) -> usize {
            self.accepted
                .iter()
                .find(|known| known.device == device && known.gain == gain)
                .map_or(0, |known| known.step)
        }

        fn remember(&mut self, device: &str, gain: [f32; 3], step: usize) {
            self.accepted.retain(|known| known.device != device);
            self.accepted.push(Accepted {
                device: device.to_owned(),
                gain,
                step,
            });
        }

        /// The original ramp of `device`, recording the current one when the display is new.
        fn base_for(&mut self, device: &str) -> Result<DisplayGamma, PlatformError> {
            if let Some(base) = self
                .base
                .iter()
                .find(|base| base.device == device && base.is_valid())
            {
                return Ok(base.clone());
            }
            let base = DisplayGamma::from_ramp(device.to_owned(), &read_ramp(device)?);
            self.base.retain(|known| known.device != device);
            self.base.push(base.clone());
            Ok(base)
        }
    }

    impl GammaRamp for WindowsGamma {
        fn snapshot(&mut self) -> Result<GammaSnapshot, PlatformError> {
            let mut displays = Vec::new();
            let mut last_error = None;
            for device in active_displays() {
                match read_ramp(&device) {
                    Ok(ramp) => displays.push(DisplayGamma::from_ramp(device, &ramp)),
                    Err(err) => {
                        log::warn!("reading the gamma ramp failed: {err}");
                        last_error = Some(err);
                    }
                }
            }
            if displays.is_empty() {
                return Err(
                    last_error.unwrap_or_else(|| PlatformError::Os("no display attached".into()))
                );
            }
            self.base.clone_from(&displays);
            self.accepted.clear();
            Ok(GammaSnapshot { displays })
        }

        fn apply(&mut self, gain: [f32; 3]) -> Result<GammaResult, PlatformError> {
            let gain = sanitize_gain(gain);
            let mut result = GammaResult::default();
            let mut applied = 0usize;
            let mut last_error = None;
            for device in active_displays() {
                let (base, dc) = match self.base_for(&device).and_then(|base| {
                    let dc = open_dc(&device)?;
                    Ok((base, dc))
                }) {
                    Ok(pair) => pair,
                    Err(err) => {
                        last_error = Some(err);
                        continue;
                    }
                };
                let first = self.first_step(&device, gain);
                let mut accepted = None;
                for (step, strength) in STRENGTHS.into_iter().enumerate().skip(first) {
                    let Some(ramp) = base.scaled(weaken(gain, strength)) else {
                        break;
                    };
                    match write_ramp(&dc, &device, &ramp) {
                        Ok(()) => {
                            accepted = Some(step);
                            break;
                        }
                        Err(err) => last_error = Some(err),
                    }
                }
                match accepted {
                    Some(step) => {
                        if step > 0 {
                            if step != first {
                                log::info!(
                                    "windows limited the gamma warmth on {device} to {:.0}%",
                                    STRENGTHS[step] * 100.0
                                );
                            }
                            result.capped = true;
                        }
                        self.remember(&device, gain, step);
                        applied += 1;
                    }
                    None => {
                        // Even the weakest warmth was refused: leave this display as it was.
                        result.capped = true;
                        if let Some(original) = base.scaled([1.0; 3]) {
                            let _ = write_ramp(&dc, &device, &original);
                        }
                    }
                }
            }
            if applied == 0 {
                return Err(
                    last_error.unwrap_or_else(|| PlatformError::Os("no display attached".into()))
                );
            }
            Ok(result)
        }

        fn restore(&mut self, snap: &GammaSnapshot) -> Result<(), PlatformError> {
            // Displays first seen by `apply` after the snapshot were warmed too: they go back as well.
            let originals = snap.merged_with(&self.base).displays;
            let mut last_error = None;
            for device in active_displays() {
                let Some(original) = originals.iter().find(|display| display.device == device)
                else {
                    continue;
                };
                if let Some(ramp) = original.scaled([1.0; 3])
                    && let Err(err) =
                        open_dc(&device).and_then(|dc| write_ramp(&dc, &device, &ramp))
                {
                    log::warn!("restoring the gamma ramp failed: {err}");
                    last_error = Some(err);
                }
            }
            self.base = originals;
            self.accepted.clear();
            match last_error {
                Some(err) => Err(err),
                None => Ok(()),
            }
        }

        fn originals(&self) -> GammaSnapshot {
            GammaSnapshot {
                displays: self
                    .base
                    .iter()
                    .filter(|display| display.is_valid())
                    .cloned()
                    .collect(),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::{DisplayGamma, GammaRamp};
        use super::{WindowsGamma, active_displays, read_ramp};

        #[test]
        fn reading_ramps_does_not_fail() {
            // A headless session may have no display; reading must still not panic.
            for device in active_displays() {
                // Some virtual displays have no ramp; that is an error, not a panic.
                if let Ok(ramp) = read_ramp(&device) {
                    assert_eq!(ramp.len(), 3);
                }
            }
            let mut gamma = WindowsGamma::new();
            let _ = gamma.snapshot();
        }

        /// Turns the real screen warm for a moment, then puts the original ramps back — run manually:
        /// `cargo test -p focus-platform -- --ignored gamma_round_trip --nocapture`
        #[test]
        #[ignore = "changes the real display gamma for a moment"]
        fn gamma_round_trip() {
            let mut gamma = WindowsGamma::new();
            let snap = gamma.snapshot().expect("snapshot");
            assert!(!snap.displays.is_empty());

            // 3400 K (focus_core::tint::rgb_gain_for_kelvin).
            let result = gamma.apply([1.0, 0.746, 0.541]).expect("apply");
            eprintln!("3400 K capped: {}", result.capped);
            for original in &snap.displays {
                let now = read_ramp(&original.device).expect("read back");
                let now = DisplayGamma::from_ramp(original.device.clone(), &now);
                assert_eq!(now.red, original.red, "red keeps its gain of 1");
                assert!(now.green.iter().zip(&original.green).all(|(a, b)| a <= b));
                assert!(now.blue.iter().zip(&original.blue).all(|(a, b)| a <= b));
                assert!(
                    now.blue.iter().zip(&original.blue).any(|(a, b)| a < b),
                    "blue is weaker on {}",
                    original.device
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(500));

            // 1900 K: usually beyond what Windows allows without an unlocked range.
            let warmest = [1.0, 0.518, 0.0];
            let result = gamma.apply(warmest).expect("apply warmest");
            eprintln!("1900 K capped: {}", result.capped);
            let first: Vec<_> = snap
                .displays
                .iter()
                .map(|display| read_ramp(&display.device).expect("read back"))
                .collect();
            // Re-applying the same gain (every tick in the shell) starts at the accepted step: same ramps.
            let again = gamma.apply(warmest).expect("apply warmest again");
            assert_eq!(again, result);
            for (display, ramp) in snap.displays.iter().zip(&first) {
                assert_eq!(read_ramp(&display.device).expect("read back"), *ramp);
            }
            std::thread::sleep(std::time::Duration::from_millis(500));

            gamma.restore(&snap).expect("restore");
            for original in &snap.displays {
                let now = read_ramp(&original.device).expect("read back");
                assert_eq!(
                    DisplayGamma::from_ramp(original.device.clone(), &now),
                    *original,
                    "original ramp is back"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> DisplayGamma {
        let channel: Vec<u16> = (0..RAMP_LEN as u32).map(|i| (i * 257) as u16).collect();
        DisplayGamma {
            device: r"\\.\DISPLAY1".into(),
            red: channel.clone(),
            green: channel.clone(),
            blue: channel,
        }
    }

    #[test]
    fn scaling_multiplies_each_channel_of_the_original() {
        let base = identity();
        let ramp = base.scaled([1.0, 0.5, 0.0]).expect("valid");
        assert_eq!(ramp[0].to_vec(), base.red);
        assert_eq!(ramp[1][255], 32_768); // 65535 * 0.5 rounded
        assert_eq!(ramp[1][0], 0);
        assert!(ramp[2].iter().all(|&value| value == 0));
        // Identity gain gives the original back exactly.
        let same = base.scaled([1.0; 3]).expect("valid");
        assert_eq!(DisplayGamma::from_ramp(base.device.clone(), &same), base);
    }

    #[test]
    fn invalid_ramps_are_not_scaled() {
        let mut broken = identity();
        broken.blue.pop();
        assert!(!broken.is_valid());
        assert!(broken.scaled([1.0; 3]).is_none());
    }

    #[test]
    fn gains_are_sanitized() {
        assert_eq!(sanitize_gain([f32::NAN, 2.0, -1.0]), [1.0, 1.0, 0.0]);
        assert_eq!(sanitize_gain([1.0, 0.746, 0.541]), [1.0, 0.746, 0.541]);
        assert_eq!(sanitize_gain([f32::INFINITY; 3]), [1.0; 3]);
    }

    #[test]
    fn weakening_moves_towards_identity() {
        let gain = [1.0, 0.5, 0.0];
        assert_eq!(weaken(gain, 1.0), gain);
        assert_eq!(weaken(gain, 0.0), [1.0; 3]);
        assert_eq!(weaken(gain, 0.5), [1.0, 0.75, 0.5]);
        // Every step is weaker than the one before, from the full warmth down to 5 %.
        assert_eq!(STRENGTHS.first(), Some(&1.0));
        assert_eq!(STRENGTHS.last(), Some(&0.05));
        let blues: Vec<f32> = STRENGTHS.iter().map(|&s| weaken(gain, s)[2]).collect();
        assert!(blues.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn snapshot_json_round_trip() {
        let snap = GammaSnapshot {
            displays: vec![identity()],
        };
        let json = serde_json::to_string(&snap).expect("serialize");
        assert!(json.starts_with(r#"{"displays":[{"device":"\\\\.\\DISPLAY1","red":[0,257,"#));
        let back: GammaSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, snap);
    }

    #[test]
    fn displays_seen_after_the_snapshot_are_restored_too() {
        let first = identity();
        let mut later = identity();
        later.device = r"\\.\DISPLAY2".into();
        later.blue[255] = 60_000;
        let mut stale = identity();
        stale.green[0] = 9;
        let mut broken = identity();
        broken.device = r"\\.\DISPLAY3".into();
        broken.red.pop();
        let snap = GammaSnapshot {
            displays: vec![first.clone()],
        };

        // The snapshot wins for a display it covers; a later display keeps the ramp recorded at first sight.
        let merged = snap.merged_with(&[stale, later.clone(), broken]);
        assert_eq!(merged.displays, vec![first, later]);
        assert_eq!(snap.merged_with(&[]), snap);
        assert_eq!(merged.merged_with(&snap.displays), merged);
        assert!(
            GammaSnapshot::default()
                .merged_with(&[])
                .displays
                .is_empty()
        );
    }

    #[test]
    fn noop_gamma() {
        let mut gamma = NoopGamma;
        assert!(matches!(gamma.snapshot(), Err(PlatformError::Unsupported)));
        assert!(matches!(
            gamma.apply([1.0, 0.8, 0.6]),
            Err(PlatformError::Unsupported)
        ));
        assert!(gamma.restore(&GammaSnapshot::default()).is_ok());
        assert_eq!(gamma.originals(), GammaSnapshot::default());
    }

    #[test]
    fn platform_gamma_is_constructible() {
        let _gamma = gamma();
    }
}
