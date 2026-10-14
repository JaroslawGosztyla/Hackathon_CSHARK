//! Windows Night light (blue light reduction), switched on and set by FocusMode on its warm-screen schedule.
//!
//! # Mechanism (researched 2026-10)
//!
//! Windows has **no documented API** for Night light. Its state and settings live in two per-user CloudStore
//! registry values (no admin rights), `Data` (REG_BINARY) under
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\CloudStore\Store\DefaultAccount\Current\`:
//!
//! * `default$windows.data.bluelightreduction.bluelightreductionstate\windows.data.bluelightreduction.bluelightreductionstate`
//! * `default$windows.data.bluelightreduction.settings\windows.data.bluelightreduction.settings`
//!
//! Both are Microsoft Bond CompactBinary v1 (see [`crate::bond`]): an envelope
//! `{0: {0: bool = true}, 1: {0: uint64 = Unix seconds of the change, 1: {1: list<int8> = payload}}}` around a
//! payload struct:
//!
//! * state: `0: int32 = 0` while Night light is on (absent = off), `10: int32 = 1` when the last switch was manual
//!   (absent = by the schedule), `20: uint64` = FILETIME of the last switch;
//! * settings: `0: bool` schedule on, `10: bool = false` "set hours" (absent = sunset to sunrise), `20`/`30`:
//!   scheduled start/end `{0: int8 hour, 1: int8 minute}`, `40: int16` night colour temperature in kelvin (absent =
//!   the Windows default, about 4000 K; the slider spans 6500 K..1200 K), `50`/`60`: sunset/sunrise times,
//!   `70: bool = true` preview (Settings sets it while the slider is dragged, so a temperature change shows at once).
//!
//! Sources: the reverse-engineered notes of `enyium/sem-reg-rs` (2023), the Bond codec of twinkle-tray PR #1338
//! (2026), and the values on a Windows 11 build 26200 machine. On Windows 10 and Windows 11 up to 23H2 a write is
//! applied at once, provided the envelope timestamp is newer than the stored one (an older one is undone).
//!
//! # Windows 11 24H2 and later: unavailable
//!
//! Build 26100+ also keeps per-device copies (`{device-id}$windows.data.bluelightreduction.*perdevice`: the same
//! payload followed by Bond's end-of-base marker) and the CloudStore no longer treats these registry values as the
//! live store. Verified on build 26200 (2026-10-13): values written here were picked up neither by the Night light
//! engine nor by the Settings page (also after restarting it); Settings' own changes never reached the registry;
//! and while a value written here was stored, Settings' "Turn on now" silently did nothing until the original
//! bytes were put back. So on these builds (or whenever per-device records exist) [`NightLight::available`] is
//! `false` and nothing is written; the shell falls back to gamma ramps ([`crate::gamma`]).
//!
//! # Rules
//!
//! * Both values are decoded completely against the schema above before anything is written. An unknown field,
//!   type, value or layout makes the integration unavailable; a partially understood value is never rewritten.
//! * FocusMode only changes state fields 0/10/20 and settings fields 40/70; the user's schedule is never touched.
//! * [`NightLightSnapshot`] keeps the raw values (base64) so the shell can persist it before the first change
//!   (crash-restore). Restore puts back the snapshot's on/off state (its exact state payload) and its colour
//!   temperature, and clears the preview flag; everything else stays as it is now.
//! * Writes are blocking and may sleep for a few hundred milliseconds (see [`NightLight::apply`]): call from a worker.

use serde::{Deserialize, Serialize};

use crate::PlatformError;
use crate::bond::{self, BondStruct, Value};

/// [`NightLightSnapshot::method`] of the CloudStore registry implementation.
pub const METHOD_CLOUD_STORE: &str = "cloudStore";
/// Warmest colour temperature the Night light slider offers.
pub const MIN_NIGHT_LIGHT_KELVIN: u32 = 1200;
/// Coldest (neutral) colour temperature.
pub const MAX_NIGHT_LIGHT_KELVIN: u32 = 6500;
/// First Windows build whose CloudStore no longer applies the registry values (Windows 11 24H2).
pub const FIRST_UNSUPPORTED_BUILD: u32 = 26_100;

/// The Night light values before FocusMode changed them; persisted by the shell for crash-restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NightLightSnapshot {
    /// Implementation that produced the snapshot ([`METHOD_CLOUD_STORE`]).
    pub method: String,
    /// The state value as read, standard base64.
    pub state: String,
    /// The settings value as read, standard base64.
    pub settings: String,
}

/// Switches Windows Night light and sets its colour temperature.
pub trait NightLight: Send {
    /// Night light can be controlled on this machine (may read the registry; cached for a minute).
    fn available(&self) -> bool;
    /// Reads the current values without changing anything. Persist the result before the first
    /// [`apply`](Self::apply).
    fn snapshot(&mut self) -> Result<NightLightSnapshot, PlatformError>;
    /// `on`: sets the night colour temperature to `kelvin` (clamped to 1200..=6500) and switches Night light on.
    /// `!on`: switches it off and leaves the temperature alone. Blocks for up to about half a second.
    fn apply(&mut self, on: bool, kelvin: u32) -> Result<(), PlatformError>;
    /// Puts back the on/off state and colour temperature recorded in `snap`. Idempotent; a snapshot this
    /// implementation cannot use (other method, corrupted) is reported as success so the shell can drop it.
    fn restore(&mut self, snap: &NightLightSnapshot) -> Result<(), PlatformError>;
}

/// Platform implementation: the CloudStore registry values on Windows, or [`NoopNightLight`] elsewhere.
pub fn night_light() -> Box<dyn NightLight> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows_impl::WindowsNightLight::new())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Box::new(NoopNightLight)
    }
}

/// Night light for unsupported platforms: unavailable, `snapshot`/`apply` fail with
/// [`PlatformError::Unsupported`], `restore` is a successful no-op (nothing was changed).
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopNightLight;

impl NightLight for NoopNightLight {
    fn available(&self) -> bool {
        false
    }

    fn snapshot(&mut self) -> Result<NightLightSnapshot, PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn apply(&mut self, _on: bool, _kelvin: u32) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }

    fn restore(&mut self, _snap: &NightLightSnapshot) -> Result<(), PlatformError> {
        Ok(())
    }
}

/// `kelvin` limited to the range of the Night light slider.
pub fn clamp_kelvin(kelvin: u32) -> u32 {
    kelvin.clamp(MIN_NIGHT_LIGHT_KELVIN, MAX_NIGHT_LIGHT_KELVIN)
}

// ---------------------------------------------------------------------------
// Value format (platform independent, unit tested)
// ---------------------------------------------------------------------------

/// State field: Night light on (`int32 = 0`).
const STATE_ON: u16 = 0;
/// State field: last switch was manual (`int32 = 1`).
const STATE_MANUAL: u16 = 10;
/// State field: FILETIME of the last switch (`uint64`).
const STATE_CHANGED_AT: u16 = 20;
/// Settings field: night colour temperature in kelvin (`int16`).
const SETTINGS_KELVIN: u16 = 40;
/// Settings field: preview, apply a temperature change at once (`bool`).
const SETTINGS_PREVIEW: u16 = 70;
/// Seconds between 1601-01-01 (FILETIME epoch) and 1970-01-01.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
const FILETIME_UNIX_OFFSET_SECS: u64 = 11_644_473_600;

fn unsupported(detail: impl std::fmt::Display) -> PlatformError {
    PlatformError::Os(format!("unsupported Night light value: {detail}"))
}

/// A decoded CloudStore value: the envelope timestamp and the payload struct.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CloudValue {
    /// Unix seconds of the last change.
    timestamp: u64,
    payload: BondStruct,
}

fn only_field(value: &BondStruct, id: u16) -> Option<&Value> {
    match (value.fields.as_slice(), &value.derived) {
        ([field], None) if field.id == id => Some(&field.value),
        _ => None,
    }
}

fn decode_value(bytes: &[u8]) -> Result<CloudValue, PlatformError> {
    let outer = bond::decode_marshalled(bytes).map_err(unsupported)?;
    let envelope = || unsupported("unknown CloudStore envelope");
    let ([meta_field, container_field], None) = (outer.fields.as_slice(), &outer.derived) else {
        return Err(envelope());
    };
    let (0, Value::Struct(meta), 1, Value::Struct(container)) = (
        meta_field.id,
        &meta_field.value,
        container_field.id,
        &container_field.value,
    ) else {
        return Err(envelope());
    };
    if only_field(meta, 0) != Some(&Value::Bool(true)) {
        return Err(envelope());
    }
    let ([stamp, wrapper_field], None) = (container.fields.as_slice(), &container.derived) else {
        return Err(envelope());
    };
    let (0, Value::UInt64(timestamp), 1, Value::Struct(wrapper)) = (
        stamp.id,
        &stamp.value,
        wrapper_field.id,
        &wrapper_field.value,
    ) else {
        return Err(envelope());
    };
    let Some(Value::Bytes(payload)) = only_field(wrapper, 1) else {
        return Err(envelope());
    };
    let payload = bond::decode_marshalled(payload).map_err(unsupported)?;
    Ok(CloudValue {
        timestamp: *timestamp,
        payload,
    })
}

fn encode_value(value: &CloudValue) -> Vec<u8> {
    let field = |id, value| bond::Field { id, value };
    let single = |id, value| BondStruct {
        fields: vec![field(id, value)],
        derived: None,
    };
    let outer = BondStruct {
        fields: vec![
            field(0, Value::Struct(single(0, Value::Bool(true)))),
            field(
                1,
                Value::Struct(BondStruct {
                    fields: vec![
                        field(0, Value::UInt64(value.timestamp)),
                        field(
                            1,
                            Value::Struct(single(
                                1,
                                Value::Bytes(bond::encode_marshalled(&value.payload)),
                            )),
                        ),
                    ],
                    derived: None,
                }),
            ),
        ],
        derived: None,
    };
    bond::encode_marshalled(&outer)
}

/// The base fields and the per-device flag: the per-device variant (Windows 11 24H2+) derives from the shared
/// schema and must not add fields of its own.
fn base_fields(payload: &BondStruct) -> Result<(&[bond::Field], bool), PlatformError> {
    match &payload.derived {
        None => Ok((&payload.fields, false)),
        Some(derived) if derived.is_empty() => Ok((&payload.fields, true)),
        Some(_) => Err(unsupported("per-device fields")),
    }
}

/// Decoded state payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NightState {
    on: bool,
    /// Last switch was manual (otherwise by the schedule).
    manual: bool,
    /// FILETIME of the last switch.
    changed_at: Option<u64>,
    /// Per-device variant (end-of-base marker).
    per_device: bool,
}

fn parse_state(payload: &BondStruct) -> Result<NightState, PlatformError> {
    let (fields, per_device) = base_fields(payload)?;
    let mut state = NightState {
        on: false,
        manual: false,
        changed_at: None,
        per_device,
    };
    for field in fields {
        match (field.id, &field.value) {
            (STATE_ON, Value::Int32(0)) => state.on = true,
            (STATE_MANUAL, Value::Int32(1)) => state.manual = true,
            (STATE_CHANGED_AT, Value::UInt64(at)) => state.changed_at = Some(*at),
            (id, value) => return Err(unsupported(format!("state field {id} = {value:?}"))),
        }
    }
    Ok(state)
}

fn build_state(state: &NightState) -> BondStruct {
    let mut fields = Vec::new();
    if state.on {
        fields.push(bond::Field {
            id: STATE_ON,
            value: Value::Int32(0),
        });
    }
    if state.manual {
        fields.push(bond::Field {
            id: STATE_MANUAL,
            value: Value::Int32(1),
        });
    }
    if let Some(at) = state.changed_at {
        fields.push(bond::Field {
            id: STATE_CHANGED_AT,
            value: Value::UInt64(at),
        });
    }
    BondStruct {
        fields,
        derived: state.per_device.then(Vec::new),
    }
}

fn validate_time(value: &Value) -> bool {
    let Value::Struct(time) = value else {
        return false;
    };
    time.derived.is_none()
        && time
            .fields
            .iter()
            .all(|field| match (field.id, &field.value) {
                (0, Value::Int8(hour)) => (0..24).contains(hour),
                (1, Value::Int8(minute)) => (0..60).contains(minute),
                _ => false,
            })
}

fn validate_settings(payload: &BondStruct) -> Result<(), PlatformError> {
    let (fields, _) = base_fields(payload)?;
    for field in fields {
        let known = match (field.id, &field.value) {
            (0 | 10 | SETTINGS_PREVIEW, Value::Bool(_)) => true,
            (20 | 30 | 50 | 60, value) => validate_time(value),
            (SETTINGS_KELVIN, Value::Int16(kelvin)) => *kelvin > 0,
            _ => false,
        };
        if !known {
            return Err(unsupported(format!(
                "settings field {} = {:?}",
                field.id, field.value
            )));
        }
    }
    Ok(())
}

/// Night colour temperature stored in the settings (`None` = Windows default).
fn settings_kelvin(payload: &BondStruct) -> Option<u32> {
    match payload.get(SETTINGS_KELVIN) {
        Some(Value::Int16(kelvin)) => u32::try_from(*kelvin).ok(),
        _ => None,
    }
}

fn settings_preview(payload: &BondStruct) -> bool {
    payload.get(SETTINGS_PREVIEW) == Some(&Value::Bool(true))
}

/// The settings payload with the temperature set (`None` removes it: Windows default) and the preview flag set or
/// removed. Every other field is kept as it is.
fn with_kelvin(payload: &BondStruct, kelvin: Option<u32>, preview: bool) -> BondStruct {
    let mut out = payload.clone();
    match kelvin.and_then(|kelvin| i16::try_from(clamp_kelvin(kelvin)).ok()) {
        Some(kelvin) => out.set(SETTINGS_KELVIN, Value::Int16(kelvin)),
        None => out.remove(SETTINGS_KELVIN),
    }
    if preview {
        out.set(SETTINGS_PREVIEW, Value::Bool(true));
    } else {
        out.remove(SETTINGS_PREVIEW);
    }
    out
}

/// Envelope timestamp for a new write: now, but always after the stored one (Windows undoes older writes).
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn next_timestamp(stored: u64, now_unix: u64) -> u64 {
    now_unix.max(stored.saturating_add(1))
}

/// Decoded state and settings of a snapshot.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn decode_snapshot(snap: &NightLightSnapshot) -> Result<(NightState, BondStruct), PlatformError> {
    if snap.method != METHOD_CLOUD_STORE {
        return Err(unsupported(format!("snapshot method {:?}", snap.method)));
    }
    let raw = |text: &str| {
        crate::base64::decode(text).ok_or_else(|| unsupported("snapshot is not base64"))
    };
    let state = decode_value(&raw(&snap.state)?)?;
    let settings = decode_value(&raw(&snap.settings)?)?;
    validate_settings(&settings.payload)?;
    Ok((parse_state(&state.payload)?, settings.payload))
}

/// `Ok` when the CloudStore registry values are the live store on this machine; `Err(reason)` otherwise.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn registry_is_live(build: Option<u32>, per_device_records: bool) -> Result<(), String> {
    if per_device_records {
        return Err("per-device Night light records exist (Windows 11 24H2+ CloudStore)".into());
    }
    match build {
        Some(build) if build >= FIRST_UNSUPPORTED_BUILD => Err(format!(
            "Windows build {build} no longer applies the Night light registry values"
        )),
        _ => Ok(()),
    }
}

/// Name of a per-device Night light record key under `...\DefaultAccount\Current`.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn is_per_device_key(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.ends_with("$windows.data.bluelightreduction.bluelightreductionstateperdevice")
        || name.ends_with("$windows.data.bluelightreduction.settingsperdevice")
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_ITEMS};
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_ENUMERATE_SUB_KEYS, REG_BINARY,
        RRF_RT_REG_BINARY, RRF_RT_REG_SZ, RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW,
        RegSetKeyValueW,
    };
    use windows::core::{PCWSTR, PWSTR, w};

    use super::{
        CloudValue, FILETIME_UNIX_OFFSET_SECS, METHOD_CLOUD_STORE, NightLight, NightLightSnapshot,
        NightState, decode_snapshot, decode_value, encode_value, is_per_device_key, next_timestamp,
        parse_state, registry_is_live, settings_kelvin, settings_preview, validate_settings,
        with_kelvin,
    };
    use crate::PlatformError;
    use crate::bond::BondStruct;

    pub(super) const CURRENT: &str =
        r"Software\Microsoft\Windows\CurrentVersion\CloudStore\Store\DefaultAccount\Current";
    pub(super) const STATE_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\CloudStore\Store\DefaultAccount\Current\default$windows.data.bluelightreduction.bluelightreductionstate\windows.data.bluelightreduction.bluelightreductionstate";
    pub(super) const SETTINGS_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\CloudStore\Store\DefaultAccount\Current\default$windows.data.bluelightreduction.settings\windows.data.bluelightreduction.settings";
    const NT_CURRENT_VERSION: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    /// Largest value accepted (real values are under 100 bytes).
    const MAX_VALUE_BYTES: usize = 4096;
    /// How long an availability probe is trusted.
    const PROBE_TTL: Duration = Duration::from_secs(60);
    /// Pause between a temperature write and switching Night light on: Windows uses a temperature written just
    /// before switching on only when the writes are not too close together.
    const SETTLE: Duration = Duration::from_millis(300);
    /// How long the preview flag stays set when the temperature changes while Night light is on (without it,
    /// Windows only stores the new temperature for the next time Night light turns on).
    const PREVIEW_PULSE: Duration = Duration::from_millis(250);

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn unix_now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs())
    }

    fn filetime_now() -> u64 {
        let since = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        (since.as_secs() + FILETIME_UNIX_OFFSET_SECS) * 10_000_000
            + u64::from(since.subsec_nanos() / 100)
    }

    fn read_binary(subkey: &str) -> Result<Vec<u8>, PlatformError> {
        let subkey = wide(subkey);
        let mut buffer = vec![0u8; MAX_VALUE_BYTES];
        let mut size = MAX_VALUE_BYTES as u32;
        // SAFETY: `subkey` is NUL-terminated and outlives the call; `buffer` holds `size` bytes; `size` is a valid
        // in/out pointer. RRF_RT_REG_BINARY makes the call fail for any other value type.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                w!("Data"),
                RRF_RT_REG_BINARY,
                None,
                Some(buffer.as_mut_ptr().cast::<c_void>()),
                Some(&mut size),
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Err(PlatformError::Os(
                "Night light values not found (Night light never used on this account)".into(),
            ));
        }
        status.ok()?;
        buffer.truncate(usize::try_from(size).unwrap_or(0).min(MAX_VALUE_BYTES));
        Ok(buffer)
    }

    fn write_binary(subkey: &str, data: &[u8]) -> Result<(), PlatformError> {
        let subkey = wide(subkey);
        let len = u32::try_from(data.len())
            .map_err(|_| PlatformError::Os("Night light value is too long".into()))?;
        // SAFETY: `subkey` is NUL-terminated and `data` holds `len` bytes; both outlive the call.
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                w!("Data"),
                REG_BINARY.0,
                Some(data.as_ptr().cast::<c_void>()),
                len,
            )
        }
        .ok()?;
        Ok(())
    }

    /// `CurrentBuildNumber` of Windows, e.g. 26200.
    fn build_number() -> Option<u32> {
        let subkey = wide(NT_CURRENT_VERSION);
        let mut buffer = [0u16; 32];
        let mut size = size_of_val(&buffer) as u32;
        // SAFETY: `subkey` is NUL-terminated; `buffer` holds `size` bytes; RRF_RT_REG_SZ NUL-terminates the result.
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(subkey.as_ptr()),
                w!("CurrentBuildNumber"),
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast::<c_void>()),
                Some(&mut size),
            )
        };
        status.ok().ok()?;
        let len = buffer.iter().position(|&unit| unit == 0)?;
        String::from_utf16_lossy(&buffer[..len]).trim().parse().ok()
    }

    /// Closes a registry key on drop.
    struct OwnedKey(HKEY);

    impl Drop for OwnedKey {
        fn drop(&mut self) {
            // SAFETY: the key came from a successful RegOpenKeyExW and is closed exactly once.
            let _ = unsafe { RegCloseKey(self.0) };
        }
    }

    fn per_device_records_exist() -> bool {
        let subkey = wide(CURRENT);
        let mut key = HKEY::default();
        // SAFETY: `subkey` is NUL-terminated; `key` is a valid out pointer.
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                None,
                KEY_ENUMERATE_SUB_KEYS,
                &mut key,
            )
        };
        if status.is_err() {
            return false;
        }
        let key = OwnedKey(key);
        // Registry key names are at most 255 characters.
        let mut name = [0u16; 256];
        for index in 0..100_000u32 {
            let mut len = name.len() as u32;
            // SAFETY: `name` holds `len` UTF-16 units; `len` is a valid in/out pointer; the optional outputs are null.
            let status = unsafe {
                RegEnumKeyExW(
                    key.0,
                    index,
                    Some(PWSTR(name.as_mut_ptr())),
                    &mut len,
                    None,
                    None,
                    None,
                    None,
                )
            };
            if status == ERROR_NO_MORE_ITEMS {
                break;
            }
            if status.is_err() {
                continue;
            }
            let len = usize::try_from(len).unwrap_or(0).min(name.len());
            if is_per_device_key(&String::from_utf16_lossy(&name[..len])) {
                return true;
            }
        }
        false
    }

    /// Both values, decoded and validated.
    struct Current {
        night: NightState,
        settings: CloudValue,
    }

    fn read_state() -> Result<(CloudValue, NightState), PlatformError> {
        let state = decode_value(&read_binary(STATE_KEY)?)?;
        let night = parse_state(&state.payload)?;
        Ok((state, night))
    }

    fn read_settings() -> Result<CloudValue, PlatformError> {
        let settings = decode_value(&read_binary(SETTINGS_KEY)?)?;
        validate_settings(&settings.payload)?;
        Ok(settings)
    }

    fn read_current() -> Result<Current, PlatformError> {
        let (_, night) = read_state()?;
        Ok(Current {
            night,
            settings: read_settings()?,
        })
    }

    /// Writes `payload` as the new value of `subkey` (timestamp after `stored`) and checks that it stayed.
    fn write_payload(subkey: &str, stored: u64, payload: BondStruct) -> Result<(), PlatformError> {
        let bytes = encode_value(&CloudValue {
            timestamp: next_timestamp(stored, unix_now()),
            payload,
        });
        write_binary(subkey, &bytes)?;
        if read_binary(subkey)? != bytes {
            return Err(PlatformError::Os(
                "Windows did not keep the Night light value".into(),
            ));
        }
        Ok(())
    }

    pub(super) struct WindowsNightLight {
        /// Cached availability probe: (available, when probed).
        probe: Cell<Option<(bool, Instant)>>,
        /// Refuse builds and layouts where the registry is not the live store (always on, except in the
        /// registry round-trip test).
        live_check: bool,
    }

    impl WindowsNightLight {
        pub(super) fn new() -> Self {
            Self {
                probe: Cell::new(None),
                live_check: true,
            }
        }

        /// Skips the build / per-device check (never the value checks). Test use only.
        #[cfg(test)]
        pub(super) fn without_live_check() -> Self {
            Self {
                probe: Cell::new(None),
                live_check: false,
            }
        }

        fn check_live(&self) -> Result<(), PlatformError> {
            if !self.live_check {
                return Ok(());
            }
            registry_is_live(build_number(), per_device_records_exist()).map_err(|reason| {
                log::info!("windows night light control unavailable: {reason}");
                PlatformError::Unsupported
            })
        }

        fn probe(&self) -> bool {
            match self.check_live().and_then(|()| read_current().map(drop)) {
                Ok(()) => true,
                Err(PlatformError::Unsupported) => false,
                Err(err) => {
                    log::warn!("windows night light control unavailable: {err}");
                    false
                }
            }
        }
    }

    impl NightLight for WindowsNightLight {
        fn available(&self) -> bool {
            if let Some((available, at)) = self.probe.get()
                && at.elapsed() < PROBE_TTL
            {
                return available;
            }
            let available = self.probe();
            self.probe.set(Some((available, Instant::now())));
            available
        }

        fn snapshot(&mut self) -> Result<NightLightSnapshot, PlatformError> {
            self.check_live()?;
            let state = read_binary(STATE_KEY)?;
            let settings = read_binary(SETTINGS_KEY)?;
            // Only values we understand completely are worth keeping (and may be written back later).
            parse_state(&decode_value(&state)?.payload)?;
            validate_settings(&decode_value(&settings)?.payload)?;
            Ok(NightLightSnapshot {
                method: METHOD_CLOUD_STORE.to_owned(),
                state: crate::base64::encode(&state),
                settings: crate::base64::encode(&settings),
            })
        }

        fn apply(&mut self, on: bool, kelvin: u32) -> Result<(), PlatformError> {
            self.check_live()?;
            let current = read_current()?;
            let kelvin = super::clamp_kelvin(kelvin);
            let mut wrote_settings = false;

            if on
                && (settings_kelvin(&current.settings.payload) != Some(kelvin)
                    || settings_preview(&current.settings.payload))
            {
                if current.night.on {
                    // Already on: the new temperature only shows at once through a short preview, as when the
                    // Settings slider is dragged.
                    write_payload(
                        SETTINGS_KEY,
                        current.settings.timestamp,
                        with_kelvin(&current.settings.payload, Some(kelvin), true),
                    )?;
                    std::thread::sleep(PREVIEW_PULSE);
                    let settings = read_settings()?;
                    write_payload(
                        SETTINGS_KEY,
                        settings.timestamp,
                        with_kelvin(&settings.payload, Some(kelvin), false),
                    )?;
                } else {
                    write_payload(
                        SETTINGS_KEY,
                        current.settings.timestamp,
                        with_kelvin(&current.settings.payload, Some(kelvin), false),
                    )?;
                    wrote_settings = true;
                }
            }

            if current.night.on != on {
                if wrote_settings {
                    std::thread::sleep(SETTLE);
                }
                let (state, night) = read_state()?;
                if night.on != on {
                    let next = NightState {
                        on,
                        manual: true,
                        changed_at: Some(filetime_now()),
                        per_device: night.per_device,
                    };
                    write_payload(STATE_KEY, state.timestamp, super::build_state(&next))?;
                    log::info!(
                        "windows night light turned {}",
                        if on { "on" } else { "off" }
                    );
                }
            }
            Ok(())
        }

        fn restore(&mut self, snap: &NightLightSnapshot) -> Result<(), PlatformError> {
            let (wanted, wanted_settings) = match decode_snapshot(snap) {
                Ok(decoded) => decoded,
                Err(err) => {
                    // Foreign or corrupted restore file: retrying can never succeed, so report success and let
                    // the shell drop it instead of failing forever.
                    log::warn!("ignoring a night light snapshot that cannot be used: {err}");
                    return Ok(());
                }
            };
            if let Err(err) = self.check_live() {
                // Nothing can have been written on this machine, so there is nothing to undo.
                log::info!("night light restore skipped: {err}");
                return Ok(());
            }
            let current = read_current()?;
            let mut wrote_settings = false;

            let settings = with_kelvin(
                &current.settings.payload,
                settings_kelvin(&wanted_settings),
                false,
            );
            if settings != current.settings.payload {
                write_payload(SETTINGS_KEY, current.settings.timestamp, settings)?;
                wrote_settings = true;
            }

            if current.night.on != wanted.on {
                if wrote_settings {
                    std::thread::sleep(SETTLE);
                }
                let (state, night) = read_state()?;
                if night.on != wanted.on {
                    // The snapshot's own state payload: the on/off state, how and when it last switched.
                    let previous = NightState {
                        per_device: night.per_device,
                        ..wanted
                    };
                    write_payload(STATE_KEY, state.timestamp, super::build_state(&previous))?;
                }
            }
            log::info!("windows night light restored");
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::{NightLight, decode_value, parse_state, settings_kelvin};
        use super::{
            SETTINGS_KEY, STATE_KEY, WindowsNightLight, build_number, read_binary, read_settings,
            read_state, registry_is_live,
        };

        #[test]
        fn probing_does_not_panic() {
            let light = WindowsNightLight::new();
            let first = light.available();
            assert_eq!(first, light.available());
            let _ = build_number();
        }

        #[test]
        fn availability_matches_the_build_gate() {
            // On Windows 11 24H2+ the control must report itself unavailable (nothing gets written there).
            let light = WindowsNightLight::new();
            if registry_is_live(build_number(), super::per_device_records_exist()).is_err() {
                assert!(!light.available());
            }
        }

        /// Writes the user's real Night light registry values (screen may turn warm for a moment), then restores
        /// them — run manually: `cargo test -p focus-platform -- --ignored night_light_round_trip`
        ///
        /// Skips the build gate so the registry round trip can be checked on Windows 11 24H2+ too, where the
        /// values are not applied (but are still written and restored).
        #[test]
        #[ignore = "writes the user's real Night light registry values"]
        fn night_light_round_trip() {
            let mut light = WindowsNightLight::without_live_check();
            let before_state = read_binary(STATE_KEY).expect("read state");
            let before_settings = read_binary(SETTINGS_KEY).expect("read settings");
            let snap = light.snapshot().expect("snapshot");

            light.apply(true, 3400).expect("apply on");
            let (_, night) = read_state().expect("state");
            assert!(night.on);
            assert_eq!(
                settings_kelvin(&read_settings().expect("settings").payload),
                Some(3400)
            );
            // A new temperature while on (preview pulse).
            light.apply(true, 2900).expect("apply warmer");
            let settings = read_settings().expect("settings");
            assert_eq!(settings_kelvin(&settings.payload), Some(2900));
            assert!(!super::settings_preview(&settings.payload));

            light.restore(&snap).expect("restore");
            let after_state = read_binary(STATE_KEY).expect("read state");
            let after_settings = read_binary(SETTINGS_KEY).expect("read settings");
            // Same payloads as before; only the envelope timestamps moved forward.
            let payload = |bytes: &[u8]| decode_value(bytes).expect("decode").payload;
            assert_eq!(payload(&after_state), payload(&before_state));
            assert_eq!(payload(&after_settings), payload(&before_settings));
            assert!(
                decode_value(&after_state).expect("decode").timestamp
                    >= decode_value(&before_state).expect("decode").timestamp
            );
            let restored = parse_state(&payload(&after_state)).expect("state");
            let original = parse_state(&payload(&before_state)).expect("state");
            assert_eq!(restored, original);
            // Restoring twice is a no-op.
            light.restore(&snap).expect("restore twice");
            assert_eq!(read_binary(STATE_KEY).expect("state"), after_state);
            assert_eq!(read_binary(SETTINGS_KEY).expect("settings"), after_settings);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> Vec<u8> {
        let clean: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        (0..clean.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).expect("hex"))
            .collect()
    }

    // Values from a Windows 11 build 26200 machine (default and per-device records).
    const STATE_OFF: &str =
        "434201000A0201002A0686DCB7B9062A2B0E1343420100D00A02C614F580A78BE8B2A7EC0100000000";
    const SETTINGS_DEFAULT: &str = "434201000A0201002A0686DCB7B9062A2B0E2243420100C20A00CA140E152E0F00CA1E0E0700CA320E122E2D00CA3C0E052E110000000000";
    const STATE_PER_DEVICE: &str =
        "434201000A0201002A06CEC9E3B9062A2B0E1443420100D00A02C614F580A78BE8B2A7EC010100000000";
    const SETTINGS_PER_DEVICE: &str = "434201000A0201002A06CCC9E3B9062A2B0E2343420100C20A00CA140E152E0F00CA1E0E0700CA320E122E2D00CA3C0E052E11000100000000";
    // Published samples (sem-reg-rs): Night light on (manual), and settings with every field incl. preview.
    const STATE_ON_SAMPLE: &str =
        "434201000a0201002a06ae81d2a9062a2b0e154342010010 00d00a02c614e6fd92d6a99181ed0100000000";
    const SETTINGS_FULL_SAMPLE: &str = "434201000a0201002a06fecfeea9062a2b0e2d43420100 0201c20a00ca140e082e0f00ca1e0e0e2e1e00cf28f829ca320e152e0300ca3c0e062e1400c2460100000000";
    const STATE_SCHEDULED_SAMPLE: &str =
        "434201000a0201002a06ae81d2a9062a2b0e1043420100c614e6fd92d6a99181ed0100000000";

    const ALL: [&str; 7] = [
        STATE_OFF,
        SETTINGS_DEFAULT,
        STATE_PER_DEVICE,
        SETTINGS_PER_DEVICE,
        STATE_ON_SAMPLE,
        SETTINGS_FULL_SAMPLE,
        STATE_SCHEDULED_SAMPLE,
    ];

    #[test]
    fn known_values_round_trip_byte_for_byte() {
        for text in ALL {
            let bytes = hex(text);
            let value = decode_value(&bytes).unwrap_or_else(|err| panic!("{text}: {err}"));
            assert_eq!(encode_value(&value), bytes, "{text}");
        }
    }

    #[test]
    fn decodes_states() {
        let off = decode_value(&hex(STATE_OFF)).expect("decode");
        assert_eq!(off.timestamp, 1_731_063_302);
        assert_eq!(
            parse_state(&off.payload).expect("state"),
            NightState {
                on: false,
                manual: true,
                changed_at: Some(133_029_458_749_276_277),
                per_device: false,
            }
        );
        let on = parse_state(&decode_value(&hex(STATE_ON_SAMPLE)).expect("decode").payload)
            .expect("state");
        assert!(on.on && on.manual && !on.per_device);
        let scheduled = parse_state(
            &decode_value(&hex(STATE_SCHEDULED_SAMPLE))
                .expect("decode")
                .payload,
        )
        .expect("state");
        assert!(!scheduled.on && !scheduled.manual);
        let per_device = parse_state(
            &decode_value(&hex(STATE_PER_DEVICE))
                .expect("decode")
                .payload,
        )
        .expect("state");
        assert!(per_device.per_device && !per_device.on);
    }

    #[test]
    fn state_payload_rebuilds_exactly() {
        for text in [
            STATE_OFF,
            STATE_PER_DEVICE,
            STATE_ON_SAMPLE,
            STATE_SCHEDULED_SAMPLE,
        ] {
            let payload = decode_value(&hex(text)).expect("decode").payload;
            let state = parse_state(&payload).expect("state");
            assert_eq!(build_state(&state), payload, "{text}");
        }
    }

    #[test]
    fn switching_on_writes_the_documented_layout() {
        let state = NightState {
            on: true,
            manual: true,
            changed_at: Some(133_029_458_749_276_277),
            per_device: false,
        };
        let value = CloudValue {
            timestamp: 1_731_063_304,
            payload: build_state(&state),
        };
        assert_eq!(
            encode_value(&value),
            hex(
                "434201000A0201002A0688DCB7B9062A2B0E1543420100 1000 D00A02C614F580A78BE8B2A7EC01 00 000000"
            )
        );
    }

    #[test]
    fn decodes_settings() {
        let default = decode_value(&hex(SETTINGS_DEFAULT))
            .expect("decode")
            .payload;
        validate_settings(&default).expect("valid");
        assert_eq!(settings_kelvin(&default), None);
        assert!(!settings_preview(&default));
        let full = decode_value(&hex(SETTINGS_FULL_SAMPLE))
            .expect("decode")
            .payload;
        validate_settings(&full).expect("valid");
        assert_eq!(settings_kelvin(&full), Some(2684));
        assert!(settings_preview(&full));
        let per_device = decode_value(&hex(SETTINGS_PER_DEVICE))
            .expect("decode")
            .payload;
        validate_settings(&per_device).expect("valid");
        assert_eq!(per_device.derived, Some(Vec::new()));
    }

    #[test]
    fn temperature_goes_between_the_schedule_times() {
        let default = decode_value(&hex(SETTINGS_DEFAULT)).expect("decode");
        let warm = CloudValue {
            timestamp: default.timestamp,
            payload: with_kelvin(&default.payload, Some(3400), false),
        };
        // zigzag(3400) = 6800 = varint 90 35, written between the end time (30) and the sunset time (50).
        let expected = "434201000A0201002A0686DCB7B9062A2B0E2643420100C20A00CA140E152E0F00CA1E0E0700CF289035CA320E122E2D00CA3C0E052E110000000000";
        assert_eq!(encode_value(&warm), hex(expected));
        // Preview adds field 70 at the end; removing both gives the original back.
        let preview = with_kelvin(&default.payload, Some(3400), true);
        assert!(settings_preview(&preview));
        assert_eq!(preview.fields.last().map(|field| field.id), Some(70));
        assert_eq!(with_kelvin(&preview, None, false), default.payload);
        // Per-device values keep their marker.
        let per_device = decode_value(&hex(SETTINGS_PER_DEVICE)).expect("decode");
        let changed = with_kelvin(&per_device.payload, Some(2000), false);
        assert_eq!(changed.derived, Some(Vec::new()));
        assert_eq!(settings_kelvin(&changed), Some(2000));
    }

    #[test]
    fn temperatures_are_clamped() {
        assert_eq!(clamp_kelvin(0), MIN_NIGHT_LIGHT_KELVIN);
        assert_eq!(clamp_kelvin(1900), 1900);
        assert_eq!(clamp_kelvin(10_000), MAX_NIGHT_LIGHT_KELVIN);
        let default = decode_value(&hex(SETTINGS_DEFAULT))
            .expect("decode")
            .payload;
        assert_eq!(
            settings_kelvin(&with_kelvin(&default, Some(500), false)),
            Some(1200)
        );
    }

    #[test]
    fn unknown_layouts_are_refused() {
        // An unknown state field (5).
        let state = BondStruct {
            fields: vec![bond::Field {
                id: 5,
                value: Value::Bool(true),
            }],
            derived: None,
        };
        assert!(parse_state(&state).is_err());
        // "On" with a value other than 0, "manual" other than 1.
        for (id, value) in [(STATE_ON, 1), (STATE_MANUAL, 2)] {
            let state = BondStruct {
                fields: vec![bond::Field {
                    id,
                    value: Value::Int32(value),
                }],
                derived: None,
            };
            assert!(parse_state(&state).is_err());
        }
        // Per-device values with fields of their own.
        let derived = BondStruct {
            fields: Vec::new(),
            derived: Some(vec![bond::Field {
                id: 1,
                value: Value::Bool(true),
            }]),
        };
        assert!(parse_state(&derived).is_err());
        assert!(validate_settings(&derived).is_err());
        // Unknown settings field, wrong type, impossible time.
        let mut settings = decode_value(&hex(SETTINGS_DEFAULT))
            .expect("decode")
            .payload;
        settings.set(80, Value::Bool(true));
        assert!(validate_settings(&settings).is_err());
        let mut settings = decode_value(&hex(SETTINGS_DEFAULT))
            .expect("decode")
            .payload;
        settings.set(SETTINGS_KELVIN, Value::Int32(3400));
        assert!(validate_settings(&settings).is_err());
        let mut settings = decode_value(&hex(SETTINGS_DEFAULT))
            .expect("decode")
            .payload;
        settings.set(
            20,
            Value::Struct(BondStruct {
                fields: vec![bond::Field {
                    id: 0,
                    value: Value::Int8(25),
                }],
                derived: None,
            }),
        );
        assert!(validate_settings(&settings).is_err());
        // Envelope variations: a payload that is not a CloudStore envelope, an extra envelope field.
        let payload_only =
            bond::encode_marshalled(&decode_value(&hex(STATE_OFF)).expect("decode").payload);
        assert!(decode_value(&payload_only).is_err());
        let mut extra = hex(STATE_OFF);
        let last = extra.len() - 1;
        extra.insert(last, 0x00);
        extra.insert(last, 0x42);
        assert!(decode_value(&extra).is_err());
        assert!(decode_value(&[]).is_err());
    }

    #[test]
    fn timestamps_always_move_forward() {
        assert_eq!(next_timestamp(100, 200), 200);
        assert_eq!(next_timestamp(300, 200), 301);
        assert_eq!(next_timestamp(u64::MAX, 0), u64::MAX);
    }

    #[test]
    fn registry_gate() {
        assert!(registry_is_live(Some(19_045), false).is_ok());
        assert!(registry_is_live(Some(22_631), false).is_ok());
        assert!(registry_is_live(None, false).is_ok());
        assert!(registry_is_live(Some(26_100), false).is_err());
        assert!(registry_is_live(Some(26_200), false).is_err());
        assert!(registry_is_live(Some(22_631), true).is_err());
        assert!(is_per_device_key(
            "{2fad7ff6-03d5-4b3c-a7d6-690ec120d0a0}$windows.data.bluelightreduction.bluelightreductionstateperdevice"
        ));
        assert!(is_per_device_key(
            "{2FAD7FF6-03D5-4B3C-A7D6-690EC120D0A0}$Windows.Data.BlueLightReduction.SettingsPerDevice"
        ));
        assert!(!is_per_device_key(
            "default$windows.data.bluelightreduction.settings"
        ));
    }

    #[test]
    fn snapshots_serialize_and_decode() {
        let snap = NightLightSnapshot {
            method: METHOD_CLOUD_STORE.to_owned(),
            state: crate::base64::encode(&hex(STATE_ON_SAMPLE)),
            settings: crate::base64::encode(&hex(SETTINGS_FULL_SAMPLE)),
        };
        let json = serde_json::to_string(&snap).expect("serialize");
        assert!(json.starts_with(r#"{"method":"cloudStore","state":"Q0IBAAoCAQAqBq"#));
        let back: NightLightSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, snap);
        let (state, settings) = decode_snapshot(&back).expect("decode");
        assert!(state.on);
        assert_eq!(settings_kelvin(&settings), Some(2684));

        let foreign = NightLightSnapshot {
            method: "gamma".to_owned(),
            ..snap.clone()
        };
        assert!(decode_snapshot(&foreign).is_err());
        let corrupted = NightLightSnapshot {
            state: "not base64!".to_owned(),
            ..snap
        };
        assert!(decode_snapshot(&corrupted).is_err());
    }

    #[test]
    fn noop_night_light() {
        let mut light = NoopNightLight;
        assert!(!light.available());
        assert!(matches!(light.snapshot(), Err(PlatformError::Unsupported)));
        assert!(matches!(
            light.apply(true, 3400),
            Err(PlatformError::Unsupported)
        ));
        let snap = NightLightSnapshot {
            method: METHOD_CLOUD_STORE.to_owned(),
            state: String::new(),
            settings: String::new(),
        };
        assert!(light.restore(&snap).is_ok());
    }

    #[test]
    fn platform_night_light_is_constructible() {
        let light = night_light();
        #[cfg(not(target_os = "windows"))]
        assert!(!light.available());
        #[cfg(target_os = "windows")]
        let _ = light.available();
    }
}
