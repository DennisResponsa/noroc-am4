import json, urllib.request, urllib.error, unittest
from pathlib import Path
ROOT=Path(__file__).parent
BASE='http://127.0.0.1:8765'
def call(path,body=None):
    req=urllib.request.Request(BASE+path,data=None if body is None else json.dumps(body).encode(),headers={'Content-Type':'application/json'})
    try:
        with urllib.request.urlopen(req) as r:return r.status,json.load(r)
    except urllib.error.HTTPError as e:return e.code,json.load(e)
class API(unittest.TestCase):
    def test_health(self):
        s,d=call('/health');self.assertEqual(s,200);self.assertEqual(d['airports'],3907)
    def test_web_app_parity(self):
        s,d=call('/search',{'limit':10});self.assertEqual(s,200);self.assertEqual(d['matches'],573)
        expected=[('PER',197,196,139,42,1950634),('DRW',191,55,193,53,1914068),('KOA',180,0,216,56,1836550),('ITO',180,41,155,83,1833505),('BIK',180,123,75,109,1832823),('LEA',180,231,66,79,1826436),('HNL',177,154,145,52,1809826),('SCL',168,0,126,116,1754295),('GUM',164,1,94,137,1725956),('DPS',160,142,154,50,1686062)]
        for r,(iata,ci,y,j,f,profit) in zip(d['routes'],expected):
            self.assertEqual(r['destination']['iata'],iata);self.assertEqual(r['ci'],ci)
            self.assertEqual(r['configuration'],dict(Y=y,J=j,F=f));self.assertLess(abs(r['profit_per_trip']-profit),1)
            self.assertLessEqual(r['hours'],13)
    def test_bad_inputs(self):
        for p in [{'origin':'VCE','destination':'VCE'},{'max_hours':0},{'trips_per_day':0},{'cargo_load':2},{'unknown':1},{'mode':'wrong'}]:
            self.assertEqual(call('/search',p)[0],400,p)
    def test_cargo_cadence_and_accounting(self):
        p=json.loads((ROOT/'examples/cargo-13h.json').read_text());s,d=call('/search',p)
        self.assertEqual(s,200);self.assertGreater(d['matches'],0)
        for r in d['routes']:
            self.assertIn('large_percent',r['configuration']);self.assertLessEqual(r['hours'],13)
            self.assertAlmostEqual(r['profit_per_day_at_cadence'],r['profit_per_trip']*24/13,delta=1)
            self.assertAlmostEqual(r['profit_per_trip'],r['revenue_per_trip']-sum(r[k] for k in ['fuel_cost','co2_cost','acheck_cost','repair_cost']),delta=1)
    def test_comparison_excludes_current(self):
        p=json.loads((ROOT/'examples/compare-vce.json').read_text());s,d=call('/compare',p)
        self.assertEqual(s,200);self.assertEqual(len(d['current']),3)
        self.assertFalse(set(p['current_destinations']) & {r['destination']['iata'] for r in d['alternatives']['routes']})
    def test_upgrades_and_distance_sort(self):
        _,base=call('/search',{'limit':1});s,d=call('/search',{'aircraft':'a388[sfc]','sort':'distance','limit':20})
        self.assertEqual(s,200);self.assertGreater(d['aircraft_effective']['speed_kmh_realism'],base['aircraft_effective']['speed_kmh_realism'])
        distances=[r['direct_km'] for r in d['routes']];self.assertEqual(distances,sorted(distances,reverse=True))
if __name__=='__main__':unittest.main(verbosity=2)
