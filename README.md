# rusttoolz

**Lars Rocha's sitoolz in Rust: the parts with real compute.** Prove any number to 72
quadrillion from a 15 MB net. Roll a proven prime up the Pocklington ladder to any size with the
proof carried along, 4,096 bits in four seconds. Forge `k·2ⁿ ± 1` twins and prove both sides,
21 of them at 82 digits in a tenth of a second. Settle 88% of any range with free logic and no
primality test. Read rule 30's edge off a table. Rebuild the XOR difference triangle and the
XOR build system from one row. Pure Rust by default, so it compiles to wasm: big integers are
`num-bigint`, everything within the net's reach is `u64`, and the hunts run on every core with
rayon. Native builds can turn on GMP with `--features gmp` for the big rolls.

```
cargo add rusttoolz
```

```rust
use rusttoolz::{net::Net, prove, twin, xortri};

let net = Net::build(10_000_000);                    // every prime to ten million, as half-gaps
net.is_prime(99_999_999_999_973)?;                   // true, by elimination, within reach
net.primes_between(10u64.pow(12), 10u64.pow(12) + 100)?;   // every prime just past a trillion
net.pi(10u64.pow(9));                                // 50847534, exact

let chain = prove::build_up(prove::mersenne_seed(127).unwrap(), 4096);   // M127 to 4096 bits: 6 rungs, 4 s on 8 cores
prove::verify_certificate(&chain, None);             // every rung re-proved from the one below

let t = twin::prove_pair(19902, 256);                // the 82-digit twin: both sides proven
twin::hunt(256, 1, 300_000, 100_000);                // 21 proven 82-digit twins in 0.12 s

xortri::from_left_edge(&[2, 4, 10, 7, 14, 13, 2]);   // [2, 6, 8, 11, 12, 5, 4]

use rusttoolz::{lanes, edgelaw, buildsystem, cert};
lanes::verdict(1717, &lanes::LANES, 8);              // composite, "fold-wheel k=16: 17x101", free
lanes::dashboard(2, 10_000, &lanes::LANES);          // 8076 wheel · 61 below horizon · 693 walks · 0 rho · 1168 test
edgelaw::edge(1_000_000_000_000);                    // rule 30's edge run at a trillion, no simulation
buildsystem::buildsystem(&buildsystem::structured(128));   // the XOR Cayley table from its own row 0
cert::to_json(&chain, cert::Convention::Slip);       // the certificate as the browser verifier reads it
```

## Credit

The ideas are Lars Rocha's; this crate ports the prime engine, the ladder, the forge and the
triangle from his Python package [`sitoolz`](https://pypi.org/project/sitoolz/).
Claude assisted with the port and the tests; that does not imply ownership.

## The net

`Net::build(limit)` sieves every prime to `limit` and keeps them as `u8` half-gaps between
consecutive odd primes, eight times smaller than the primes. A composite has a factor at most
its square root, so any number to `reach = largest²` with no factor in the net is prime, by
elimination. `Net::from_npy` reads the Python package's own `base_gaps.npy` and `meta.json`, so
the shipped 15 MB net that proves to 72,092,246,241,000,049 loads unchanged.

| call | what |
|---|---|
| `primes()`, `primes_upto(l)`, `first(n)`, `nth(k)` | the primes, decoded as they go |
| `is_prime(n)`, `factor(n)`, `factorize(n)` | proven, within the reach; `Err(OutOfReach)` past it |
| `primes_between(lo, hi)`, `twins(lo, hi)` | a segmented sieve seeded from the net, anywhere to the reach |
| `pi(x)`, `prime_count(x)` | exact counts: from the net inside it, Lucy_Hedgehog `O(x^0.75)` beyond |
| marks | a checkpoint prime every 1,024 half-gaps, so `nth` and `count_upto` walk from the nearest mark |
| `save`, `load` | the net's own file: three `u64` and the half-gaps |

## The ladder

From a proven `q`, `N = 2·R·q + 1` with `q > √N` is proven by one witness `a` with
`a^(N−1) ≡ 1 (mod N)` and `gcd(a^((N−1)/q) − 1, N) = 1`. `build_up(seed, bits)` takes `R` just
under `q/2` at each rung, the largest the theorem allows, so the bit length doubles per rung and
M127 reaches 4,096 bits in six rungs and four seconds, 8,192 bits in seven. Each rung tests a
batch of `R` across every core with rayon and keeps the largest that proves, with the last rung
sized to land on the target. `R` is sieved against the small primes and each candidate screened with Fermat base 2
before the twelve-witness gate. `build_up_small` is the other climb, the smallest `R` at every
rung, many short steps. Seeds are Mersenne primes
by `lucas_lehmer` or any prime the net proves. `verify_certificate` replays the climb: a seed
must be a Mersenne prime or in the net, every other rung must chain from the one below and pass
its witness, and a tampered witness or a shifted `q` fails.

## The forge

`k·2ⁿ + 1` is a Proth number: `N − 1 = k·2ⁿ` is factored by construction, and
`proth_proven` finds a Pocklington witness per prime factor. `k·2ⁿ − 1` has `N + 1 = k·2ⁿ`, and
`lucas_proven` runs the Lucas test with a discriminant of Jacobi symbol −1. `bad_residues` and
`sieve_k` strike the `k` that a small prime kills on either side (`k ≡ ±2^(−n) mod p`), and
`hunt` screens the survivors in parallel with rayon and proves the hits: at `n = 256`, 21 proven
82-digit twins below `k = 300,000` in 0.12 s; at `n = 1024`, four proven 314-digit twins below
400,000 in 0.76 s. `hunt_stream` runs span by span, appends every `+1` prime to a `.jsonl` as the
packet the Python forge writes (`k, n, digits, upper, upper_proven_pocklington, lower,
lower_status, lower_proven_lucas`), and keeps a checkpoint so a stopped hunt resumes.
`estimate(n)` says how big a search is: at `n = 256`, about one `k` in 89 gives a `+1` prime and
one in 23,848 a twin.

## The lanes

`lanes` is the free-logic verdict stack from `numtools`: a wheel of small-prime lanes, the horizon
below which surviving the wheel is being prime (289 for the default wheel), the square lane, the
Fermat near-square walk, the fold-wheel sharpened by the mod-4 fork (`fold_sharp(1717)` is
`(17, 101, 16)`, a shift left by four), the extended fold horizon (11441 = 17 × 673), the rho walk
as the one heuristic, and `verdict`, which spends a real test only on the thin band of primes past
the fold horizon. `dashboard(2, 10000)` is the cookbook's accounting: 88.3% deterministic, 0%
heuristic, 11.7% one test. Every verdict is checked against the exact test on every number to
20,000.

## The gap forge

`gaps::GapForge::new(n)` builds the primes to `n` by elimination and reads the distances off:
`gaps`, `evens`, `twins`, `sexy`, `where_gap`, `gap_counts`, the `jumping_champion` (6 to a
million), the gap `records`, and `coverage(M)`, how well a number `M` sieves the distances, with
the gaps it misses most. The session's gap-LCM number catches over 99% of the gaps to a million.

## Montgomery, measured

`mont` is Montgomery multiplication on `u64` limbs with a windowed modpow, tested against
`num-bigint` at every size. It comes out even, 0.7× to 1.1× from 512 to 8,192 bits, because
`num-bigint`'s modpow already works in Montgomery form, so the ladder and the forge keep
`num-bigint`'s and the module stands as the place a Karatsuba multiply inside the reduction
would pull ahead.

## numtoolz

`numtoolz` is the other half of the Python package: the 52 Mersenne exponents, the Mersenne
primes through the cycle8 corner identity (`8·(2^(p−3) − 1) ^ 7`), the even perfect numbers,
the RT sieve of doubled primes with its survivors and the cap below which every survivor is twice
a prime (34,322 for RT), the linear sieve that marks every composite exactly once, and the prime
XOR deltas. Every doctest number in the Python module is a test here.

## The build system and the edge law

`buildsystem` is `numtoolz`'s XOR Cayley table, `T[i][j] = a_i ^ a_j` with `a = xor_prefix(index)`,
rebuilt from its own row 0, with `structured(hi)` as the index that is itself a formula.
`edgelaw` is rule 30's light-cone edge by lookup: the run depends only on `v2(g + 1)`, the table
of 49 rungs is measured to rung 30 and taken from OEIS A094604 beyond, `measure` and `verify`
check it by evolving the edge alone, and `edge(g)` answers for any generation to `2^48 − 1`.

## The city, the car and the wall

`city` is Lars's addressed city, the engine's `wlaw::city` carried here unchanged: a district
number lays its streets and raises 64 buildings from their source primes, every one a packed lot
card. `motion` is the one tick law of speed the site and the engine drive by, with its settling
speed, its closed form and the whole flat-ground step. `contact` is first contact along a segment
in exact fractions: a live position is decoded from binary64 into dyadic integers, so nothing is
rounded. `district` composes the three under the studio's `district-clearance-v1` profile: a
district in metres, the car's clearance square, the stop with a micrometre of skin, a checkpoint
every 32 ticks with the block between kept as one packed word of five-bit lanes holding the XOR
triangle's left edge (any command read in at most five shift/XORs), `seek` back to any tick
through the commands it reads, the sweep tested only against the lots the street layout puts
under the car, and branches that share a prefix. `convoy` is the braking reserve between two
cars on one lane: the closure, the tail, the required and fitted gap, and first contact.
`zipper` admits two queues of packed car cards through one gate with exact exit gaps. The tests hold all four
to the reference files the Spindexin site, the engine and `sitoolz` share: district 45 cell
for cell, the six studio trajectories, the 56 contact scenes, and the six portable district
drives replayed at zero difference with their three first contacts exact.

## Two engines under the ladder

Every modular exponentiation in the ladder and the forge goes through `modpow`, which has two
engines. The default is `num-bigint`'s own, pure Rust, which is what compiles to wasm and what the
game engine ships. For native builds that want the speed:

```
cargo build --release --features gmp
cargo run --release --features gmp --example rollball -- 8192
```

With the feature every exponentiation is GMP's `powm` through the `rug` crate, which builds its own
GMP the first time. On the desktop that measured the numbers above, M127 to 4,096 bits takes 11.8 s
pure and 2.2 s with GMP; to 8,192 bits, 7 min 24 s pure and 2 min 47 s with GMP. The gap widens
with the size, since the exponentiation is the whole cost at the top rung. `rusttoolz::modpow::ENGINE`
says which engine a build carries.

## The certificate, in three languages

`cert::to_json` and `cert::from_json` write and read a chain as `[[seed, null, null], [N, q, a],
...]` in either of the two conventions in use: `numtools` (the middle entry is the previous
proven prime, as the Python package writes it) and `slip` (the middle entry is the multiplier `k`
with `N = 2·k·q + 1`, as the arcade's JavaScript verifier also reads). `sniff` tells them apart.
The test reads the 4,096-bit chain the Python package ships and re-proves every rung here.

## The triangle

XOR each neighbour pair, repeat down to one number: Pascal's triangle with XOR, so row `k` is
the XOR of the inputs at the submask positions of `k`, the gasket read as arithmetic. `row_at`
gives any row in `popcount(k)` shift-XORs, `apex` the bottom, `square` the session's packed
layout, and `from_left_edge` / `from_right_edge` rebuild the input from one edge.

## Check

```
cargo test --release                                  # the net against a sieve and against the Python net, the ladder, the forge, the triangle
cargo clippy --release --all-targets -- -D warnings
cargo run --release --example rollball -- 4096        # M127 up the ladder, timed and verified
cargo run --release --example twin_forge -- 256 1 300000
cargo run --release --example verdict -- 2 10000      # the free-logic accounting
cargo run --release --example edge                    # rule 30's edge, lookup against measurement
```

## Changes

**0.1.0** (2026-11-01) — the net, the ladder, the forge, the triangle, the lanes, the build
system, the edge law, the gap forge, numtoolz, certificates in three conventions, and the city,
motion, contact and district drive laws shared with the engine. Held to the Python package's
numbers and to the Spindexin studios' reference cases.
