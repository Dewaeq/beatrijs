use crate::{board::Board, defs::{Score, Player, Piece, Value}, bitboard::BitBoard};

pub fn evaluate(board: &Board) -> Score {
    let white_material = count_material(board, Player::White);
    let black_material = count_material(board, Player::Black);

    let eval = if board.turn == Player::White {
        white_material - black_material
    } else {
        black_material - white_material
    };

    eval
}

fn count_material(board: &Board, side: Player) -> Score {
    let mut score = 0;

    score += BitBoard::count(board.player_piece_bb(side, Piece::Pawn)) as Score * Value::PAWN;
    score += BitBoard::count(board.player_piece_bb(side, Piece::Knight)) as Score * Value::KNIGHT;
    score += BitBoard::count(board.player_piece_bb(side, Piece::Bishop)) as Score * Value::BISHOP;
    score += BitBoard::count(board.player_piece_bb(side, Piece::Rook)) as Score * Value::ROOK;
    score += BitBoard::count(board.player_piece_bb(side, Piece::Queen)) as Score * Value::QUEEN;

    score
}
