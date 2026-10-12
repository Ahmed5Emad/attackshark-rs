// Log-only shim: records every HID feature-report ioctl Wine attempts.
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <unistd.h>

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
        if (nr == 0x06 || nr == 0x07) {
            unsigned char *b = (unsigned char *)arg;
            int log = open("/tmp/wine-hid-ioctl.log",
                           O_WRONLY | O_CREAT | O_APPEND, 0644);
            if (log >= 0) {
                char line[256];
                int n = snprintf(line, sizeof line,
                                 "fd=%d %s len=%u id=%02x data=%02x%02x%02x%02x%02x%02x%02x%02x\n",
                                 fd, nr == 0x06 ? "SET" : "GET", len,
                                 b ? b[0] : 0, b ? b[1] : 0, b ? b[2] : 0,
                                 b ? b[3] : 0, b ? b[4] : 0, b ? b[5] : 0,
                                 b ? b[6] : 0, b ? b[7] : 0);
                (void)write(log, line, (size_t)n);
                close(log);
            }
        }
    }
    return real_ioctl(fd, req, arg);
}
