// win_hid_shim.c — logging proxy for the vendor hidapi.dll (32-bit ONLY).
//
// The vendor app (AttackShark.exe, 32-bit) imports 9 stdcall symbols from
// hidapi.dll (verified from its IAT 2026-10-12):
//   _hid_init@0 _hid_enumerate@8 _hid_open_path@4 _hid_free_enumeration@4
//   _hid_write@12 _hid_read@12 _hid_send_feature_report@12
//   _hid_get_feature_report@12 _hid_set_nonblocking@8
//
// Deploy: close the app, rename real hidapi.dll -> hidapi_real.dll, drop the
// built shim as hidapi.dll next to it, run the app, click Apply, read
// %TEMP%\hidapi-shim.log, then restore the real DLL.
//
// Build with 32-bit MinGW (on any box that has it; this Windows box has NO
// compiler, which is why the DLL is not prebuilt):
//   i686-w64-mingw32-gcc -shared -m32 -O2 -o hidapi.dll win_hid_shim.c \
//       win_hid_shim.def -Wl,--enable-stdcall-fixup
// MSVC alternative: cl /LD /D_USING_V110_SDK71_ win_hid_shim.c hidapi_shim.def
//
// Log format per line: <ms-tick> <func> <len> <hex bytes...> ; <result>
// See win_hid_shim.def for the export list (stdcall decorations preserved).

#include <windows.h>
#include <stdio.h>
#include <string.h>

static HMODULE g_real = NULL;
static FILE *g_log = NULL;
static CRITICAL_SECTION g_cs;

static void ensure_init(void) {
    static LONG once = 0;
    if (InterlockedCompareExchange(&once, 1, 0) != 0) return;
    InitializeCriticalSection(&g_cs);
    g_real = LoadLibraryA("hidapi_real.dll");
    if (!g_real) {
        char dir[MAX_PATH] = {0};
        GetModuleFileNameA(NULL, dir, sizeof(dir));
        char *s = dir;
        char *last = dir;
        while (*s) { if (*s == '\\' || *s == '/') last = s + 1; s++; }
        strcpy(last, "hidapi_real.dll");
        g_real = LoadLibraryA(dir);
    }
    char tmp[MAX_PATH] = {0};
    GetTempPathA(sizeof(tmp), tmp);
    strcat(tmp, "hidapi-shim.log");
    g_log = fopen(tmp, "a");
    if (g_log) {
        fprintf(g_log, "=== shim attached tick=%lu real=%p ===\n",
                GetTickCount(), (void *)g_real);
        fflush(g_log);
    }
}

// The vendor DLL's export table is nonstandard (static parse shows garbage),
// so resolve each symbol plain first, then by its stdcall-decorated alias
// (the decorated form is guaranteed to exist — the exe imports exactly that).
static const char *decorated(const char *n) {
    if (!strcmp(n, "hid_init")) return "_hid_init@0";
    if (!strcmp(n, "hid_enumerate")) return "_hid_enumerate@8";
    if (!strcmp(n, "hid_open_path")) return "_hid_open_path@4";
    if (!strcmp(n, "hid_free_enumeration")) return "_hid_free_enumeration@4";
    if (!strcmp(n, "hid_write")) return "_hid_write@12";
    if (!strcmp(n, "hid_read")) return "_hid_read@12";
    if (!strcmp(n, "hid_send_feature_report")) return "_hid_send_feature_report@12";
    if (!strcmp(n, "hid_get_feature_report")) return "_hid_get_feature_report@12";
    if (!strcmp(n, "hid_set_nonblocking")) return "_hid_set_nonblocking@8";
    return NULL;
}

static FARPROC fn(const char *name) {
    ensure_init();
    if (!g_real) return NULL;
    FARPROC p = GetProcAddress(g_real, name);
    if (!p) {
        const char *d = decorated(name);
        if (d) p = GetProcAddress(g_real, d);
    }
    return p;
}

static void log_buf(const char *tag, const unsigned char *b, unsigned len, int rc) {
    ensure_init();
    EnterCriticalSection(&g_cs);
    if (g_log && b) {
        unsigned cap = len > 128 ? 128 : len;
        fprintf(g_log, "%lu %s len=%u rc=%d ", GetTickCount(), tag, len, rc);
        for (unsigned i = 0; i < cap; i++) fprintf(g_log, "%02x ", b[i]);
        if (len > cap) fprintf(g_log, "...");
        fprintf(g_log, "\n");
        fflush(g_log);
    }
    LeaveCriticalSection(&g_cs);
}

// ---- forwarded API (stdcall, same signatures as hidapi 0.x) ----

struct hid_device_info; // opaque

__declspec(dllexport) int __stdcall hid_init(void) {
    FARPROC p = fn("hid_init");
    int rc = p ? ((int(__stdcall *)(void))p)() : -1;
    log_buf("hid_init", (unsigned char *)"", 0, rc);
    return rc;
}

__declspec(dllexport) struct hid_device_info *__stdcall hid_enumerate(
    unsigned short vid, unsigned short pid) {
    FARPROC p = fn("hid_enumerate");
    struct hid_device_info *r = p
        ? ((struct hid_device_info *(__stdcall *)(unsigned short, unsigned short))p)(vid, pid)
        : NULL;
    unsigned char b[4] = {vid & 0xff, vid >> 8, pid & 0xff, pid >> 8};
    log_buf("hid_enumerate", b, 4, r ? 1 : 0);
    return r;
}

__declspec(dllexport) void *__stdcall hid_open_path(const char *path) {
    FARPROC p = fn("hid_open_path");
    void *r = p ? ((void *(__stdcall *)(const char *))p)(path) : NULL;
    size_t plen = path ? strlen(path) : 0;
    log_buf("hid_open_path", (unsigned char *)(path ? path : ""), (unsigned)plen, r ? 1 : 0);
    return r;
}

__declspec(dllexport) void __stdcall hid_free_enumeration(struct hid_device_info *d) {
    FARPROC p = fn("hid_free_enumeration");
    if (p) ((void(__stdcall *)(struct hid_device_info *))p)(d);
    log_buf("hid_free_enumeration", (unsigned char *)"", 0, 0);
}

__declspec(dllexport) int __stdcall hid_write(void *dev, const unsigned char *data, size_t len) {
    FARPROC p = fn("hid_write");
    int rc = p ? ((int(__stdcall *)(void *, const unsigned char *, size_t))p)(dev, data, len) : -1;
    log_buf("hid_write", data, (unsigned)len, rc);
    return rc;
}

__declspec(dllexport) int __stdcall hid_read(void *dev, unsigned char *data, size_t len) {
    FARPROC p = fn("hid_read");
    int rc = p ? ((int(__stdcall *)(void *, unsigned char *, size_t))p)(dev, data, len) : -1;
    if (rc > 0) log_buf("hid_read", data, (unsigned)rc, rc);
    return rc;
}

__declspec(dllexport) int __stdcall hid_send_feature_report(
    void *dev, const unsigned char *data, size_t len) {
    FARPROC p = fn("hid_send_feature_report");
    int rc = p ? ((int(__stdcall *)(void *, const unsigned char *, size_t))p)(dev, data, len) : -1;
    log_buf("hid_send_feature_report", data, (unsigned)len, rc);
    return rc;
}

__declspec(dllexport) int __stdcall hid_get_feature_report(
    void *dev, unsigned char *data, size_t len) {
    FARPROC p = fn("hid_get_feature_report");
    int rc = p ? ((int(__stdcall *)(void *, unsigned char *, size_t))p)(dev, data, len) : -1;
    if (rc >= 0) log_buf("hid_get_feature_report", data, (unsigned)len, rc);
    else log_buf("hid_get_feature_report", data, (unsigned)len, rc);
    return rc;
}

__declspec(dllexport) int __stdcall hid_set_nonblocking(void *dev, int nb) {
    FARPROC p = fn("hid_set_nonblocking");
    int rc = p ? ((int(__stdcall *)(void *, int))p)(dev, nb) : -1;
    unsigned char b[1] = {(unsigned char)nb};
    log_buf("hid_set_nonblocking", b, 1, rc);
    return rc;
}

BOOL APIENTRY DllMain(HMODULE h, DWORD reason, LPVOID r) {
    (void)h; (void)r;
    if (reason == DLL_PROCESS_DETACH && g_log) fclose(g_log);
    return TRUE;
}
