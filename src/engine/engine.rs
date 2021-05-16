use std::rc::Rc;

use crate::input::{Input, Sample};
use crate::parser::ast;

pub struct Engine {}

// Simple use cases (filtration)
//
//     - Requests longer than 500ms
//     duration > 500ms
//
//     - Requests longer than 500ms intermixed with content_length matched by labels
//     duration > 500ms and content_length (but that's an advanced case)
//
//     - Requests bigger than 200 KB
//     content_length > 200
//     content_length > 200 and duration (but that's an advanced case)
//
//
// Advanced use cases (with resampling)
//
//     - RPS per series
//     rate(integral(duration > bool 0)[1s])
//
//     - RPS total
//     sum(rate(integral(duration > bool 0)[1s]))
//
//     - RPS by HTTP method
//     sum(rate(integral(duration > bool 0)[1s])) on "method"
//
//     - Throughput (MB/s) as a moving 5m window
//     rate(integral(content_length / (1024 * 1024))[5m])
//
//     - Request duration distribution
//     TODO: ...
//
// Advanced use cases require defining an evaluation step. I.e. every rate() calculation
// should be reported at some constant frequency (unlike the original samples that may
// appear at random times). Every aggregation such as sum() takes all the series (vertical
// axis) at a give sampling step and combines them. That's how different series are aligned
// in time. And since we define the time alignment, we can start combining instant vectors
// using the original Prometheus rules - by matching labels.

// Time axis is the horizontal one.
// Series axis is the vertical one.
//
// Range-vectors essentialy defines a moving time window.
//
// Horizontal functions accept range-vectors and do the aggregation over the time axis:
//   - rate
//   - increase
//   - delta
//   - <agg>_over_time
//   - ...
//
// Vertical functions accept instant-vectors and do modification of its values:
//   - abs
//   - ceil
//   - exp
//   - log
//   - ...
//
// [some] Operators accept instant-vectos and do the aggregation over the series axis:
//   - sum [on] - group time series
//   - min/max/avg/topk/bottomk
//   - count
//   - ...

impl Engine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&self, query: &ast::AST, input: &mut Input) {
        for sample in self.do_execute(&query.root, input) {
            println!("{:?}", sample);
        }
    }

    fn do_execute(
        &self,
        expr: &ast::Expr,
        input: &mut Input,
    ) -> Box<dyn std::iter::Iterator<Item = Rc<Sample>>> {
        match expr {
            ast::Expr::VectorSelector(ref selector) => {
                Box::new(VectorSelector::new(selector, input.cursor()))
            }
            ast::Expr::UnaryExpr(op, expr) => {
                Box::new(UnaryExpr::new(op, self.do_execute(expr, input)))
            }
        }
    }
}

struct UnaryExpr {
    op: ast::UnaryOp,
    inner: Box<dyn std::iter::Iterator<Item = Rc<Sample>>>,
}

impl UnaryExpr {
    fn new(op: ast::UnaryOp, inner: Box<dyn std::iter::Iterator<Item = Rc<Sample>>>) -> Self {
        UnaryExpr { op, inner }
    }
}

impl std::iter::Iterator for UnaryExpr {
    type Item = Rc<Sample>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(s) => Some(Rc::new(Sample {
                name: s.name.clone(),
                value: match self.op {
                    ast::UnaryOp::Add => s.value,
                    ast::UnaryOp::Sub => -s.value,
                },
                timestamp: s.timestamp,
                labels: s.labels.clone(),
            })),
            None => None,
        }
    }
}

struct VectorSelector {
    selector: ast::VectorSelector,
    inner: Box<dyn std::iter::Iterator<Item = Rc<Sample>>>,
}

impl VectorSelector {
    fn new(
        selector: ast::VectorSelector,
        inner: Box<dyn std::iter::Iterator<Item = Rc<Sample>>>,
    ) -> Self {
        VectorSelector { selector, inner }
    }
}

impl std::iter::Iterator for VectorSelector {
    type Item = Rc<Sample>;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = match self.inner.next() {
            Some(s) => s,
            None => return None,
        };

        match self
            .selector
            .matchers()
            .iter()
            .all(|m| match sample.label(m.label()) {
                Some(v) => m.matches(v),
                None => false,
            }) {
            true => Some(sample),
            false => None,
        }
    }
}
