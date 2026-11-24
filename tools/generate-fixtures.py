# Regenerate golden cases from the retained embedded rule sources.
# Uses only Python standard-library modules; the Rust app does not require Python.
import ast,json,random,collections
from pathlib import Path
from types import SimpleNamespace as NS
rng=random.Random(250026)
ROOT=Path(__file__).resolve().parents[1]
games=json.loads((ROOT/'analysis/games.json').read_text())
def convert(d):
 if isinstance(d,dict):return NS(**{k:(collections.defaultdict(int,v) if k=='scoreTypes' else convert(v)) for k,v in d.items()})
 if isinstance(d,list):return [convert(v) for v in d]
 return d
def normal(v):
 if hasattr(v,'_asdict'):return {k:normal(x) for k,x in v._asdict().items()}
 if isinstance(v,(list,tuple)):return [normal(x) for x in v]
 if isinstance(v,bool):return int(v)
 return v
rows=[];errs=[]
for g in games:
 p=ROOT/'analysis/reference-rules'/(g['id']+'.py')
 if not p.exists():continue
 ns=dict(IsMatchElim=lambda m:m.round in (3,4,5,6,7,8,9),IsMatchRoundRobin=lambda m:m.round==16,IsMatchTopNFinals=lambda m:m.round==15,IsMatchTieredTopNFinals=lambda m:m.round in (40,41,42))
 exec(p.read_text(),ns)
 for skill in [False,True]:
  controls=[c for c in g['controls'] if c['name'].startswith('skills_')==skill]
  for case in range(25):
   shared={};alli=[{},{}]
   for c in controls:
    n=c['name'].removeprefix('skills_');maxi=len(c.get('choices',[]))-1 if c['kind']=='wxChoice' else c['max'];v=0 if case==0 else rng.randint(c['min'],max(c['min'],maxi))
    if n.startswith('a1_'):alli[0][n[3:]]=v
    elif n.startswith('a2_'):alli[1][n[3:]]=v
    else:shared[n]=v
   count=2;roundnum=30 if skill else [2,3,4,5,6,15,16,40][case%8]
   inp={'matchTuple':{'round':roundnum},'scoreTypes':shared,'alliances':[{'scoreTypes':alli[i],'teams':[{'dq':False if case<3 else rng.random()<.15,'noShow':False if case<3 else rng.random()<.15} for _ in range(1 if skill else g['rounds'][1]['teamsPerAlliance'])]} for i in range(count)]}
   if skill:inp['alliances'][1]['scoreTypes']={k:0 for k in alli[0]};inp['alliances'][1]['teams']=[{'dq':False,'noShow':False}]
   skill_id=1 if case%2 else 2
   for name in (['calc_skills_score','check_skills_constraints'] if skill else ['calc_match_score','check_constraints']):
    try:
     args=[skill_id,convert(inp)] if skill else [convert(inp)];out=normal(ns[name](*args));rows.append({'game':g['id'],'name':name,'skill':skill_id if skill else None,'input':inp,'output':out})
    except Exception as e:errs.append((g['id'],name,str(e)))
(ROOT/'tests/scoring-fixtures.json').write_text(json.dumps(rows,separators=(',',':')))
assert not errs, errs
print('fixtures',len(rows),'errors',len(errs));print(collections.Counter(tuple(e) for e in errs))
