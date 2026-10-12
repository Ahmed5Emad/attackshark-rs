//! Shared transport facts for the 1d57:fa60 family.
//! Provenance:
//! - hiddriver_1.dll in V6 SOFT is sha256-identical to the X3/X6 DLL
//!   (f9efbf06...), so SET_REPORT feature transport is shared.
//! - Report layouts below come from the X11 driver (same VID:PID, same
//!   software generation) and the X6 protocol doc. They are CANDIDATES for
//!   V6 until a live ACK confirms each one. Nothing here is sent without an
//!   explicit subcommand.

/// SET_REPORT feature header (bmRequestType 0x21 / bRequest 0x09 is handled
/// inside hidapi's send_feature_report; wIndex 2 = config interface).
pub const IFACE: u16 = 2;

/// Polling-rate codes (X11): 125->0x08, 250->0x04, 500->0x02, 1000->0x01.
pub fn polling_code(hz: u16) -> Option<u8> {
    match hz {
        125 => Some(0x08),
        250 => Some(0x04),
        500 => Some(0x02),
        1000 => Some(0x01),
        _ => None,
    }
}

/// 9-byte report 0x06: 06 09 01 <code> <0xff-code> 00 00 00 00.
/// VERIFIED on V6 2026-10-12: vendor capture shows all four codes on the
/// wire, each with the exact complement byte (01/fe, 02/fd, 04/fb, 08/f7).
/// NOTE: this is the POLLING-RATE report, not toggles — the mask-looking
/// byte is the rate code. Report 0x09 never appears on the wire.
pub fn build_polling(hz: u16) -> Option<[u8; 9]> {
    let code = polling_code(hz)?;
    let mut r = [0u8; 9];
    r[0] = 0x06;
    r[1] = 0x09;
    r[2] = 0x01;
    r[3] = code;
    r[4] = 0xff - code;
    Some(r)
}

/// Config-write ACK. X6 form: 03 10 50 00 <report-id>.
/// V6 form (observed live 2026-10-12): 03 24 50 00 <report-id>.
/// Our polling SET(0x06) got back 03 24 50 00 06. Accept both prefixes.
pub fn is_ack(report: &[u8]) -> bool {
    report.len() == 5
        && report[0] == 0x03
        && (report[1] == 0x10 || report[1] == 0x24)
        && report[2] == 0x50
}

/// Battery form 1 (X6 doc): 03 10 40 .. pct10 -> pct = b[4]*10.
pub fn decode_battery_x6(report: &[u8]) -> Option<u8> {
    if report.len() == 5 && report[0] == 0x03 && report[1] == 0x10 && report[2] == 0x40 {
        Some(report[4].saturating_mul(10))
    } else {
        None
    }
}

/// Battery form 2 (attack-shark-x11-driver): interrupt `03 55 40 01 <pct>`,
/// pct 0..=100. Never call on heartbeat bytes (V6 heartbeat
/// `03 24 41 01 23` would false-positive at 35%).
pub fn decode_battery_x11(report: &[u8]) -> Option<u8> {
    if report.len() >= 5
        && report[0] == 0x03
        && report[1] == 0x55
        && report[2] == 0x40
        && report[3] == 0x01
        && report[4] <= 100
    {
        Some(report[4])
    } else {
        None
    }
}

/// Battery form 3 (V6 hypothesis from X6/X11 ground truth): interrupt
/// `03 24 40 .. <pct>`, pct 0..=100. V6 heartbeat is `03 24 41 ..`, so a
/// 0x40 sibling on the same 0x24 family is the expected battery slot.
pub fn decode_battery_v6(report: &[u8]) -> Option<u8> {
    if report.len() >= 5
        && report[0] == 0x03
        && report[1] == 0x24
        && report[2] == 0x40
        && report[4] <= 100
    {
        Some(report[4])
    } else {
        None
    }
}

/// Battery form 4 (V6 live observation): the heartbeat itself
/// `03 24 41 01 <b>` carries the charge in its last byte (direct 0..=100).
/// Evidence: X6 ground truth folds battery into the heartbeat; V6 byte4
/// ticked 0x23 -> 0x22 on its own over ~30min of use (35 -> 34). Guarded to
/// <= 100 so a non-battery 0x41 variant can never display garbage.
pub fn decode_battery_v6_hb(report: &[u8]) -> Option<u8> {
    match report {
        [0x03, 0x24, 0x41, _, b] if *b <= 100 => Some(*b),
        _ => None,
    }
}
/// Layout: [0]=04 [1]=38 [2]=01, [3]=ripple? (GUESS — always 0x00 in the
/// capture), [4]=angle snap (1=on, 0=off — USER-CONFIRMED 2026-10-12: the
/// only 01->00->01 flip in the capture was the angle switch; vendor
/// default is ON), [5]=stage mask (0x3f), [6]=reserved (always 0x00),
/// [7]=RESERVED (vendor sends 0x20 always — the X11 lift mapping does NOT
/// apply to V6), [8..16]=DPI lows (raw=dpi/50-1),
/// [16..24]=DPI highs, [24]=active stage (1-based), [25..49]=8 per-stage
/// RGB triples, [49]=0x02, [50..51]=checksum BE16 = sum([3..50]) & 0xffff,
/// [52..56]=0.
/// VERIFIED byte-exact on V6 2026-10-12 (vendor shim capture,
/// tools/hiddriver_capture_2026-10-12.log): checksum formula holds on all
/// 12 samples; active byte swept 1..6 with checksum tracking; stage
/// encoding proven by a 950->9000 change landing as 0x12->0xb3.
/// Pass b7=0x20 (vendor value) until the V6 lift-off byte is found.
pub fn build_dpi_linear(
    stages: [u16; 8],
    active: u8,
    angle: bool,
    ripple: bool,
    b7: u8,
    colors: [[u8; 3]; 8],
) -> Option<[u8; 56]> {
    if active < 1 || active > 8 {
        return None;
    }
    let mut r = [0u8; 56];
    r[0] = 0x04;
    r[1] = 0x38;
    r[2] = 0x01;
    if angle {
        r[4] = 1;
    }
    if ripple {
        // GUESS: never observed nonzero in the capture; [3] stayed 0x00.
        r[3] = 1;
    }
    r[5] = 0x3f;
    r[7] = b7;
    r[24] = active;
    r[49] = 0x02;
    for (i, &dpi) in stages.iter().enumerate() {
        if dpi < 50 || dpi % 50 != 0 {
            return None;
        }
        let raw: u16 = dpi / 50 - 1;
        r[8 + i] = (raw & 0xff) as u8;
        r[16 + i] = (raw >> 8) as u8;
        r[25 + i * 3..25 + i * 3 + 3].copy_from_slice(&colors[i]);
    }
    let sum: u32 = r[3..50].iter().map(|&b| b as u32).sum();
    r[50] = ((sum & 0xffff) >> 8) as u8;
    r[51] = (sum & 0xff) as u8;
    Some(r)
}

/// LED modes for report 0x05 (X11 driver; V6 shares the Windows UI labels).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LightMode {
    Off = 0,
    Static = 16,
    Breathing = 32,
    Neon = 48,
    ColorBreathing = 64,
    StaticDpi = 80,
    BreathingDpi = 96,
}

/// User preferences for the 13..15-byte report 0x05 (wValue 0x0305, iface 2).
/// Layout: [0]=05 [1]=0f [2]=01, [3]=light mode, [4]=(deep-sleep bucket<<4) |
/// (6-led_speed), [5]=0x08+deep_sleep_min*0x10, [6..9]=RGB, [9]=sleep_min*2,
/// [10]=(debounce_ms-4)/2+2, [11]=derived (see below), [12]=checksum
/// sum([3..11])&0xff, [13..15]=0 padding (wireless sends 15B, wired 13B).
/// [11]: count of RGB channels >= 0x64, plus 1 when mode == BreathingDpi.
/// Checksum [12] = (sum([3..12]) - 1) & 0xff — V6-corrected: the X11
/// formula (plain sum) is off by one against all 4 vendor-captured 05
/// blobs (af/bc/c3/af across the 4 samples).
/// VERIFIED field-by-field on V6 2026-10-12: capture
/// `05 0f 01 00 03 a8 00 ff 00 01 04 01 af` decodes exactly to
/// {Off, led_speed 3, green, sleep 0.5min, deep-sleep 10min, debounce
/// 8ms}; later Applies moved sleep 01->0e (0.5->7min) and debounce
/// 04->0b with the checksum tracking both.
#[derive(Debug, Clone, Copy)]
pub struct Prefs {
    pub light: LightMode,
    pub led_speed: u8, // 1..=5, 5 = fastest
    pub rgb: [u8; 3],
    pub sleep_min: f32,      // 0.5..=30
    pub deep_sleep_min: u8,  // 1..=60
    pub debounce_ms: u8,     // 4..=50, even
}

pub fn build_prefs(p: Prefs) -> Option<[u8; 15]> {
    if !(1..=5).contains(&p.led_speed) { return None; }
    if !(0.5..=30.0).contains(&p.sleep_min) { return None; }
    if !(1..=60).contains(&p.deep_sleep_min) { return None; }
    if p.debounce_ms < 4 || p.debounce_ms > 50 || p.debounce_ms % 2 != 0 { return None; }
    let mut r = [0u8; 15];
    r[0] = 0x05;
    r[1] = 0x0f;
    r[2] = 0x01;
    r[3] = p.light as u8;
    let bucket = (p.deep_sleep_min - 1) / 16;
    let hw_speed = 6 - p.led_speed;
    r[4] = (bucket << 4) | (hw_speed & 0x0f);
    r[5] = 0x08u8.wrapping_add(p.deep_sleep_min.wrapping_mul(0x10));
    r[6] = p.rgb[0];
    r[7] = p.rgb[1];
    r[8] = p.rgb[2];
    r[9] = (p.sleep_min * 2.0).round() as u8;
    r[10] = (p.debounce_ms - 4) / 2 + 0x02;
    let mut count = 0u8;
    for &c in &p.rgb {
        if c >= 0x64 { count += 1; }
    }
    r[11] = if p.light == LightMode::BreathingDpi { count + 1 } else { count };
    let sum: u32 = r[3..12].iter().map(|&b| b as u32).sum();
    r[12] = ((sum & 0xff) as u8).wrapping_sub(1);
    Some(r)
}

/// Static commit header the vendor app sends once per Apply (report 0x0c,
/// 10 bytes, never varies across the capture).
pub const COMMIT_0C: [u8; 10] = [0x0c, 0x0a, 0x01, 0xfe, 0x01, 0xfe, 0, 0, 0, 0];

/// Button slots in the 59-byte report 0x08 (offsets of each 3-byte slot).
/// Header [0]=08 [1]=3b [2]=01; slots [3..57]; checksum [58] =
/// (sum([2..58])-1)&0xff. Slot value = [action, modifier, key/hid].
/// Structure + checksum VERIFIED byte-exact on V6 2026-10-12 (vendor
/// capture; captured map checksums to 0x79 by this formula). NOTE: the
/// vendor DEFAULT map differs from blank() in 3 slots — wheel-up sends
/// 0x0a (we label SCROLL_UP_ACT=9), wheel-down 0x09 (we say 10), and the
/// FORWARD slot holds 0x3c (unknown action, we use ACT_FORWARD=6).
/// So blank() is our safe template, not the factory map. No defaults are
/// ever sent blind — use set_buttons_raw with a deliberate map.
pub mod buttons {
    pub const LEFT: usize = 3;
    pub const RIGHT: usize = 6;
    pub const MIDDLE: usize = 9;
    pub const DPI: usize = 18;
    pub const FORWARD: usize = 21;
    pub const BACKWARD: usize = 24;
    pub const SCROLL_UP: usize = 51;
    pub const SCROLL_DOWN: usize = 54;

    // Firmware actions (subset; full list in PROTOCOL.md).
    pub const DISABLE: u8 = 1;
    pub const LEFT_CLICK: u8 = 2;
    pub const RIGHT_CLICK: u8 = 3;
    pub const MIDDLE_CLICK: u8 = 4;
    pub const ACT_BACKWARD: u8 = 5;
    pub const ACT_FORWARD: u8 = 6;
    pub const DOUBLE_CLICK: u8 = 7;
    pub const FIRE: u8 = 8;
    pub const SCROLL_UP_ACT: u8 = 9;
    pub const SCROLL_DOWN_ACT: u8 = 10;
    pub const DPI_CYCLE: u8 = 13;
    pub const DPI_UP: u8 = 14;
    pub const DPI_DOWN: u8 = 15;
    pub const ANGLE_SNAP: u8 = 16;
    pub const KEYBOARD: u8 = 17;
    pub const CUSTOM_MACRO: u8 = 18;
    pub const PLAY_PAUSE: u8 = 24;
    pub const MUTE: u8 = 26;
    pub const VOL_UP: u8 = 27;
    pub const VOL_DOWN: u8 = 28;

    /// Factory button map, byte-exact from the vendor capture's first Apply
    /// (`tools/hiddriver_capture_2026-10-12.log` line 83): left/right/
    /// middle click, backward(05)/forward-slot(06), DPI-cycle(0d),
    /// forward-slot holds 0x3c, wheel-up 0x0a, wheel-down 0x09, rest
    /// disabled. Checksum 0x79 matches the formula — see test.
    pub const FACTORY_MAP: [u8; 59] = [
        0x08, 0x3b, 0x01, 0x02, 0x00, 0x00, 0x03, 0x00, 0x00, 0x04, 0x00, 0x00,
        0x05, 0x00, 0x00, 0x06, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x3c, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x79,
    ];

    /// Blank map: every slot [action=DISABLE handled by caller] — starts from
    /// the driver's inactive default [0x01,0x00,0x00] per slot with the three
    /// hardware-wired slots (DPI cycle, wheel up/down) preset.
    pub fn blank() -> [u8; 59] {
        let mut r = [0u8; 59];
        r[0] = 0x08;
        r[1] = 0x3b;
        r[2] = 0x01;
        let mut i = 3;
        while i <= 54 {
            r[i] = 0x01;
            i += 3;
        }
        r[DPI] = DPI_CYCLE;
        r[SCROLL_UP] = SCROLL_UP_ACT;
        r[SCROLL_DOWN] = SCROLL_DOWN_ACT;
        finish(r)
    }

    pub fn set_slot(mut r: [u8; 59], offset: usize, action: u8, modifier: u8, key: u8) -> [u8; 59] {
        r[offset] = action;
        r[offset + 1] = modifier;
        r[offset + 2] = key;
        finish(r)
    }

    fn finish(mut r: [u8; 59]) -> [u8; 59] {
        let sum: u32 = r[2..58].iter().map(|&b| b as u32).sum();
        r[58] = ((sum & 0xff) as u8).wrapping_sub(1);
        r
    }
}

/// Custom-macro packet sequence (X11 driver; report 0x09 NEVER appears in the
/// V6 vendor capture, so this stays a candidate until a macro Apply is
/// captured on Windows): the 0x08 button map with the
/// target button set to CUSTOM_MACRO(18), followed by three 64-byte 0x09
/// pages. Page layout: [0]=09 [1]=0x40(page0/1) or 0x0c(page2,12B),
/// [2]=button id, [3]=page index; page0[4]=play mode, [8]=repeat count,
/// [29]=event count, events from [30] (17 max); page1 events from [4]
/// (30 max); page2 = checksum BE over page0[8..]+page1[4..] at [10..12].
/// Event = 2 bytes [delay|0x80=release, key]; delay byte = 2*((ms+5)/20)+1.
/// UNTESTED on V6. Documented for the egui macro editor; live-test like the
/// others before shipping.
pub mod macros {
    /// Repeat-N-times play mode (matches the driver's default).
    pub const PLAY_N_TIMES: u8 = 0;
    /// Button ids used in packet [2] (driver CUSTOM_MACRO_BUTTONS).
    pub const BTN_LEFT: u8 = 0;
    pub const BTN_RIGHT: u8 = 1;
    pub const BTN_MIDDLE: u8 = 2;
    pub const BTN_EXTRA4: u8 = 3;
    pub const BTN_EXTRA5: u8 = 4;

    pub fn delay_byte(ms: u32) -> u8 {
        (2 * ((ms + 5) / 20) + 1) as u8
    }

    /// Pack up to 47 (delay,key) events into the three 0x09 pages.
    /// Returns (page0, page1, page2), each 64 bytes.
    pub fn build_packets(button_id: u8, mode: u8, times: u8, events: &[(u8, u8)]) -> [[u8; 64]; 3] {
        let mut p0 = [0u8; 64];
        let mut p1 = [0u8; 64];
        let mut p2 = [0u8; 64];
        p0[0] = 0x09;
        p0[1] = 0x40;
        p0[2] = button_id;
        p0[3] = 0x00;
        p0[4] = mode;
        p0[8] = times;
        p1[0] = 0x09;
        p1[1] = 0x40;
        p1[2] = button_id;
        p1[3] = 0x01;
        p2[0] = 0x09;
        p2[1] = 0x0c;
        p2[2] = button_id;
        p2[3] = 0x02;
        let count = events.len().min(47) as u8;
        p0[29] = count;
        let mut flat = Vec::with_capacity(events.len() * 2);
        for &(d, k) in events.iter().take(47) {
            flat.push(d);
            flat.push(k);
        }
        for (i, &b) in flat.iter().take(34).enumerate() {
            p0[30 + i] = b;
        }
        for (i, &b) in flat.iter().skip(34).enumerate() {
            if 4 + i < 64 {
                p1[4 + i] = b;
            }
        }
        let mut sum: u32 = p0[8..].iter().map(|&b| b as u32).sum::<u32>();
        sum += p1[4..].iter().map(|&b| b as u32).sum::<u32>();
        p2[10] = ((sum >> 8) & 0xff) as u8;
        p2[11] = (sum & 0xff) as u8;
        [p0, p1, p2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polling_1000_layout() {
        assert_eq!(
            build_polling(1000).unwrap(),
            [0x06, 0x09, 0x01, 0x01, 0xfe, 0, 0, 0, 0]
        );
        assert_eq!(
            build_polling(125).unwrap(),
            [0x06, 0x09, 0x01, 0x08, 0xf7, 0, 0, 0, 0]
        );
        assert!(build_polling(999).is_none());
    }

    #[test]
    fn dpi_v6state_checksum() {
        // Bytes captured from the live-ACKed V6 set-dpi run: checksum 0x0fa6.
        let stages = [800u16, 1600, 2400, 3200, 8000, 25000, 50, 50];
        let colors = [[0xff, 0, 0]; 8];
        let r = build_dpi_linear(stages, 2, true, false, 0x20, colors).unwrap();
        assert_eq!(r[0], 0x04);
        assert_eq!(r[8], 0x0f); // 800 -> raw 15
        assert_eq!((r[13], r[21]), (0xf3, 0x01)); // 25000 -> raw 499 LE
        let sum: u32 = r[3..50].iter().map(|&b| b as u32).sum();
        assert_eq!(((sum & 0xffff) >> 8) as u8, r[50]);
        assert_eq!((sum & 0xff) as u8, r[51]);
    }

    #[test]
    fn prefs_default_vector() {
        // Mirrors the X11 driver's DEFAULT_OPTIONS buffer before options.
        let p = Prefs {
            light: LightMode::Off,
            led_speed: 3,
            rgb: [0, 255, 0],
            sleep_min: 0.5,
            deep_sleep_min: 10,
            debounce_ms: 8,
        };
        let r = build_prefs(p).unwrap();
        assert_eq!(&r[0..3], &[0x05, 0x0f, 0x01]);
        assert_eq!(r[3], 0x00);
        assert_eq!(r[9], 0x01); // 0.5 min * 2
        assert_eq!(r[10], 0x04); // (8-4)/2+2
        let sum: u32 = r[3..12].iter().map(|&b| b as u32).sum();
        assert_eq!(((sum & 0xff) as u8).wrapping_sub(1), r[12]);
    }

    #[test]
    fn buttons_blank_checksum_rule() {
        let r = buttons::blank();
        assert_eq!((r[0], r[1], r[2]), (0x08, 0x3b, 0x01));
        assert_eq!(r[buttons::DPI], buttons::DPI_CYCLE);
        let sum: u32 = r[2..58].iter().map(|&b| b as u32).sum();
        assert_eq!(((sum & 0xff) as u8).wrapping_sub(1), r[58]);
    }

    #[test]
    fn v6_ack_forms() {
        assert!(is_ack(&[0x03, 0x24, 0x50, 0x00, 0x04]));
        assert!(is_ack(&[0x03, 0x10, 0x50, 0x00, 0x06]));
        assert!(!is_ack(&[0x03, 0x24, 0x41, 0x01, 0x23]));
    }

    fn unhex(s: &str) -> Vec<u8> {
        s.split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect()
    }

    /// Our DPI builder must reproduce the vendor app's own bytes exactly.
    /// Reference: tools/hiddriver_capture_2026-10-12.log line 80
    /// (stages 950/1900/2850/3800/9450/10000 + fillers, active 2).
    #[test]
    fn dpi_reproduces_vendor_blob() {
        let want = unhex(
            "04 38 01 00 01 3f 00 20 12 25 38 4b bc c7 00 00 00 00 00 00 00 00 00 00 02 ff 00 00 00 ff 00 00 00 ff ff ff 00 00 ff ff ff 00 ff ff 40 00 ff ff ff 02 0f d4 00 00 00 00",
        );
        let stages = [950u16, 1900, 2850, 3800, 9450, 10000, 50, 50];
        let colors = [
            [0xff, 0, 0],
            [0, 0xff, 0],
            [0, 0, 0xff],
            [0xff, 0xff, 0],
            [0, 0xff, 0xff],
            [0xff, 0, 0xff],
            [0xff, 0x40, 0],
            [0xff, 0xff, 0xff],
        ];
        let got = build_dpi_linear(stages, 2, true, false, 0x20, colors).unwrap();
        assert_eq!(got.to_vec(), want);
    }

    /// Our prefs builder must reproduce the vendor 05 blob exactly
    /// (capture line 81).
    #[test]
    fn prefs_reproduces_vendor_blob() {
        let want = unhex("05 0f 01 00 03 a8 00 ff 00 01 04 01 af 00 00");
        let got = build_prefs(Prefs {
            light: LightMode::Off,
            led_speed: 3,
            rgb: [0, 255, 0],
            sleep_min: 0.5,
            deep_sleep_min: 10,
            debounce_ms: 8,
        })
        .unwrap();
        assert_eq!(got.to_vec(), want);
    }

    /// All four vendor-sent polling codes round-trip with complement bytes.
    #[test]
    fn polling_all_vendor_codes() {
        for (hz, code, comp) in [(1000u16, 0x01u8, 0xfeu8), (500, 0x02, 0xfd), (250, 0x04, 0xfb), (125, 0x08, 0xf7)] {
            let r = build_polling(hz).unwrap();
            assert_eq!((r[3], r[4]), (code, comp), "hz={hz}");
        }
    }

    /// Every vendor-sent 04 blob must satisfy the checksum formula, and the
    /// commit header + button-map checksum must match too.
    #[test]
    fn vendor_checksums_hold() {
        let blobs04 = [
            "04 38 01 00 01 3f 00 20 12 25 38 4b bc c7 00 00 00 00 00 00 00 00 00 00 02 ff 00 00 00 ff 00 00 00 ff ff ff 00 00 ff ff ff 00 ff ff 40 00 ff ff ff 02 0f d4 00 00 00 00",
            "04 38 01 00 01 3f 00 20 b3 25 38 4b bc c7 00 00 00 00 00 00 00 00 00 00 01 ff 00 00 00 ff 00 00 00 ff ff ff 00 00 ff ff ff 00 ff ff 40 00 ff ff ff 02 10 74 00 00 00 00",
            "04 38 01 00 00 3f 00 20 b3 25 38 4b bc c7 00 00 00 00 00 00 00 00 00 00 01 ff 00 00 00 ff 00 00 00 ff ff ff 00 00 ff ff ff 00 ff ff 40 00 ff ff ff 02 10 73 00 00 00 00",
        ];
        for h in blobs04 {
            let b = unhex(h);
            assert_eq!(b.len(), 56);
            let sum: u32 = b[3..50].iter().map(|&x| x as u32).sum();
            assert_eq!(((sum & 0xffff) >> 8) as u8, b[50]);
            assert_eq!((sum & 0xff) as u8, b[51]);
        }
        let blobs05 = [
            "05 0f 01 00 03 a8 00 ff 00 01 04 01 af 00 00",
            "05 0f 01 00 03 a8 00 ff 00 0e 04 01 bc 00 00",
            "05 0f 01 00 03 a8 00 ff 00 0e 0b 01 c3 00 00",
        ];
        for h in blobs05 {
            let b = unhex(h);
            let sum: u32 = b[3..12].iter().map(|&x| x as u32).sum();
            assert_eq!(((sum & 0xff) as u8).wrapping_sub(1), b[12]);
        }
        assert_eq!(
            COMMIT_0C.to_vec(),
            unhex("0c 0a 01 fe 01 fe 00 00 00 00")
        );
        let map = unhex("08 3b 01 02 00 00 03 00 00 04 00 00 05 00 00 06 00 00 0d 00 00 3c 00 00 01 00 00 01 00 00 01 00 00 01 00 00 01 00 00 01 00 00 01 00 00 01 00 00 01 00 00 0a 00 00 09 00 00 00 79");
        let sum: u32 = map[2..58].iter().map(|&x| x as u32).sum();
        assert_eq!(((sum & 0xff) as u8).wrapping_sub(1), map[58]);
        // The factory-map const is that same vendor blob, transcribed 1:1.
        assert_eq!(buttons::FACTORY_MAP.to_vec(), map);
    }
}
