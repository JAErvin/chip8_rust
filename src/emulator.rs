use crate::cpu;

use std::time::SystemTime;
//use std::thread;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::WindowCanvas;
use sdl2::rect::Rect;
use std::time::Duration;

const FPS: u64 = 60;
const NANOS_PER_CYCLE: u64 = 1000000000 / FPS;
const SCR_WIDTH: usize = 768;
const SCR_HEIGHT: usize = 1536;
const PADDING: usize = 1;
const PX_WIDTH: usize = 22; //+ 2*padding==24
const PX_HEIGHT: usize = 10; //+ 2*padding==12

const BG_COLOR: Color = Color::RGB(0,0,0);
const FG_COLOR: Color = Color::RGB(128,128,128);

fn select_key(keycode: Keycode) -> Option<usize> {
    return match keycode {
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0xC),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),
        _ => None
    }
    
}



pub struct Emulator {
    cpu: cpu::CPU,
    //sdl_context: sdl2::Sdl,
    //video_subsystem: sdl2::VideoSubsystem,
    //window: sdl2::video::Window,
    canvas: WindowCanvas,
    event_pump: sdl2::EventPump,
}

#[allow(dead_code)]
impl Emulator {
    pub fn new() -> Emulator {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem.window("chip8", SCR_HEIGHT as u32, SCR_WIDTH as u32)
            .position_centered()
            .build()
            .unwrap();
        let canvas: WindowCanvas = window.into_canvas()
            .build()
            .unwrap();
        let event_pump = sdl_context.event_pump().unwrap();
        Emulator{
            cpu: cpu::CPU::new(),
            //sdl_context: sdl_context,
            //video_subsystem: video_subsystem,
            //window: window,
            canvas: canvas,
            event_pump: event_pump,
        }
    }

    fn draw(&mut self) {
        self.canvas.set_draw_color(BG_COLOR);
        self.canvas.clear();
        let mut rects: Vec<Rect> = vec!();
        self.canvas.set_draw_color(FG_COLOR);
        let rects_to_draw = self.cpu.get_gfx();
        for i in 0..rects_to_draw.len() {
            //print!("{}", if rects_to_draw[i] {"T "} else { "F "});
            //if i == cpu::GFX_COLS { println!(""); }
            if rects_to_draw[i] {
                //println!("PIXEL!!");
                let (x, y) = cpu::index_to_coords(i as u16);
                // actual width = 2x padding + px_width
                // actual height = 2x padding + px_height
                rects.push(Rect::new(
                        (PADDING + (x * (PADDING + PX_WIDTH  + PADDING))) as i32,
                        (PADDING + (y * (PADDING + PX_HEIGHT + PADDING))) as i32,
                        (PADDING + PX_WIDTH  + PADDING) as u32,
                        (PADDING + PX_HEIGHT + PADDING) as u32,
                        ));
            }
        }

        self.canvas.fill_rects(&rects).unwrap();
        self.canvas.present();
    }
    
    fn read_input(&mut self) -> bool {
        //returns true if should quit
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit{..} => return true,
                Event::KeyDown{keycode: Some(Keycode::Escape),..} => return true,
                Event::KeyDown{keycode: Some(key),..} => {
                    if let Some(key_num) = select_key(key) {
                        self.cpu.set_key(key_num, true);
                    }
                },
                Event::KeyUp{keycode: Some(key),..} => {
                    if let Some(key_num) = select_key(key) {
                        self.cpu.set_key(key_num, false);
                    }
                },
                _ => {}

            }
        }
        return false;
    }


    pub fn run(&mut self, rom: &[u8; cpu::ROM_SIZE]) {
        self.cpu.load_rom(rom);
        self.draw(); //init
        loop {
            let cycle_start = SystemTime::now();
            if self.read_input() { return; };
            self.cpu.perform_cycle();
            self.draw(); //TODO: Delete
            if self.cpu.just_drew() {
                //println!("just drew");
                //self.draw();
            }
            let sleep_time = Duration::from_nanos(NANOS_PER_CYCLE)
                .checked_sub(SystemTime::now()
                    .duration_since(cycle_start)
                    .unwrap()
                );
            if let Some(pos_sleep_time) = sleep_time {
                std::thread::sleep(pos_sleep_time);
            }
            //std::thread::sleep(Duration::from_millis(2000));
        }
    }
}
