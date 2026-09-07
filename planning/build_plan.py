"""Produce a reproducible conservative replacement plan using the native AM4Help API."""
import json, urllib.request, collections, datetime
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
fleet=json.loads((ROOT/'web/fleet.json').read_text())
cache={}
def api(p):
 key=json.dumps(p,sort_keys=True)
 if key not in cache:
  req=urllib.request.Request('http://127.0.0.1:8765/search',data=json.dumps(p).encode(),headers={'Content-Type':'application/json'})
  try:
   with urllib.request.urlopen(req) as r: cache[key]=json.load(r)
  except urllib.error.HTTPError as e:cache[key]={'routes':[],'error':json.load(e)}
 return cache[key]
def pair(a,b):return '|'.join(sorted([a,b]))
def sector(model):return 'cargo' if model=='A380-800F' else 'pax'
def key(r):return json.dumps([r['id'],r['a'],r['b'],r['name']],ensure_ascii=False,separators=(',',':'))
models={'A380-800':('a388',3,[.75,.9,.99]),'A380-800F':('a388f',2,[.60,.8,.99]),'A330-900neo':('a339',1,[.75,.9,.99]),'B747-8':('b748',1,[.75,.9,.99])}
occupied={pair(r['a'],r['b']) for r in fleet}
counts=collections.Counter((sector(r['model']),pair(r['a'],r['b'])) for r in fleet)
mixed=collections.defaultdict(set)
for r in fleet:mixed[(sector(r['model']),pair(r['a'],r['b']))].add(r['model'])
selected_pairs=set();output=[];summaries=[]
for hub in sorted({r['hub'] for r in fleet}):
 rows=[r for r in fleet if r['hub']==hub]; hstart=len(output)
 for model in sorted({r['model'] for r in rows}):
  code,engines,loads=models[model]; scens=[]
  for engine in range(engines):
   for load in loads:
    scens.append(dict(origin=hub,aircraft=f'{code}[{engine}sfc]',mode='realism',max_hours=13,cadence_hours=13,trips_per_day=2,allow_invalid_tpd=True,aircraft_per_route=1,align_max_time=True,fuel_price=340,co2_price=125,pax_load=load,cargo_load=load,large_training=6 if sector(model)=='cargo' else 0,heavy_training=6 if sector(model)=='cargo' else 0,limit=4000))
  all_routes=[{r['destination']['iata']:r for r in api(p)['routes']} for p in scens]
  valid=set.intersection(*(set(d) for d in all_routes))
  valid={d for d in valid if pair(hub,d) not in occupied}
  # Display reference: engine 0, 90% pax / 80% cargo. Decisions pass every scenario.
  ref=1
  def evaluate(row,n):
   dest=row['b'] if row['a']==hub else row['a'];bases=[];fits=[]
   for p,m in zip(scens,all_routes):
    if n==1:r=m.get(dest)
    else:r=next(iter(api(dict(p,destination=dest,aircraft_per_route=n))['routes']),None)
    fits.append(bool(r))
    if r is None:
     r=next(iter(api(dict(p,destination=dest,aircraft_per_route=n,max_hours=26,align_max_time=False,ci=200))['routes']),None)
    if r is None:
     r=next(iter(api(dict(p,destination=dest,aircraft_per_route=1,trips_per_day=1,max_hours=26,align_max_time=False,ci=200))['routes']),None)
     if r:r=dict(r,optimistic_upper_bound=True)
    bases.append(r)
   return bases,fits
  group=[r for r in rows if r['model']==model]
  # Weakest baseline first; ties are deterministic. Re-evaluate duplicate demand after each move.
  prepared=[]
  for row in group:
   n=counts[(sector(model),pair(row['a'],row['b']))]
   if hub not in (row['a'],row['b']) or len(mixed[(sector(model),pair(row['a'],row['b']))])>1:
    prepared.append((float('inf'),row,None,None));continue
   bases,fits=evaluate(row,n)
   score=min((r['profit_per_trip'] for r in bases if r),default=float('inf'))
   prepared.append((score,row,bases,fits))
  for _,row,bases,fits in sorted(prepared,key=lambda x:(x[0],x[1]['a'],x[1]['b'],x[1]['name'])):
   item={'key':key(row),'hub':hub,'model':model,'aircraft_id':row['id'],'aircraft_name':row['name'],'old_a':row['a'],'old_b':row['b'],'status':'keep','reason':'Nessuna alternativa libera supera la soglia in tutti gli scenari.'}
   oldpair=(sector(model),pair(row['a'],row['b']));n=counts[oldpair]
   if bases is None:
    item.update(status='verify',reason='Hub ambiguo o domanda condivisa con modelli diversi: nessuna sostituzione automatica.');output.append(item);continue
   if n!=sum(sector(x['model'])==sector(model) and pair(x['a'],x['b'])==oldpair[1] for x in fleet):bases,fits=evaluate(row,n)
   if not all(bases):
    item.update(status='verify',reason='Il modello non riesce a valutare la tratta attuale in tutti gli scenari; non considero il margine mancante uguale a zero.');output.append(item);continue
   item.update(current=bases[ref],current_fits_all_13h=all(fits),scenario_count=len(scens),current_aircraft_count=n)
   choices=[]
   for dest in valid:
    if pair(hub,dest) in selected_pairs:continue
    cand=[m[dest] for m in all_routes]
    deltas=[c['profit_per_trip']-b['profit_per_trip'] for c,b in zip(cand,bases)]
    if all(g>=max(100000,abs(b['profit_per_trip'])*.10) for g,b in zip(deltas,bases)):
     choices.append((min(deltas),cand[ref]['direct_km'],dest,cand,deltas))
   if choices:
    best=max(x[0] for x in choices)
    # Among options within 5% of best guaranteed improvement, prefer longest route.
    gain,_,dest,cand,deltas=max((x for x in choices if x[0]>=best*.95),key=lambda x:(x[1],x[0],x[2]))
    selected_pairs.add(pair(hub,dest));counts[oldpair]-=1
    item.update(status='replace',reason='Almeno +10% e +$100.000 per partenza in ogni scenario, anche rispetto alla tratta attuale ottimizzata.',replacement=cand[ref],gain_min=min(deltas),gain_max=max(deltas),gain_reference=deltas[ref],gain_percent_min=min(g/abs(b['profit_per_trip'])*100 for g,b in zip(deltas,bases)),reference_parameters=scens[ref],scenarios=[{'engine':i//len(loads),'engine_name':api(scens[i])['aircraft_effective']['engine'],'load':loads[i%len(loads)],'old_profit':b['profit_per_trip'],'new_profit':c['profit_per_trip'],'gain':g,'new_hours':c['hours'],'new_ci':c['ci'],'new_configuration':c['configuration'],'new_tickets':c['tickets'],'new_stopover':c['stopover']} for i,(b,c,g) in enumerate(zip(bases,cand,deltas))])
   elif not all(fits):item.update(status='adjust',reason='Nessuna sostituzione supera la soglia economica; domanda o durata non soddisfano tutti gli scenari del ciclo di 13 ore. Non attribuisco perdite alla tratta.')
   output.append(item)
 result=output[hstart:];summary=dict(hub=hub,total=len(result),replace=sum(r['status']=='replace' for r in result),keep=sum(r['status']=='keep' for r in result),adjust=sum(r['status']=='adjust' for r in result),verify=sum(r['status']=='verify' for r in result))
 summaries.append(summary);print(summary,flush=True)
plan={'generated_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_commit':'d243dcd13d102b28a548b6346af62fbd62c7c9aa','method':{'minimum_gain':100000,'minimum_gain_percent':10,'hours':13,'trips_demand':2,'fuel':340,'co2':125,'modifications':'Velocità +10%, fuel −10%, CO₂ −10% su attuale e nuova; verificare o installare prima di applicare il piano.','loads_pax':[.75,.9,.99],'loads_cargo':[.6,.8,.99],'engines':'Tutte le varianti disponibili per il modello','selection':'Prima i margini attuali più deboli; destinazioni già occupate escluse e ogni nuova coppia usata una sola volta. Fra alternative entro il 5% dal miglior incremento minimo scelgo la più lunga.','comparison':'Confronto per partenza dopo ottimizzazione della tratta attuale; costi cambio tratta, upgrade, personale e marketing esclusi. Il delta non è una misura delle perdite effettive.','fallback':'Se la tratta attuale non entra in 13 ore, confronto col suo margine per volo a CI 200 entro 26 ore. Se la domanda non consente 2 voli, uso una stima favorevole alla tratta attuale con 1 volo e 1 aereo, ignorando la concorrenza, come confronto conservativo. Non la tratto come ricavo zero.','reference':'Engine 0; riempimento 90% pax, 80% cargo. Tariffe, CI e configurazione mostrati si riferiscono a questo scenario; usare il motore effettivamente posseduto prima di applicarli.'},'summary':summaries,'rows':output}
(ROOT/'web/replacement-plan.json').write_text(json.dumps(plan,ensure_ascii=False))
print('TOTAL',collections.Counter(r['status'] for r in output), 'calls',len(cache))
