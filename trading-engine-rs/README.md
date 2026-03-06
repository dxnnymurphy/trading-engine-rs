# Trading-Engine-Rs

### Introduction
Trading Engine is probably the wrong name for this repo, but it is an investigation into various trading techniques and software implemented in Rust. Currently built off of the Crypto market (due to job restrictions), and mainly using the Kraken API.

### 1. Candles
#### Design (TBC)
Kraken API --(websocket)--> WebSocketClient --(mpsc)--> CandleTracker --(mpsc)--> Strategy --(mpsc)--> Execution --(mpsc)--> Kraken API
