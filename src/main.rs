extern crate piston_window;
extern crate rand;

use piston_window::*;
use rand::Rng;

const BOARD_WIDTH: usize = 300;
const BOARD_HEIGHT: usize = 300;
const CELL_SIZE: f64 = 3.0;

type CellValue = i32;

#[derive(Clone)]
struct Board {
    arr: [[CellValue; BOARD_HEIGHT]; BOARD_WIDTH],
}

impl Board {
    fn randomize(&mut self) {
        for i in 0..BOARD_WIDTH {
            for j in 0..BOARD_HEIGHT {
                let num = rand::thread_rng().gen_range(0, 2);
                self.arr[i][j] = num;
            }
        }
    }

    fn next_gen(&self) -> Self {
        let mut f_next = self.clone();
        for i in 0..BOARD_WIDTH {
            for j in 0..BOARD_HEIGHT {
                f_next.arr[i][j] = rule_life(self, &(i, j));
            }
        }
        f_next
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            arr: [[0; BOARD_HEIGHT]; BOARD_WIDTH],
        }
    }
}

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Cells", [640, 480])
        .exit_on_esc(true)
        .build()
        .unwrap();

    // Initialize
    // Base 1d array
    let mut grid_raw = vec![0; BOARD_WIDTH * BOARD_HEIGHT];

    // Vector of 'width' elements slices
    let mut grid_base: Vec<_> = grid_raw.as_mut_slice().chunks_mut(BOARD_HEIGHT).collect();

    // Final 2d array
    let grid: &mut [&mut [CellValue]] = grid_base.as_mut_slice();

    let mut f: Board = Default::default();
    for i in 0..BOARD_WIDTH {
        f.arr[i].copy_from_slice(&grid[i][0..BOARD_HEIGHT]);
    }
    f.randomize();

    while let Some(event) = window.next() {
        f = f.next_gen();
        window.draw_2d(&event, |context, graphics, _device| {
            clear([0.0, 0.0, 0.0, 0.1], graphics);
            for i in 0..BOARD_WIDTH {
                for j in 0..BOARD_HEIGHT {
                    let cell_val: f32 = f.arr[i][j] as f32;
                    if cell_val != 0.0 {
                        rectangle(
                            [cell_val, cell_val, cell_val, 1.0],
                            [
                                i as f64 * CELL_SIZE,
                                j as f64 * CELL_SIZE,
                                CELL_SIZE,
                                CELL_SIZE,
                            ],
                            context.transform,
                            graphics,
                        );
                    }
                }
            }
        });
    }
}

fn rule_life(board: &Board, pos: &(usize, usize)) -> CellValue {
    let neighborhood: [(i32, i32); 8] = [
        (-1, -1),
        (-1, 0),
        (-1, 1),
        (0, -1),
        (0, 1),
        (1, -1),
        (1, 0),
        (1, 1),
    ];
    let dims: (i32, i32) = (BOARD_WIDTH as i32, BOARD_HEIGHT as i32);
    let pos_i: (i32, i32) = (pos.0 as i32, pos.1 as i32);
    let mut alive_neighbors: i32 = 0;
    for (x1, x2) in &neighborhood {
        let check_pos: (usize, usize) = (
            (pos_i.0 + dims.0 + *x1) as usize,
            (pos_i.1 + dims.1 + *x2) as usize,
        );
        alive_neighbors += board.arr[check_pos.0 % BOARD_WIDTH][check_pos.1 % BOARD_HEIGHT];
    }
    //let alive_neighbors: i32 = neighborhood
    //    .iter()
    //    .map(|(x1, x2)| {
    //        let check_pos: (usize, usize) = (
    //            (pos_i.0 + dims.0 + *x1) as usize,
    //            (pos_i.1 + dims.1 + *x2) as usize,
    //        );
    //        board.arr[check_pos.0 % BOARD_WIDTH][check_pos.1 % BOARD_HEIGHT]
    //    })
    //    .sum();
    if alive_neighbors <= 1 {
        0
    } else if alive_neighbors == 2 {
        board.arr[pos.0][pos.1]
    } else if alive_neighbors == 3 {
        1
    } else {
        0
    }
}
