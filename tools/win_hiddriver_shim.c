// win_hiddriver_shim.c — logging proxy for the vendor hiddriver_N.dll (32-bit ONLY).
//
// Why this exists: the vendor app (AttackShark.exe, 32-bit) does NOT send config
// through hidapi.dll (enumerate-only, proven 2026-10-12). It GetProcAddress's 7
// names from hiddriver_N.dll (verified ASCII in the exe; Hid_Read/Hid_Write are
// absent narrow AND wide, so the live config path is SetFeature):
//   Set_VIDPID, Open_DevMonitor, Close_DevMonitor,
//   Open_FeatureDevice, Close_FeatureDevice, SetFeature, GetFeature
// The DLL picked per model is a wide string (hiddriver_1..18 minus 8, 4k_1).
// All 17 standard ones are byte-identical (md5 07823eb5...), so ONE build works
// under ANY of those names: the shim learns its own filename at runtime and
// loads <same>_real.dll from the same folder (e.g. shim as hiddriver_1.dll
// forwards to hiddriver_1_real.dll).
//
// Signatures (verified from the real DLL's code: ret-imm + ebp refs agree):
//   Set_VIDPID(a,b)        2 args, ret 8   — stores VID/PID globals
//   Open_DevMonitor(a,b,c) 3 args, ret 12  — hwnd/msg-ish monitor
//   Close_DevMonitor()     0 args          — no frame, tiny stub
//   Open_FeatureDevice()   0 args          — opens via stored VID/PID
//   Close_FeatureDevice()  0 args          — uses global handle
//   SetFeature(buf,len)    2 args, ret 8   — wraps HidD_SetFeature(handle,buf,len)
//   GetFeature(buf,len)    2 args, ret 8   — wraps HidD_GetFeature(handle,buf,len)
//   Hid_Read / Hid_Write   arity unconfirmed + never called by the exe, so they
//                          are naked jmp trampolines: raw stack dwords are logged,
//                          then control jumps to the real function. No signature
//                          knowledge needed, no stack risk.
//
// Deploy (one DLL only — find which N the app has loaded first):
//   cmd /c "tasklist /m hiddriver*"   (or PowerShell: (Get-Process AttackShark).Modules)
//   1. Close the vendor app. Rename e.g. hiddriver_1.dll -> hiddriver_1_real.dll
//   2. Copy the built shim in as hiddriver_1.dll (same name as the renamed one)
//   3. Start the app, Apply pattern #1 / #2 / #3, close the app
//   4. Read %TEMP%\hiddriver-shim.log, then restore the real DLL.
// If unsure which N: the shim binary is name-agnostic, copy it over every
// hiddriver_N.dll (after backing each up as hiddriver_N_real.dll).
//
// Build on Linux (this Windows box has NO compiler):
//   i686-w64-mingw32-gcc -shared -m32 -O2 -o hiddriver_shim.dll \
//       win_hiddriver_shim.c win_hiddriver_shim.def -Wl,--enable-stdcall-fixup
//
// Log format: <ms-tick> <dll-tag> <func> len=%u rc=%d <hex...>   (Set/GetFeature)
//             <ms-tick> <dll-tag> <func> vid:pid 0405... rc=%d   (Set_VIDPID)
//             <ms-tick> <dll-tag> <func>(%08x %08x %08x) rc=%d   (controls)
//             <ms-tick> <dll-tag> Hid_Read/Write raw ret=... args=6 dwords

#include <windows.h>
#include <stdio.h>
#include <string.h>
#include <stdint.h>

static HMODULE g_self = NULL;
static HMODULE g_real = NULL;
static FILE *g_log = NULL;
static CRITICAL_SECTION g_cs;
static char g_tag[32] = "hiddriver";
static LONG g_once = 0;
static FARPROC hidrv_pRead = NULL;
static FARPROC hidrv_pWrite = NULL;

void hidrv_ensure_init(void) {
    if (InterlockedCompareExchange(&g_once, 1, 0) != 0) return;
    InitializeCriticalSection(&g_cs);
    char self[MAX_PATH] = {0};
    GetModuleFileNameA(g_self, self, sizeof(self));
    char *b = self + strlen(self);
    while (b > self && *(b - 1) != '\\' && *(b - 1) != '/') b--;
    strncpy(g_tag, b, sizeof(g_tag) - 1);
    g_tag[sizeof(g_tag) - 1] = '\0';
    char *dot = strrchr(b, '.');
    if (dot) *dot = '\0';
    char real[MAX_PATH] = {0};
    snprintf(real, sizeof(real), "%s_real.dll", self);
    g_real = LoadLibraryA(real);
    char tmp[MAX_PATH] = {0};
    GetTempPathA(sizeof(tmp), tmp);
    strcat(tmp, "hiddriver-shim.log");
    g_log = fopen(tmp, "a");
    if (g_log) {
        fprintf(g_log, "=== shim attached tick=%lu tag=%s real=%s h=%p ===\n",
                GetTickCount(), b, real, (void *)g_real);
        fflush(g_log);
    }
    if (g_real) {
        hidrv_pRead = GetProcAddress(g_real, "Hid_Read");
        hidrv_pWrite = GetProcAddress(g_real, "Hid_Write");
    }
}

static FARPROC fn(const char *name) {
    hidrv_ensure_init();
    if (!g_real) return NULL;
    return GetProcAddress(g_real, name);
}

static void log_buf(const char *func, const unsigned char *b, unsigned len, int rc) {
    hidrv_ensure_init();
    EnterCriticalSection(&g_cs);
    if (g_log) {
        unsigned cap = len > 96 ? 96 : len;
        fprintf(g_log, "%lu %s %s len=%u rc=%d ", GetTickCount(), g_tag, func, len, rc);
        if (b) {
            for (unsigned i = 0; i < cap; i++) fprintf(g_log, "%02x ", b[i]);
            if (len > cap) fprintf(g_log, "...");
        } else fprintf(g_log, "(null)");
        fprintf(g_log, "\n");
        fflush(g_log);
    }
    LeaveCriticalSection(&g_cs);
}

static void log_call(const char *func, unsigned a, unsigned b, unsigned c,
                     int nargs, int rc) {
    hidrv_ensure_init();
    EnterCriticalSection(&g_cs);
    if (g_log) {
        if (nargs == 0)
            fprintf(g_log, "%lu %s %s() rc=%d\n", GetTickCount(), g_tag, func, rc);
        else if (nargs == 2)
            fprintf(g_log, "%lu %s %s(%04x:%04x) rc=%d\n",
                    GetTickCount(), g_tag, func, a & 0xffff, b & 0xffff, rc);
        else
            fprintf(g_log, "%lu %s %s(%08x %08x %08x) rc=%d\n",
                    GetTickCount(), g_tag, func, a, b, c, rc);
        fflush(g_log);
    }
    LeaveCriticalSection(&g_cs);
}

// Generic raw-stack tracer for the naked trampolines. ctx points at the
// pushal block: ctx[0..7]=EDI..EAX, ctx[8]=return addr, ctx[9..]=caller args.
void hidrv_trace(unsigned id, unsigned *ctx) {
    hidrv_ensure_init();
    const char *nm = (id == 8) ? "Hid_Read" : "Hid_Write";
    EnterCriticalSection(&g_cs);
    if (g_log) {
        unsigned *stk = ctx + 9;
        fprintf(g_log, "%lu %s %s raw ret=%08x args=%08x %08x %08x %08x %08x %08x\n",
                GetTickCount(), g_tag, nm, ctx[8],
                stk[0], stk[1], stk[2], stk[3], stk[4], stk[5]);
        fflush(g_log);
    }
    LeaveCriticalSection(&g_cs);
}

// ---- forwarded API (stdcall, undecorated — see .def) ----

__declspec(dllexport) int __stdcall Set_VIDPID(int vid, int pid) {
    FARPROC p = fn("Set_VIDPID");
    int rc = p ? ((int(__stdcall *)(int, int))p)(vid, pid) : -1;
    log_call("Set_VIDPID", (unsigned)vid, (unsigned)pid, 0, 2, rc);
    return rc;
}

__declspec(dllexport) int __stdcall Open_DevMonitor(void *a, int b, int c) {
    FARPROC p = fn("Open_DevMonitor");
    int rc = p ? ((int(__stdcall *)(void *, int, int))p)(a, b, c) : -1;
    log_call("Open_DevMonitor", (unsigned)(uintptr_t)a, (unsigned)b, (unsigned)c, 3, rc);
    return rc;
}

__declspec(dllexport) int __stdcall Close_DevMonitor(void) {
    FARPROC p = fn("Close_DevMonitor");
    int rc = p ? ((int(__stdcall *)(void))p)() : -1;
    log_call("Close_DevMonitor", 0, 0, 0, 0, rc);
    return rc;
}

__declspec(dllexport) int __stdcall Open_FeatureDevice(void) {
    FARPROC p = fn("Open_FeatureDevice");
    int rc = p ? ((int(__stdcall *)(void))p)() : -1;
    log_call("Open_FeatureDevice", 0, 0, 0, 0, rc);
    return rc;
}

__declspec(dllexport) int __stdcall Close_FeatureDevice(void) {
    FARPROC p = fn("Close_FeatureDevice");
    int rc = p ? ((int(__stdcall *)(void))p)() : -1;
    log_call("Close_FeatureDevice", 0, 0, 0, 0, rc);
    return rc;
}

__declspec(dllexport) int __stdcall SetFeature(unsigned char *buf, int len) {
    FARPROC p = fn("SetFeature");
    int rc = p ? ((int(__stdcall *)(unsigned char *, int))p)(buf, len) : -1;
    log_buf("SetFeature", buf, (unsigned)(len < 0 ? 0 : len), rc);
    return rc;
}

__declspec(dllexport) int __stdcall GetFeature(unsigned char *buf, int len) {
    FARPROC p = fn("GetFeature");
    int rc = p ? ((int(__stdcall *)(unsigned char *, int))p)(buf, len) : -1;
    if (rc) log_buf("GetFeature", buf, (unsigned)(len < 0 ? 0 : len), rc);
    else log_call("GetFeature", 0, 0, 0, 0, rc);
    return rc;
}

// ---- naked jmp trampolines (no arity assumed) ----
static void (*hidrv_pEnsure)(void) = hidrv_ensure_init;
static void (*hidrv_pTrace)(unsigned, unsigned *) = hidrv_trace;

__attribute__((naked)) __declspec(dllexport) void Hid_Read(void) {
    __asm__ volatile (
        "pushal\n\t"
        "call *%0\n\t"
        "movl %%esp, %%eax\n\t"
        "pushl %%eax\n\t"
        "pushl $8\n\t"
        "call *%1\n\t"
        "addl $8, %%esp\n\t"
        "popal\n\t"
        "jmp *%2\n\t"
        :: "m" (hidrv_pEnsure), "m" (hidrv_pTrace), "m" (hidrv_pRead)
        : "memory");
}

__attribute__((naked)) __declspec(dllexport) void Hid_Write(void) {
    __asm__ volatile (
        "pushal\n\t"
        "call *%0\n\t"
        "movl %%esp, %%eax\n\t"
        "pushl %%eax\n\t"
        "pushl $9\n\t"
        "call *%1\n\t"
        "addl $8, %%esp\n\t"
        "popal\n\t"
        "jmp *%2\n\t"
        :: "m" (hidrv_pEnsure), "m" (hidrv_pTrace), "m" (hidrv_pWrite)
        : "memory");
}

BOOL APIENTRY DllMain(HMODULE h, DWORD reason, LPVOID r) {
    (void)r;
    if (reason == DLL_PROCESS_ATTACH) {
        g_self = h;  // real DLL is loaded lazily in hidrv_ensure_init (loader-lock safe)
    } else if (reason == DLL_PROCESS_DETACH) {
        if (g_log) { fclose(g_log); g_log = NULL; }
    }
    return TRUE;
}
