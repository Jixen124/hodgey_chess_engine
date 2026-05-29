use std::time::Duration;
use shakmaty::{Chess, Position, FromSetup, fen::Fen, uci::UciMove, Color, CastlingMode, EnPassantMode};
use shakmaty::zobrist::Zobrist64;
use hodgey_chess_engine::find_best_move_with_time;
use uci_parser::{UciCommand, UciOption, UciResponse};

const OPTIONS: [HodgeyUciOption; 3] = [
    HodgeyUciOption::new("MoveOverhead", 100, 0, 1000),
    HodgeyUciOption::new("MaxMoveTime", 10_000, 0, 60_000),
    HodgeyUciOption::new("MaxPercentageOfTimePerMove", 10, 1, 10)
];

struct HodgeyUciOption {
    name: &'static str,
    default: i32,
    min: i32,
    max: i32,
    value: i32
}

impl HodgeyUciOption {
    const fn new(name: &'static str, default: i32, min: i32, max: i32) -> HodgeyUciOption {
        HodgeyUciOption { name, default, min, max, value: default }
    }
    
    fn to_uci_option(&self) -> UciOption {
        UciOption::spin(self.name, self.default, self.min, self.max)
    }
}

struct GameState {
    chess: Chess,
    previously_seen_hashes: Vec<u64>,
    debug_enabled: bool,
    options: [HodgeyUciOption; 3]
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            options: OPTIONS,
            chess: Default::default(),
            previously_seen_hashes: Default::default(),
            debug_enabled: Default::default()
        }
    }
}

impl GameState {
    /// Exepcts that name will be a valid option name
    fn option_value_unchecked(&self, name: &str) -> i32 {
        let option = self.options.iter().find(|opt| opt.name == name);
        //Expect valid option to be passed
        option.unwrap().value
    }

    fn max_move_time(&self) -> Duration {
        let value = self.option_value_unchecked("MaxMoveTime");
        Duration::from_millis(value as u64)
    }

    fn move_overhead(&self) -> Duration {
        let value = self.option_value_unchecked("MoveOverhead");
        Duration::from_millis(value as u64)
    }

    fn percentage_of_time_per_move(&self) -> i32 {
        self.option_value_unchecked("MaxPercentageOfTimePerMove")
    }
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
            UciCommand::SetOption { name, value } => set_option(name, value, &mut game_state),
            UciCommand::Quit => break,
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
    for opt in OPTIONS {
        println!("{}", UciResponse::Option(opt.to_uci_option()));
    }
}

fn set_option(name: String, value: Option<String>, game_state: &mut GameState) {
    let option = game_state.options.iter_mut().find(|opt| opt.name == name);
    let Some(option) = option else {
        //return if option doesn't exist
        return;
    };
    let value = value.and_then(|v| v.parse::<i32>().ok());
    
    let Some(value) = value else {
        //return if no value given
        return;
    };

    if value >= option.min && value <= option.max {
        option.value = value;
    }
}

fn print_best_move(options: &uci_parser::UciSearchOptions, game_state: &mut GameState) {
    let search_time = match game_state.chess.turn() {
        Color::White => options.wtime,
        Color::Black => options.btime,
    }
        //use only the correct percentage of remaining time
        .map(|d| d * 100 / game_state.percentage_of_time_per_move() as u32)
        //limit move time to max move time
        .map(|d| d.min(game_state.max_move_time()));

    let mut search_time = options.movetime
        .or(search_time)
        .unwrap_or(game_state.max_move_time());
    
    if search_time < game_state.move_overhead() {
        search_time = Duration::ZERO;
    }
    else {
        search_time -= game_state.move_overhead();
    }

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
            //Ignores moves it can't understand
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