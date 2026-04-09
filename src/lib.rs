//! Axiom — Post-Human Programming Language Core
//! Programs are payload trees, not text files. Every value carries confidence.

use std::collections::HashMap;

/// Axiom value — every value in Axiom carries confidence
#[derive(Debug, Clone)]
pub struct AxiomValue {
    pub data: AxiomData,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub enum AxiomData {
    Quant(f64),
    Disc(String),
    Seq(Vec<AxiomValue>),
    Struct(HashMap<String, AxiomValue>),
    Bool(bool),
    Fuzz(f64), // 0.0-1.0 fuzzy boolean
    Null,
}

/// Axiom type system — types are payload shape specifications
#[derive(Debug, Clone)]
pub enum AxiomType {
    Quant { min: f64, max: f64 },
    Disc { options: Vec<String> },
    Seq { element: Box<AxiomType>, max_len: Option<usize> },
    Struct { fields: HashMap<String, AxiomType> },
    Bool,
    Fuzz,
    Any,
}

/// A payload node in an Axiom program tree
#[derive(Debug, Clone)]
pub struct AxiomPayload {
    pub op: String,
    pub inputs: Vec<AxiomValue>,
    pub output_type: AxiomType,
    pub constraints: Vec<Constraint>,
    pub id: u64,
    pub children: Vec<usize>, // child payload indices
}

/// A constraint on a payload
#[derive(Debug, Clone)]
pub struct Constraint {
    pub kind: ConstraintKind,
    pub expression: String,
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub enum ConstraintKind {
    Must,       // hard constraint
    Should,     // soft constraint
    Prefer,     // optimization hint
}

/// An Axiom program — a tree of payloads
#[derive(Debug, Clone)]
pub struct AxiomProgram {
    pub payloads: Vec<AxiomPayload>,
    pub intent: IntentSpec,
    pub confidence_threshold: f64,
}

/// Intent specification — what the program should achieve
#[derive(Debug, Clone)]
pub struct IntentSpec {
    pub goal: String,
    pub hard_constraints: Vec<String>,
    pub soft_constraints: Vec<String>,
    pub domain: String,
}

/// Axiom runtime error
#[derive(Debug, Clone)]
pub enum AxiomError {
    TypeMismatch { expected: String, got: String },
    ConstraintViolation { constraint: String },
    ConfidenceTooLow { required: f64, actual: f64 },
    MissingPayload { id: u64 },
    DivisionByZero,
}

/// The Axiom interpreter
pub struct AxiomRuntime {
    program: AxiomProgram,
    outputs: HashMap<u64, AxiomValue>,
    next_id: u64,
}

impl AxiomRuntime {
    pub fn new(program: AxiomProgram) -> Self {
        Self { program, outputs: HashMap::new(), next_id: 0 }
    }

    /// Execute a payload by id, returns result value
    pub fn execute(&mut self, payload_id: usize) -> Result<AxiomValue, AxiomError> {
        if payload_id >= self.program.payloads.len() {
            return Err(AxiomError::MissingPayload { id: payload_id as u64 });
        }
        let payload = &self.program.payloads[payload_id];

        // Execute children first (topological order assumed)
        let mut child_results = vec![];
        for &child_id in &payload.children {
            let result = self.execute(child_id)?;
            child_results.push(result);
        }

        // Check confidence threshold
        let avg_conf: f64 = if child_results.is_empty() {
            1.0
        } else {
            child_results.iter().map(|v| v.confidence).sum::<f64>() / child_results.len() as f64
        };
        if avg_conf < self.program.confidence_threshold {
            return Err(AxiomError::ConfidenceTooLow {
                required: self.program.confidence_threshold, actual: avg_conf,
            });
        }

        // Execute operation
        let result = self.exec_op(&payload.op, &payload.inputs, &child_results)?;

        // Check hard constraints
        for c in &payload.constraints {
            if c.kind == ConstraintKind::Must {
                // In full impl, evaluate expression against result
                // For now, just track
            }
        }

        // Propagate confidence (Bayesian combination)
        let mut conf = result.confidence;
        for child in &child_results {
            conf = bayesian(conf, child.confidence);
        }

        let final_val = AxiomValue { data: result.data, confidence: conf };
        self.outputs.insert(payload_id as u64, final_val.clone());
        Ok(final_val)
    }

    fn exec_op(&mut self, op: &str, inputs: &[AxiomValue], children: &[AxiomValue]) -> Result<AxiomValue, AxiomError> {
        let all_inputs: Vec<&AxiomValue> = inputs.iter().chain(children.iter()).collect();

        match op {
            "add" => {
                let a = quant_val(all_inputs.get(0))?;
                let b = quant_val(all_inputs.get(1))?;
                Ok(AxiomValue { data: AxiomData::Quant(a + b), confidence: 0.99 })
            }
            "sub" => {
                let a = quant_val(all_inputs.get(0))?;
                let b = quant_val(all_inputs.get(1))?;
                Ok(AxiomValue { data: AxiomData::Quant(a - b), confidence: 0.99 })
            }
            "mul" => {
                let a = quant_val(all_inputs.get(0))?;
                let b = quant_val(all_inputs.get(1))?;
                Ok(AxiomValue { data: AxiomData::Quant(a * b), confidence: 0.99 })
            }
            "div" => {
                let a = quant_val(all_inputs.get(0))?;
                let b = quant_val(all_inputs.get(1))?;
                if b.abs() < 1e-10 { return Err(AxiomError::DivisionByZero); }
                Ok(AxiomValue { data: AxiomData::Quant(a / b), confidence: 0.95 })
            }
            "sort_asc" => {
                let seq = seq_val(all_inputs.get(0))?;
                let mut nums: Vec<f64> = seq.iter().filter_map(|v| quant_val(Some(v)).ok()).collect();
                nums.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let sorted: Vec<AxiomValue> = nums.iter().map(|n| AxiomValue { data: AxiomData::Quant(*n), confidence: 0.99 }).collect();
                Ok(AxiomValue { data: AxiomData::Seq(sorted), confidence: 0.95 })
            }
            "sort_desc" => {
                let seq = seq_val(all_inputs.get(0))?;
                let mut nums: Vec<f64> = seq.iter().filter_map(|v| quant_val(Some(v)).ok()).collect();
                nums.sort_by(|a, b| b.partial_cmp(a).unwrap());
                let sorted: Vec<AxiomValue> = nums.iter().map(|n| AxiomValue { data: AxiomData::Quant(*n), confidence: 0.99 }).collect();
                Ok(AxiomValue { data: AxiomData::Seq(sorted), confidence: 0.95 })
            }
            "filter" => {
                let seq = seq_val(all_inputs.get(0))?;
                let threshold = quant_val(all_inputs.get(1))?;
                let filtered: Vec<AxiomValue> = seq.iter()
                    .filter(|v| quant_val(Some(v)).map(|n| n > threshold).unwrap_or(false))
                    .cloned()
                    .collect();
                Ok(AxiomValue { data: AxiomData::Seq(filtered), confidence: 0.9 })
            }
            "aggregate_sum" => {
                let seq = seq_val(all_inputs.get(0))?;
                let sum: f64 = seq.iter().filter_map(|v| quant_val(Some(v)).ok()).sum();
                Ok(AxiomValue { data: AxiomData::Quant(sum), confidence: 0.95 })
            }
            "measure" => {
                if let Some(v) = all_inputs.get(0) {
                    Ok(AxiomValue { data: AxiomData::Quant(v.confidence), confidence: 1.0 })
                } else {
                    Ok(AxiomValue { data: AxiomData::Quant(0.0), confidence: 0.0 })
                }
            }
            _ => Err(AxiomError::TypeMismatch { expected: format!("known op"), got: op.to_string() }),
        }
    }
}

fn quant_val(v: Option<&AxiomValue>) -> Result<f64, AxiomError> {
    match v {
        Some(AxiomValue { data: AxiomData::Quant(n), .. }) => Ok(*n),
        Some(AxiomValue { data: AxiomData::Fuzz(f), .. }) => Ok(*f),
        _ => Err(AxiomError::TypeMismatch { expected: "Quant".to_string(), got: "other".to_string() }),
    }
}

fn seq_val(v: Option<&AxiomValue>) -> Result<&Vec<AxiomValue>, AxiomError> {
    match v {
        Some(AxiomValue { data: AxiomData::Seq(s), .. }) => Ok(s),
        _ => Err(AxiomError::TypeMismatch { expected: "Seq".to_string(), got: "other".to_string() }),
    }
}

fn bayesian(c1: f64, c2: f64) -> f64 {
    1.0 / (1.0 / c1.max(0.001) + 1.0 / c2.max(0.001))
}

/// Builder for constructing Axiom programs
pub struct AxiomBuilder {
    payloads: Vec<AxiomPayload>,
    intent: IntentSpec,
}

impl AxiomBuilder {
    pub fn new(goal: &str) -> Self {
        Self { payloads: vec![], intent: IntentSpec {
            goal: goal.to_string(), hard_constraints: vec![], soft_constraints: vec![], domain: String::new(),
        }}
    }

    pub fn add_payload(mut self, op: &str, inputs: Vec<AxiomValue>, children: Vec<usize>) -> (Self, usize) {
        let id = self.payloads.len() as u64;
        self.payloads.push(AxiomPayload {
            op: op.to_string(), inputs, output_type: AxiomType::Any,
            constraints: vec![], id, children,
        });
        let idx = self.payloads.len() - 1;
        (self, idx)
    }

    pub fn with_constraint(mut self, payload_idx: usize, kind: ConstraintKind, expr: &str) -> Self {
        if let Some(p) = self.payloads.get_mut(payload_idx) {
            p.constraints.push(Constraint { kind, expression: expr.to_string(), weight: 1.0 });
        }
        self
    }

    pub fn build(self) -> AxiomProgram {
        AxiomProgram { payloads: self.payloads, intent: self.intent, confidence_threshold: 0.5 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quant_value() {
        let v = AxiomValue { data: AxiomData::Quant(42.0), confidence: 0.9 };
        assert_eq!(quant_val(Some(&v)).unwrap(), 42.0);
    }

    #[test]
    fn test_bayesian() {
        assert!((bayesian(0.5, 0.5) - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_add_operation() {
        let program = AxiomBuilder::new("add numbers")
            .add_payload("add", vec![
                AxiomValue { data: AxiomData::Quant(3.0), confidence: 1.0 },
                AxiomValue { data: AxiomData::Quant(4.0), confidence: 1.0 },
            ], vec![]).0.build();
        let mut rt = AxiomRuntime::new(program);
        let result = rt.execute(0).unwrap();
        match result.data {
            AxiomData::Quant(n) => assert!((n - 7.0).abs() < 0.01),
            _ => panic!("expected quant"),
        }
    }

    #[test]
    fn test_sort_desc() {
        let seq = AxiomValue { data: AxiomData::Seq(vec![
            AxiomValue { data: AxiomData::Quant(3.0), confidence: 1.0 },
            AxiomValue { data: AxiomData::Quant(1.0), confidence: 1.0 },
            AxiomValue { data: AxiomData::Quant(5.0), confidence: 1.0 },
        ]), confidence: 1.0 };
        let program = AxiomBuilder::new("sort descending")
            .add_payload("sort_desc", vec![seq], vec![]).0.build();
        let mut rt = AxiomRuntime::new(program);
        let result = rt.execute(0).unwrap();
        match &result.data {
            AxiomData::Seq(items) => {
                let nums: Vec<f64> = items.iter().filter_map(|v| quant_val(Some(v)).ok()).collect();
                assert_eq!(nums, vec![5.0, 3.0, 1.0]);
            }
            _ => panic!("expected seq"),
        }
    }

    #[test]
    fn test_div_by_zero() {
        let program = AxiomBuilder::new("div")
            .add_payload("div", vec![
                AxiomValue { data: AxiomData::Quant(1.0), confidence: 1.0 },
                AxiomValue { data: AxiomData::Quant(0.0), confidence: 1.0 },
            ], vec![]).0.build();
        let mut rt = AxiomRuntime::new(program);
        assert!(rt.execute(0).is_err());
    }

    #[test]
    fn test_builder_with_constraint() {
        let (b, idx) = AxiomBuilder::new("test")
            .add_payload("add", vec![], vec![]);
        let program = b.with_constraint(idx, ConstraintKind::Must, "result > 0").build();
        assert_eq!(program.payloads[idx].constraints.len(), 1);
    }
}
