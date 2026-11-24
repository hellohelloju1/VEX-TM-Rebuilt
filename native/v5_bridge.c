// Intel helper uses the same exported ABI as the supplied app's ctypes driver.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>
#include <dlfcn.h>
#include <pthread.h>
#include <stdatomic.h>
#include <time.h>
static int (*Count)(void),(*New)(int);
static void (*Close)(int),(*Idle)(int,int),(*Disable)(int,int),(*Enable)(int,int),(*Auto)(int,int),(*SetActive)(int,int),(*SetTimer)(int,int),(*SetFieldID)(int,int,int),(*SetMatchName)(int,int,int,int),(*Free)(void*);
static char*(*GetStatus)(int);
static atomic_int handle=-1,armed=0,done=0,tripped=0;
static _Atomic uint64_t heartbeat=0;
static uint64_t now(void){struct timespec t;clock_gettime(CLOCK_MONOTONIC,&t);return(uint64_t)t.tv_sec*1000+t.tv_nsec/1000000;}
static void stop(void){int h=atomic_load(&handle);atomic_store(&armed,0);if(h>=0){Disable(h,0);Idle(h,0);}}
static void*watch(void*p){(void)p;while(!atomic_load(&done)){usleep(100000);if(atomic_load(&armed)&&now()-atomic_load(&heartbeat)>2000){atomic_store(&armed,0);atomic_store(&tripped,1);int h=atomic_load(&handle);if(h>=0)Disable(h,0);}}return NULL;}
#define LOAD(name) do{*(void**)(&name)=dlsym(lib,#name);if(!name){fprintf(stdout,"TMRS {\"ok\":false,\"error\":\"Missing "#name" export\"}\n");return 2;}}while(0)
int main(int argc,char**argv){setvbuf(stdout,NULL,_IOLBF,0);if(argc!=2)return 2;void*lib=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);if(!lib){printf("TMRS {\"ok\":false,\"error\":\"Cannot load installed V5 driver\"}\n");return 2;}LOAD(Count);LOAD(New);LOAD(Close);LOAD(Idle);LOAD(Disable);LOAD(Enable);LOAD(Auto);LOAD(SetActive);LOAD(SetTimer);LOAD(SetFieldID);LOAD(SetMatchName);LOAD(GetStatus);LOAD(Free);atomic_store(&heartbeat,now());pthread_t thread;pthread_create(&thread,NULL,watch,NULL);char line[2048];while(fgets(line,sizeof(line),stdin)){char cmd[40];int a=0,b=0,c=0;if(sscanf(line,"%39s %d %d %d",cmd,&a,&b,&c)<1)continue;atomic_store(&heartbeat,now());if(!strcmp(cmd,"count")){printf("TMRS {\"ok\":true,\"count\":%d}\n",Count());continue;}if(!strcmp(cmd,"open")){int n=Count();if(a<0||a>=n){puts("TMRS {\"ok\":false,\"error\":\"Controller index is not available\"}");continue;}stop();int old=atomic_exchange(&handle,-1);if(old>=0)Close(old);int h=New(a);atomic_store(&tripped,0);atomic_store(&handle,h);Disable(h,0);Idle(h,0);puts("TMRS {\"ok\":true}");continue;}int h=atomic_load(&handle);if(!strcmp(cmd,"close")){stop();if(h>=0)Close(h);atomic_store(&handle,-1);puts("TMRS {\"ok\":true}");break;}if(h<0){puts("TMRS {\"ok\":false,\"error\":\"Connect a controller first\"}");continue;}if(!strcmp(cmd,"status")){char*json=GetStatus(h);printf("TMRS {\"ok\":true,\"watchdog_tripped\":%s,\"status\":%s}\n",atomic_load(&tripped)?"true":"false",json?json:"null");if(json)Free(json);continue;}if(!strcmp(cmd,"auto")){Auto(h,0);atomic_store(&armed,1);}else if(!strcmp(cmd,"driver")){Enable(h,0);atomic_store(&armed,1);}else if(!strcmp(cmd,"disable")){atomic_store(&armed,0);Disable(h,0);}else if(!strcmp(cmd,"idle")){stop();Idle(h,0);}else if(!strcmp(cmd,"active")){SetActive(h,a);}else if(!strcmp(cmd,"timer")){SetTimer(h,a);}else if(!strcmp(cmd,"field")){SetFieldID(h,a,b);}else if(!strcmp(cmd,"match")){SetMatchName(h,a,b,c);}else if(strcmp(cmd,"heartbeat")){puts("TMRS {\"ok\":false,\"error\":\"Unknown command\"}");continue;}puts("TMRS {\"ok\":true}");}stop();int h=atomic_load(&handle);if(h>=0)Close(h);atomic_store(&done,1);pthread_join(thread,NULL);return 0;}
