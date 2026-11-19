import hashlib, json, pathlib, re, subprocess, sys
record_path, fixtures, binary = map(pathlib.Path, sys.argv[1:])
record = json.loads(record_path.read_text())
memory = []
for entry in record['fixtures']:
    for decoder in record['decoders']:
        result = subprocess.run(['/usr/bin/time', '-l' if sys.platform == 'darwin' else '-v', str(binary.resolve()), decoder['mode'], '0', str(fixtures/entry['file'])], check=True, text=True, capture_output=True)
        if sys.platform == 'darwin': rss = int(re.search(r'(\d+)\s+maximum resident set size', result.stderr).group(1))
        else: rss = int(re.search(r'Maximum resident set size \(kbytes\): (\d+)', result.stderr).group(1))*1024
        memory.append({'class':entry['classes'],'file':entry['file'],'decoder':decoder['label'],'peak_rss_bytes':rss})
record['memory'] = memory
record['rss_binary_sha256'] = hashlib.sha256(binary.read_bytes()).hexdigest()
record_path.write_text(json.dumps(record, indent=1)+'\n')
print('single decodes',len(memory),'rss binary sha256',record['rss_binary_sha256'])
