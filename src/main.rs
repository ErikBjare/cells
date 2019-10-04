extern crate gl;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;
extern crate piston_window;
extern crate rand;
extern crate window;

extern crate palette;
use palette::{Hsv, LinSrgb};

use gl::types::GLuint;
use graphics::image::Image;
use graphics::Transformed;
use opengl_graphics::{GlGraphics, OpenGL, Texture, TextureSettings};
use piston::event_loop::*;
use piston::input::*;
use piston_window::{OpenGLWindow, PistonWindow, Window};
use rand::Rng;
use sdl2_window::Sdl2Window;
use window::WindowSettings;

const BOARD_WIDTH: usize = 300;
const BOARD_HEIGHT: usize = 300;
const CELL_SIZE: f64 = 3.0;

const _NEIGHBORHOOD_NEUMANN: [(i32, i32); 4] = [(-1, 0), (0, -1), (0, 1), (1, 0)];
const NEIGHBORHOOD_MOORE: [(i32, i32); 8] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

type CellValue = i32;

trait Rule {
    fn apply(board: &Board, pos: &(usize, usize)) -> CellValue;
    fn color(board: &Board, val: CellValue) -> [f32; 4];
    fn next_gen(board: &Board) -> Board {
        let mut f_next = board.clone();
        for i in 0..BOARD_WIDTH {
            for j in 0..BOARD_HEIGHT {
                f_next.arr[i][j] = Self::apply(board, &(i, j));
            }
        }
        f_next
    }
}

struct LifeRule {}

impl Rule for LifeRule {
    fn apply(board: &Board, pos: &(usize, usize)) -> CellValue {
        let dims: (i32, i32) = (BOARD_WIDTH as i32, BOARD_HEIGHT as i32);
        let pos_i: (i32, i32) = (pos.0 as i32, pos.1 as i32);
        let mut alive_neighbors: i32 = 0;
        for (x1, x2) in &NEIGHBORHOOD_MOORE {
            let check_pos: (usize, usize) = (
                (pos_i.0 + dims.0 + *x1) as usize,
                (pos_i.1 + dims.1 + *x2) as usize,
            );
            alive_neighbors += board.arr[check_pos.0 % BOARD_WIDTH][check_pos.1 % BOARD_HEIGHT];
        }
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
    fn color(board: &Board, val: CellValue) -> [f32; 4] {
        [val as f32, val as f32, val as f32, 1.0]
    }
}

struct CyclicRule {}

impl Rule for CyclicRule {
    fn apply(board: &Board, pos: &(usize, usize)) -> CellValue {
        let dims: (i32, i32) = (BOARD_WIDTH as i32, BOARD_HEIGHT as i32);
        let pos_i: (i32, i32) = (pos.0 as i32, pos.1 as i32);
        let center_val = board.arr[pos.0][pos.1];
        let mut return_val = center_val;
        for (x1, x2) in &NEIGHBORHOOD_MOORE {
            let check_pos: (usize, usize) = (
                (pos_i.0 + dims.0 + *x1) as usize,
                (pos_i.1 + dims.1 + *x2) as usize,
            );
            if board.arr[check_pos.0 % BOARD_WIDTH][check_pos.1 % BOARD_HEIGHT]
                == (center_val + 1) % board.states
            {
                return_val = (center_val + 1) % board.states;
                break;
            }
        }
        return_val
    }
    fn color(board: &Board, val: CellValue) -> [f32; 4] {
        let c = LinSrgb::from(Hsv::new(
            360.0 * (val as f64) / (board.states as f64),
            1.0,
            1.0,
        ));
        let (r, g, b) = c.into_components();
        [r as f32, g as f32, b as f32, 1.0]
    }
}

#[derive(Clone)]
struct Board {
    arr: [[CellValue; BOARD_HEIGHT]; BOARD_WIDTH],
    states: i32,
}

impl Board {
    fn randomize(&mut self) {
        for i in 0..BOARD_WIDTH {
            for j in 0..BOARD_HEIGHT {
                let num = rand::thread_rng().gen_range(0, self.states);
                self.arr[i][j] = num;
            }
        }
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            arr: [[0; BOARD_HEIGHT]; BOARD_WIDTH],
            states: 8,
        }
    }
}

fn build_fbo(window: &dyn Window) -> (GLuint, Texture) {
    let draw_size = window.size();
    // It would also be possible to create a texture by hand using gl::GenTextures and call
    // gl::TexImage2D with a null pointer for the data argument, which would require another unsafe
    // block but would save this allocation
    let texture_buf = vec![0u8; draw_size.width as usize * draw_size.height as usize];
    let texture = Texture::from_memory_alpha(
        &texture_buf,
        draw_size.width as u32,
        draw_size.height as u32,
        &TextureSettings::new(),
    )
    .expect("texture");

    let fbo;
    unsafe {
        let mut fbos: [GLuint; 1] = [0];
        // Create a Framebuffer Object that we can draw to later
        gl::GenFramebuffers(1, fbos.as_mut_ptr());
        fbo = fbos[0];
        // Switch to it as the active framebuffer
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
        // Set up the framebuffer object so that draws to it will go to the texture
        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0, // draw colors, not depth or stencil data
            gl::TEXTURE_2D,        // the texture's type
            texture.get_id(),
            0,
        ); // mipmap level
    }
    (fbo, texture)
}

type ActiveRule = CyclicRule;

fn main() {
    let opengl = OpenGL::V3_2;
    let window_settings = WindowSettings::new("Cells", [1920, 1080])
        .exit_on_esc(true)
        .graphics_api(opengl);
    let mut window: PistonWindow<Sdl2Window> = window_settings.build().expect("window");
    gl::load_with(|s| window.window.get_proc_address(s) as *const _);

    // Initialize
    let mut board: Board = Default::default();
    board.randomize();

    // Will be used to check diff
    let mut last_gen: Board = board.clone();

    let ref mut gl = GlGraphics::new(opengl);
    let (fbo, texture) = build_fbo(&window);

    let mut events = Events::new(EventSettings::new().lazy(false));
    while let Some(event) = events.next(&mut window) {
        // Computes the next generation
        event.update(|_args| {
            last_gen = board.clone();
            board = ActiveRule::next_gen(&board);
        });

        // This was a massive help: https://stackoverflow.com/questions/47855009/how-do-i-stop-piston-from-making-the-screen-flash-when-i-dont-call-graphicsc
        event.render(|args| {
            // Switch to the texture framebuffer and draw the cursor
            unsafe {
                gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            }
            gl.draw(args.viewport(), |c, g| {
                graphics::rectangle(
                    [0.0, 0.0, 0.0, 0.1],
                    [
                        0.0,
                        0.0,
                        CELL_SIZE * BOARD_WIDTH as f64,
                        CELL_SIZE * BOARD_HEIGHT as f64,
                    ],
                    c.transform,
                    g,
                );
                for i in 0..BOARD_WIDTH {
                    for j in 0..BOARD_HEIGHT {
                        let cell_val = board.arr[i][j];
                        if cell_val != last_gen.arr[i][j] {
                            graphics::rectangle(
                                ActiveRule::color(&board, cell_val),
                                [
                                    i as f64 * CELL_SIZE,
                                    j as f64 * CELL_SIZE,
                                    CELL_SIZE,
                                    CELL_SIZE,
                                ],
                                c.transform,
                                g,
                            );
                        }
                    }
                }
            });

            // Switch to the window framebuffer and draw the texture
            unsafe {
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            }
            gl.draw(args.viewport(), |c, g| {
                graphics::clear([0f32, 0f32, 0f32, 1f32], g);
                // I can't entirely explain this.  We already applied the viewport transform when
                // we were rendering the cursor, so I think the texture is right-side-up for GL,
                // but piston::Image is expecting an image laid out in screen coordinates.
                // Since there is an offset in the viewport transform, the flip has to be applied
                // first, otherwise it would flip across the origin.
                Image::new().draw(
                    &texture,
                    &c.draw_state,
                    c.transform
                        .prepend_transform(graphics::math::scale(-1., 1.)),
                    g,
                );
            });
        });
    }
}
