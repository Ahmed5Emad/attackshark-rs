# Vendor-app report — ATTACK SHARK V6 SOFT (Windows)

Live host: Windows 11, app `C:\Program Files (x86)\Attack Shark Software\AttackShark.exe`
(1,725,656 bytes), observed running as PID 12528 with window "Attack Shark Driver".
Mouse present as USB `VID_1D57 PID_2224` (composite + HID MI_00..MI_03; config on MI_02).
Python on this box: 3.14.7, 64-bit. No compiler (no cargo/rustc/cl/gcc/clang/tcc),
no WSL, no USBPcap. Vendor `hidapi.dll` / `hiddriver_*.dll` are **32-bit** —
anything injected or swapped must be built **i686**.

## What the app loads

Folder listing (verified on disk): `AttackShark.exe` + `hidapi.dll` (84,480 B) +
`hiddriver_1..18.dll` (81,408 B each) + `hiddriver_4k_1.dll` (322,560 B) +
`DuiLib.dll`, `Hook.dll`, `IsTask.dll`, MSVC runtimes. 17 of 18 hiddriver DLLs
are byte-identical per repo README (md5 `07823eb5…`) — one shared protocol.

## What the app imports from hidapi.dll (verified from the exe IAT)

`AttackShark.exe` is 32-bit PE32 and imports exactly 9 symbols from `hidapi.dll`
(stdcall-decorated, hidapi 0.x ABI):

- `_hid_init@0`, `_hid_enumerate@8`, `_hid_open_path@4`, `_hid_free_enumeration@4`
- `_hid_write@12`, `_hid_read@12`
- `_hid_send_feature_report@12`, `_hid_get_feature_report@12`
- `_hid_set_nonblocking@8`

It also imports SetupDi (`SetupDiGetClassDevsW`, `…EnumDeviceInterfaces`,
`…GetDeviceInterfaceDetailW`) for enumeration, `BluetoothGATT*` for BT, and
`RegisterDeviceNotificationW` for hotplug. Command bytes are built in
`AttackShark.exe` per model; `hiddriver_*.dll` exposes
Open/Close_FeatureDevice, GetFeature/SetFeature, Hid_Read/Hid_Write over
WriteFile/ReadFile (per repo README).

## Transport — one open discrepancy

- Repo README (usbmon ground truth): the sync loop is GET_REPORT Feature `0xA0`
  on iface 0 returning `a0 00 00 00 00 00 00 00`, and config goes over
  **interrupt OUT/IN** (`Hid_Write`/`Hid_Read`), not control transfers.
- `src/lib.rs` `Mouse::send_report` uses hidapi `send_feature_report`
  (control SET_REPORT) and gets a live ACK `03 24 50 00 <id>` on the interrupt pipe.
- Both can be true (feature SET + interrupt ACK), but don't assume: the capture
  below settles which transfer carries the `09 …` bytes.

## Report 0x04 vs 0x09 — current repo truth (not yet vendor-confirmed)

From `src/protocol.rs` + `PROTOCOL.md` (X11-driver builders, same VID/generation):

- **Report `0x04` (56 B) is the DPI/sensor carrier — live-proven on V6 hardware**
  (ACK `03 24 50 00 04`, feel-validated 100 vs 25000 DPI):
  `[8..16]` stage low bytes, `[16..24]` stage high bytes,
  raw LE = `dpi/50 − 1`; active stage byte `[24]` (1-based);
  per-stage RGB `[25..49]`; checksum BE16 `sum(r[3..50]) & 0xffff` at `[50..52]`.
  6 real stages + 2 hidden 50-DPI fillers.
- **Report `0x09` with header `09 40` is macro page 0** (3-page macro write):
  `[2]` button id, `[3]` page index, `[4]` play mode, `[8]` repeat count,
  `[29]` event count, events from `[30]`; page-2 checksum BE16 at `[10..12]`
  over page0[8..]+page1[4..]. NOT a DPI carrier in this layout.
- `src/lib.rs::set_config09` comment claims the vendor app writes a 64-byte
  `09 40` full-config blob on every Apply. **Unresolved conflict** — the capture
  below decides it. If the app really sends `09 40` on Apply, diff two captures
  (see table) and the DPI offsets/encoding/active byte fall out mechanically.

## What I tried live on this box (honest log)

- ✅ App confirmed running (PID 12528); PnP confirms `VID_1D57 PID_2224` MI_00–MI_03.
- ✅ IAT dump of `AttackShark.exe` gives the 9-function hidapi surface above.
- ✅ Export tables re-parsed with corrected struct layout (earlier "bogus"
  counts were my own off-by-one: `Major/Minor` are WORDs, not DWORDs).
  Truth: vendor `hidapi.dll` is standard — Base=1, 21 funcs / 21 names, full
  hidapi 0.x surface (`_hid_init@0`, `_hid_open@12`, `_hid_read_timeout@16`,
  `_hid_get_input_report@12`, …). Shim `tools/hidapi.dll` verified too:
  18 exports covering all 9 decorated names the exe imports. Note the vendor
  DLL exports ONLY decorated (`_hid_*@N`) names — the shim's decorated-name
  fallback is what makes forwarding work.
- ✅ Read-only HID probe (`tools/win_hid_probe.py`, system hid.dll/setupapi,
  verified live): 8 HID nodes — V6 `1d57:2224` on MI_00 (kbd), MI_01 (mouse),
  MI_02 collections 01–04 (config channel = MI_02), MI_03 (kbd), plus one
  `0627:0001` node. First version returned 0 devices — bug was the
  `SetupDiGetClassDevsW` restype defaulting to 32-bit `c_int`, truncating the
  64-bit handle; fixed by declaring restype/argtypes (lesson noted in the script).
- ✅ Proxy `hidapi.dll` built (Linux i686 cross-compile, user-built),
  verified (PE32, all 9 decorated imports covered), deployed, captured,
  removed. App folder is back to stock (`hidapi.dll` 84,480 B);
  shim kept at `tools\hidapi_shim_built.dll`, vendor backup at
  `tools\hidapi_vendor_orig.dll`.

## Capture recipe (pick ONE)

### A — Proxy DLL (recommended, no install/reboot, no admin)

1. Close the vendor app. In the app folder rename `hidapi.dll` → `hidapi_real.dll`.
2. Build `tools/win_hid_shim.c` + `tools/win_hid_shim.def` as **32-bit**
   `hidapi.dll` (build line is at the top of the .c file) and drop it next to
   `hidapi_real.dll`.
3. Start the app from Explorer. Set DPI stages to a known pattern #1
   (e.g. all stages 800 except stage1=400, active=1), click **Apply**.
4. Change ONLY active stage (→ 3), Apply. Change stage1 400→1600, Apply.
5. Close the app, swap the real DLL back, open `%TEMP%\hidapi-shim.log` —
   every `send_feature_report` / `write` / `get_feature_report` with full hex.
6. Paste the `09 …` blobs into the diff table below; send me the log.

### B — USBPcap + Wireshark (bus ground truth, needs install + reboot)

1. Install USBPcap (check "install USBPcap driver"), reboot.
2. Wireshark → capture on `USBPcap#` covering the mouse → filter
   `usb.transfer_type == 0x02 || hid`.
3. In the app: Apply pattern #1, Apply pattern #2 (same patterns as above).
4. Export the `SET_REPORT` / URB_INTERRUPT out packets (full bytes) → send the .pcapng.

## Diff table (fill from EITHER capture)

| Apply # | stages set in UI | active | raw blob bytes (hex) |
|---|---|---|---|
| 1 | e.g. 400/800/800/800/800/800 | 1 | `<paste>` |
| 2 | same, active → 3 | 3 | `<paste>` |
| 3 | stage1 400 → 1600 | 3 | `<paste>` |

Decode rules once the table is filled: bytes that change only between #1→#2 =
active-stage byte; bytes that change only in #3 = stage-1 value bytes
(compare against raw = dpi/50−1 LE); trailing bytes that track all changes =
checksum (test BE16 sum, then sum−1 u8 variants documented in `protocol.rs`).

## Session log 2026-10-12, this Windows box (all live, all verified)

**Shim capture: hidapi.dll is enumerate-only.** With the logging proxy in
place the app started fine and the mouse worked, but one full session
(startup + 3 user Apply clicks) logged ONLY `hid_init` / `hid_enumerate` /
`hid_free_enumeration` — zero `open_path` / `write` / `send_feature_report`.
Config traffic does NOT pass through hidapi; it goes via `hiddriver_*.dll`
(interrupt writes), matching the repo README's usbmon finding. The 9 hidapi
imports are real but used for scanning only.

**Vendor scan table** (VID:PID per `hid_enumerate` call, in order):
`045e:0085,004d,004e,00be,0001,0003,0040,0055,00b1,0007,0002,0006,0008,0004,000e,00e5,0224,00ea`,
then `1d57:fa60`, `1d57:fa65` ×2, `1d57:fa60`. Wired `1d57:2224` is NOT scanned.

**Device reads back on MI_02 col04** (own ctypes handle, system hid.dll;
col01–03 refuse everything, err=1):
- `GET 0xA0` → `a0 01 00 00 00 00 00 00` (byte[1]=`01`)
- `GET 0x04` len 56 → `08 3b 01 02 00 00 03 00 00 04 00 00 05 00 00 06 00 00 0d 00 00 3c 00 00 [01 00 00 ×10] 0a 00 00 09 00`
  = the LIVE button map (header `08 3b 01`; left=02 right=03 middle=04
  back=05 fwd=06 dpi-cycle=0d, ten disabled slots, scroll 0a/09).
  Note it answers report-`08` content to a `0x04` query at len 56; len 57
  times out. `0x05`/`0x06`/`0x09` refuse (err 87) or time out (err 121) at
  every tried length — report `09` is write-only from the host side.

**hiddriver_1.dll surface** (corrected `<IIHHIIIIIII` parse):
`Set_VIDPID(1) Open_DevMonitor(2) Close_DevMonitor(3) Open_FeatureDevice(4)
Close_FeatureDevice(5) SetFeature(6) GetFeature(7) Hid_Read(8) Hid_Write(9)` —
undecorated names. Crude `ret`-scan hints Set/GetFeature take 2 args
(ret 8b); other arities inconclusive (first-C3 heuristic, unreliable).

**Open:** `AttackShark.exe` contains NO `hiddriver` string literal, yet loads
one of these DLLs (it imports LoadLibraryW/GetProcAddress) — name is built
at runtime or loaded via Hook.dll. Next: list the app's loaded modules while
running, or search Hook.dll, then proxy THAT dll the same shim way.

**Friction log (own mistakes, for the record):** first enumerate returned 0
(64-bit handle truncated by ctypes `c_int` default — fixed); first swap
command died on POSIX-shell `$var` expansion (use literal paths); restores
blocked twice by the app secretly still running (same PID); one log read
showed 0 bytes (stale — file was still being written); first export parse was
off by one (`Major/Minor` are WORDs) and falsely cried "obfuscated".
