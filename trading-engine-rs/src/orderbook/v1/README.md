### Order Book V1

#### Summary
The book has pre allocated vectors of price levels and arena nodes, with preallocated Orders.
```
levels: Vec<PriceLevel>
arena: Vec<ArenaNode>
```
OrderID information is stored to fetch orders quickly too
```
order_index: HashMap<OrderId, ArenaId>
```
When receiving commands the book will fetch a new / existing arena node and alloc, storing the current order information within it. 

Ordered price maps are done using BTreeMap
```
asks: BTreeMap<Price, LevelId>
bids: BTreeMap<Reverse<Price>, LevelId>
```

Generating OrderID
Order IDs are set by - 
```
yyyymmddorderoffset
```
Where orderoffset is a set preallocated number (1,000,000 by default).
e.g. 202606260005672

#### Latency
Current Latency Stats are:
```
=== new_order latency histogram (n=1000) ===
throughput: 33917.68 ops/s
p50   : 12007 ns
p90   : 16007 ns
p99   : 61023 ns
p99.9 : 577023 ns
max   : 635391 ns

=== new_order latency histogram (n=10000) ===
throughput: 39466.53 ops/s
p50   : 11007 ns
p90   : 12007 ns
p99   : 42015 ns
p99.9 : 304127 ns
max   : 517119 ns

=== new_order latency histogram (n=100000) ===
throughput: 40487.27 ops/s
p50   : 11007 ns
p90   : 12007 ns
p99   : 35007 ns
p99.9 : 365055 ns
max   : 1523711 ns
```
To be improved, but a decent start!