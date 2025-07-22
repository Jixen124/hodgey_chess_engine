use std::time::Duration;
use shakmaty::{Chess, Position, FromSetup, fen::Fen, uci::UciMove, Color, CastlingMode, EnPassantMode};
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use hodgey_chess_engine::find_best_move_with_time;
use uci_parser::{UciCommand, UciResponse, UciOption, UciOptionType};

const MOVE_OVERHEAD: Duration = Duration::from_millis(100);
const MAX_MOVE_TIME: Duration = Duration::from_secs(10);
//Currently doesn't actually support any options, just pretends it does
const OPTIONS: [(&str, UciOptionType); 1] = [
    ("move overhead", UciOptionType::Spin { default: 100, min: 0, max: 1000 })
];

#[derive(std::default::Default)]
struct GameState {
    chess: Chess,
    previously_seen_hashes: Vec<u64>,
    debug_enabled: bool,
}

fn main() {
    let mut game_state = GameState::default();

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Should be able to read input");
        
        if game_state.debug_enabled {
            // Writes inputs to stderr for debugging
            eprintln!("{}", input);
        };

        let Ok(message) =  UciCommand::new(&input) else {
            continue; //Ignore unknown commands
        };

        match message {
            UciCommand::Uci => print_uci_response(),
            UciCommand::Debug(should_debug) => game_state.debug_enabled = should_debug,
            UciCommand::IsReady => println!("{}", UciResponse::readyok()), 
            UciCommand::Position { fen, moves } => update_position(fen, moves, &mut game_state),
            UciCommand::Go(search_options) => print_best_move(&search_options, &mut game_state),
            UciCommand::Quit => break,
            // TODO make options work in some form
            //SetOption, Register, UciNewGame, Stop, and PonderHit are all currently ignored
            _ => {}
        }
    }
}

fn print_uci_response() {
    println!("{}", UciResponse::Name("Hodgeybot"));
    println!("{}", UciResponse::Author("Jixen"));
    display_options();
    println!("{}", UciResponse::uciok());
}

fn display_options() {
    for (name, option_type) in OPTIONS {
        println!("{}", UciResponse::Option::<String>(UciOption::new(name, option_type)));
    }
}

fn print_best_move(options: &uci_parser::UciSearchOptions, game_state: &mut GameState) {
    let search_time = match game_state.chess.turn() {
        Color::White => options.wtime,
        Color::Black => options.btime,
    }
        .map(|d| (d / 8).min(MAX_MOVE_TIME))
        .unwrap_or(MAX_MOVE_TIME);

    let mut search_time = options.movetime.unwrap_or(search_time);
    search_time -= MOVE_OVERHEAD; //Add move overhead or something (I have no clue what I am doing)

    let best_move = find_best_move_with_time(&game_state.chess, search_time, &mut game_state.previously_seen_hashes);
    println!("bestmove {}", best_move.to_uci(CastlingMode::Standard));
}

fn update_position(fen: Option<String>, moves: Vec<String>, game_state: &mut GameState) {
    let mut chess = fen
        .and_then(|fen| fen.parse::<Fen>().ok())
        .and_then(|fen| Chess::from_setup(fen.into_setup(), CastlingMode::Standard).ok())
        .unwrap_or_default();

    let starting_pos_hash: Zobrist64 = chess.zobrist_hash(EnPassantMode::Legal);
    let mut hashes_seen = vec![starting_pos_hash.0];

    for m in moves {
        let Some(legal_move) = m.parse::<UciMove>().ok()
            .and_then(|m| m.to_move(&chess).ok())
        else {
            continue;
        };

        if legal_move.is_zeroing() {
            hashes_seen.clear();
        }
        chess.play_unchecked(legal_move);
        let hash: Zobrist64 = chess.zobrist_hash(EnPassantMode::Legal);
        hashes_seen.push(hash.0);
    }

    game_state.chess = chess;
    game_state.previously_seen_hashes = hashes_seen;
}