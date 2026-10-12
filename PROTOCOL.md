# ATTACK SHARK V6 protocol (native Linux)

Ground truth: vendor shim capture `tools/hiddriver_capture_2026-10-12.log`
(29 SetFeature writes from the real Windows app via `hiddriver_17.dll`,
wired `1d57:2224`). One Apply = `0c` commit + `04` + `05` + `06` + `08`.
Report `0x09` NEVER appears on the wire — the old "64-byte 09 40
full-config blob" theory from static RE is dead; 09 is macro pages only
(X11 layout, uncaptured). Also: `hidapi.dll` is enumerate-only for wired
(the 9 imports scan, never send); the wireless dongle (`1d57:fa60`,
MI_02 col03/04) IS driven through hidapi — see
`tools/hidapi_wireless_capture_2026-10-12.log`.

## Transport

- Device: VID `0x1D57`, PID `0x2224` (wired) / `0xFA60` / `0xFA65` (2.4G).
- Config channel: HID interface 2. All writes: HID **SET_REPORT Feature**
  via hidapi `send_feature_report`.
- ACK for report id `NN`: interrupt IN `03 24 50 00 NN`
  (X6 docs show `03 10 50 …` — accept both prefixes).
- Live heartbeat (interrupt IN, ~1 Hz): `03 24 41 01 23`.
- Feature `0xA0` reads back sync poll; GET `0x0a` (128 B) dumps the whole
  flash config (contains the 04/05/08 payloads back-to-back).

## Reports (all VERIFIED byte-exact against the capture; builders in
`src/protocol.rs` reproduce vendor bytes, locked by unit tests)

### 0x0c — commit header (10 B, static)
`0c 0a 01 fe 01 fe 00 00 00 00` — sent once per Apply. `send_commit()`.
ACK `03 24 50 00 0c`. REQUIRED before 0x05: the mouse silently ignores a
lone prefs write (proven live 2026-10-12: lone 0x05 → no ACK; 0x0c then
0x05 → ACK `03 24 50 00 05`). All UI applies send the commit first.

### 0x04 — DPI + sensor (56 B)
`[04][38][01][angle?][ripple?][mask=0x3f][angle-copy?][0x20][8×dpi_lo]
[8×dpi_hi][active 1-based][8×RGB][0x02][ck_hi][ck_lo][00 ×4]`
- `raw = dpi/50 − 1` split lo/hi (proven: a 950→9000 edit landed as
  `0x12`→`0xb3`). Stages 7–8 are 50-DPI fillers. Active byte swept 1–6
  in the capture with the checksum tracking.
- `ck` = BE16 `sum(bytes[3..50]) & 0xffff` (holds on all 12 samples).
- `[7]` is `0x20` always — the X11 lift mapping does NOT apply to V6
  (V6 lift-off byte still unknown). `[4]` = angle-snap (1=on, default ON,
  user-confirmed 2026-10-12), `[3]` = ripple (guess, default off),
  `[6]` reserved (always 0x00).
- Factory stage colors: red green blue yellow cyan magenta orange white.

### 0x05 — prefs / lighting / power (15 B)
`[05][0f][01][light][deep-hi|speed][deep-lo][R][G][B][sleep×2]
[debounce][rgb-count][ck][00][00]` — the X11 `Prefs` mapping decodes the
captured blob field-perfect
(`…00 03 a8 00 ff 00 01 04 01 af` = Off, led 3, green, sleep 0.5 min,
deep-sleep 10 min, debounce 8 ms).
- `ck` = `(sum(bytes[3..12]) − 1) & 0xff` — V6-corrected, the X11 plain
  sum is off by one (verified on all 4 samples: af/bc/c3/af).
- Later Applies moved sleep `01`→`0e` (0.5→7 min) and debounce `04`→`0b`.

### 0x06 — polling rate (9 B)
`[06][09][01][code][0xff−code][00 ×4]`, codes 1000→`01`, 500→`02`,
250→`04`, 125→`08` — all four seen on the wire with exact complements.

### 0x08 — button remap (59 B)
Header `08 3b 01`, eighteen 3-byte `[action, modifier, key]` slots,
`ck = (sum(bytes[2..58]) − 1) & 0xff` (= `0x79` on the vendor map).
Structure + checksum VERIFIED; the vendor DEFAULT map differs from our
`buttons::blank()` template in 3 slots: wheel-up `0x0a`, wheel-down
`0x09` (we label 9/10 the other way round), FORWARD slot `0x3c`
(unknown action). So `blank()` stays a safe template, not the factory
map — never send blind, always build deliberately + verify on device.
The byte-exact vendor default map ships as `buttons::FACTORY_MAP`
(unit-tested against the capture blob, checksum `0x79`).

### Factory reset (`Mouse::reset_factory`, UI toolbar `reset`)
Replays the vendor app's own default Apply: `0c` commit → `04` DPI
(800/1600/2400/3200/8000/25000, active S2, angle-snap on) → `05`
prefs (Off, green, sleep 0.5 min, deep-sleep 10 min, debounce 8 ms) →
`06` polling 1000 Hz → `08` factory button map. The UI mirrors the
same defaults into its own state, so the next per-section Apply sends
factory values instead of the stale pre-reset ones.

## Heartbeat / input decode (`protocol::decode_event`)
- `03 24 41 …` heartbeat (last byte = battery %).
- `03 24 50 00 NN` config ACK; `03 10 30 00 NN` button event
  (NN = extra-button id).

## Open items
1. Ripple byte `[3]` is a guess (never nonzero in the capture). Lift-off
   byte location still unknown.
2. Confirm factory DPI stage table (capture shows customs:
   950/1900/2850/3800/9450/10000 — odd but checksum-clean).
3. Macro-page 09 layout: needs a macro-Apply capture on Windows.
4. Profiles live in a local file (device is write-only) — not yet built.

## Code map
- `src/protocol.rs` — builders + checksums + unit tests (9 passing,
  incl. byte-exact vendor-blob reproductions).
- `src/lib.rs` — `Mouse`: `open`, `send`, `set_polling`, `set_dpi`,
  `set_dpi_full`, `set_prefs`, `set_buttons_raw`, `send_commit`,
  `send_macro_packets`, `read_event`.
- `src/bin/ui.rs` — eframe UI calling `Mouse` methods directly.
- `tools/win_hid_shim.*` / `tools/win_hiddriver_shim.*` — the logging
  proxies; `tools/hidapi*.dll`, `tools/hiddriver_shim.dll` — built
  32-bit binaries; `tools/*capture*.log` — the ground-truth logs.
