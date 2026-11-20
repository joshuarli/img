import json,pathlib,subprocess,sys
fixtures=pathlib.Path(sys.argv[1])
items=json.loads((fixtures/'manifest.json').read_text())
a,b=sys.argv[2:]; mismatches=[]
for item in items:
 name=item['name']; outs=[]
 for index, exe in enumerate((a,b)):
  out=pathlib.Path('.work/dav1d')/f'out{index}'
  args=[exe,str(fixtures/(name+'.obu')),str(out)] if 'avif-qual-img' not in exe else [exe,'raw','0',str(fixtures/(name+'.obu')),str(out)]
  subprocess.run(args,check=True); outs.append(out.read_bytes())
 if outs[0]!=outs[1]: mismatches.append(name)
print('fixtures',len(items),'mismatches',len(mismatches)); print('\n'.join(mismatches)); sys.exit(bool(mismatches))
