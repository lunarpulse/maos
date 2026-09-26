#define _GNU_SOURCE
#include <poll.h>
#include <errno.h>
#include <signal.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
/* Emulates the T2 allow-list's mismatch action (Errno(EPERM)) for the two
   pre-main calls Rust std makes that the allow-list omits; toggled per case. */
int poll(struct pollfd *f, nfds_t n, int t) {
  if (getenv("P19_POLL")) { errno = EPERM; return -1; }
  struct timespec ts = { t / 1000, (long)(t % 1000) * 1000000L };
  return ppoll(f, n, t < 0 ? NULL : &ts, NULL);
}
sighandler_t signal(int s, sighandler_t h) {
  if (getenv("P19_SIGNAL")) { errno = EPERM; return SIG_ERR; }
  struct sigaction a, o; memset(&a,0,sizeof a); a.sa_handler=h; sigaction(s,&a,&o); return o.sa_handler;
}
