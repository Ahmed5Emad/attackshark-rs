// Routes 0xa0 feature reports to the mouse's vendor interface with a
// valid length. Wine sends bare 1-byte SETs (id only, no payload) which
// the kernel rejects with EINVAL; those are the app's config query,
// rebuilt here as the 8-byte handshake the device answers. GETs and
// longer SETs pass through to the vendor node unchanged.
// Rootless: LD_PRELOAD=/path/wine_hid_fix.so wine AttackShark.exe
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <unistd.h>
#include <linux/hidraw.h>
#include <linux/input.h>

static int vfd = -2;

static void dbg(const char *msg, int a, int b) {
    char line[160];
    int n = snprintf(line, sizeof line, "fix: %s %d %d\n", msg, a, b);
    int log = open("/tmp/wine-fix-debug.log", O_WRONLY | O_CREAT | O_APPEND,
                   0644);
    if (log >= 0) {
        (void)write(log, line, (size_t)n);
        close(log);
    }
}

static int find_vendor(void) {
    for (int i = 0; i < 32; i++) {
        char p[32];
        snprintf(p, sizeof p, "/dev/hidraw%d", i);
        int fd = open(p, O_RDWR);
        if (fd < 0)
            continue;
        struct hidraw_devinfo info;
        memset(&info, 0, sizeof info);
        if (ioctl(fd, HIDIOCGRAWINFO, &info) == 0 && info.vendor == 0x1d57 &&
            (info.product == 0x2224 || info.product == 0xfa60 ||
             info.product == 0xfa65)) {
            int sz = 0;
            if (ioctl(fd, HIDIOCGRDESCSIZE, &sz) == 0 && sz > 0 && sz < 8192) {
                struct hidraw_report_descriptor desc;
                memset(&desc, 0, sizeof desc);
                desc.size = sz;
                if (ioctl(fd, HIDIOCGRDESC, &desc) == 0) {
                    for (int k = 0; k + 1 < sz; k++) {
                        if (desc.value[k] == 0x85 && desc.value[k + 1] == 0xa0) {
                            vfd = fd;
                            return fd;
                        }
                    }
                }
            }
        }
        close(fd);
    }
    vfd = -1;
    return -1;
}

int ioctl(int fd, unsigned long req, ...) {
    static int (*real_ioctl)(int, unsigned long, ...) = NULL;
    if (!real_ioctl)
        real_ioctl = dlsym(RTLD_NEXT, "ioctl");
    va_list ap;
    va_start(ap, req);
    void *arg = va_arg(ap, void *);
    va_end(ap);
    unsigned dir = (req >> 30) & 3;
    if (((req >> 8) & 0xff) == 'H') {
        unsigned nr = req & 0xff;
        unsigned len = (req >> 16) & 0x3fff;
        if ((nr == 0x06 && dir == 1) || (nr == 0x07 && dir == 2)) {
            unsigned char *b = (unsigned char *)arg;
            if (b && b[0] == 0xa0) {
                if (vfd == -2)
                    find_vendor();
                dbg("a0 seen, vfd =", vfd, (int)len);
                if (vfd >= 0) {
                    if (nr == 0x06) {
                        // SET_FEATURE -> interrupt OUT write (the kernel
                        // rejects 0xa0 feature ioctls, but the device
                        // answers the same bytes as output reports).
                        // Bare-ID query carries no payload: send the
                        // 8-byte config handshake instead.
                        unsigned char q[8] = {0xa0, 0x08, 0, 0, 0, 0, 0, 0};
                        unsigned char *out = b;
                        unsigned olen = len;
                        if (len < 8) {
                            out = q;
                            olen = 8;
                        }
                        ssize_t w = write(vfd, out, olen);
                        dbg("SET->OUT wrote =", (int)w, (int)olen);
                        if (w >= 0)
                            return (int)olen;
                        return -1;
                    }
                    int r = real_ioctl(vfd, req, arg);
                    dbg("passthru rc =", r, (int)nr);
                    return r;
                }
            }
        }
    }
    return real_ioctl(fd, req, arg);
}
