//! # Hodgey Chess Engine
//!
//! `hodgey_chess_engine` is a very simple chess engine for [hodgeybot](https://github.com/Jixen124/hodgeybot).
//! hodgeybot can also be found [here](https://lichess.org/@/Hodgeybot) on lichess.
//! This requires using the [shakmaty crate](https://crates.io/crates/shakmaty) for handling chess games.

use std::time::{Duration, Instant};

use shakmaty::{zobrist::{Zobrist64, ZobristHash}, Chess, Move, Outcome, Position};
use evaluation::{evaluate_board, move_score, capture_score};

mod evaluation;
mod piece_square_tables;
mod test_fens;

const INFINITY: i32 = i32::MAX;
const NEG_INFINITY: i32 = -INFINITY;
const REALLY_BIG_CHECKMATE_NUMBER: i32 = 100_000_000;
const TRANSPOSITION_TABLE_LENGTH: usize = 1024 * 1024 * 8;
const TABLE_INDEX_MASK: usize = TRANSPOSITION_TABLE_LENGTH - 1;

#[derive(Clone, Copy, PartialEq)]
enum TranspositionTableFlag {
    None,
    Exact,
    Lowerbound,
    Upperbound
}

#[derive(Clone, Copy)]
struct TranspositionTableData {
    hash: u64,
    score: i32,
    depth: u16,
    best_move_index: u8,
    flag: TranspositionTableFlag
}

impl TranspositionTableData {
    const fn new() -> TranspositionTableData {
        TranspositionTableData {
            hash: 0,
            score: 0,
            depth: 0,
            best_move_index: 0,
            flag: TranspositionTableFlag::None
        }
    }
}

/// Finds the best move for a given depth.
pub fn find_best_move_with_depth(chess: &Chess, max_depth: u16, previously_seen_hashes: &mut Vec<u64>) -> Move {
    let mut transposition_table: Vec<TranspositionTableData> = vec![TranspositionTableData::new(); TRANSPOSITION_TABLE_LENGTH];
    
    let mut moves = chess.legal_moves();
    
    if moves.len() == 1 {
        return moves[0].clone();
    }

    let mut depth = 2;

    while depth < max_depth {
        let mut best_score = NEG_INFINITY;

        for (index, m) in moves.clone().iter().enumerate() {
            let mut new_chess = chess.clone();
            new_chess.play_unchecked(&m);

            let score = -nega_max(&new_chess, depth, NEG_INFINITY, -best_score,
                                        &mut transposition_table, previously_seen_hashes);
            if score > best_score {
                best_score = score;
                for i in (0..index).rev() {
                    moves.swap(i, i+1);
                }
            }
        }
        
        //This is in outer loop to make sure that faster checkmates are selected
        //Possibly not needed now with iterative deepening?
        //First move that gives me a checkmate possibly good enough?
        if best_score.abs() >= REALLY_BIG_CHECKMATE_NUMBER {
            break;
        }

        depth += 2;
    }

    return moves[0].clone();
}

/// Finds the best move searching for a given minimum search time.
/// 
/// # WARNING
/// Currently goes well over the given time.
pub fn find_best_move_with_time(chess: &Chess, min_search_time: Duration, previously_seen_hashes: &mut Vec<u64>) -> Move {
    let start_time = Instant::now();

    let mut transposition_table: Vec<TranspositionTableData> = vec![TranspositionTableData::new(); TRANSPOSITION_TABLE_LENGTH];
    
    let mut moves = chess.legal_moves();
    
    if moves.len() == 1 {
        return moves[0].clone();
    }

    let mut depth = 2;

    while Instant::now() - start_time < min_search_time {
        let mut best_score = NEG_INFINITY;

        for (index, m) in moves.clone().iter().enumerate() {
            if Instant::now() - start_time >= min_search_time {
                break;
            }

            let mut new_chess = chess.clone();
            new_chess.play_unchecked(&m);

            let score = -nega_max(&new_chess, depth, NEG_INFINITY, -best_score,
                                        &mut transposition_table, previously_seen_hashes);
            if score > best_score {
                best_score = score;
                for i in (0..index).rev() {
                    moves.swap(i, i+1);
                }
            }
        }
        
        //This is in outer loop to make sure that faster checkmates are selected
        //Possibly not needed now with iterative deepening?
        //First move that gives me a checkmate possibly good enough?
        if best_score.abs() >= REALLY_BIG_CHECKMATE_NUMBER {
            break;
        }

        depth += 2;
    }

    return moves[0].clone();
}

fn nega_max(chess: &Chess, depth: u16, mut alpha: i32, mut beta: i32,
            transposition_table: &mut Vec<TranspositionTableData>, previously_seen_hashes: &mut Vec<u64>) -> i32 {
    
    if let Some(outcome) = chess.outcome() {
        return match outcome {
            Outcome::Draw => 0,
            _ => -REALLY_BIG_CHECKMATE_NUMBER - depth as i32
        };
    }

    let hash: Zobrist64 = chess.zobrist_hash(shakmaty::EnPassantMode::Legal);
    let hash = hash.0;
    
    //Engine will evaluate a draw if a single repetition occurs
    if previously_seen_hashes.contains(&hash) {
        // A draw is given zero score
        return 0;
    }

    if depth == 0 {
        return quiescence_search(&chess, alpha, beta);
    }

    let original_alpha = alpha;

    let table_index = hash as usize & TABLE_INDEX_MASK;
    if transposition_table[table_index].hash == hash && transposition_table[table_index].depth >= depth {
        if transposition_table[table_index].flag == TranspositionTableFlag::Exact {
            return transposition_table[table_index].score;
        }
        else if transposition_table[table_index].flag == TranspositionTableFlag::Lowerbound {
            alpha = alpha.max(transposition_table[table_index].score);
        }
        else if transposition_table[table_index].flag == TranspositionTableFlag::Upperbound {
            beta = beta.min(transposition_table[table_index].score);
        }
        
        if alpha >= beta {
            return transposition_table[table_index].score;
        }
    }

    previously_seen_hashes.push(hash);

    let mut value = NEG_INFINITY;
    let mut best_move_index = 0;

    let mut moves = chess.legal_moves();
    moves.sort_unstable_by_key(|m| move_score(m));


    if transposition_table[table_index].hash == hash && (transposition_table[table_index].best_move_index as usize) < moves.len() {
        //Search best move first if there is an entry in the transposition table
        let mut new_chess = chess.clone();
        best_move_index = transposition_table[table_index].best_move_index as usize;
        new_chess.play_unchecked(&moves[best_move_index]);
        let score = -nega_max(&new_chess, depth - 1, -beta, -alpha,
                                    transposition_table, previously_seen_hashes);
        value = value.max(score);
        alpha = alpha.max(value);
    }
    
    if !(alpha >= beta) {
        for (index, m) in moves.iter().enumerate() {
            if transposition_table[table_index].hash == hash && index == transposition_table[table_index].best_move_index as usize {
                continue;
            }

            let mut new_chess = chess.clone();
            new_chess.play_unchecked(m);
            let score = -nega_max(&new_chess, depth - 1, -beta, -alpha,
                                        transposition_table, previously_seen_hashes);
            if score > value {
                value = score;
                best_move_index = index;
                alpha = alpha.max(value);
                if alpha >= beta {
                    break;
                }
            }
        }
    }

    previously_seen_hashes.pop();

    if transposition_table[table_index].depth < depth {
        transposition_table[table_index].hash = hash;
        transposition_table[table_index].score = value;
        transposition_table[table_index].depth = depth;
        transposition_table[table_index].best_move_index = best_move_index as u8;
        
        transposition_table[table_index].flag = if value <= original_alpha {
            TranspositionTableFlag::Upperbound
        }
        else if value >= beta {
            TranspositionTableFlag::Lowerbound
        }
        else {
            TranspositionTableFlag::Exact
        }
    }
    
    value
}

fn quiescence_search(chess: &Chess, mut alpha: i32, beta: i32) -> i32 {
    let stand_pat = evaluate_board(chess.board()) * if chess.turn().is_white() {1} else {-1};
    
    if stand_pat >= beta {
        return beta;
    }

    if alpha < stand_pat {
        alpha = stand_pat;
    }
    
    let mut capture_moves = chess.capture_moves();
    capture_moves.sort_unstable_by_key(|m| capture_score(m));

    for m in &capture_moves {
        let mut new_chess = chess.clone();
        new_chess.play_unchecked(m);
        let score = -quiescence_search(&new_chess, -beta, -alpha);

        if score >= beta {
            return beta;
        }

        if score > alpha {
            alpha = score;
        }
    }
    
    return alpha;
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::fen::Fen;
    use shakmaty::{CastlingMode, Chess, FromSetup};

    #[test]
    //This is just to test performace, it asserts nothing
    fn test_fens_time() {
        for fen in test_fens::WIN_AT_CHESS {
            let setup = Fen::from_ascii(fen.as_bytes()).expect("Fen should be valid").0;
            let chess = Chess::from_setup(setup, CastlingMode::Standard).expect("position should be valid");
            find_best_move_with_depth(&chess, 2, &mut Vec::new());
        }
    }

    #[test]
    //This is just to test performace, it asserts nothing
    fn test_position_time() {
        let setup = Fen::from_ascii("2rq1bk1/1b4pp/pn3n2/1p1Ppp2/1PP1P3/7P/3N1PP1/R2QRBK1 w - - 0 23".as_bytes()).expect("Fen should be valid").0;
        let chess = Chess::from_setup(setup, CastlingMode::Standard).expect("position should be valid");
        find_best_move_with_depth(&chess, 8, &mut Vec::new());
    }

    #[test]
    //Checks the the program successfully solves the lasker position
    fn lasker_position() {
        let setup = Fen::from_ascii("8/k7/3p4/p2P1p2/P2P1P2/8/8/K7 w - -".as_bytes()).expect("Fen should be valid").0;
        let chess = Chess::from_setup(setup, CastlingMode::Standard).expect("position should be valid");
        assert!(find_best_move_with_depth(&chess, 20, &mut Vec::new()).to_string() == "Ka1-b1");
    }

    #[test]
    //Makes sure both my methods agree on best move from a test position
    //This might stop working if my searches become faster. It is kinda luck and hardware based.
    fn time_and_depth_agree() {
        let setup = Fen::from_ascii("2rq1bk1/1b4pp/pn3n2/1p1Ppp2/1PP1P3/7P/3N1PP1/R2QRBK1 w - - 0 23".as_bytes()).expect("Fen should be valid").0;
        let chess = Chess::from_setup(setup, CastlingMode::Standard).expect("position should be valid");
        let m1 = find_best_move_with_depth(&chess, 8, &mut Vec::new());
        let m2 = find_best_move_with_time(&chess, Duration::from_millis(500), &mut Vec::new());
        assert!(m1 == m2);
    }
}