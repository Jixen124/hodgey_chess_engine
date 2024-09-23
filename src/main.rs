use std::io::prelude::*;
use std::time::Duration;
use shakmaty::{Chess, Position, FromSetup, fen::Fen, uci::UciMove};
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use hodgey_chess_engine::find_best_move_with_time;

#[derive(std::default::Default)]
struct GameState {
    chess: Chess,
    previously_seen_hashes: Vec<u64>,
    white_time: u64,
    black_time: u64,
    exact_move_time: bool //If the time to move is exact (instead of total time remaining)
                            //I will use this later to make better moves (probably)
}

fn main() {
    let mut game_state = GameState::default();

    //Create output file if it doesn't exist
    if !std::path::Path::new("output.txt").exists() {
        std::fs::File::create("output.txt").expect("Should be able to create file");
    }

    loop {
        let mut input_buffer = String::new();
        std::io::stdin().read_line(&mut input_buffer).unwrap();
        
        // Writes inputs to a file to help debugging
        write_to_output_file(&input_buffer);

        let mut input_tokens = input_buffer.trim().split_whitespace();
        
        let first_token = match input_tokens.next() {
            Some(token) => token,
            None => continue
        };

        match first_token {
            "uci" => {
                identify_engine();
                display_options();
                println!("uciok");
            },
            "isready" => println!("readyok"),
            "quit" => break,
            "setoption" => todo!("Options cannot be set yet"),
            "position" => {
                update_position(input_tokens.map(|str| str.to_string()).collect(), &mut game_state);
            },
            "go" => {
                while let Some(value_type) = input_tokens.next() {
                    let value: u64 = input_tokens.next().unwrap().parse().unwrap();
                    
                    game_state.exact_move_time = false;

                    match value_type {
                        "wtime" => game_state.white_time = value,
                        "btime" => game_state.black_time = value,
                        "movetime" => {
                            game_state.exact_move_time = true;

                            if game_state.chess.turn().is_white() {
                                game_state.white_time = value;
                            }
                            else {
                                game_state.black_time = value;
                            }
                        },
                        _ => continue //Ignore unknown commands
                    }
                }

                return_best_move(&mut game_state)
            }
            _ => continue //Ignore unknown commands
        }
    }
}

fn write_to_output_file(input_buffer: &String) {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open("output.txt")
        .unwrap();
    file.write_all(input_buffer.as_bytes()).unwrap();
}

fn return_best_move(game_state: &mut GameState) {
    let mut remaining_millis = if game_state.chess.turn().is_white() {game_state.white_time} else {game_state.black_time};

    remaining_millis -= 100; //Add some overhead or something
    //My strange time equation, don't question it :)
    let min_search_ms = 5000.min(remaining_millis / 20);
    let min_search_time = Duration::from_millis(min_search_ms);

    let best_move = find_best_move_with_time(&game_state.chess, min_search_time, &mut game_state.previously_seen_hashes);
    println!("bestmove {}", best_move.to_uci(shakmaty::CastlingMode::Standard).to_string());
}

fn update_position(position: Vec<String>, game_state: &mut GameState) {
    let mut fen_and_moves = position.iter();
    let fen = fen_and_moves.next().expect("fen should be included");
    
    let mut chess = if fen == "startpos" {
        Chess::new()
    }
    else {
        let setup = Fen::from_ascii(fen.as_bytes()).expect("Fen should be valid").0;
        Chess::from_setup(setup, shakmaty::CastlingMode::Standard).expect("position should be valid")
    };

    let starting_pos_hash: Zobrist64 = chess.zobrist_hash(shakmaty::EnPassantMode::Legal);
    let mut hashes_seen = vec![starting_pos_hash.0];

    if let Some(_) = fen_and_moves.next() {
        for m in fen_and_moves {
            let selected_move = UciMove::from_ascii(m.as_bytes()).expect("Move should be valid");
            let legal_move = selected_move.to_move(&chess).expect("Move should be legal");
            if legal_move.is_zeroing() {
                hashes_seen.clear();
            }
            chess.play_unchecked(&legal_move);
            let hash: Zobrist64 = chess.zobrist_hash(shakmaty::EnPassantMode::Legal);
            hashes_seen.push(hash.0);
        }
    }

    game_state.chess = chess;
    game_state.previously_seen_hashes = hashes_seen;
}

fn identify_engine() {
    println!("id name Hodgeybot");
    println!("id author Jixen");
}

//Currently doesn't actually support any options, just pretends it does
fn display_options() {
    println!("option name move overhead type spin default 100 min 0 max 1000");
}