# BitForce - a UCI chess engine written in Rust from scratch.

> **Status:** early development (`v0.1.0`). Move generation is complete and verified with perft.
> Search and evaluation are basic and still being worked on.

## Features

- **Bitboard board representation** — one bitboard per piece type and color, plus per-color occupancy
- **FEN** parsing and serialization
- **Fully legal move generation** — pins, checks, en passant, castling and promotions are handled during
  generation using pinned rays and check masks
- **Sliding piece attacks** via [Hyperbola Quintessence](https://chessprogramming.org/Hyperbola_Quintessence) and a [precomputed first-rank
  lookup table](https://chessprogramming.org/First_Rank_Attacks)
- **Compact 32-bit move encoding**
- **Make / unmake move** with castling rights and en passant tracking
- **Perft** with bulk counting at the leaf level
- **Search** — negamax with [alpha-beta pruning](https://chessprogramming.org/Alpha-Beta), fixed depth, mate-distance scoring
- **Evaluation** — material only (`ClassicEval`)
- **UCI protocol**
- Stack-allocated move buffers via [`primitive-buffer`](https://crates.io/crates/primitive-buffer), move
  generation does not allocate on the heap

## Building

Building requires **Rust 1.85 or newer**:

```sh
cargo build --release
```

The binary is placed at `target/release/bitforce`.

## Usage

BitForce speaks UCI over stdin/stdout, so you can plug it into any UCI-compatible GUI.

You can also talk to it directly:

```sh
cargo run --release
```

```text
uci
id name BitForce
id author stitzed
uciok
isready
readyok
position startpos moves e2e4 e7e5
go depth 5
bestmove <move>
quit
```

### Supported UCI commands

| Command                                      | Notes                                         |
| -------------------------------------------- | --------------------------------------------- |
| `uci`                                        | Replies with engine name, author and `uciok`  |
| `isready`                                    | Replies with `readyok`                        |
| `position startpos [moves ...]`              | Moves in long algebraic notation (`e2e4`, `e7e8q`) |
| `position fen <FEN> [moves ...]`             |                                               |
| `go [depth N]`                               | Fixed-depth search, depth defaults to `6`     |
| `quit`                                       | Exits                                         |

### Current limitations

- No time management: `go wtime/btime/movetime/infinite` are not supported, only `go depth N`
- No `stop`, `setoption` or `ucinewgame`
- No `info` output during search, only the final `bestmove`
- No transposition table, move ordering or quiescence search yet

## Testing and benchmarks

```sh
cargo test                      # perft checks (start position depth 5, Kiwipete depth 4)
cargo test -- --ignored         # slower, deeper perft checks (start position depth 6, Kiwipete depth 5)
cargo bench                     # perft speed benchmark (criterion)
```

Tests are compiled with the `release` profile, since perft is far too slow in debug mode.

## Project structure

```text
src/
├── main.rs          # entry point, runs the UCI loop
├── lib.rs           # library crate root
├── uci.rs           # UCI command parsing and handling
├── board.rs         # ChessBoard: state, FEN, make/unmake
├── board/movegen.rs # attack generation and legal move generation
├── bitboard.rs      # Bitboard type, compile-time masks and lookup tables
├── castle.rs        # castling rights and masks
├── piece.rs         # Color, Kind, Piece
├── piece_move.rs    # Move encoding
├── square.rs        # Square (newtype over u8)
├── search.rs        # alpha-beta search
├── eval.rs          # position evaluation
├── perft.rs         # perft
└── errors.rs        # error types
benches/nps.rs       # perft benchmark
tests/perft.rs       # perft correctness tests
```