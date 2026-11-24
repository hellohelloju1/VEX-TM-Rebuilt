import ast,json,re,pathlib,base64,xml.etree.ElementTree as ET
import argparse
parser=argparse.ArgumentParser(description='Compile decoded game metadata into bounded scoring IR; does not execute embedded rules.')
parser.add_argument('decoded_directory',type=pathlib.Path)
parser.add_argument('output_json',type=pathlib.Path)
args=parser.parse_args()
args.output_json.parent.mkdir(parents=True,exist_ok=True)
def encode(n):
 if isinstance(n,ast.AST):return {'node':type(n).__name__,**{k:encode(v) for k,v in ast.iter_fields(n) if k not in ('ctx','type_comment')}}
 if isinstance(n,list):return [encode(v) for v in n]
 return n
allgames=[]
for p in sorted(args.decoded_directory.glob('*.json')):
 d=json.loads(p.read_text());source=d.pop('scoringModule','');d['id']=p.stem
 d['rules']=encode(ast.parse(source)) if source else None
 def controls(xml):
  a=[]
  if not xml:return a
  root=ET.fromstring(base64.b64decode(xml)); parents={c:p for p in root.iter() for c in p}
  for e in root.iter():
   name=e.attrib.get('name','');cls=e.attrib.get('class','')
   if cls not in ('wxSpinCtrl','wxCheckBox','wxRadioButton','wxChoice','wxComboBox'):continue
   if not name or name.startswith('wx'):continue
   vals={x.tag.split('}')[-1]:x.text for x in e}
   ancestor=e; skill=False
   while ancestor in parents:
    ancestor=parents[ancestor]; skill |= ancestor.attrib.get('name')=='skillsPanel'
   name=name.removeprefix('in_').removeprefix('match_')
   if skill:name='skills_'+name
   a.append({'name':name,'label':vals.get('label') or name.replace('_',' '),'kind':cls,'choices':[x.text or '' for x in e.iter() if x.tag.split('}')[-1]=='item'],'min':int(vals.get('min') or 0),'max':int(vals.get('max') or (1 if cls in ('wxCheckBox','wxRadioButton') else 1000)),'default':int(vals.get('value') or vals.get('checked') or 0) if (vals.get('value') or vals.get('checked') or '0').isdigit() else 0})
  return a
 d['controls']=controls(d.pop('interfaceXml',''))
 for s in d.get('skills',[]):s['controls']=controls(s.get('scoring',{}).pop('interfaceXml',''))
 for scope in [d.get('defaultScoring',{})]+[s.get('scoring',{}) for s in d.get('skills',[])]:
  scope['expressions']={k:encode(ast.parse(v.replace('&&',' and ').replace('||',' or '),mode='eval').body) for k,v in scope.items() if k.endswith('Formula')}
  scope['constraints']=[encode(ast.parse(v,mode='eval').body) for v in scope.get('scoringConstraints',[])]
 allgames.append(d)
args.output_json.write_text(json.dumps(allgames,separators=(',',':')))
print([(d['id'],len(d['controls'])) for d in allgames])
