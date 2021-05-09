use crate::input::Input;
use crate::parser::ast::*;

pub struct Engine {}

impl Engine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&self, query: &AST, input: &mut Input) {
        match query.root {
            NodeKind::VectorSelector(ref selector) => loop {
                let record = match input.take_one().unwrap() {
                    Some(r) => r,
                    None => return,
                };

                let matched = selector
                    .matchers()
                    .iter()
                    .all(|m| match record.label(m.label()) {
                        Some(v) => m.matches(v),
                        None => false,
                    });

                if matched {
                    println!("{:?}", record);
                }
            },
        }
    }
}
