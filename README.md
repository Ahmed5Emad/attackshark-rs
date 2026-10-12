# attackshark-rs — native Linux tool for ATTACK SHARK V6-family mice

Rust port of ATTACK SHARK V6 SOFT (Windows). No Wine, no driver, talks HID
directly. Built with REA-driven reverse engineering (see Protocol below).

## Device

USB 1d57:fa60 / fa65 ("LXDDZ 2.4G Wireless Device"). Four HID interfaces:

| node | iface | usage | role |
|------|-------|-------|------|
| hidraw0 | 0 | 0001/0006 | boot mouse |
| hidraw2 | 1 | 0001/0002 | mouse + wheel + consumer |
| hidraw3 | 2 | 0001/0080, 000c/0001, 000a/0000, 000b/0000 | CONFIG CHANNEL |
| hidraw4 | 3 | 0001/0006 | keyboard modifiers |

Config channel report map (from 206-byte descriptor): vendor feature
reports 04(55B) 05(14B) 06(8B) 07(7B) 08(58B) 09(63B) 0A/0B(7B) 0C(9B)
10(7B) A0(7B) 22(63B), vendor input 03(4B).

## Protocol (observed evidence, not guesses)

- Source: ATTACK SHARK V6 SOFT .exe = Inno Setup image (Guangzhou
  Junxingcheng). Payload: AttackShark.exe 1.7MB + hidapi.dll +
  hiddriver_1..18.dll + hiddriver_4k_1.dll. 17 of 18 hiddriver DLLs are
  byte-identical (md5 07823eb5...) => one shared protocol for all standard
  models incl. V6; 4K variant differs. DLL exports: Open/Close_FeatureDevice,
  Open/Close_DevMonitor, GetFeature/SetFeature, Hid_Read/Hid_Write over
  WriteFile/ReadFile. Command bytes are built in AttackShark.exe per model.
- usbmon capture of the Windows app's sync loop (342 URBs, bus 3 dev 4):
  GET_REPORT Feature 0xA0 on iface 0 -> `a0 00 00 00 00 00 00 00` (retried
  ~every 5s, zeros = "no data", app waits forever under Wine = stuck sync
  screen). Zero host->device transfers during sync: config goes over
  interrupt OUT/IN (Hid_Write/Hid_Read), not control.
- This tool reproduces the A0 read natively: `info` prints
  `feature A0 (8B): a0 00 00 00 00 00 00 00` — same bytes as usbmon.

## Usage

  cargo build
  ./target/debug/attackshark-rs list          # find the mouse
  ./target/debug/attackshark-rs info          # open config channel + read A0
  ./target/debug/attackshark-rs watch [secs]  # stream input reports live
  ./target/debug/attackshark-rs set-feature <id> <hex bytes...>

Needs hidraw access: install ../attackshark-linux/99-attackshark.rules, then
re-plug the receiver (or chmod 666 /dev/hidraw* until next replug/reboot).

## Status 2026-10-12: confirmed live on V6 hardware

- Transport: SET_REPORT feature on iface 2 (hidapi send_feature_report).
- V6 ACK form: `03 24 50 00 <report-id>` on the interrupt pipe.
- `set-polling 1000` -> ACK `03 24 50 00 06`. Reversible (125/250/500/1000).
- `set-dpi` (X6-linear 56B layout, stages 800/1600/2400/3200/8000/25000,
  active=2) -> ACK `03 24 50 00 04`. Feel-validated live: stage2=100 crawls,
  stage2=25000 flies, restored to 1600. V6 heartbeat is `03 24 41 01 23`
  (not X6's 03 10 form); battery decode still open.
- V6 driver DLL is sha256-identical to X3/X6's hiddriver_1.dll.

## What's left

- Feel-test: move the mouse (should feel unchanged at 1600 active), then
  try a DPI button press on the mouse and run `watch` to learn the stage
  event form.
- Lighting / button remap / sleep / debounce reports: same transport,
  layouts from the X11 driver's builders (LightingBuilder etc.) —
  each needs one live ACK test like above.
- Battery: find which report carries it on V6 (X6's 03 10 40 form absent).
