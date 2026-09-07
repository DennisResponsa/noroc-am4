import json, unittest
from pathlib import Path
R=Path(__file__).resolve().parents[1]
class Plan(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.fleet=json.loads((R/'web/fleet.json').read_text());cls.plan=json.loads((R/'web/replacement-plan.json').read_text());cls.rows=cls.plan['rows'];cls.moves=[r for r in cls.rows if r['status']=='replace']
 def test_inventory_coverage(self):
  self.assertEqual(len(self.rows),len(self.fleet));self.assertEqual(len(set(r['key'] for r in self.rows)),len(self.rows))
 def test_no_conflicting_destinations(self):
  old={tuple(sorted((r['a'],r['b']))) for r in self.fleet};new=[tuple(sorted((r['hub'],r['replacement']['destination']['iata']))) for r in self.moves]
  self.assertEqual(len(new),len(set(new)));self.assertFalse(old.intersection(new))
 def test_economic_threshold_and_time_every_scenario(self):
  for r in self.moves:
   self.assertTrue(r['scenarios']);self.assertEqual(len(r['scenarios']),r['scenario_count'])
   for s in r['scenarios']:
    self.assertGreaterEqual(s['gain']+1,max(100000,abs(s['old_profit'])*.10));self.assertLessEqual(s['new_hours'],13);self.assertGreater(s['new_profit'],0)
   self.assertAlmostEqual(r['gain_min'],min(s['gain'] for s in r['scenarios']))
 def test_summary_matches_rows(self):
  for h in self.plan['summary']:
   rows=[r for r in self.rows if r['hub']==h['hub']];self.assertEqual(h['total'],len(rows))
   for status in ['replace','keep','adjust','verify']:self.assertEqual(h[status],sum(r['status']==status for r in rows))
 def test_cargo_stopovers_explicit(self):
  cargo=[r for r in self.moves if r['model']=='A380-800F'];self.assertTrue(cargo)
  for r in cargo:
   self.assertIn('large',r['replacement']['tickets']);self.assertIn('large_percent',r['replacement']['configuration'])
   if r['replacement']['direct_km']>10400:self.assertIsNotNone(r['replacement']['stopover'])
if __name__=='__main__':unittest.main(verbosity=2)
