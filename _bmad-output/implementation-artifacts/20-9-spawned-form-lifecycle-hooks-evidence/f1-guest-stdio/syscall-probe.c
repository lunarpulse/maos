#define _GNU_SOURCE
#include <stdio.h>
#include <errno.h>
#include <unistd.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/epoll.h>
#include <sys/eventfd.h>
#include <pthread.h>
static void *f(void *a){return a;}
static void rep(const char*n,long r,int e){char b[160];int l=snprintf(b,sizeof b,"%-14s ret=%ld errno=%d (%s)\n",n,r,r<0?e:0,r<0?strerror(e):"ok");write(1,b,l);}
int main(){
 long r; errno=0; r=syscall(SYS_epoll_create1,0); rep("epoll_create1",r,errno);
 errno=0; r=syscall(SYS_eventfd2,0,0); rep("eventfd2",r,errno);
 errno=0; r=syscall(SYS_timerfd_create,1,0); rep("timerfd_create",r,errno);
 errno=0; r=syscall(SYS_socketpair,1,1,0,(int[2]){0}); rep("socketpair",r,errno);
 pthread_t t; int e=pthread_create(&t,0,f,0); errno=e; rep("pthread_create",e?-1:0,e); if(!e)pthread_join(t,0);
 return 0;}
