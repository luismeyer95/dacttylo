# dacttylo

A terminal-based application to practice typing code.


<!-- GETTING STARTED -->
## Quick Start

### Prerequisites

You should install the Rust toolchain to build the project. See https://www.rust-lang.org/tools/install.

### Installation

```sh
cargo install --path .
```

This will build and install the binary at `$HOME/.cargo/bin`.

<!-- USAGE EXAMPLES -->
## Usage

Navigate the CLI options with these two commands:

```sh
dacttylo --help
dacttylo help <subcommand>
```

### Practice Mode

Normal practice session

```sh
dacttylo practice -f <filepath>
```

Record your inputs during this session with the save option `-s, --save`. There can only be one input record at any one time for a given file.
- `best` will keep the fastest time input record between a potentially existing record for this practice file and the input record for this next session.
- `override` will do exactly that, override any existing input record for this file with the input record of this next session.

```sh
dacttylo practice -f <filepath> -s best
```

Why record your inputs at all? To race against your past self on your next sessions. Provide the `-g, --ghost` option to try loading an existing input record for the file and start racing.

```sh
dacttylo practice -f <filepath> -g
```

### LAN Multiplayer Mode

Race against other people on the same local network using the `host` and `join` subcommands.

```sh
dacttylo host -u user1 -f README.md
```

```sh
dacttylo join user1 -u user2
```
