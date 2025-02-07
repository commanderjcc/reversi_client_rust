# reversi_client_rust

# Description

This is a Rust library for a Reversi game client, specifically designed for **BYU CS 470**. It includes the `ReversiClient` struct, the `GameState` struct, and the `ReversiStrategy` trait, along with methods to manage game state and make moves.

Note, that this library is not a complete implementation of the Reversi game. It is designed to be used as a starting point for building a Reversi agent.

> *No support is provided or guaranteed by the TAs for anything here. Only use this if you know Rust, want to learn Rust, and are a strong programmer already. Pull requests are welcome.* 

# Usage

```rust
use reversi_client::{ReversiClient, RandomStrategy, ReversiError};

fn main() -> Result<(), ReversiError> {
    let server_address = "127.0.0.1"; // Replace with your server address
    let player_number = 2; // Set the player number (1 or 2)
    let strategy = RandomStrategy; // Replace with your own strategy

    let mut client = ReversiClient::connect(server_address, player_number, strategy)?;
    client.run()?;

    Ok(())
}
```

# API

## ReversiClient

The client struct manages the connection to the server, game state, and applies the given strategy whenever a move is needed. 

```rust
pub struct ReversiClient<S: ReversiStrategy> {
    stream: TcpStream,
    player_number: i8,
    strategy: S,
    game_minutes: f32,
    board: [[i8; 8]; 8],
}
```

### Fields
Field Name | Type | Description
--- | --- | ---
`stream` | `TcpStream` | The TCP stream used to communicate with the server.
`player_number` | `i8` | The player number (1 or 2).
`strategy` | `ReversiStrategy` | The strategy to use for choosing moves.
`game_minutes` | `f32` | The number of minutes the player has to make their moves. This should be updated by the server on each turn.
`board` | `[[i8; 8]; 8]` | The current board state. Each cell is either 0 (empty), 1, or 2.

### connect()

`ReversiClient::connect()` - Connects to the server and initializes the game state, returns a client struct.

Parameter Name | Type | Description
--- | --- | ---
`server_address:`| `&str` | The ip address of the server, without the port. Port is assumed ot be `3333 + player_number`.
`player_number:`| `i8` | The player number (1 or 2).
`strategy:` | `ReversiStrategy` | The strategy to use for choosing moves.

### run()

`ReversiClient::run()` - Starts the game loop, handling incoming messages and making moves according to the strategy.

### get_valid_moves()

`ReversiClient::get_valid_moves()` - Returns a vector of valid moves for a given player and board state.

Parameter Name | Type | Description
--- | --- | ---
`board` | `&[[i8; 8]; 8]` | The current board state. Each cell is either 0 (empty), 1, or 2. Passed so that you can get valid moves for any player or position.
`player` | `i8` | The number of the player that is making the move.


## ReversiStrategy

This trait defines the strategy for choosing moves in the game. You can implement your own strategy by creating a struct that implements this trait.

```rust
pub trait ReversiStrategy {
    fn choose_move(&self, valid_moves: &[(i8, i8)]) -> (i8, i8);
}
```

You can use the `&self` parameter of the `choose_move()` method to access the fields of your client and calculate the best move based on your own strategy.

For example consider the following strategy that chooses a random valid move:

```rust
pub struct RandomStrategy;

impl ReversiStrategy for RandomStrategy {
    fn choose_move(&self, valid_moves: &[(i8, i8)]) -> (i8, i8) {
        let mut rng = rand::thread_rng();
        valid_moves[rng.gen_range(0..valid_moves.len())]
    }
}
```