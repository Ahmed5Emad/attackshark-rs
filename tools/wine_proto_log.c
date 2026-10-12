// Protocol oracle: records every HID command the vendor app attempts
// under Wine — full feature-report payloads plus interrupt OUT writes.
// Click through the app; /tmp/wine-proto.log becomes the spec we
// implement natively on Linux.
// Usage: LD_PRELOAD=<this.so> wine AttackShark.exe   (rootless)
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/stat.h>
#include <unistd.h>

static int logfd = -2;

static void emit(const char *tag, int fd, const unsigned char *b,
                 unsigned len) {
    if (logfd == -2)
        logfd = open("/tmp/wine-proto.log", O_WRONLY | O_CREAT | O_APPEND,
                     0644);
    if (logfd < 0)
        return;
    char head[96];
    int n = snprintf(head, sizeof head, "%s fd=%d len=%u ", tag, fd, len);
    (void)write(logfd, head, (size_t)n);
    unsigned cap = len > 128 ? 128 : len;
    char hex[384];
    for (unsigned i = 0; i < cap; i++)
        snprintf(hex + i * 3, 4, "%02x ", b[i]);
    (void)write(logfd, hex, (size_t)(cap * 3));
    (void)write(logfd, "\n", 1);
}

// hidraw fds only (avoid logging every socket/X11 write).
static int hid_fds[64];
static int hid_n = 0;

static int is_hidraw(int fd) {
    for (int i = 0; i < hid_n; i++)
        if (hid_fds[i] == fd)
            return 1;
    for (int i = 0; i < hid_n; i++)
        if (hid_fds[i] == -1) {
            hid_fds[i] = fd;
            return 2; // new, unclassified below
        }
    if (hid_n < 64) {
        hid_fds[hid_n++] = fd;
        return 2;
    }
    return 0;
}

static int check_hidraw(int fd) {
    int r = is_hidraw(fd);
    if (r != 2)
        return r;
    char link[64], dest[128];
    snprintf(link, sizeof link, "/proc/self/fd/%d", fd);
    ssize_t n = readlink(link, dest, sizeof dest - 1);
    int ok = (n > 0 && memmem(dest, (size_t)n, "hidraw", 6) != NULL);
    // patch the cache entry
    for (int i = 0; i < hid_n; i++)
        if (hid_fds[i] == fd)
            hid_fds[i] = ok ? fd : -2;
    return ok;
}

ssize_t write(int fd, const void *buf, size_t count) {
    static ssize_t (*real_write)(int, const void *, size_t) = NULL;
    if (!real_write)
        real_write = dlsym(RTLD_NEXT, "write");
    if (check_hidraw(fd) == 1 && count > 0)
        emit("OUT", fd, (const unsigned char *)buf,
             (unsigned)(count > 128 ? 128 : count));
    return real_write(fd, buf, count);
}

int ioctl(int fd, unsigned long req, ...) {
    static int (*real_ioctl)(int, unsigned long, ...) = NULL;
    if (!real_ioctl)
        real_ioctl = dlsym(RTLD_NEXT, "ioctl");
    va_list ap;
    va_start(ap, req);
    void *arg = va_arg(ap, void *);
    va_end(ap);
    if (((req >> 8) & 0xff) == 'H') {
        unsigned nr = req & 0xff;
        unsigned len = (req >> 16) & 0x3fff;
        if (nr == 0x06 || nr == 0x07)
            emit(nr == 0x06 ? "SET" : "GET", fd,
                 (const unsigned char *)arg, len);
    }
    return real_ioctl(fd, req, arg);
}
