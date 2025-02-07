// reversi_client/src/lib.rs

use rand::Rng;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReversiError {
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Protocol error: {0}")]
    ProtocolError(String),
}

pub struct GameState {
    pub turn: i8,
    pub round: i32,
    pub t1: f32,
    pub t2: f32,
    pub board: [[i8; 8]; 8], // 0: empty, 1: player1, 2: player2
}

pub trait ReversiStrategy {
    fn choose_move(&self, valid_moves: &[(i8, i8)]) -> (i8, i8);
}

pub struct RandomStrategy;

impl ReversiStrategy for RandomStrategy {
    fn choose_move(&self, valid_moves: &[(i8, i8)]) -> (i8, i8) {
        let mut rng = rand::thread_rng();
        valid_moves[rng.gen_range(0..valid_moves.len())]
    }
}

pub struct ReversiClient<S: ReversiStrategy> {
    stream: TcpStream,
    player_number: i8,
    strategy: S,
    game_minutes: f32,
    board: [[i8; 8]; 8],
}

impl<S: ReversiStrategy> ReversiClient<S> {
    pub fn connect(
        server_addr: &str,
        player_number: i8,
        strategy: S,
    ) -> Result<Self, ReversiError> {
        let port = 3333 + player_number as i32;
        let addr = SocketAddr::from_str(&format!("{}:{}", server_addr, port))
            .map_err(|e| ReversiError::ConnectionError(e.to_string()))?;

        let mut stream = TcpStream::connect(addr)?;

        let mut buffer = [0u8; 1024];
        let mut bytes_read = stream.read(&mut buffer)?;

        while bytes_read == 0 {
            bytes_read = stream.read(&mut buffer)?;
        }

        let message = String::from_utf8_lossy(&buffer[..bytes_read]);

        let parts: Vec<&str> = message.split(' ').collect();
        println!("{:?}", parts);
        let server_player_number = parts[0].parse().unwrap_or(-1);
        let game_minutes = parts[1].trim().parse::<f32>().unwrap_or(0.0);

        if player_number != server_player_number {
            return Err(ReversiError::ProtocolError(format!(
                "Player number mismatch: expected {}, got {}",
                player_number, server_player_number
            )));
        }

        Ok(ReversiClient {
            stream,
            player_number,
            strategy,
            game_minutes,
            board: [[0; 8]; 8],
        })
    }

    pub fn run(&mut self) -> Result<(), ReversiError> {
        let mut buffer = [0u8; 1024];
        let mut past_4_turns = false;
        let mut num_turns = 0u8;

        loop {
            let bytes_read = self.stream.read(&mut buffer)?;
            if bytes_read == 0 {
                return Err(ReversiError::ConnectionError("Connection closed".into()));
            }

            let message = String::from_utf8_lossy(&buffer[..bytes_read]);
            if let Ok(state) = self.parse_message(&message) {
                println!("Parsed state!");
                if state.turn == self.player_number {
                    if !past_4_turns {  // On the first 4 turns
                        num_turns += 1; 
                        if num_turns <= 4 {
                            let valid_moves = self.get_valid_moves_first_4(&state.board);
                            if !valid_moves.is_empty() {
                                let (row, col) = self.strategy.choose_move(&valid_moves);
                                self.send_move(row, col)?;
                                continue;
                            }
                        } else {
                            past_4_turns = true;
                        }
                    }
                
                    let valid_moves = self.get_valid_moves(&state.board, self.player_number);
                    let (row, col) = self.strategy.choose_move(&valid_moves);
                    self.send_move(row, col)?;
                }
            } else {
                println!("Failed to parse message");
            }
        }
    }

    fn parse_message(&mut self, message: &str) -> Result<GameState, ReversiError> {
        let parts: Vec<&str> = message.split('\n').collect();
        println!("{:?}", parts);
        let turn=  parts[0].parse::<i32>().unwrap_or(-1);
        if turn == -999 {
            println!("Game over");
            return Err(ReversiError::ProtocolError("Game over".into()));
        }

        if parts.len() < 69 {
            return Err(ReversiError::ProtocolError("Message was too short to contain the board".into()));
        }

        let mut board: [[i8; 8]; 8] = [[0; 8]; 8];
        let mut index = 4;
        for i in 0..8 {
            for j in 0..8 {
                board[i][j] = parts[index].parse().unwrap_or(0);
                index += 1;
            }
        }

        self.board = board;


        let small_turn = turn as i8;

        Ok(GameState {
            turn: small_turn,
            round: parts[1].parse().unwrap_or(0),
            t1: parts[2].parse::<f32>().unwrap_or(0.0),
            t2: parts[3].parse::<f32>().unwrap_or(0.0),
            board,
        })
    }

    fn get_valid_moves_first_4(&self, board: &[[i8; 8]; 8]) -> Vec<(i8, i8)> {
        let mut moves = Vec::with_capacity(4);

        if board[3][3] == 0 {
            moves.push((3, 3));
        }

        if board[3][4] == 0 {
            moves.push((3, 4));
        }

        if board[4][3] == 0 {
            moves.push((4, 3));
        }

        if board[4][4] == 0 {
            moves.push((4, 4));
        }

        moves
    }

    pub fn get_valid_moves(&self, board: &[[i8; 8]; 8], player: i8) -> Vec<(i8, i8)> {
        let mut moves: Vec<(i8, i8)> = Vec::with_capacity(24);
        let opponent = 3 - player; // Player numbers are 1 or 2

        // Directions: (dx, dy) for all 8 possible directions
        const DIRECTIONS: [(i8, i8); 8] = [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1),
        ];

        for (i, row) in board.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                if cell != 0 {
                    continue;
                }

                'directions: for &(dx, dy) in &DIRECTIONS {
                    let mut x = i as i8 + dx;
                    let mut y = j as i8 + dy;
                    let mut found_opponent = false;

                    while x >= 0 && x < 8 && y >= 0 && y < 8 {
                        let current = board[x as usize][y as usize];

                        match current {
                            // Empty space, can't flank
                            0 => break,
                            // Found player's piece after opponent's
                            p if p == player => {
                                if found_opponent {
                                    moves.push((i as i8, j as i8));
                                    break 'directions;
                                }
                                break;
                            }
                            // Found opponent's piece
                            p if p == opponent => {
                                found_opponent = true;
                            }
                            _ => break,
                        }

                        x += dx;
                        y += dy;
                    }
                }
            }
        }

        moves
    }

    fn send_move(&mut self, row: i8, col: i8) -> Result<(), ReversiError> {
        let move_str = format!("{}\n{}\n", row, col);
        self.stream.write_all(move_str.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{net::TcpListener, thread};

    use super::*;

    fn create_mock_stream() -> TcpStream {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let _ = socket.write_all(b"");
        });

        TcpStream::connect(addr).unwrap()
    }


    #[test]
    fn test_parse_message_valid() {
        let mut client = ReversiClient {
            stream: create_mock_stream(),
            player_number: 1,
            strategy: RandomStrategy,
            game_minutes: 0.0,
            board: [[0; 8]; 8],
        };

        let message = "1\n0\n0.0\n0.0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n";
        let state = client.parse_message(message).unwrap();

        assert_eq!(state.turn, 1);
        assert_eq!(state.round, 0);
        assert_eq!(state.t1, 0.0);
        assert_eq!(state.t2, 0.0);
        assert_eq!(state.board, [[0; 8]; 8]);
    }

    #[test]
    fn test_parse_message_invalid() {
        let mut client = ReversiClient {
            stream: create_mock_stream(),
            player_number: 1,
            strategy: RandomStrategy,
            game_minutes: 0.0,
            board: [[0; 8]; 8],
        };

        let message = "invalid\nmessage\n";
        let result = client.parse_message(message);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_message_game_over() {
        let mut client = ReversiClient {
            stream: create_mock_stream(),
            player_number: 1,
            strategy: RandomStrategy,
            game_minutes: 0.0,
            board: [[0; 8]; 8],
        };

        let message = "-999\n";
        let result = client.parse_message(message);
        if let Err(ReversiError::ProtocolError(ref e)) = result {
            assert_eq!(e, "Game over");
        } else {
            panic!("Expected ProtocolError with 'Game over'");
        }
    }

    #[test]
    fn test_get_valid_moves_first_4() {
        let client = ReversiClient {
            stream: create_mock_stream(),
            player_number: 1,
            strategy: RandomStrategy,
            game_minutes: 0.0,
            board: [[0; 8]; 8],
        };

        let board = [[0; 8]; 8];
        let valid_moves = client.get_valid_moves_first_4(&board);

        assert_eq!(valid_moves, vec![(3, 3), (3, 4), (4, 3), (4, 4)]);
    }

    #[test]
    fn test_get_valid_moves() {
        let client = ReversiClient {
            stream: create_mock_stream(),
            player_number: 1,
            strategy: RandomStrategy,
            game_minutes: 0.0,
            board: [[0; 8]; 8],
        };

        let mut board = [[0; 8]; 8];
        board[3][3] = 2;
        board[3][4] = 1;
        board[4][3] = 1;
        board[4][4] = 2;

        let valid_moves = client.get_valid_moves(&board, 1);

        assert_eq!(valid_moves, vec![(2, 3), (3, 2), (4, 5), (5, 4)]);
    }

    #[test]
    fn test_send_move() {
        let mut client = ReversiClient {
            stream: create_mock_stream(),
            player_number: 1,
            strategy: RandomStrategy,
            game_minutes: 0.0,
            board: [[0; 8]; 8],
        };

        let result = client.send_move(3, 4);

        assert!(result.is_ok());
    }
}