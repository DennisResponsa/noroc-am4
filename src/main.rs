use std::{collections::HashSet, io::Read, num::NonZeroU8, path::PathBuf};
use am4::{aircraft::{Aircraft, db::Aircrafts}, airport::{Airport, db::Airports}, route::{Ci, db::{DemandMatrix, DistanceMatrix}, demand::CargoDemand, config::ConfigAlgorithm, metrics::ConfigVariant, ticket::Ticket, search::{AbstractConfig, AbstractRoute, Routes, schedule::*}}, user::*};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tiny_http::{Header, Response, Server};

const UPSTREAM: &str = "d243dcd13d102b28a548b6346af62fbd62c7c9aa";
const LIMITATIONS: &[&str] = &[
 "Profitto AM4Help stimato: ricavo meno fuel, CO2, A-check e riparazioni. Esclude personale, marketing e costo di cambio rotta.",
 "Domanda dal database AM4Help, non dal conto live; eventi o aggiornamenti possono cambiarla.",
 "profit_per_day_at_cadence è una proiezione aritmetica: occorre dimensionare domanda e velivoli sul massimo numero di partenze nello stesso giorno (2 per ciclo di 13 h).",
 "Il motore/configurazione scelto è un parametro, non una lettura automatica del velivolo posseduto.",
];

struct Engine { ac: Aircrafts, ap: Airports, distances: DistanceMatrix, demand: DemandMatrix }
impl Engine {
 fn load(root: &std::path::Path) -> Result<Self, String> {
  let read = |name| std::fs::read(root.join(name)).map_err(|e| e.to_string());
  let ac = Aircrafts::from_bytes(&read(am4::AC_FILENAME)?).map_err(|e| format!("{e:?}"))?;
  let ap = Airports::from_bytes(&read(am4::AP_FILENAME)?).map_err(|e| format!("{e:?}"))?;
  let mut bytes = read(am4::DEM_FILENAME0)?; bytes.extend(read(am4::DEM_FILENAME1)?);
  let demand = DemandMatrix::from_bytes(&bytes).map_err(|e| format!("{e:?}"))?;
  let distances = DistanceMatrix::from_airports(ap.data());
  Ok(Self {ac,ap,distances,demand})
 }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct Params {
 aircraft: String, origin: String, destination: Option<String>, mode: String,
 min_hours: f32, max_hours: f32, min_km: f32, max_km: f32,
 trips_per_day: u8, aircraft_per_route: u8, align_max_time: bool, ci: u8,
 config_algorithm: String, inflate_distance: bool, allow_invalid_tpd: bool,
 fuel_price: u16, co2_price: u16, pax_load: f32, cargo_load: f32,
 fuel_training: u8, co2_training: u8, repair_training: u8, wear_training: u8,
 large_training: u8, heavy_training: u8,
 sort: String, limit: usize, exclude: Vec<String>, cadence_hours: Option<f32>,
}
impl Default for Params {
 fn default() -> Self { Self {
  aircraft: "a388".into(), origin: "VCE".into(), destination: None, mode: "realism".into(),
  min_hours: 0., max_hours: 13., min_km: 100., max_km: 40075., trips_per_day: 1, aircraft_per_route: 1,
  align_max_time: true, ci: 200, config_algorithm: "auto".into(), inflate_distance: false, allow_invalid_tpd: false,
  fuel_price: 900, co2_price: 120, pax_load: 0.99, cargo_load: 0.99,
  fuel_training: 0, co2_training: 0, repair_training: 0, wear_training: 0, large_training: 0, heavy_training: 0,
  sort: "profit_per_trip".into(), limit: 20, exclude: vec![], cadence_hours: None,
 }}
}
fn ap_json(a: &Airport) -> Value { json!({"iata":a.iata.to_string(),"icao":a.icao.to_string(),"name":a.name.to_string(),"country":a.country,"runway_ft":a.rwy}) }
fn ac_json(a: &Aircraft) -> Value { json!({"model":a.name.to_string(),"shortname":a.shortname.to_string(),"engine_priority":a.priority.get(),"engine":a.ename,"speed_kmh_realism":a.speed,"fuel":a.fuel,"co2":a.co2,"capacity":a.capacity,"range_km":a.range,"runway_ft":a.rwy,"check_cost_base":a.check_cost,"maintenance_hours":a.maint}) }
fn err(e: impl std::fmt::Display) -> String { e.to_string() }
fn settings(p: &Params) -> Result<Settings, String> {
 Ok(Settings { fuel_price: FuelPrice::new(p.fuel_price), co2_price: Co2Price::new(p.co2_price),
 load: AircraftLoad::new(p.pax_load).map_err(err)?, cargo_load: AircraftLoad::new(p.cargo_load).map_err(err)?,
 allow_invalid_tpd: p.allow_invalid_tpd,
 training: Training { fuel: FuelTraining::new(p.fuel_training).map_err(err)?, co2: Co2Training::new(p.co2_training).map_err(err)?, repair: RepairTraining::new(p.repair_training).map_err(err)?, wear: WearTraining::new(p.wear_training).map_err(err)?, l: LargeTraining::new(p.large_training).map_err(err)?, h: HeavyTraining::new(p.heavy_training).map_err(err)? },
 ..Settings::default() })
}
fn search(e: &Engine, p: &Params) -> Result<Value, String> {
 if p.min_hours < 0. || p.max_hours <= p.min_hours || p.max_hours > 168. {return Err("Intervallo ore non valido (0 < max <=168)".into())}
 if p.min_km < 100. || p.max_km <= p.min_km || p.max_km > 50000. {return Err("Intervallo km non valido (100 <= min < max <=50000)".into())}
 if p.limit == 0 || p.limit > 4000 {return Err("limit deve essere 1..4000".into())}
 if !["profit_per_trip","profit_per_day","distance"].contains(&p.sort.as_str()) {return Err("sort: profit_per_trip, profit_per_day oppure distance".into())}
 if let Some(c) = p.cadence_hours {if !c.is_finite() || c <= 0. || c >168. {return Err("cadence_hours non valida".into())}}
 let mode = match p.mode.as_str() {"realism"=>GameMode::Realism,"easy"=>GameMode::Easy,_=>return Err("mode deve essere realism oppure easy".into())};
 let origin = e.ap.search(&p.origin).map_err(|x|format!("Hub: {x:?}"))?;
 let custom = e.ac.search(&p.aircraft).map_err(|x|format!("Velivolo: {x:?}"))?;
 let ac = custom.effective();
 if mode == GameMode::Realism && origin.rwy < ac.rwy {return Err("Pista dell'hub insufficiente per il velivolo".into())}
 let excluded: HashSet<String> = p.exclude.iter().map(|x|x.to_uppercase()).collect();
 let destinations = if let Some(d) = &p.destination {
  let d = e.ap.search(d).map_err(|x|format!("Destinazione: {x:?}"))?;
  if d.idx == origin.idx {return Err("Origine e destinazione devono essere diverse".into())}
  vec![d.clone()]
 } else {e.ap.data().iter().filter(|x| !excluded.contains(&x.iata.to_string()) && !excluded.contains(&x.icao.to_string())).cloned().collect()};
 let s = settings(p)?;
 let sc = SearchConfig { user_settings: &s,
  distance_filter: format!("{}..{}",p.min_km,p.max_km).parse().map_err(|x|format!("{x:?}"))?,
  flight_time_filter: if p.min_hours == 0. {format!("..{}",p.max_hours)} else {format!("{}..{}",p.min_hours,p.max_hours)}.parse().map_err(|x|format!("{x:?}"))?,
  schedule: ScheduleStrategy { trips_per_day: TripsPerDayStrategy::Strict(NonZeroU8::new(p.trips_per_day).ok_or("trips_per_day deve essere >0")?), num_aircraft: NumAircraftStrategy::Strict(NonZeroU8::new(p.aircraft_per_route).ok_or("aircraft_per_route deve essere >0")?) },
  ci: if p.align_max_time {CiStrategy::AlignConstraint} else {CiStrategy::Strict(Ci::new(p.ci).map_err(err)?)},
  config: p.config_algorithm.parse::<ConfigAlgorithm>().map_err(|_|"Algoritmo di configurazione non valido")?,
  inflate_distance_with_stopover: p.inflate_distance,
  sort_by: if p.sort=="profit_per_day" {SortBy::ProfitPerAcPerDay} else {SortBy::ProfitPerTrip},
 };
 let routes = Routes::<AbstractRoute, AbstractConfig>::new(&e.ap,&e.distances,origin,&destinations).with_aircraft(&ac,&mode).schedule(&e.demand,&e.distances,&sc);
 let mut output: Vec<Value> = routes.routes().iter().map(|r| {
  let dem = e.demand.get_unchecked(origin.idx,r.destination.idx);let cargo = CargoDemand::from(&dem);
  let config = match r.config {ConfigVariant::Pax(c)=>json!({"Y":c.y,"J":c.j,"F":c.f}),ConfigVariant::Cargo(c)=>json!({"large_percent":c.l,"heavy_percent":c.h})};
  let tickets = match &r.ticket {Ticket::Pax(t)|Ticket::VIP(t)=>json!({"Y":t.y,"J":t.j,"F":t.f}),Ticket::Cargo(t)=>json!({"large":t.l,"heavy":t.h})};
  let cadence_ok = p.cadence_hours.map(|h| r.flight_time.get()<=h);
  json!({"destination":ap_json(r.destination),"direct_km":r.direct_distance.get(),"total_km":r.total_distance.get(),"stopover":r.stopover.as_ref().map(|s|ap_json(s.0)),"hours":r.flight_time.get(),"ci":r.ci.get(),"demand":{"Y":dem.y,"J":dem.j,"F":dem.f,"large":cargo.l,"heavy":cargo.h},"configuration":config,"tickets":tickets,"trips_per_day":r.trips_per_day.get(),"aircraft_per_route":r.num_aircraft.get(),"revenue_per_trip":r.revenue,"fuel_lbs":r.fuel,"co2_units":r.co2,"fuel_cost":r.fuel*p.fuel_price as f32/1000.,"co2_cost":r.co2*p.co2_price as f32/1000.,"acheck_cost":r.acheck_cost,"repair_cost":r.repair_cost,"profit_per_trip":r.profit,"profit_per_day_engine":r.profit*r.trips_per_day.get() as f32,"cadence_feasible":cadence_ok,"profit_per_day_at_cadence":p.cadence_hours.filter(|_|cadence_ok==Some(true)).map(|h|r.profit*24./h),"contribution_per_trip":r.contribution})
 }).collect();
 if p.sort=="distance" {output.sort_by(|a,b|b["direct_km"].as_f64().unwrap().total_cmp(&a["direct_km"].as_f64().unwrap()));}
 let count=output.len();output.truncate(p.limit);
 Ok(json!({"engine":"AM4Help Rust 0.2","source_commit":UPSTREAM,"parameters":p,"origin":ap_json(origin),"aircraft_effective":ac_json(&ac),"matches":count,"rejected":routes.errors().len(),"routes":output,"limitations":LIMITATIONS,"schedule_note":if p.allow_invalid_tpd {"Il controllo tpd×tempo<=24 è disattivato come nell'app. Con 2 tpd e 13 h si dimensiona conservativamente la domanda; non sono 2 voli al giorno sostenibili in media. Usare cadence_hours=13 per la media 24/13."}else{"Numero intero di voli al giorno come nell'app. cadence_hours, se fornito, riporta separatamente la media del ciclo."}}))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Compare { parameters: Params, current_destinations: Vec<String>, #[serde(default)] exclude_existing: Vec<String> }
fn compare(e: &Engine, input: Compare) -> Result<Value,String> {
 if input.current_destinations.len()>500 {return Err("Massimo 500 tratte".into())}
 let mut p=input.parameters.clone();p.destination=None;p.exclude.extend(input.exclude_existing);p.exclude.extend(input.current_destinations.clone());
 let alternatives=search(e,&p)?;
 let current: Vec<Value> = input.current_destinations.iter().map(|d| {let mut c=input.parameters.clone();c.destination=Some(d.clone());c.limit=1;
  match search(e,&c) {Ok(v)=>json!({"destination":d,"evaluation":v,"status":"modello ottimizzato, non profitto osservato del velivolo attuale"}),Err(x)=>json!({"destination":d,"error":x})}
 }).collect();
 Ok(json!({"current":current,"alternatives":alternatives,"note":"Confronto alla pari con gli stessi parametri. Non attribuisce perdite effettive senza costi e carichi osservati."}))
}

fn run() -> Result<(),String> {
 let root=std::env::var_os("AM4_DATA_DIR").map(PathBuf::from).unwrap_or_else(||PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/am4/assets"));
 let engine=Engine::load(&root)?;
 let port=std::env::var("AM4_PORT").unwrap_or("8765".into()).parse::<u16>().map_err(err)?;
 let server=Server::http((std::net::Ipv4Addr::LOCALHOST,port)).map_err(err)?;
 eprintln!("AM4Help API pronta su http://127.0.0.1:{port} — commit {UPSTREAM}");
 for mut request in server.incoming_requests() {
  let path=request.url().split('?').next().unwrap_or("").to_string();let method=request.method().as_str().to_string();
  if method=="GET" && (path=="/" || path=="/docs" || path=="/fleet" || path=="/fleet.json" || path=="/prices.json") {
   let (body, mime) = if path=="/prices.json" {(include_str!("../web/prices.json"), "application/json; charset=utf-8")} else if path=="/fleet" || path=="/fleet.json" {(include_str!("../web/fleet.json"), "application/json; charset=utf-8")} else {(include_str!("../web/index.html"), "text/html; charset=utf-8")};
   let response=Response::from_string(body).with_header(Header::from_bytes("Content-Type",mime).unwrap()).with_header(Header::from_bytes("Cache-Control","no-store").unwrap());
   let _=request.respond(response);continue;
  }
  if method=="GET" && (path=="/replacement-plan.json" || path=="/plan.js") {
   let (body,mime)=if path=="/plan.js" {(include_str!("../web/plan.js"),"text/javascript; charset=utf-8")} else {(include_str!("../web/replacement-plan.json"),"application/json; charset=utf-8")};
   let _=request.respond(Response::from_string(body).with_header(Header::from_bytes("Content-Type",mime).unwrap()).with_header(Header::from_bytes("Cache-Control","no-store").unwrap()));continue;
  }
  // Loopback only; reject requests originating from arbitrary web pages.
  let bad_origin=request.headers().iter().any(|h| h.field.equiv("Origin") && ![format!("http://127.0.0.1:{port}"),format!("http://localhost:{port}")].contains(&h.value.as_str().to_string()));
  let mut body=String::new();
  let result: Result<Value,String> = if bad_origin {Err("Origin non consentita".into())} else if method=="GET" {match path.as_str() {
   "/health"=>Ok(json!({"status":"ok","source_commit":UPSTREAM,"airports":engine.ap.data().len(),"aircraft_variants":engine.ac.data().len()})),
   "/aircraft"=>Ok(json!(engine.ac.data().iter().map(ac_json).collect::<Vec<_>>())),
   "/airports"=>Ok(json!(engine.ap.data().iter().map(ap_json).collect::<Vec<_>>())),
   "/api-docs"=>Ok(json!({"name":"Noroc API basata sul motore AM4Help","endpoints":["GET /health","GET /aircraft","GET /airports","POST /search","POST /compare"],"example":Params::default(),"limitations":LIMITATIONS})),
   _=>Err("Endpoint non trovato".into())
  }} else if method=="POST" {
   match request.as_reader().take(1_048_577).read_to_string(&mut body) {Err(x)=>Err(x.to_string()),Ok(_) if body.len()>1_048_576=>Err("Corpo troppo grande".into()),Ok(_)=>match path.as_str() {
    "/search"=>serde_json::from_str::<Params>(&body).map_err(err).and_then(|p|search(&engine,&p)),
    "/compare"=>serde_json::from_str::<Compare>(&body).map_err(err).and_then(|p|compare(&engine,p)),
    _=>Err("Endpoint non trovato".into())
   }}
  } else {Err("Metodo non consentito".into())};
  let (status,value)=match result {Ok(v)=>(200,v),Err(x)=>(400,json!({"error":x}))};
  let response=Response::from_string(value.to_string()).with_status_code(status).with_header(Header::from_bytes("Content-Type","application/json; charset=utf-8").unwrap()).with_header(Header::from_bytes("Cache-Control","no-store").unwrap());
  let _=request.respond(response);
 }
 Ok(())
}
fn main() {if let Err(e)=run(){eprintln!("{e}");std::process::exit(1)}}

