#include <stdlib.h>
#include <stdio.h>
#include <string.h>
static const char*mode="idle";
int Count(void){return 1;}int New(int index){return index+1;}void Close(int h){(void)h;}void Idle(int h,int t){(void)h;(void)t;mode="idle";}void Disable(int h,int t){(void)h;(void)t;mode="disable";}void Enable(int h,int t){(void)h;(void)t;mode="driver";}void Auto(int h,int t){(void)h;(void)t;mode="auto";}void SetActive(int h,int a){(void)h;(void)a;}void SetTimer(int h,int t){(void)h;(void)t;}void SetFieldID(int h,int a,int b){(void)h;(void)a;(void)b;}void SetMatchName(int h,int a,int b,int c){(void)h;(void)a;(void)b;(void)c;}void Free(void*p){free(p);}char*GetStatus(int h){(void)h;char b[200];snprintf(b,sizeof b,"{\"Mode\":\"%s\",\"FieldStatus\":[\"Active\"],\"TeamStatus\":[]}",mode);return strdup(b);}
