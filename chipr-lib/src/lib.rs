use EmulationError::Unsupported0x0NNN;

use crate::EmulationError::UnknownInstruction;
use crate::Operations::{Operation0x00EE, Operation0x1NNN, Operation0x2NNN, Operation0x3NNN, Operation0x4NNN, Operation0x5XY0};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

const PROGRAM_START : usize = 0x200;
const FONT_START: usize = 0x50;

const MEMORY_SIZE : usize = 4096;
const COLUMNS : usize = 64;
const ROWS : usize = 32;

const DEFAULT_FONT : [u8; 80] = [
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
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

struct Memory {
    raw: [u8; MEMORY_SIZE]
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            raw: [0; MEMORY_SIZE]
        }
    }

    pub fn new_with_program_and_font(program : Option<&[u8]>, font : Option<[u8; 80]>) -> Memory {
        let mut base = program.map_or(Memory::new(), |p| {
            let mut m : Memory = Memory::new();
            p.iter().enumerate().for_each(|(index, byte)| {
                m.raw[index + PROGRAM_START] = *byte;
            });
            m
        });
        font.map(|f| {
           f.iter().enumerate().for_each(|(index, byte)| {
               base.raw[index + FONT_START] = *byte;
           })
        });
        base
    }
}


#[derive(Clone, Copy)]
enum MonochromePixel {
    Black,
    White
}

enum Key {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
    A = 0xA,
    B = 0xB,
    C = 0xC,
    D = 0xD,
    E = 0xE,
    F = 0xF,
}

struct Display {
    pixels: [[MonochromePixel; COLUMNS]; ROWS]
}

impl Display {
    pub fn new() -> Display {
        Display {
            pixels: [[MonochromePixel::Black; COLUMNS]; ROWS]
        }
    }
}

struct Configuration {
    shift_operations_sets_ry_into_rx: bool,
    bnnn_is_bxnn: bool,
}

pub struct Chip8Machine {
    memory: Memory,
    display: Display,
    program_counter: usize,
    index_register: u16,
    stack: Vec<u16>,
    delay_timer: u8,
    sound_timer: u8,
    registers: Registers,
    current_key_pressed : Option<Key>,
    configuration: Configuration
}

#[derive(Clone)]
struct Registers {
    data : [u8; 16]
}

impl Registers {
    pub fn update(&self, index: usize, new_value: u8) -> Registers {
        let mut new = self.clone();
        new.data[index] = new_value;
        new
    }

    pub fn new() -> Registers {
        Registers{
            data: [0; 16]
        }
    }
}

pub fn init_chip8machine(program : Option<&[u8]>) -> Chip8Machine {
    let memory = Memory::new_with_program_and_font(program, Some(DEFAULT_FONT));
    let program_counter: usize = if program.is_some() {
        PROGRAM_START
    } else {
        0
    };

    Chip8Machine {
        memory,
        program_counter,
        display: Display::new(),
        index_register: 0,
        stack: vec![],
        delay_timer: 0,
        sound_timer: 0,
        registers : Registers::new(),
        current_key_pressed: None,
        configuration : Configuration {
            shift_operations_sets_ry_into_rx : false,
            bnnn_is_bxnn: false
        }
    }
}



struct Cycle {
    new_state: Chip8Machine,
    should_update_display: bool
}

enum EmulationError {
    UnknownInstruction(u16),
    Unsupported0x0NNN
}

enum Operations {
    Operation0x00E0,
    Operation0x00EE,
    Operation0x0NNN,

    Operation0x1NNN(u16),
    Operation0x2NNN(u16),
    Operation0x3NNN(u16),
    Operation0x4NNN(u16),

    Operation0x5XY0(u8, u8)
}

struct OpCode {
    raw: u16,
    destructured: (u8, u8, u8, u8)
}

impl OpCode {
    pub fn build(high: u8, low: u8) -> OpCode {
        OpCode {
            raw: ((high << 8) + low) as u16,
            destructured : (
                (high & 0xF0) >> 4,
                high & 0xF,
                (low & 0xF0) >> 4,
                low & 0xF
            )
        }
    }

    pub fn get_nnn(&self) -> u16 {
        self.raw & 0x0FFF
    }
}



impl Chip8Machine {
    fn fetch(&self) -> Result<Operations, EmulationError> {
        let opcode = OpCode::build(self.memory.raw[self.program_counter], self.memory.raw[self.program_counter+1]);
        match opcode.destructured {

            (0, 0, 0xE, 0) => Ok(Operation0x00EE),
            (0, 0, 0xE, 0xE) => Ok(Operation0x00EE),

            (1, _, _, _) => Ok(Operation0x1NNN(opcode.get_nnn())),

            (2, _, _, _) => Ok(Operation0x2NNN(opcode.get_nnn())),

            (3, _, _, _) => Ok(Operation0x3NNN(opcode.get_nnn())),

            (4, _, _, _) => Ok(Operation0x4NNN(opcode.get_nnn())),

            (5, x, y, 0) => Ok(Operation0x5XY0(x, y)),

            (0, _, _, _) => Err(Unsupported0x0NNN),
            _ => Err(UnknownInstruction(opcode.raw))
        }
    }

    pub fn cycle(&self) -> Result<Cycle, EmulationError> {
        let op = self.fetch()?;
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
