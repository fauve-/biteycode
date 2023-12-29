use std::collections::HashMap;

use anyhow::{bail, Context, Result};

pub const PUSH: i64 = 1;
pub const HALT: i64 = 3;
pub const ADD: i64 = 4;
pub const SUB: i64 = 5;
pub const MUL: i64 = 6;
pub const DIV: i64 = 7;
pub const NOT: i64 = 8;
pub const AND: i64 = 9;
pub const OR: i64 = 10;
pub const POP: i64 = 11;
pub const DUP: i64 = 12;
pub const ISEQ: i64 = 13;
pub const ISGT: i64 = 14;
pub const ISGE: i64 = 15;
pub const JMP: i64 = 16;
pub const JIF: i64 = 17;
pub const LOAD: i64 = 18;
pub const STORE: i64 = 19;
pub const CALL: i64 = 20;
pub const RET: i64 = 21;
pub const PRNSTK: i64 = 22;

const TRUE: i64 = 1;
const FALSE: i64 = 0;

#[derive(Debug, Clone)]
struct Frame {
    variables: HashMap<i64, i64>,
    return_address: usize,
}

impl Frame {
    fn new(return_address: usize) -> Self {
        Self {
            variables: HashMap::new(),
            return_address,
        }
    }

    // I hate that it gets something by default.
    // my vm will not.
    fn get(&self, key: i64) -> i64 {
        match self.variables.get(&key) {
            Some(val) => *val,
            None => 0,
        }
    }

    fn set(&mut self, key: i64, value: i64) {
        self.variables.insert(key, value);
    }
}

pub struct Cpu {
    program: Vec<i64>,
    frames: Vec<Frame>,
    instruction_pointer: usize,
    stack: Vec<i64>,
    halted: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            stack: vec![],
            instruction_pointer: 0,
            halted: false,
            program: vec![],
            frames: vec![Frame::new(0)],
        }
    }

    pub fn load_program(&mut self, program: Vec<i64>) {
        self.program = program;
    }

    pub fn step(&mut self, instruction: i64) -> Result<()> {
        if self.halted {
            // Probably better to develop our own error type.
            bail!("Processing instruction while halted")
        }

        match instruction {
            HALT => {
                self.halted = true;
            }
            PUSH => {
                // get immediate value
                let next_word = self.get_next_word()?;
                self.stack.push(next_word);
            }
            ADD | SUB | MUL | DIV | AND | OR | ISEQ | ISGT | ISGE => {
                let val = self.binary_op(instruction)?;
                self.push_stack(val);
            }
            NOT => {
                let val = self.pop_stack()?;
                if Self::i64_to_bool(val) {
                    self.push_stack(0);
                } else {
                    self.push_stack(1);
                }
            }
            POP => {
                let _ = self.pop_stack()?;
            }
            DUP => {
                let val = self.pop_stack()?;
                // we can just copy because it's a i64.
                let copied = val;
                self.push_stack(val);
                self.push_stack(copied);
            }
            JMP => {
                let target_address = self.get_next_word()?;
                // we should really trap if the number is negative.
                self.instruction_pointer = target_address as usize;
            }
            JIF => {
                let conditional_val = self.pop_stack()?;
                let target_address = self.get_next_word()?;
                if Self::i64_to_bool(conditional_val) {
                    self.instruction_pointer = target_address as usize;
                }
            }
            LOAD => {
                let variable_identifier = self.get_next_word()?;
                let val = self.get_current_frame().get(variable_identifier);
                self.push_stack(val);
            }
            STORE => {
                let variable_identifier = self.get_next_word()?;
                let val = self.pop_stack()?;
                self.get_current_frame().set(variable_identifier, val);
            }
            CALL => {
                let target_address = self.get_next_word()?;
                self.frames.push(Frame::new(self.instruction_pointer));
                self.instruction_pointer = target_address as usize;
            }
            RET => {
                let target_address = self.get_current_frame().return_address;
                self.frames.pop();
                self.instruction_pointer = target_address;
            }
            PRNSTK => {
                println!("{:?}", self.get_current_frame());
                println!("{:?}", self.stack);
            }
            instruction => {
                bail!("Received invalid instruction {instruction}")
            }
        }

        Ok(())
    }

    fn get_current_frame(&mut self) -> &mut Frame {
        // there will always be one frame.
        self.frames.last_mut().unwrap()
    }

    fn binary_op(&mut self, instruction: i64) -> Result<i64> {
        // remember it's reverse polish.
        let right = self.pop_stack()?;
        let left = self.pop_stack()?;

        let val = match instruction {
            ADD => left + right,
            SUB => left - right,
            MUL => left * right,
            DIV => left / right,
            ISEQ => {
                if left == right {
                    TRUE
                } else {
                    FALSE
                }
            }
            ISGT => {
                if left > right {
                    TRUE
                } else {
                    FALSE
                }
            }
            ISGE => {
                if left >= right {
                    TRUE
                } else {
                    FALSE
                }
            }
            AND | OR => {
                let left = Self::i64_to_bool(left);
                let right = Self::i64_to_bool(right);
                match instruction {
                    AND => {
                        if left && right {
                            1
                        } else {
                            0
                        }
                    }
                    OR => {
                        if left || right {
                            1
                        } else {
                            0
                        }
                    }
                    instruction => {
                        bail!("Received invalid instruction {instruction}")
                    }
                }
            }
            instruction => {
                bail!("Received invalid instruction {instruction}")
            }
        };
        Ok(val)
    }

    fn i64_to_bool(val: i64) -> bool {
        val != 0
    }

    fn push_stack(&mut self, val: i64) {
        self.stack.push(val)
    }

    fn pop_stack(&mut self) -> Result<i64> {
        match self.stack.pop() {
            Some(val) => Ok(val),
            None => bail!("Tried to pop empty stack."),
        }
    }

    pub fn get_latest_return_value(&mut self) -> Result<i64> {
        self.pop_stack()
    }

    fn get_next_word(&mut self) -> Result<i64> {
        let word = self.program.get(self.instruction_pointer).copied();
        self.instruction_pointer += 1;
        match word {
            Some(word) => Ok(word),
            None => bail!("Program tried to load out of bounds word."),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        if self.program.is_empty() {
            self.halted = true;
            bail!("Loaded empty program")
        }

        loop {
            if self.halted {
                break;
            }

            let instruction = self.get_next_word()?;
            self.step(instruction)
                .context("Unable to execute program.")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn add_two() {
        let program = vec![PUSH, 42, PUSH, 42, ADD, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(84, val);
    }

    #[test]
    fn sub_two() {
        let program = vec![PUSH, 42, PUSH, 42, SUB, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(0, val);
    }

    #[test]
    fn mul_two() {
        let program = vec![PUSH, 42, PUSH, 42, MUL, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(1764, val);
    }

    #[test]
    fn div_two() {
        let program = vec![PUSH, 4, PUSH, 2, DIV, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(2, val);
    }

    #[test]
    fn not() {
        let program = vec![PUSH, 1, NOT, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(0, val);
    }

    #[test]
    fn and() {
        let program = vec![PUSH, 1, PUSH, 2, AND, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(1, val);
    }

    #[test]
    fn or() {
        let program = vec![PUSH, 1, PUSH, 0, OR, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(1, val);
    }

    #[test]
    fn dup() {
        let program = vec![PUSH, 1, DUP, ADD, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(2, val);
    }

    #[test]
    fn is_eq() {
        let program = vec![PUSH, 1, PUSH, 1, ISEQ, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(1, val);
    }

    #[test]
    fn is_gt() {
        let program = vec![PUSH, 2, PUSH, 1, ISGT, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(1, val);
    }

    #[test]
    fn is_gte() {
        let program = vec![PUSH, 2, PUSH, 1, ISGE, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(1, val);

        let program = vec![PUSH, 1, PUSH, 1, ISGE, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(1, val);
    }

    #[test]
    fn jmp() {
        let program = vec![JMP, 5, PUSH, 420, HALT, JMP, 2];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
    }

    #[test]
    fn jif() {
        let program = vec![PUSH, 1, JIF, 5, POP, PUSH, 0, JIF, 4, PUSH, 420, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(420, val)
    }

    #[test]
    fn load() {
        let program = vec![LOAD, 0, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(0, val)
    }

    #[test]
    fn store() {
        let program = vec![PUSH, 42, STORE, 0, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.get_current_frame().get(0);
        assert_eq!(42, val)
    }

    #[test]
    fn load_and_store() {
        let program = vec![PUSH, 42, STORE, 0, LOAD, 0, HALT];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(42, val)
    }

    #[test]
    fn bigger_program() {
        // we're just really checking if the program halts.
        let program = vec![
            // Init a with "6"
            PUSH, 6, STORE, 0, // Init b with "4"
            PUSH, 4, STORE, 1, // Load a and b into the stack
            LOAD, 0, // Stack contains a
            LOAD, 1,    // Stack contains a, b
            ISGT, // Stack contains a > b
            JIF, 21, // This is the "else" path
            LOAD, 1, // Stack contains b
            STORE, 2, // Set c to the stack head, meaning c = b
            JMP, 25, // This is the "if" path, and this is the address 21
            LOAD, 0, // Stack contains a
            STORE, 2, // Set c to the stack head, meaning c = a
            // Done; this is address 25
            HALT,
        ];

        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.get_current_frame().get(2);
        assert_eq!(val, 6);
    }

    #[test]
    fn add_multiply_without_multiplying() {
        let program = vec![
            // Init a with "6"
            PUSH, 6, STORE, 0, // Init b with "4"
            PUSH, 4, STORE, 1, // Init total to 0
            PUSH, 0, STORE, 2, // While part
            // Here is address 12
            LOAD, 1, // Stack contains b
            PUSH, 1,    // Stack contains b, 1
            ISGE, // Stack contains b >= 1
            NOT,  // Stack contains b < 1
            JIF, 36, // 36 is the address of the HALT label
            // Inner loop part
            LOAD, 0, // Stack contains a
            LOAD, 2,   // Stack contains a, total
            ADD, // Stack contains a + total
            STORE, 2, // Save in total, meaning total = a + total
            LOAD, 1, // Stack contains b
            PUSH, 1,   // Stack contains b, 1
            SUB, // Stack contains b - 1
            STORE, 1, // Save in b, meaning b = b - 1
            JMP, 12, // Go back to the start of the loop
            HALT,
        ];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.get_current_frame().get(2);
        assert_eq!(24, val);
    }

    #[test]
    fn funcall_no_args_return() {
        let program = vec![CALL, 3, HALT, RET];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        assert!(cpu.stack.is_empty());
    }

    #[test]
    fn funcall_returns_no_arguments_int_return() {
        let program = vec![CALL, 3, HALT, PUSH, 7, RET];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(7, val)
    }

    #[test]
    fn doubles_given_argument() {
        let program = vec![PUSH, 3, CALL, 5, HALT, PUSH, 2, MUL, RET];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(6, val)
    }

    #[test]
    fn maximum() {
        let program = vec![
            PUSH, 6, // Push the first argument
            PUSH, 4, // Push the second argument
            CALL, 7,    // Call "max"
            HALT, // Here is address 7, the start of "max" function
            STORE, 1, // Store b in local variable 1; the stack now contains [a]
            STORE, 0, // Store a in local variable 0; the stack is now empty
            LOAD, 0, // The stack now contains [a]
            LOAD, 1,    // The stack now contains [a, b]
            ISGE, // The stack now contains [a > b]
            JIF, 21, // If the top of the stack is true (a > b), jump to the "if" path
            LOAD, 1,   // "else" path: load b on the stack
            RET, // Here is address 23
            LOAD, 0, // "if" path: load a on the stack
            RET,
        ];
        let mut cpu = Cpu::new();
        cpu.load_program(program);
        cpu.run().unwrap();
        let val = cpu.pop_stack().unwrap();
        assert_eq!(6, val)
    }
}
