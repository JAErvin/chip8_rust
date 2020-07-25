use rand::Rng;

const MEM_SIZE: usize = 0x1000;
const ROM_START: usize = 0x200;
pub const ROM_SIZE: usize = MEM_SIZE - ROM_START;
pub const GFX_COLS: usize = 64;
pub const GFX_ROWS: usize = 32;
const FONT_LOC: usize = 0x50;
const FONT_NUM_ROWS: usize = 5;

pub fn coords_to_index(x: u8, y: u8) -> usize {
    (y as usize * GFX_COLS) + x as usize
}
pub fn index_to_coords(i: u16) -> (usize, usize) {
    (
        i as usize % GFX_COLS as usize,   //x, 0-indexed
        (i as usize / GFX_COLS as usize), //y, 0-indexed
    )
}

pub struct CPU {
    opcode: u16, // big-endian
    mem: [u8; MEM_SIZE],
    regs: [u8; 16],                   // named V0..VF
    keys: [bool; 16],                 // true iff key is pressed, from key 0 to key F
    gfx: [bool; GFX_ROWS * GFX_COLS], // pixels, true or false (easier than trying some bit array)
    stack: [u16; 16],                 // stores pc on each jump
    sp: u8,
    i: u16,
    pc: u16,
    delay_timer: u8,
    sound_timer: u8,
    pub ignore_keypress: bool, //hacky workaround

    // memory layout
    // 0x000-0x1FF - Chip 8 interpreter (contains font set in emu)
    // 0x050-0x0A0 - Used for the built in 4x5 pixel font set (0-F)
    // 0x200-0xFFF - most chip8 programs (eti660 chip8 roms start at 0x600)
    // 
    // TODO: Can (maybe?) use virtual addresses,
    //  reducing memory space needed by eliminating 0x0..0x50 and 0xA1..0x1FF

    // gfx layout
    // +--------------------+
    // |(00,00)      (63,00)|
    // |                    |
    // |(00,31)      (63,31)|
    // +--------------------+

    // sprites
    //
    // up to 15 bytes, each byte being a row of pixels
    // sprites are XORed with gfx to turn on/off pixels
    // font sprites for hex digits 0-F are located in the first section of mem
}

impl CPU {
    pub fn new() -> CPU {
        let mut cpu = CPU {
            opcode: 0,
            mem: [0u8; MEM_SIZE],
            regs: [0; 16],
            keys: [false; 16],
            gfx: [false; GFX_COLS * GFX_ROWS],
            stack: [0; 16],
            sp: 0,
            i: 0,
            pc: ROM_START as u16,
            delay_timer: 0,
            sound_timer: 0,
            ignore_keypress: false,
        };
        cpu.load_font();
        cpu
    }

    fn load_font(&mut self) {
        // 16 chars
        const CHARS: [u8; FONT_NUM_ROWS * 16] = [
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
            0xF0, 0x90, 0xF0, 0x90, 0x90, // a
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // b
            0xF0, 0x80, 0x80, 0x80, 0xF0, // c
            0xE0, 0x90, 0x90, 0x90, 0xE0, // d
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // e
            0xF0, 0x80, 0xF0, 0x80, 0x80, // f
        ];
        self.mem[FONT_LOC..FONT_LOC + (FONT_NUM_ROWS * 16)].copy_from_slice(&CHARS);
    }

    fn fetch(&mut self) {
        self.opcode =
            (self.mem[self.pc as usize] as u16) << 8 | (self.mem[(self.pc + 1) as usize] as u16);
        self.pc += 2;
    }

    // helper functions that should help with readability
    // could have been macros, but this will type check

    fn nibble2_usize(&self) -> usize { ((self.opcode & 0xF00) >> 8) as usize }
    fn nibble3_usize(&self) -> usize { ((self.opcode & 0xF0) >> 4) as usize }
    fn nibble2_reg(&mut self) -> &mut u8 { &mut self.regs[self.nibble2_usize()] as &mut u8 }
    fn nibble3_reg(&mut self) -> &mut u8 { &mut self.regs[self.nibble3_usize()] as &mut u8 }
    fn lower_4_val(&self) -> u8 { (self.opcode & 0xF) as u8 }
    fn lower_8_val(&self) -> u8 { (self.opcode & 0xFF) as u8 }
    fn lower_12_val(&self) -> u16 { (self.opcode & 0xFFF) as u16 }
    fn fetch_sprite_row(&self, i: usize) -> [bool; 8] {
        // returns a byte at self.mem[i] as an array of bools
        [
            (self.mem[i] & 0b10000000) == 0b10000000,
            (self.mem[i] & 0b01000000) == 0b01000000,
            (self.mem[i] & 0b00100000) == 0b00100000,
            (self.mem[i] & 0b00010000) == 0b00010000,
            (self.mem[i] & 0b00001000) == 0b00001000,
            (self.mem[i] & 0b00000100) == 0b00000100,
            (self.mem[i] & 0b00000010) == 0b00000010,
            (self.mem[i] & 0b00000001) == 0b00000001,
        ]
    }
    //
    // INSTRUCTIONS
    //
    // split into separate functions in attempt to improve readability of
    // the decoding of opcodes
    //

    fn clear_screen(&mut self) { self.gfx = [false; GFX_ROWS * GFX_COLS]; } //0x00E0
    fn subroutine_return(&mut self) {
        // 0x00EE
        self.sp -= 1; //predecrement operator would be nice here...
        self.pc = self.stack[self.sp as usize];
    }
    fn jump(&mut self) { self.pc = self.lower_12_val(); } // 0x1NNN
    fn subroutine_call(&mut self) {
        // 0x2NNN
        self.stack[self.sp as usize] = self.pc;
        self.pc = self.opcode & 0xFFF;
        self.sp += 1;
    }
    fn skip_if(&mut self) {
        // 0x3XNN
        if *self.nibble2_reg() == self.lower_8_val() {
            self.pc += 2;
        }
    }
    fn skip_if_not(&mut self) {
        // 0x4XNN
        if *self.nibble2_reg() != self.lower_8_val() {
            self.pc += 2;
        }
    }
    fn skip_if_xy_eq(&mut self) {
        // 0x5XY0
        if *self.nibble2_reg() == *self.nibble3_reg() {
            self.pc += 2;
        }
    }
    fn set_immediate(&mut self) { *self.nibble2_reg() = self.lower_8_val(); } //0x6XNN
    fn add_immediate(&mut self) {
        //0x7XNN
        *self.nibble2_reg() = self.nibble2_reg().wrapping_add(self.lower_8_val());
    }
    fn set(&mut self) { *self.nibble2_reg() = *self.nibble3_reg(); } //0x8XY0
    fn or(&mut self) { *self.nibble2_reg() |= *self.nibble3_reg(); } //0x8XY1
    fn and(&mut self) { *self.nibble2_reg() &= *self.nibble3_reg(); } //0x8XY2
    fn xor(&mut self) { *self.nibble2_reg() ^= *self.nibble3_reg(); } //0x8XY3
    fn add(&mut self) {
        //0x8XY4
        let (val, overflow) = self.nibble2_reg().overflowing_add(*self.nibble3_reg());
        self.regs[15] = overflow as u8;
        *self.nibble2_reg() = val;
    }
    fn sub_xy(&mut self) {
        //0x8XY5
        let (val, overflow) = self.nibble2_reg().overflowing_sub(*self.nibble3_reg());
        self.regs[15] = !overflow as u8;
        *self.nibble2_reg() = val;
    }
    fn right_shift(&mut self) {
        //0x8XY6
        self.regs[15] = *self.nibble2_reg() & 0x1;
        //TODO: confirm if logical or arithmetic shift... found conflicting info
        *self.nibble2_reg() >>= 1;
    }
    fn sub_yx(&mut self) {
        //0x8XY7
        let (val, overflow) = self.nibble3_reg().overflowing_sub(*self.nibble2_reg());
        self.regs[15] = !overflow as u8;
        *self.nibble2_reg() = val;
    }
    fn left_shift(&mut self) {
        //0x8XYE
        self.regs[15] = *self.nibble2_reg() >> 7; //only first bit
        *self.nibble2_reg() <<= 1;
    }
    fn skip_if_xy_neq(&mut self) {
        //0x9XY0
        if *self.nibble2_reg() != *self.nibble3_reg() {
            self.pc += 2;
        }
    }
    fn set_i_immediate(&mut self) { self.i = self.lower_12_val(); } //ANNN
    fn jump_offset(&mut self) { self.pc = self.lower_12_val() + self.regs[0] as u16 } //0xBNNN
    fn set_rand(&mut self) {
        //0xCNNN
        let mut rng = rand::thread_rng();
        let rng_val: u8 = rng.gen();
        *self.nibble2_reg() = rng_val & self.lower_8_val();
    }

    fn draw_sprite(&mut self) {
        //std::thread::sleep(Duration::from_millis(1000));
        // opcode = DXYN
        // Draws a sprite at coordinate (VX, VY) that has a width of 8 pixels and a height of N pixels.
        // Each row of 8 pixels is read as bit-coded starting from memory location I; I value doesn’t
        // change after the execution of this instruction. As described above, VF is set to 1 if any
        // pixels are flipped from set to unset when the sprite is drawn, and to 0 if that doesn’t happen.
        // pixels wrap around.
        let vx: u8 = *self.nibble2_reg() % GFX_COLS as u8;
        let vy: u8 = *self.nibble3_reg() % GFX_ROWS as u8;
        let height: u8 = self.lower_4_val();
        let mut ret = false;
        let mut mem_i = self.i as usize; // dont modify i
        for row in vy..vy + height {
            let wrapped_y = row % GFX_ROWS as u8;
            let sprite_row: [bool; 8] = self.fetch_sprite_row(mem_i);
            mem_i += 1; //prep for next row
            let draw_row: [bool; 8] = [
                sprite_row[0] ^ self.gfx[coords_to_index(vx, wrapped_y)],
                sprite_row[1] ^ self.gfx[coords_to_index((vx + 1) % GFX_COLS as u8, wrapped_y)],
                sprite_row[2] ^ self.gfx[coords_to_index((vx + 2) % GFX_COLS as u8, wrapped_y)],
                sprite_row[3] ^ self.gfx[coords_to_index((vx + 3) % GFX_COLS as u8, wrapped_y)],
                sprite_row[4] ^ self.gfx[coords_to_index((vx + 4) % GFX_COLS as u8, wrapped_y)],
                sprite_row[5] ^ self.gfx[coords_to_index((vx + 5) % GFX_COLS as u8, wrapped_y)],
                sprite_row[6] ^ self.gfx[coords_to_index((vx + 6) % GFX_COLS as u8, wrapped_y)],
                sprite_row[7] ^ self.gfx[coords_to_index((vx + 7) % GFX_COLS as u8, wrapped_y)],
            ];
            if !ret {
                for i in 0..draw_row.len() {
                    if !draw_row[i] && sprite_row[i] {
                        ret = true;
                        break;
                    }
                }
            }
            // set the pixels.
            // would look cleaner if gfx implemented as circular array,
            // eg    self.gfx[gfx_i..gfx_i + 8].copy_from_slice(&draw_row);
            // until then, using multiple single assignments instead of
            // branching in the hopes that the compiler will optimize it better
            // than when trying to deal with the branch.
            // ...maybe will actually get around to testing that...
            for col in 0..8 {
                let wrapped_i = coords_to_index((vx + col) % GFX_COLS as u8, wrapped_y);
                self.gfx[wrapped_i] = draw_row[col as usize];
            }
        }
        self.regs[15] = ret as u8;
    }
    fn skip_if_key(&mut self) {
        //0xEX9E
        if self.keys[*self.nibble2_reg() as usize] {
            self.pc += 2;
        }
    }
    fn skip_if_not_key(&mut self) {
        //0xEXA1
        if !self.keys[*self.nibble2_reg() as usize] {
            self.pc += 2;
        }
    }
    fn get_delay(&mut self) { *self.nibble2_reg() = self.delay_timer; } //FX07
    fn get_key(&mut self) {
        //FX0A
        self.pc -= 2; //jump back to this same instruction as (poor) way of blocking
        if self.ignore_keypress {
            return;
        }
        for i in 0..self.keys.len() {
            if self.keys[i] {
                self.pc += 2; //jump to next instruction
                *self.nibble2_reg() = i as u8;
                self.ignore_keypress = true;
                break;
            }
        }
    }
    fn set_delay(&mut self) { self.delay_timer = *self.nibble2_reg(); } //0xFX15
    fn set_sound(&mut self) { self.sound_timer = *self.nibble2_reg(); } //0xFX18
    fn add_i(&mut self) { self.i = self.i.wrapping_add(*self.nibble2_reg() as u16); } //0xFX1E
    fn get_char(&mut self) {
        //0xFX29
        self.i = FONT_LOC as u16 + (*self.nibble2_reg() * FONT_NUM_ROWS as u8) as u16;
    }
    fn store_bcd(&mut self) {
        //0xFX33
        let val: u8 = *self.nibble2_reg();
        self.mem[self.i as usize] = val / 100;
        self.mem[self.i as usize + 1] = (val % 100) / 10;
        self.mem[self.i as usize + 2] = val % 10;
    }
    fn reg_dump(&mut self) {
        //0xFX55
        for reg_num in 0..=self.nibble2_usize() {
            self.mem[self.i as usize + reg_num] = self.regs[reg_num];
        }
    }
    fn reg_load(&mut self) {
        //0xFX65
        for reg_num in 0..=self.nibble2_usize() {
            self.regs[reg_num] = self.mem[self.i as usize + reg_num];
        }
    }

    pub fn execute(&mut self) {
        match self.opcode {
            0x00E0 => self.clear_screen(),
            0x00EE => self.subroutine_return(),
            0x0000..=0x0FFF => self.jump(), //temp. good enough for now?
            0x1000..=0x1FFF => self.jump(),
            0x2000..=0x2FFF => self.subroutine_call(),
            0x3000..=0x3FFF => self.skip_if(),
            0x4000..=0x4FFF => self.skip_if_not(),
            0x5000..=0x5FF0 => self.skip_if_xy_eq(),
            0x6000..=0x6FFF => self.set_immediate(),
            0x7000..=0x7FFF => self.add_immediate(),
            0x8000..=0x8FFF => {
                match self.lower_4_val() {
                    0x0 => self.set(),
                    0x1 => self.or(),
                    0x2 => self.and(),
                    0x3 => self.xor(),
                    0x4 => self.add(),
                    0x5 => self.sub_xy(),
                    0x6 => self.right_shift(),
                    0x7 => self.sub_yx(),
                    0xE => self.left_shift(),
                    _ => panic!("unknown opcode!"),
                }
            }
            0x9000..=0x9FF0 => self.skip_if_xy_neq(),
            0xA000..=0xAFFF => self.set_i_immediate(),
            0xB000..=0xBFFF => self.jump_offset(),
            0xC000..=0xCFFF => self.set_rand(),
            0xD000..=0xDFFF => self.draw_sprite(),
            0xE000..=0xEFFF => {
                match self.lower_8_val() {
                    0x9E => self.skip_if_key(),
                    0xA1 => self.skip_if_not_key(),
                    _ => panic!("unknown opcode!"),
                }
            }
            0xF000..=0xFFFF => {
                match self.lower_8_val() {
                    0x07 => self.get_delay(), 
                    0x0A => self.get_key(), 
                    0x15 => self.set_delay(), 
                    0x18 => self.set_sound(), 
                    0x1E => self.add_i(), 
                    0x29 => self.get_char(), 
                    0x33 => self.store_bcd(), 
                    0x55 => self.reg_dump(), 
                    0x65 => self.reg_load(), 
                    _ => panic!("unknown opcode!"),
                }
            }
            _ => panic!("unknown opcode!"),
        }
    }

    pub fn just_drew(&mut self) -> bool { (self.opcode & 0xF000) >> 12 == 0xD }
    pub fn should_play_sound(&self) -> bool { self.sound_timer > 0 }
    // temp until better method implemented
    pub fn set_key(&mut self, key: usize, state: bool) { self.keys[key] = state; }
    pub fn update_timers(&mut self) {
        self.delay_timer = self.delay_timer.saturating_sub(1);
        self.sound_timer = self.sound_timer.saturating_sub(1);
    }

    pub fn perform_cycle(&mut self) {
        self.fetch();
        self.execute(); //also decodes
    }

    pub fn load_rom(&mut self, rom: &[u8; ROM_SIZE]) {
        self.mem[ROM_START..(ROM_START + rom.len())].copy_from_slice(rom);
    }

    pub fn get_gfx(&self) -> [bool; GFX_ROWS * GFX_COLS] { self.gfx }
}
