use thiserror::Error;

#[derive(Debug, Error)]
pub enum FenParseError<'a> {
    #[error("invalid FEN format")]
    InvalidFormat,

    #[error("too many rows in FEN (extra '/' or invalid layout)")]
    ExtraRow,

    #[error("row overflow: column index {0} exceeds board boundary")]
    RowOverflow(u8),

    #[error("unknown piece char: {0}")]
    InvalidPieceChar(char),

    #[error("unexpected character in piece setup: {0}")]
    UnexpectedChar(char),
    
    #[error("invalid color char: {0}")]
    InvalidColor(&'a str),
    
    #[error("invalid castle: {0}")]
    InvalidCastle(#[source] CastlingParseError),

    #[error("invalid en passant square: {0}")]
    InvalidEnPassantSquare(#[source] SquareParseError),

    #[error("invalid fifty move counter: {0}")]
    InvalidFiftyMoveCounter(&'a str),

    #[error("invalid fullmove counter: {0}")]
    InvalidFullmoveCounter(&'a str),
}

#[derive(Debug, Error)]
pub enum CastlingParseError {
    #[error("expected K, Q, k or q found {0}")]
    InvalidChar(char),
    
    #[error("invalid format")]
    InvalidFormat
}

#[derive(Error, Debug)]
pub enum SquareParseError {
    #[error("length must be 2, found {0}")]
    InvalidLength(usize),

    #[error("column must be a lowercase ASCII letter, found '{0}'")]
    InvalidColumnChar(char),

    #[error("row must be a single digit, found '{0}'")]
    InvalidRowChar(char),
}