use std::{
    fmt::{self, Write},
    sync::{Arc, Mutex},
};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use rand::{Rng, SeedableRng};

pub fn get_routes() -> Router {
    let state = BoardState::new();

    Router::new()
        .route("/12/board", get(board))
        .route("/12/reset", post(reset))
        .route("/12/place/:team/:column", post(place))
        .route("/12/random-board", get(random_board))
        .with_state(state)
}

#[derive(Clone)]
pub struct BoardState {
    seed: Arc<Mutex<rand::rngs::StdRng>>,
    grid: Arc<Mutex<Grid>>,
}

impl BoardState {
    fn new() -> Self {
        Self {
            seed: Arc::new(Mutex::new(rand::rngs::StdRng::seed_from_u64(2024))),
            grid: Arc::new(Mutex::new(Grid::new())),
        }
    }
}

async fn board(State(state): State<BoardState>) -> impl IntoResponse {
    let grid = state.grid.lock().unwrap();

    if let Some(team) = grid.winner() {
        return (StatusCode::OK, format!("{}{} wins!\n", grid, team));
    }

    (StatusCode::OK, grid.to_string())
}

async fn reset(State(state): State<BoardState>) -> impl IntoResponse {
    let mut seed = state.seed.lock().unwrap();
    *seed = rand::rngs::StdRng::seed_from_u64(2024);
    let mut grid = state.grid.lock().unwrap();
    *grid = Grid::new();
    (StatusCode::OK, grid.to_string())
}

async fn place(
    Path((team, mut column)): Path<(String, u8)>,
    State(state): State<BoardState>,
) -> impl IntoResponse {
    let team = match team.as_str() {
        "cookie" => Item::Cookie,
        "milk" => Item::Milk,
        _other => return (StatusCode::BAD_REQUEST, "".to_string()),
    };

    if !(1..5).contains(&column) {
        return (StatusCode::BAD_REQUEST, "".to_string());
    }

    column -= 1;

    let mut grid = state.grid.lock().unwrap();

    if let Some(team) = grid.winner() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            format!("{}{} wins!\n", grid, team),
        );
    }

    let mut placed = false;

    for row in grid.0.iter_mut().rev() {
        if row[column as usize] == Item::Empty {
            row[column as usize] = team;
            placed = true;
            break;
        }
    }

    if !placed {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            format!("{}No winner.\n", grid),
        );
    }

    if let Some(team) = grid.winner() {
        return (StatusCode::OK, format!("{}{} wins!\n", grid, team));
    } else if grid.full() {
        return (StatusCode::OK, format!("{}No winner.\n", grid));
    }

    (StatusCode::OK, grid.to_string())
}

async fn random_board(State(state): State<BoardState>) -> String {
    let mut grid = state.grid.lock().unwrap();
    let mut seed = state.seed.lock().unwrap();

    *grid = Grid::new_rand(&mut seed);
    if let Some(team) = grid.winner() {
        return format!("{}{} wins!\n", grid, team);
    }

    format!("{}No winner.\n", grid)
}

#[derive(Default, PartialEq)]
enum Item {
    Cookie,
    #[default]
    Empty,
    Milk,
}

impl From<&Item> for char {
    fn from(val: &Item) -> Self {
        match val {
            Item::Cookie => '🍪',
            Item::Empty => '⬛',
            Item::Milk => '🥛',
        }
    }
}

#[derive(Default)]
struct Grid([[Item; 4]; 4]);

impl Grid {
    fn new() -> Self {
        Grid([
            [Item::Empty, Item::Empty, Item::Empty, Item::Empty],
            [Item::Empty, Item::Empty, Item::Empty, Item::Empty],
            [Item::Empty, Item::Empty, Item::Empty, Item::Empty],
            [Item::Empty, Item::Empty, Item::Empty, Item::Empty],
        ])
    }

    fn new_rand(seed: &mut rand::rngs::StdRng) -> Self {
        let mut grid = Grid::default();
        for row in 0..4 {
            for col in 0..4 {
                grid.0[row][col] = if seed.gen::<bool>() {
                    Item::Cookie
                } else {
                    Item::Milk
                };
            }
        }
        grid
    }

    fn winner(&self) -> Option<char> {
        // Check each row
        'row: for row in 0..4 {
            if self.0[row][0] == Item::Empty {
                continue;
            }
            for col in 0..4 {
                if self.0[row][col] != self.0[row][0] {
                    continue 'row;
                }
            }
            return Some((&self.0[row][0]).into());
        }

        // Check each column
        'col: for col in 0..4 {
            if self.0[0][col] == Item::Empty {
                continue;
            }
            for row in 0..4 {
                if self.0[row][col] != self.0[0][col] {
                    continue 'col;
                }
            }
            return Some((&self.0[0][col]).into());
        }

        // Check each diagnal
        if self.0[0][0] != Item::Empty && (0..4).all(|pos| self.0[pos][pos] == self.0[0][0]) {
            return Some((&self.0[0][0]).into());
        }

        if self.0[0][3] != Item::Empty && (0..4).all(|pos| self.0[pos][3 - pos] == self.0[0][3]) {
            return Some((&self.0[0][3]).into());
        }

        None
    }

    fn full(&self) -> bool {
        self.0
            .iter()
            .all(|row| row.iter().all(|pos| pos != &Item::Empty))
    }
}

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in &self.0 {
            f.write_str("⬜")?; // Add a border at the beginning of the row
            for column in row {
                f.write_char(column.into())?; // Add each column (converted) to the formatter
            }
            f.write_str("⬜\n")?; // Add a border at the end of the row and a newline
        }
        for _ in 0..6 {
            f.write_char('⬜')?; // Add the bottom border
        }
        f.write_char('\n')?;

        Ok(())
    }
}
