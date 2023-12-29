// let's implement an assembler real fast.

use std::{collections::HashMap, io::Write, vec};

use anyhow::{bail, Context, Result};
use cpu::{
    Cpu, ADD, AND, CALL, DIV, DUP, HALT, ISEQ, ISGE, ISGT, JIF, JMP, LOAD, MUL, NOT, OR, POP,
    PRNSTK, PUSH, RET, STORE, SUB,
};
mod cpu;

#[derive(Clone, Debug)]
enum ProgramValue {
    Instruction(i64),
    Value(i64),
    Constant(String, i64),
    FunctionLabel(String),
    Label(String),
}

fn parse_line(line: String) -> Result<Vec<ProgramValue>> {
    // it's a label
    // we'll outline our grammar here.
    let mut split_lines = line.trim().split(' ').filter(|v| !v.is_empty());
    // we'll skip empty lines
    let Some(mut word) = split_lines.next() else {
        return Ok(vec![]);
    };

    word = word.trim();

    // we can define constants
    if is_label(word) {
        match split_lines.next() {
            Some(argument) => {
                let constant = argument
                    .parse::<i64>()
                    .context("Label argument was not number")?;
                return Ok(vec![ProgramValue::Constant(word.to_string(), constant)]);
            }
            // A label with no value will demarcate the next instruction address.
            None => return Ok(vec![ProgramValue::FunctionLabel(word.to_string())]),
        }
    }

    if is_comment(word) {
        return Ok(vec![]);
    }

    match word.to_lowercase().as_str() {
        "push" => {
            let argument = get_labeled_or_unlabled_argument(&mut split_lines)?;
            Ok(vec![ProgramValue::Instruction(PUSH), argument])
        }
        "add" => Ok(vec![ProgramValue::Instruction(ADD)]),
        "halt" => Ok(vec![ProgramValue::Instruction(HALT)]),
        "sub" => Ok(vec![ProgramValue::Instruction(SUB)]),
        "mul" => Ok(vec![ProgramValue::Instruction(MUL)]),
        "div" => Ok(vec![ProgramValue::Instruction(DIV)]),
        "not" => Ok(vec![ProgramValue::Instruction(NOT)]),
        "and" => Ok(vec![ProgramValue::Instruction(AND)]),
        "or" => Ok(vec![ProgramValue::Instruction(OR)]),
        "pop" => Ok(vec![ProgramValue::Instruction(POP)]),
        "dup" => Ok(vec![ProgramValue::Instruction(DUP)]),
        "iseq" => Ok(vec![ProgramValue::Instruction(ISEQ)]),
        "isgt" => Ok(vec![ProgramValue::Instruction(ISGT)]),
        "isge" => Ok(vec![ProgramValue::Instruction(ISGE)]),
        "load" => {
            let argument = get_labeled_or_unlabled_argument(&mut split_lines)?;
            Ok(vec![ProgramValue::Instruction(LOAD), argument])
        }
        "jmp" => {
            let argument = get_labeled_or_unlabled_argument(&mut split_lines)?;
            Ok(vec![ProgramValue::Instruction(JMP), argument])
        }
        "jif" => {
            let argument = get_labeled_or_unlabled_argument(&mut split_lines)?;
            Ok(vec![ProgramValue::Instruction(JIF), argument])
        }
        "store" => {
            let argument = get_labeled_or_unlabled_argument(&mut split_lines)?;
            Ok(vec![ProgramValue::Instruction(STORE), argument])
        }
        "call" => {
            let argument = get_labeled_or_unlabled_argument(&mut split_lines)?;
            Ok(vec![ProgramValue::Instruction(CALL), argument])
        }
        "ret" => Ok(vec![ProgramValue::Instruction(RET)]),
        "prnstk" => Ok(vec![ProgramValue::Instruction(PRNSTK)]),
        other => bail!("Received invalid instruction {other}"),
    }
}

fn get_labeled_or_unlabled_argument<'a, Iter>(iterator: &mut Iter) -> Result<ProgramValue>
where
    Iter: Iterator<Item = &'a str>,
{
    let token = get_token(iterator)?;
    if is_label(token.clone()) {
        Ok(ProgramValue::Label(token))
    } else {
        Ok(ProgramValue::Value(
            token.parse::<i64>().context("Not number")?,
        ))
    }
}

fn is_label<T: Into<String>>(string: T) -> bool {
    string.into().starts_with(':')
}

fn is_comment<T: Into<String>>(string: T) -> bool {
    string.into().starts_with(";;")
}

fn get_token<'a, Iter>(iterator: &mut Iter) -> Result<String>
where
    Iter: Iterator<Item = &'a str>,
{
    match iterator.next() {
        Some(token) => Ok(token.to_string()),
        None => {
            bail!("No token present when required")
        }
    }
}

fn parse_program(program: String) -> Result<Vec<i64>> {
    let mut value_stream = vec![];
    // first grab the lines
    for line in program.lines() {
        let parsed = parse_line(line.to_string())?;
        value_stream.extend(parsed);
    }

    // gather all our constants.
    let mut constants = HashMap::new();
    let mut after_constant_remapping = vec![];
    for value in value_stream.into_iter() {
        if let ProgramValue::Constant(name, value) = value {
            constants.insert(name, value);
        } else {
            after_constant_remapping.push(value);
        }
    }

    // now we convert our function labels into constants
    let mut after_function_labels = vec![];
    let mut instruction_number = 0;
    for value in after_constant_remapping.iter() {
        match value {
            ProgramValue::FunctionLabel(label) => {
                constants.insert(label.to_string(), instruction_number);
            }
            program_value => {
                instruction_number += 1;
                after_function_labels.push(program_value);
            }
        }
    }

    // now rename our constants
    let mut after_renaming = vec![];
    let mut after_label_iter = after_function_labels.into_iter();
    loop {
        let Some(value) = after_label_iter.next() else {
            // token stream complete.
            break;
        };

        // now destructure the labels
        match value {
            ProgramValue::Label(name) => {
                let Some(constant) = constants.get(name) else {
                    bail!("Used undeclared constant {name}")
                };
                after_renaming.push(ProgramValue::Value(*constant));
            }
            program_value => after_renaming.push(program_value.clone()),
        }
    }

    // now everything should be just a stream of instructions and values
    // we can convert to just numbers
    let mut out = vec![];
    for value in after_renaming.into_iter() {
        match value {
            ProgramValue::Instruction(inst) => out.push(inst),
            ProgramValue::Value(val) => out.push(val),
            value => {
                bail!("Invalid value leaked through {value:?}")
            }
        }
    }
    Ok(out)
}

fn emit_bytecode(filename: String, instructions: Vec<i64>) -> Result<()> {
    let mut file = std::fs::File::create(filename).context("Unable to create outfile")?;
    for instruction in instructions.into_iter() {
        file.write(&instruction.to_be_bytes())
            .context("Could not write instruction")?;
    }
    file.flush().context("Could not flush file")?;
    Ok(())
}

fn load_bytecode(filename: String) -> Result<Vec<i64>> {
    let file = std::fs::read(filename).context("Could not open file")?;

    let mut instructions = vec![];
    for chunk in file.as_slice().chunks(8) {
        let buf: [u8; 8] = chunk.try_into().unwrap();
        instructions.push(i64::from_be_bytes(buf));
    }
    Ok(instructions)
}

fn main() {
    let incoming_program =
        std::fs::read_to_string("/Users/patrickcrawford/dev/projects/stackvm/progn")
            .expect("Could not load program");
    println!("loaded program from disk");

    let parsed = match parse_program(incoming_program) {
        Ok(parsed) => parsed,
        Err(err) => panic!("Could not parse program {err:#}"),
    };
    println!("parsed program");

    emit_bytecode("bytecode".to_string(), parsed).expect("Could not emit bytecode");
    println!("Emitted bytecode");

    let bytecode = load_bytecode("bytecode".to_string()).expect("Could not load bytecode");
    println!("loaded bytecode");

    let mut cpu = Cpu::new();
    cpu.load_program(bytecode);
    cpu.run().expect("Could not run program");
    let last_value = cpu
        .get_latest_return_value()
        .expect("Could not get last return value");
    println!("we ran our dumb program and all we got was {last_value}");
}
