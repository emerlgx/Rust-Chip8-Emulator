extern crate piston_window;

use std::env;
use std::fs;
use rand::Rng;

use piston_window::*;

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

type OpCode = u16;

#[derive(Clone, Copy)]
struct Machine {
   memory: [u8; 4096],
   gfx: [bool; 64*32],
   v: [u8; 16],
   stack: [u16; 16],
   key: [bool; 16],
   
   i: u16,
   pc: u16,
   sp: u8,
   delay_timer: u8,
   sound_timer: u8,
   draw_flag: bool,
   await_keypress: bool,
   keypress_register: u8
}

const CHIP8_FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];


fn main() {
    // Emulator Stuff
    let args: Vec<String> = env::args().collect();
    let program_dir = &args[1];
    let debug_mode: bool = args.contains(&"debug".to_owned());

    let mut opcode_history: Vec<OpCode> = Vec::new();

    let mut machine = init_machine();
    let program = fs::read(program_dir).expect("Couldn't open the program!");
    machine = load_program(machine, program);

    if debug_mode {
        println!("Debug Mode");
        print_mem(machine);
    }

    let mut paused = false;

    // Display Stuff
    let mut window: PistonWindow =
        WindowSettings::new("Chip 8", [1280, 720])
        .exit_on_esc(true).build().unwrap();

    let mut event_settings = EventSettings::new();
    event_settings.set_ups(500);
    event_settings.set_max_fps(60);
    window.set_event_settings(event_settings);

    let mut glyphs = window.load_font("assets/NotoSans-Regular.ttf").unwrap();
    let sound_indicator: G2dTexture = Texture::from_path(
        &mut window.create_texture_context(),
        &"assets/soundOnWhite.png",
        Flip::None,
        &TextureSettings::new()
    ).unwrap();

    'main: while let Some(event) = window.next() {
        let pc = machine.pc;
        let opcode: OpCode = (machine.memory[pc as usize] as u16) << 8 | machine.memory[(pc+1) as usize] as u16;
        match event {
            Event::Loop(Loop::Update(ref _upd)) => {
                if machine.await_keypress || paused {
                    continue 'main;
                }
                opcode_history.push(opcode);
                if opcode_history.len() > 20 { opcode_history.remove(0); }
                machine = run_cycle(machine);
            },
            Event::Loop(Loop::Render(ref _ren)) => {
                if paused {
                    continue 'main;
                }

                // Display the results
                let opcode_history_ref = &opcode_history;
                window.draw_2d(&event, |context, graphics, device| {
                    clear(BLACK, graphics);
                    // Main Game Display
                    for i in 0..32 {
                        for j in 0..64 {
                            if machine.gfx[(i*64 + j) as usize] {
                                let (x, y) = ((j*16) as f64, (i*16) as f64);
                                rectangle(WHITE, // red
                                    [x, y, 16.0, 16.0],
                                    context.transform,
                                    graphics);
                            }
                        }
                    }

                    // Opcode History
                    for (i, code) in opcode_history_ref.into_iter().enumerate() {
                        let code_output = format!("0x{:0>4X}", code);
                        text::Text::new_color([0.0, 1.0, 0.0, 1.0], 32).draw(
                            &code_output,
                            &mut glyphs,
                            &context.draw_state,
                            context.transform.trans(66.0*16.0, 32.0*(2.0 + i as f64)), graphics
                        ).unwrap();
                    }
                    
                    // Sound Indicator
                    if machine.sound_timer > 0 {
                        image(&sound_indicator, context.transform.trans(64.0, 33.0 * 16.0), graphics);
                    }

                    // Update glyphs before rendering.
                    glyphs.factory.encoder.flush(device);
                });


                //update program counter and timers
                if machine.draw_flag {
                    machine.draw_flag = false;
                }
                if machine.delay_timer > 0 && !machine.await_keypress {
                    machine.delay_timer -= 1;
                }
                if machine.sound_timer > 0 && !machine.await_keypress {
                    machine.sound_timer -= 1;
                }
            },
            Event::Input(ref _inp, _) => {
                if let Some(press_args) = event.press_args() {
                    
                    let pressed_key;
                    match press_args {
                        Button::Keyboard(Key::D1) => pressed_key = 0x1,
                        Button::Keyboard(Key::D2) => pressed_key = 0x2,
                        Button::Keyboard(Key::D3) => pressed_key = 0x3,
                        Button::Keyboard(Key::D4) => pressed_key = 0xC,
                        Button::Keyboard(Key::Q) => pressed_key = 0x4,
                        Button::Keyboard(Key::W) => pressed_key = 0x5,
                        Button::Keyboard(Key::E) => pressed_key = 0x6,
                        Button::Keyboard(Key::R) => pressed_key = 0xD,
                        Button::Keyboard(Key::A) => pressed_key = 0x7,
                        Button::Keyboard(Key::S) => pressed_key = 0x8,
                        Button::Keyboard(Key::D) => pressed_key = 0x9,
                        Button::Keyboard(Key::F) => pressed_key = 0xE,
                        Button::Keyboard(Key::Z) => pressed_key = 0xA,
                        Button::Keyboard(Key::X) => pressed_key = 0x0,
                        Button::Keyboard(Key::C) => pressed_key = 0xB,
                        Button::Keyboard(Key::V) => pressed_key = 0xF,
                        Button::Keyboard(Key::Space) => {
                            paused = !paused;
                            pressed_key = 0x10
                        },
                        _ => pressed_key = 0x10
                    }
                    if pressed_key <= 0xF {
                        machine.key[pressed_key as usize] = true;
                        if machine.await_keypress {
                            machine.v[machine.keypress_register as usize] = pressed_key;
                            machine.await_keypress = false;
                        }
                    }

                }
                if let Some(release_args) = event.release_args() {
                    match release_args {
                        Button::Keyboard(Key::D1) => machine.key[0x1] = false,
                        Button::Keyboard(Key::D2) => machine.key[0x2] = false,
                        Button::Keyboard(Key::D3) => machine.key[0x3] = false,
                        Button::Keyboard(Key::D4) => machine.key[0xC] = false,
                        Button::Keyboard(Key::Q) => machine.key[0x4] = false,
                        Button::Keyboard(Key::W) => machine.key[0x5] = false,
                        Button::Keyboard(Key::E) => machine.key[0x6] = false,
                        Button::Keyboard(Key::R) => machine.key[0xD] = false,
                        Button::Keyboard(Key::A) => machine.key[0x7] = false,
                        Button::Keyboard(Key::S) => machine.key[0x8] = false,
                        Button::Keyboard(Key::D) => machine.key[0x9] = false,
                        Button::Keyboard(Key::F) => machine.key[0xE] = false,
                        Button::Keyboard(Key::Z) => machine.key[0xA] = false,
                        Button::Keyboard(Key::X) => machine.key[0x0] = false,
                        Button::Keyboard(Key::C) => machine.key[0xB] = false,
                        Button::Keyboard(Key::V) => machine.key[0xF] = false,
                        _ => ()
                    }
                }
            },
            _ => {
                //println!("unknown event type");
            }
        }
    }
}

fn init_machine () -> Machine {
    let mut machine = Machine {
        memory: [0; 4096],
        gfx: [false; 64*32],
        v: [0; 16],
        stack: [0; 16],
        key: [false; 16],

        i: 0,
        pc: 0x200,
        sp: 0,
        delay_timer: 0,
        sound_timer: 0,
        draw_flag: false,
        await_keypress: false,
        keypress_register: 0
    };

    for i in 0..80 {
        machine.memory[i as usize] = CHIP8_FONTSET[i as usize];
    }
    return machine;
}

fn load_program (machine: Machine, program: Vec<u8>) -> Machine {
    let mut new_machine = machine.clone();

    for i in 0..program.len() {
        new_machine.memory[i + 512] = program[i];
    }

    return new_machine;
}

fn run_cycle(prev_state: Machine) -> Machine {
    let mut next_state = prev_state.clone();
    let pc = prev_state.pc;
    // get the opcode
    let opcode: OpCode = (prev_state.memory[pc as usize] as u16) << 8 | prev_state.memory[(pc+1) as usize] as u16;


    // execute the opcode
    match opcode & 0xF000 {
        0x0000 => {
            match opcode {
                0x00E0 => {
                    next_state.gfx = [false; 64*32];
                    next_state.draw_flag = true;
                    next_state.pc += 2;
                },
                0x00EE => {
                    next_state.sp -= 1;
                    next_state.pc = next_state.stack[next_state.sp as usize];
                    next_state.pc += 2;
                },
                _ => {
                    println!("Opcode: 0x0NNN");
                    next_state.pc += 2;
                },
                
            }
        },
        0x1000 => {
            next_state.pc = opcode & 0x0FFF;
        },
        0x2000 => {
            next_state.stack[next_state.sp as usize] = next_state.pc;
            next_state.sp += 1;
            next_state.pc = opcode & 0x0FFF;
        },
        0x3000 => {
            let x = ((opcode & 0x0F00) >> 8) as usize;
            let v_test = next_state.v[x];
            if v_test == (opcode & 0x00FF) as u8 {
                next_state.pc += 4;
            } else {
                next_state.pc += 2;
            }
        },
        0x4000 => {
            let x = ((opcode & 0x0F00) >> 8) as usize;
            let v_test = next_state.v[x];
            if v_test != (opcode & 0x00FF) as u8 {
                next_state.pc += 4;
            } else {
                next_state.pc += 2;
            }
        },
        0x5000 => {
            let x = ((opcode & 0x0F00) >> 8) as usize;
            let y = ((opcode & 0x00F0) >> 4) as usize;

            if next_state.v[x] == next_state.v[y] {
                next_state.pc += 4;
            } else {
                next_state.pc += 2;
            }
        },
        0x6000 => {
            let x = ((opcode & 0x0F00) >> 8) as usize;
            next_state.v[x] = (opcode & 0x00FF) as u8;
            next_state.pc += 2;
        },
        0x7000 => {
            let x: usize = ((opcode & 0x0F00) >> 8) as usize;
            next_state.v[x] = next_state.v[x].overflowing_add((opcode & 0x00FF) as u8).0;
            next_state.pc += 2;
        },
        0x8000 => {
            let x: usize = ((opcode & 0x0F00) >> 8) as usize;
            let y: usize = ((opcode & 0x00F0) >> 4) as usize;
            match opcode & 0xF00F {
                0x8000 => {
                    next_state.v[x] = next_state.v[y];
                    next_state.pc += 2;
                },
                0x8001 => {
                    next_state.v[x] = next_state.v[x] | next_state.v[y];
                    next_state.pc += 2;
                },
                0x8002 => {
                    next_state.v[x] = next_state.v[x] & next_state.v[y];
                    next_state.pc += 2;
                },
                0x8003 => {
                    next_state.v[x] = next_state.v[x] ^ next_state.v[y];
                    next_state.pc += 2;
                },
                0x8004 => {
                    let (val, has_overflow) = next_state.v[x].overflowing_add(next_state.v[y]);
                    
                    next_state.v[x] = val;
                    next_state.v[0xF] = if has_overflow {1} else {0};
                    next_state.pc += 2;
                },
                0x8005 => {
                    let (val, has_overflow) = next_state.v[x].overflowing_sub(next_state.v[y]);
                    
                    next_state.v[x] = val;
                    next_state.v[0xF] = if has_overflow {0} else {1};
                    next_state.pc += 2;
                },
                0x8006 => {
                    next_state.v[0xF] = next_state.v[x] & 0x01;
                    next_state.v[x] = next_state.v[x] >> 1;
                    next_state.pc += 2;
                },
                0x8007 => {
                    let (val, has_overflow) = next_state.v[y].overflowing_sub(next_state.v[x]);
                    
                    next_state.v[x] = val;
                    next_state.v[0xF] = if has_overflow {0} else {1};
                    next_state.pc += 2;
                },
                0x800E => {
                    next_state.v[0xF] = next_state.v[x] & 0x80;
                    next_state.v[x] = next_state.v[x] << 1;
                    next_state.pc += 2;
                },
                _ => {
                    println!("Unknown opcode: {:X}", opcode);
                    std::process::exit(0);
                }
            }
        },
        0x9000 => {
            let x: usize = ((opcode & 0x0F00) >> 8) as usize;
            let y: usize = ((opcode & 0x00F0) >> 4) as usize;
            if next_state.v[x] != next_state.v[y] {
                next_state.pc += 4;
            } else {
                next_state.pc += 2;
            }
        },
        0xA000 => {
            next_state.i = opcode & 0x0FFF;
            next_state.pc += 2;
        },
        0xB000 => {
            next_state.pc = (opcode & 0x0FFF) + next_state.v[0] as u16;
        },
        0xC000 => {
            let x: usize = ((opcode & 0x0F00) >> 8) as usize;

            let mut rng = rand::thread_rng();
            let rand_val: u8 = rng.gen_range(0, 255);
            let rand_mask = (opcode & 0x00FF) as u8;

            next_state.v[x] = rand_val & rand_mask;
            next_state.pc += 2;
        },
        0xD000 => {
            let x : u16 = next_state.v[((opcode & 0x0F00) >> 8) as usize] as u16 %64;
            let y : u16 = next_state.v[((opcode & 0x00F0) >> 4) as usize] as u16 %32;

            let height = opcode & 0x000F;

            let mut pixel: u8;
            let i = next_state.i;
            
            next_state.v[0xF] = 0;
            // draw the sprite
            'rows: for yline in 0..height {
                if y + yline > 32 {
                    break 'rows;
                }
                pixel = next_state.memory[(i + yline) as usize];
                'cols: for xline in 0..8 {
                    if x + xline > 64 {
                        break 'cols;
                    }
                    if (pixel & (0x80 >> xline)) != 0 {
                        if next_state.gfx[(x+xline + (y+yline)*64) as usize] == true {
                            next_state.v[0xF] = 1;
                        }
                        next_state.gfx[(x+xline + (y+yline)*64) as usize] = !next_state.gfx[(x+xline + (y+yline)*64) as usize];
                    } 
                }
            }
            next_state.draw_flag = true;
            next_state.pc += 2;
        },
        0xE000 => {
            let x: usize = ((opcode & 0x0F00) >> 8) as usize;
            match opcode & 0xF0FF {
                0xE09E => {
                    if next_state.key[next_state.v[x] as usize] {
                        next_state.pc += 4;
                    } else {
                        next_state.pc += 2;
                    }
                },
                0xE0A1 => {
                    if !next_state.key[next_state.v[x] as usize]  {
                        next_state.pc += 4;
                    } else {
                        next_state.pc += 2;
                    }
                },
                _ => {
                    println!("Unknown opcode: {:X}", opcode);
                    std::process::exit(0);
                }
            }
        },
        0xF000 => {
            let x = ((opcode & 0x0F00) >> 8) as usize;
            match opcode & 0xF0FF {
                0xF007 => {
                    next_state.v[x] = next_state.delay_timer;
                    next_state.pc += 2;
                },
                0xF00A => {
                    next_state.await_keypress = true;
                    next_state.keypress_register = x as u8;
                    next_state.pc += 2;
                },
                0xF015 => {
                    next_state.delay_timer = next_state.v[x];
                    next_state.pc += 2;
                },
                0xF018 => {
                    next_state.sound_timer = next_state.v[x];
                    next_state.pc += 2;
                },
                0xF01E => {
                    let (val, has_overflow) = next_state.i.overflowing_add(next_state.v[x] as u16);
                    next_state.i = val;
                    next_state.v[0xF] = if has_overflow {1} else {0};
                    next_state.pc += 2;
                },
                0xF029 => {
                    next_state.i = next_state.v[x] as u16 * 5;
                    next_state.pc += 2;
                },
                0xF033 => {
                    let i = next_state.i as usize;
                    next_state.memory[i] = next_state.v[x] / 100;
                    next_state.memory[i+1] = (next_state.v[x] / 10) % 10;
                    next_state.memory[i+2] = next_state.v[x] % 10;
                    next_state.pc += 2;
                }
                0xF055 => {
                    for offset in 0..=x {
                        next_state.memory[next_state.i as usize + offset] = next_state.v[offset];
                    }
                    next_state.i += x as u16 + 1;
                    next_state.pc += 2;
                },
                0xF065 => {
                    for offset in 0..=x {
                        next_state.v[offset] = next_state.memory[next_state.i as usize + offset];
                    }
                    next_state.i += x as u16 + 1;
                    next_state.pc += 2;
                },
                _ => {
                    println!("Unknown opcode: {:X}", opcode);
                    std::process::exit(0);
                }
            }
        },
        _ => {
            println!("Unknown opcode: {:X}", opcode);
            std::process::exit(0);
        }
    }
    return next_state;
}

fn print_mem(machine: Machine) {
    let mem = machine.memory;
    
    println!("0x000 to 0x050:");
    for i in 0..80 {
        print!("0x{:0>4X} ", mem[i as usize]);
        if (i+1)%5 == 0 {
            print!("\n");
        }
    }

    println!("0x200 to 0xFFF:");
    for i in 512..4096 {
        print!("0x{:0>4X} ", mem[i as usize]);
        if (i+1)%8 == 0 {
            print!("\n");
        }
    }

}

fn _print_gfx(machine:Machine) {
    let gfx = machine.gfx;

    for i in 0..32 {
        for j in 0..64 {
            print!("{}", if gfx[i*64 + j] {"X"} else {" "});
        }
        print!("\n");
    }
    print!("\n");
}