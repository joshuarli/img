#!/usr/bin/env python3
import pathlib, sys

def boxes(data, start, end):
    while start < end:
        size = int.from_bytes(data[start:start+4], 'big')
        kind = data[start+4:start+8]
        header = 8
        if size == 1:
            size = int.from_bytes(data[start+8:start+16], 'big'); header = 16
        if size < header or start + size > end: raise ValueError('invalid box')
        yield kind, start + header, start + size
        start += size
    if start != end: raise ValueError('trailing bytes')

def extract(data):
    top = list(boxes(data, 0, len(data)))
    meta = next((a, b) for k, a, b in top if k == b'meta')
    children = list(boxes(data, meta[0]+4, meta[1]))
    pitm = next((a, b) for k, a, b in children if k == b'pitm')
    version = data[pitm[0]]
    primary = int.from_bytes(data[pitm[0]+4:pitm[0]+(6 if version == 0 else 8)], 'big')
    a, end = next((a, b) for k, a, b in children if k == b'iloc')
    version = data[a]; a += 4
    offset_size, length_size = data[a] >> 4, data[a] & 15
    base_size, index_size = data[a+1] >> 4, (data[a+1] & 15 if version in (1, 2) else 0); a += 2
    def take(n):
        nonlocal a
        if a+n > end: raise ValueError('truncated iloc')
        x = int.from_bytes(data[a:a+n], 'big'); a += n; return x
    count = take(4 if version == 2 else 2)
    for _ in range(count):
        item = take(4 if version == 2 else 2)
        method = take(2) & 15 if version in (1, 2) else 0
        reference = take(2); base = take(base_size); extents = take(2)
        chunks = []
        for _ in range(extents):
            if index_size: take(index_size)
            offset, length = take(offset_size), take(length_size)
            if item == primary:
                if method != 0 or reference != 0 or not any(k == b'mdat' and base+offset >= x and base+offset+length <= y for k, x, y in top): raise ValueError('unsupported primary extent')
                chunks.append(data[base+offset:base+offset+length])
        if item == primary:
            if not chunks: raise ValueError('empty primary item')
            return b''.join(chunks)
    raise ValueError('primary item missing')

if __name__ == '__main__':
    pathlib.Path(sys.argv[2]).write_bytes(extract(pathlib.Path(sys.argv[1]).read_bytes()))
