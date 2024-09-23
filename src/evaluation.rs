use shakmaty::{Board, Color, Move, Role};
use crate::piece_square_tables;

// Returns an evaluation of the current board position from the perspective of white
#[inline]
pub fn evaluate_board(board: &Board) -> i32 {
    let mut white_material_score = 0;
    let mut black_material_score = 0;

    for square in board.white().intersect(board.pawns()) {
        white_material_score += piece_square_tables::PAWN[square as usize];
    }
    for square in board.white().intersect(board.bishops()) {
        white_material_score += piece_square_tables::BISHOP[square as usize];
    }
    for square in board.white().intersect(board.knights()) {
        white_material_score += piece_square_tables::KNIGHT[square as usize];
    }
    for square in board.white().intersect(board.rooks()) {
        white_material_score += piece_square_tables::ROOK[square as usize];
    }
    for square in board.white().intersect(board.queens()) {
        white_material_score += piece_square_tables::QUEEN[square as usize];
    }
    white_material_score += piece_square_tables::KING[board.king_of(Color::White).unwrap() as usize];

    for square in board.black().intersect(board.pawns()) {
        black_material_score += piece_square_tables::PAWN[square as usize ^ 56];
    }
    for square in board.black().intersect(board.bishops()) {
        black_material_score += piece_square_tables::BISHOP[square as usize ^ 56];
    }
    for square in board.black().intersect(board.knights()) {
        black_material_score += piece_square_tables::KNIGHT[square as usize ^ 56];
    }
    for square in board.black().intersect(board.rooks()) {
        black_material_score += piece_square_tables::ROOK[square as usize ^ 56];
    }
    for square in board.black().intersect(board.queens()) {
        black_material_score += piece_square_tables::QUEEN[square as usize ^ 56];
    }
    black_material_score += piece_square_tables::KING[board.king_of(Color::Black).unwrap() as usize ^ 56];

    let material_difference = white_material_score - black_material_score;
    let total_material = white_material_score + black_material_score;

    //encourages trading when up material
    let trade_bonus = 100 * material_difference / total_material;

    material_difference + trade_bonus
}

//Gives moves a score for sorting, lower scores are better
#[inline]
pub const fn move_score(m: &Move) -> i32 {
    let mut score = if m.is_promotion() {60} else {0};
    if let Some(role) = m.capture() {
        score += match m.role() {
            Role::Pawn => 1,
            Role::Bishop => 3,
            Role::Knight => 3,
            Role::Rook => 5,
            _ => 9
        } - match role {
            Role::Pawn => 10,
            Role::Bishop => 30,
            Role::Knight => 30,
            Role::Rook => 50,
            _ => 90
        }
    }
    score
}

//Gives captures a score for sorting, lower scores are better
#[inline]
pub fn capture_score(m: &Move) -> i32 {
    let role = m.capture().unwrap();
    return match m.role() {
        Role::Pawn => 1,
        Role::Bishop => 3,
        Role::Knight => 3,
        Role::Rook => 5,
        _ => 9
    } - match role {
        Role::Pawn => 10,
        Role::Bishop => 30,
        Role::Knight => 30,
        Role::Rook => 50,
        _ => 90
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::{Board, Square};
    
    #[test]
    //Confirms that bot properly evaluates trades as good when up material
    fn trades() {
        let board1 = Board::from_ascii_board_fen("7k/7p/8/8/8/Q7/P7/K7".as_bytes()).expect("Fen should be valid");
        let board2 = Board::from_ascii_board_fen("6qk/7p/8/8/8/Q7/P7/KQ6".as_bytes()).expect("Fen should be valid");
        assert!(evaluate_board(&board1) > evaluate_board(&board2));
    }

    #[test]
    //Basic test of move ordering
    fn ordering() {
        let move1 = Move::Normal { role: Role::Pawn, from: Square::A1, capture: Some(Role::Rook), to: Square::A1, promotion: None };
        let move2 = Move::Normal { role: Role::Pawn, from: Square::A1, capture: Some(Role::Queen), to: Square::A1, promotion: None };
        let move3 = Move::Normal { role: Role::Bishop, from: Square::A1, capture: Some(Role::Rook), to: Square::A1, promotion: None };

        let move_score1 = move_score(&move1);
        let move_score2 = move_score(&move2);
        let move_score3 = move_score(&move3);

        //Better moves should have lower scores
        assert!(move_score2 < move_score1); //m2 better than m1
        assert!(move_score1 < move_score3); //m1 better than m3
    }
}