# Noroc · Pianificazione tratte

[Apri la pagina](https://dennisresponsa.github.io/noroc-am4/)

Interfaccia web in italiano e API funzionante costruite sul **motore Rust originale di AM4Help**, con database originale incluso. Non riscrive le formule. Binario pronto per questo Mac Apple Silicon; codice sorgente incluso per ricompilarlo su altri sistemi.

Avvio dal terminale nella cartella:

```sh
./run.sh
```

Indirizzo: http://127.0.0.1:8765. Per cambiare porta: `AM4_PORT=8766 ./run.sh`. Per terminare: Ctrl+C.

## Chiamate

| Metodo | Percorso | Funzione |
|---|---|---|
| GET | /health | Stato, versione e dimensioni del database |
| GET | / | Interfaccia per tratte, tariffe e rifornimenti |
| GET | /api-docs | Elenco parametri e valori predefiniti |
| GET | /aircraft | Velivoli e varianti motore |
| GET | /airports | Aeroporti |
| POST | /search | Ricerca tratte oppure valutazione di una destinazione |
| POST | /compare | Valutazione delle tratte attuali e alternative, per un hub |

```sh
curl http://127.0.0.1:8765/search \
  -H 'Content-Type: application/json' \
  --data-binary @examples/a380-13h.json
```

Per il cargo usare `examples/cargo-13h.json`. Per confrontare un hub usare `examples/compare-vce.json`.

I risultati riportano distanza diretta e totale, eventuale scalo, durata, CI, domanda, configurazione ottimizzata, tariffe, ricavi, consumo, costi e profitto stimato. `sort` accetta `profit_per_trip`, `profit_per_day` o `distance`. Quest'ultimo ordina per distanza diretta: una tratta lunga deve comunque avere domanda e margine adeguati. `exclude` esclude destinazioni già assegnate.

`aircraft` usa la sintassi originale: `a388`, `a388f`, oppure ad esempio `a388[1sfc]` per variante motore 1 e modifiche velocità/carburante/CO2. Selezionare solo modifiche effettivamente possedute. I prezzi fuel e CO2 sono per 1.000 unità; i carichi attesi sono frazioni 0–1. La reputazione non coincide automaticamente con il carico atteso.

## Partenze ogni 13 ore

Il motore nativo usa un numero intero di viaggi giornalieri. Un ciclo continuo di 13 ore equivale in media a 24/13 = 1,846 partenze al giorno; in alcuni giorni se ne effettuano due. Gli esempi usano quindi **2 viaggi per dimensionare conservativamente la domanda**, abilitando l'opzione nativa `allow_invalid_tpd`, e `cadence_hours=13` per mostrare separatamente la proiezione media. Non significa che due voli da 13 ore entrino in 24 ore.

`align_max_time=true` calcola il CI per avvicinare la durata al limite di 13 ore; l'arrotondamento intero può lasciare qualche minuto. `aircraft_per_route` deve includere tutti i velivoli che condividono la domanda della coppia di aeroporti. Se i velivoli sono diversi servono valutazioni coordinate: questa API non ottimizza una flotta mista simultaneamente.

## Cosa significa profitto

La stima AM4Help sottrae carburante, CO2, A-check e riparazioni. Non include personale, marketing e costo di cambio tratta. La domanda è quella del database, da verificare nel gioco. `/compare` ottimizza anche la configurazione delle tratte attuali: non rappresenta il loro profitto effettivamente osservato.

Gli esempi sono parametri dimostrativi, **non raccomandazioni per sostituire le tratte Noroc**. Prima di consigliare cambi bisogna confrontare configurazione, miglioramenti, carichi e costi reali. L'API non legge automaticamente il telefono e non modifica il gioco.

## Provenienza e verifica

- App: https://am4.pages.dev/
- Help: https://abc8747.github.io/am4/
- Sorgente: https://github.com/abc8747/am4
- Commit incluso: `d243dcd13d102b28a548b6346af62fbd62c7c9aa`
- Licenza originale MIT: `vendor/LICENSE-AM4`.
- Motore vendorizzato in `vendor/am4`, wrapper HTTP in `src/main.rs`.
- Compilazione da sorgente: `cargo build --release` (Rust e connessione per dipendenze richiesti).
- Test: con server attivo, `python3 tests.py`.

Confronto con l'app effettuato su VCE, A380-800 RR Trent 972, Realism, nessuna modifica, 1 viaggio/dì, massimo 13 ore, CI allineato, load 99%, fuel 900 e CO2 120. Coincidono 573 risultati e i primi dieci aeroporti, CI, configurazioni e profitti visualizzati. Questa prova verifica il motore; non certifica che i parametri di prova corrispondano ai velivoli dell'account.


## Pagina web

`web/` contiene la pagina pronta per GitHub Pages. Il motore originale è compilato in WebAssembly: le ricerche online funzionano interamente nel browser, senza un server sul Mac. Il primo utilizzo scarica circa 46 MB; i dati della flotta sono una fotografia delle rilevazioni, non una connessione al gioco.

Per ricompilare: installare Rust e wasm-bindgen-cli **0.2.128**, poi `scripts/build-web.sh`. Pubblicare il contenuto di `web/` come radice del sito. Non pubblicare la sola API HTTP su GitHub Pages: GitHub Pages serve file statici.

### Prezzi per classe

Motore AM4Help: Economy ×1,10, Business ×1,08, First ×1,06; passeggeri arrotondati per difetto e ridotti di $2. Cargo Large ×1,10, Heavy ×1,08, arrotondati per difetto a due decimali. I prezzi sono calcolati dal motore sulla distanza diretta, non da distanze arrotondate. Le tariffe visualizzate non sono una lettura dei prezzi attualmente impostati nel gioco.

### Orari fuel e CO₂

La pagina usa i 2.976 valori della scheda FUEL di `AM Fuel DB.xlsx`, trovato fra i documenti personali, con giorni 1–31 e orari GMT. Non è confermato il mese di validità o che il ciclo sia ancora attuale. I file di testo contengono filtri e conversioni derivate: non vengono usati come orologio live. Un valore CO₂ 19 al giorno 6, 02:00 GMT è conservato nel dato ma contrassegnato come anomalo ed escluso dalle occasioni.

La selezione della data è in ora locale. `Intl.DateTimeFormat` con `Europe/Chisinau` converte ogni istante UTC, gestisce ora legale e passaggi al giorno precedente/successivo. Settembre: GMT+3; inverno: GMT+2. La pagina permette anche Europe/Rome e UTC. Le soglie iniziali sono fuel 420 e CO₂ 115; il prezzo nel gioco va verificato prima di acquistare.

### Modifiche agli aeromobili

Le alternative indicano se l'upgrade velocità è necessario per rispettare il limite con il motore selezionato. Il risparmio di fuel/CO₂ è stimato al 10% del rispettivo costo con le altre condizioni costanti. Non implica un tempo di recupero già verificato: costo dell'upgrade e modifiche possedute devono essere letti dal gioco. I consigli non modificano la flotta.

Verifiche: `python3 tests.py` e, con Node e API locale attiva, `node test-web.mjs`. La seconda controlla la parità del motore WebAssembly con quello nativo, le tariffe dell'inventario e i cambi di data in estate/inverno.
