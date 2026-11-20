import pathlib, struct, sys, zlib
out=pathlib.Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
def chunk(kind,data): return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data))
def make(name,depth,color,raw,extra=b'',width=8,height=1,interlace=0):
    (out/name).write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',width,height,depth,color,0,0,interlace))+extra+chunk(b'IDAT',zlib.compress(raw))+chunk(b'IEND',b''))
for depth in (1,2,4,8):
    n=1<<depth
    palette=bytes(v for i in range(n) for v in (i*255//(n-1),255-i*255//(n-1),i*255//(n-1)//2))
    samples=[i%n for i in range(8)]
    packed=bytes(sum(samples[i+j] << (8-depth*(j+1)) for j in range(8//depth)) for i in range(0,8,8//depth))
    make(f'palette-{depth}.png',depth,3,b'\x00'+packed,chunk(b'PLTE',palette)+chunk(b'tRNS',bytes(i*255//(n-1) for i in range(n))))
    make(f'gray-{depth}.png',depth,0,b'\x00'+packed,chunk(b'tRNS',struct.pack('>H',n-1)))
make('rgb-trns.png',8,2,b'\x00'+bytes([1,2,3,4,5,6]*4),chunk(b'tRNS',struct.pack('>HHH',1,2,3)))
for depth,color,channels in [(8,0,1),(8,2,3),(8,6,4),(16,0,1),(16,2,3),(16,6,4)]:
    w,h=13,11
    pixels=[[[(x*1337+y*509+c*701)%65536 for c in range(channels)] for x in range(w)] for y in range(h)]
    raw=bytearray()
    for x0,y0,dx,dy in [(0,0,8,8),(4,0,8,8),(0,4,4,8),(2,0,4,4),(0,2,2,4),(1,0,2,2),(0,1,1,2)]:
        for y in range(y0,h,dy):
            row=[v for x in range(x0,w,dx) for v in pixels[y][x]]
            if row:
                raw.append(0)
                for v in row: raw.extend(struct.pack('>H',v) if depth==16 else bytes([v%256]))
    make(f'adam7-{depth}-{color}.png',depth,color,bytes(raw),width=w,height=h,interlace=1)
for depth in (1,2,4):
    for color in (0,3):
        w,h=13,11; n=1<<depth; raw=bytearray()
        for x0,y0,dx,dy in [(0,0,8,8),(4,0,8,8),(0,4,4,8),(2,0,4,4),(0,2,2,4),(1,0,2,2),(0,1,1,2)]:
            for y in range(y0,h,dy):
                row=[(x+y)%n for x in range(x0,w,dx)]
                if row:
                    raw.append(0)
                    for i in range(0,len(row),8//depth):
                        raw.append(sum(v << (8-depth*(j+1)) for j,v in enumerate(row[i:i+8//depth])))
        extra=chunk(b'PLTE',bytes(v for i in range(n) for v in (i*255//(n-1),0,255-i*255//(n-1))))+chunk(b'tRNS',bytes(i*255//(n-1) for i in range(n))) if color==3 else chunk(b'tRNS',struct.pack('>H',n-1))
        make(f'adam7-{depth}-{color}.png',depth,color,bytes(raw),extra,w,h,1)
make('gray16-trns.png',16,0,b'\x00'+b''.join(struct.pack('>H',v) for v in [0,1,255,256,32768,65535,1000,10]),chunk(b'tRNS',struct.pack('>H',256)))
make('rgb16-trns.png',16,2,b'\x00'+b''.join(struct.pack('>H',v) for v in [1,256,65535,0,0,0]*4),chunk(b'tRNS',struct.pack('>HHH',1,256,65535)))
