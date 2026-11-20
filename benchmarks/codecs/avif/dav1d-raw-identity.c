#include <dav1d/dav1d.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(int argc,char **argv) {
 if(argc!=3)return 1; FILE *f=fopen(argv[1],"rb"); if(!f)return 2; fseek(f,0,SEEK_END); long n=ftell(f); rewind(f);
 Dav1dSettings s; dav1d_default_settings(&s); s.n_threads=1; s.max_frame_delay=1; s.apply_grain=1; Dav1dContext *c=NULL; Dav1dData d={0}; Dav1dPicture p={0};
 if(dav1d_open(&c,&s)<0)return 3; unsigned char *b=dav1d_data_create(&d,n); if(!b||fread(b,1,n,f)!=(size_t)n)return 4; fclose(f);
 if(dav1d_send_data(c,&d)<0||dav1d_get_picture(c,&p)<0||p.p.bpc!=8||p.p.layout<1||p.p.layout>3)return 5;
 f=fopen(argv[2],"wb"); if(!f)return 6; int w=p.p.w,h=p.p.h,cw=p.p.layout==3?w:(w+1)/2,ch=p.p.layout==1?(h+1)/2:h;
 for(int i=0;i<3;i++)for(int y=0;y<(i?ch:h);y++)if(fwrite((unsigned char*)p.data[i]+y*p.stride[!!i],1,i?cw:w,f)!=(size_t)(i?cw:w))return 7;
 fclose(f); dav1d_picture_unref(&p); dav1d_data_unref(&d); dav1d_close(&c); return 0;
}
