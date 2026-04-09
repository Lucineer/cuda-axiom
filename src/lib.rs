//! Axiom — Post-Human Programming Language Core
//! 
//! Axiom programs are payload trees, not text files. Every value carries confidence.
//! The compiler turns intent into a payload tree, the runtime executes it.
//!
//! Types: Quant(f64), Disc(String), Seq(Vec<Value>), Struct(HashMap), Bool, Fuzz(0-1), Null
//! Operations: arithmetic, list, string, aggregation, structural, probabilistic

use std::collections::HashMap;
use std::fmt;

// ============================================================
// CORE VALUE TYPE
// ============================================================

/// Every value in Axiom carries confidence
#[derive(Debug, Clone)]
pub struct Value {
    pub data: DataType,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub enum DataType {
    Quant(f64),
    Disc(String),
    Seq(Vec<Value>),
    Struct(HashMap<String, Value>),
    Bool(bool),
    Fuzz(f64),
    Null,
}

impl Value {
    pub fn quant(n: f64) -> Self { Value { data: DataType::Quant(n), confidence: 1.0 } }
    pub fn quant_c(n: f64, c: f64) -> Self { Value { data: DataType::Quant(n), confidence: c.clamp(0.0,1.0) } }
    pub fn disc(s: &str) -> Self { Value { data: DataType::Disc(s.to_string()), confidence: 1.0 } }
    pub fn bool_(b: bool) -> Self { Value { data: DataType::Bool(b), confidence: 1.0 } }
    pub fn fuzz(f: f64) -> Self { Value { data: DataType::Fuzz(f.clamp(0.0,1.0)), confidence: 1.0 } }
    pub fn seq(items: Vec<Value>) -> Self { Value { data: DataType::Seq(items), confidence: 1.0 } }
    pub fn null() -> Self { Value { data: DataType::Null, confidence: 0.0 } }
    pub fn is_null(&self) -> bool { matches!(self.data, DataType::Null) }
    pub fn is_quant(&self) -> bool { matches!(self.data, DataType::Quant(_)) }
    pub fn is_disc(&self) -> bool { matches!(self.data, DataType::Disc(_)) }
    pub fn is_seq(&self) -> bool { matches!(self.data, DataType::Seq(_)) }
    pub fn is_struct(&self) -> bool { matches!(self.data, DataType::Struct(_)) }
    pub fn is_bool(&self) -> bool { matches!(self.data, DataType::Bool(_)) }
    pub fn is_fuzz(&self) -> bool { matches!(self.data, DataType::Fuzz(_)) }
    pub fn as_f64(&self) -> Option<f64> {
        match &self.data {
            DataType::Quant(n) => Some(*n),
            DataType::Fuzz(f) => Some(*f),
            DataType::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match &self.data { DataType::Disc(s) => Some(s), _ => None }
    }
    pub fn as_vec(&self) -> Option<&Vec<Value>> {
        match &self.data { DataType::Seq(v) => Some(v), _ => None }
    }
    pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
        match &self.data { DataType::Struct(m) => Some(m), _ => None }
    }
    pub fn len(&self) -> usize {
        match &self.data {
            DataType::Seq(v) => v.len(),
            DataType::Disc(s) => s.len(),
            DataType::Struct(m) => m.len(),
            _ => 0,
        }
    }
    pub fn with_confidence(mut self, c: f64) -> Self { self.confidence = c.clamp(0.0,1.0); self }
    pub fn merge_confidence(self, other: &Value) -> Self {
        self.with_confidence(bayesian(self.confidence, other.confidence))
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.data {
            DataType::Quant(n) => write!(f, "{:.4} (c={:.2})", n, self.confidence),
            DataType::Disc(s) => write!(f, "'{}' (c={:.2})", s, self.confidence),
            DataType::Seq(v) => write!(f, "[{} items] (c={:.2})", v.len(), self.confidence),
            DataType::Struct(m) => write!(f, "{{{} fields}} (c={:.2})", m.len(), self.confidence),
            DataType::Bool(b) => write!(f, "{} (c={:.2})", b, self.confidence),
            DataType::Fuzz(fuzz) => write!(f, "~{:.2} (c={:.2})", fuzz, self.confidence),
            DataType::Null => write!(f, "null"),
        }
    }
}

fn bayesian(c1: f64, c2: f64) -> f64 {
    let c1 = c1.max(0.001);
    let c2 = c2.max(0.001);
    1.0 / (1.0/c1 + 1.0/c2)
}

// ============================================================
// TYPE SYSTEM
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum AxiomType {
    Quant { min: f64, max: f64 },
    Disc { max_len: Option<usize> },
    Seq { element: Box<AxiomType>, max_len: Option<usize> },
    Struct { fields: HashMap<String, AxiomType> },
    Bool,
    Fuzz,
    Any,
}

impl AxiomType {
    pub fn check(&self, val: &Value) -> TypeResult {
        match self {
            AxiomType::Any => TypeResult::Pass,
            AxiomType::Quant { min, max } => match val.as_f64() {
                Some(n) if n >= *min && n <= *max => TypeResult::Pass,
                Some(n) => TypeResult::Fail(format!("Quant {} not in [{}, {}]", n, min, max)),
                None => TypeResult::Fail("Expected Quant".to_string()),
            },
            AxiomType::Disc { max_len } => match val.as_str() {
                Some(s) if max_len.map_or(true, |m| s.len() <= m) => TypeResult::Pass,
                Some(_) => TypeResult::Fail("Disc too long".to_string()),
                None => TypeResult::Fail("Expected Disc".to_string()),
            },
            AxiomType::Bool => if val.is_bool() { TypeResult::Pass } else { TypeResult::Fail("Expected Bool".to_string()) },
            AxiomType::Fuzz => if val.is_fuzz() { TypeResult::Pass } else { TypeResult::Fail("Expected Fuzz".to_string()) },
            AxiomType::Seq { element, max_len } => match val.as_vec() {
                Some(v) if max_len.map_or(true, |m| v.len() <= m) => {
                    for (i, item) in v.iter().enumerate() {
                        if let TypeResult::Fail(e) = element.check(item) {
                            return TypeResult::Fail(format!("Seq[{}]: {}", i, e));
                        }
                    }
                    TypeResult::Pass
                }
                Some(_) => TypeResult::Fail("Seq too long".to_string()),
                None => TypeResult::Fail("Expected Seq".to_string()),
            },
            AxiomType::Struct { fields } => match val.as_map() {
                Some(m) => {
                    for (name, ftype) in fields {
                        match m.get(name) {
                            Some(v) => if let TypeResult::Fail(e) = ftype.check(v) {
                                return TypeResult::Fail(format!("{}.{}", name, e));
                            },
                            None => return TypeResult::Fail(format!("Missing field: {}", name)),
                        }
                    }
                    TypeResult::Pass
                }
                None => TypeResult::Fail("Expected Struct".to_string()),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum TypeResult {
    Pass,
    Fail(String),
}

// ============================================================
// CONSTRAINTS
// ============================================================

#[derive(Debug, Clone)]
pub struct Constraint {
    pub kind: ConstraintKind,
    pub expression: String,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintKind {
    Must,
    Should,
    Prefer,
}

// ============================================================
// OPCODES — the operation set
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Opcode {
    // Arithmetic
    Add, Sub, Mul, Div, Mod, Pow, Neg, Abs,
    // Comparison
    Eq, Neq, Lt, Lte, Gt, Gte,
    // List
    SortAsc, SortDesc, Reverse, First, Last, Rest, Count,
    Filter, Map, Reduce, Flatten, Unique, Slice, Zip, Append,
    // String
    Upper, Lower, Strip, Split, Join, Replace, Contains, Len,
    StartsWith, EndsWith, Substr,
    // Struct
    Get, Keys, Values, Merge, HasKey,
    // Aggregation
    Sum, Mean, Min, Max, Median, Variance, StdDev,
    // Logic
    And, Or, Not, Xor,
    // Type
    TypeOf, IsNull, IsQuant, IsDisc,
    // Probabilistic
    WithConfidence, MeasureConfidence, ClampConfidence,
    FuzzFromThreshold, FuzzCombine, FuzzNot,
    // Control
    Nop,
}

/// Execute an opcode against inputs
pub fn exec(opcode: &Opcode, inputs: &[Value]) -> ExecResult {
    let conf = inputs.iter().map(|v| v.confidence).fold(1.0, |acc, c| bayesian(acc, c));
    
    match opcode {
        // === ARITHMETIC ===
        Opcode::Add => binop_f64(inputs, |a,b| a + b, conf),
        Opcode::Sub => binop_f64(inputs, |a,b| a - b, conf),
        Opcode::Mul => binop_f64(inputs, |a,b| a * b, conf),
        Opcode::Div => {
            if let (Some(a), Some(b)) = (inputs.get(0).and_then(|v| v.as_f64()), inputs.get(1).and_then(|v| v.as_f64())) {
                if b.abs() < 1e-10 { return ExecResult::Error("Division by zero".to_string()); }
                return ExecResult::Ok(Value::quant_c(a / b, conf * 0.98));
            }
            ExecResult::Error("Need two numbers".to_string())
        }
        Opcode::Mod => {
            if let (Some(a), Some(b)) = (inputs.get(0).and_then(|v| v.as_f64()), inputs.get(1).and_then(|v| v.as_f64())) {
                if b.abs() < 1e-10 { return ExecResult::Error("Mod by zero".to_string()); }
                return ExecResult::Ok(Value::quant_c(a % b, conf));
            }
            ExecResult::Error("Need two numbers".to_string())
        }
        Opcode::Pow => {
            if let (Some(a), Some(b)) = (inputs.get(0).and_then(|v| v.as_f64()), inputs.get(1).and_then(|v| v.as_f64())) {
                match a.powf(b) {
                    result if result.is_finite() => ExecResult::Ok(Value::quant_c(result, conf * 0.97)),
                    _ => ExecResult::Error("Overflow".to_string()),
                }
            } else { ExecResult::Error("Need two numbers".to_string()) }
        }
        Opcode::Neg => unop_f64(inputs, |n| -n, conf),
        Opcode::Abs => unop_f64(inputs, |n| n.abs(), conf),
        
        // === COMPARISON ===
        Opcode::Eq => {
            if inputs.len() >= 2 {
                let eq = inputs[0].data == inputs[1].data;
                return ExecResult::Ok(Value::bool_(eq).with_confidence(conf));
            }
            ExecResult::Error("Need two values".to_string())
        }
        Opcode::Neq => {
            if inputs.len() >= 2 {
                return ExecResult::Ok(Value::bool_(inputs[0].data != inputs[1].data).with_confidence(conf));
            }
            ExecResult::Error("Need two values".to_string())
        }
        Opcode::Lt => cmpop(inputs, |a,b| a < b, conf),
        Opcode::Lte => cmpop(inputs, |a,b| a <= b, conf),
        Opcode::Gt => cmpop(inputs, |a,b| a > b, conf),
        Opcode::Gte => cmpop(inputs, |a,b| a >= b, conf),
        
        // === LIST ===
        Opcode::SortAsc => listop(inputs, |items| {
            let mut nums: Vec<f64> = items.iter().filter_map(|v| v.as_f64()).collect();
            nums.sort_by(|a,b| a.partial_cmp(b).unwrap());
            nums.iter().map(|n| Value::quant_c(*n, conf)).collect()
        }),
        Opcode::SortDesc => listop(inputs, |items| {
            let mut nums: Vec<f64> = items.iter().filter_map(|v| v.as_f64()).collect();
            nums.sort_by(|a,b| b.partial_cmp(a).unwrap());
            nums.iter().map(|n| Value::quant_c(*n, conf)).collect()
        }),
        Opcode::Reverse => listop(inputs, |items| items.iter().rev().cloned().collect()),
        Opcode::First => {
            if let Some(v) = inputs.get(0).and_then(|v| v.as_vec()).and_then(|v| v.first()) {
                ExecResult::Ok(v.clone())
            } else { ExecResult::Error("Empty or not a seq".to_string()) }
        }
        Opcode::Last => {
            if let Some(v) = inputs.get(0).and_then(|v| v.as_vec()).and_then(|v| v.last()) {
                ExecResult::Ok(v.clone())
            } else { ExecResult::Error("Empty or not a seq".to_string()) }
        }
        Opcode::Rest => {
            if let Some(v) = inputs.get(0).and_then(|v| v.as_vec()) {
                if v.len() > 1 { return ExecResult::Ok(Value::seq(v[1..].to_vec()).with_confidence(conf)); }
            }
            ExecResult::Ok(Value::seq(vec![]))
        }
        Opcode::Count => {
            if let Some(v) = inputs.get(0) {
                return ExecResult::Ok(Value::quant(v.len() as f64).with_confidence(conf));
            }
            ExecResult::Error("Need a value".to_string())
        }
        Opcode::Filter => {
            if let (Some(list), Some(threshold)) = (inputs.get(0).and_then(|v| v.as_vec()), inputs.get(1).and_then(|v| v.as_f64())) {
                let filtered: Vec<Value> = list.iter()
                    .filter(|v| v.as_f64().map_or(false, |n| n > threshold))
                    .cloned().collect();
                return ExecResult::Ok(Value::seq(filtered).with_confidence(conf * 0.95));
            }
            ExecResult::Error("Need seq and threshold".to_string())
        }
        Opcode::Flatten => {
            if let Some(list) = inputs.get(0).and_then(|v| v.as_vec()) {
                let mut flat = vec![];
                for item in list {
                    if let Some(inner) = item.as_vec() { flat.extend(inner.iter().cloned()); }
                    else { flat.push(item.clone()); }
                }
                return ExecResult::Ok(Value::seq(flat).with_confidence(conf * 0.98));
            }
            ExecResult::Error("Need a seq".to_string())
        }
        Opcode::Unique => {
            if let Some(list) = inputs.get(0).and_then(|v| v.as_vec()) {
                let mut seen: Vec<String> = vec![];
                let mut result = vec![];
                for item in list {
                    let key = format!("{:?}", item.data);
                    if !seen.contains(&key) { seen.push(key); result.push(item.clone()); }
                }
                return ExecResult::Ok(Value::seq(result).with_confidence(conf * 0.99));
            }
            ExecResult::Error("Need a seq".to_string())
        }
        Opcode::Slice => {
            if let (Some(list), Some(start), Some(end)) = (
                inputs.get(0).and_then(|v| v.as_vec()),
                inputs.get(1).and_then(|v| v.as_f64().map(|n| n as usize)),
                inputs.get(2).and_then(|v| v.as_f64().map(|n| n as usize)),
            ) {
                let end = end.min(list.len());
                let start = start.min(end);
                return ExecResult::Ok(Value::seq(list[start..end].to_vec()).with_confidence(conf));
            }
            ExecResult::Error("Need seq, start, end".to_string())
        }
        Opcode::Append => {
            if let (Some(mut list), Some(item)) = (inputs.get(0).and_then(|v| v.as_vec().cloned()), inputs.get(1)) {
                if let DataType::Seq(ref mut v) = list { v.push(item.clone()); }
                return ExecResult::Ok(Value::seq(list.iter().cloned().collect()).with_confidence(conf));
            }
            ExecResult::Error("Need seq and item".to_string())
        }
        Opcode::Map => ExecResult::Error("Map requires lambda (not yet implemented)".to_string()),
        Opcode::Reduce => ExecResult::Error("Reduce requires lambda (not yet implemented)".to_string()),
        Opcode::Zip => {
            if let (Some(a), Some(b)) = (inputs.get(0).and_then(|v| v.as_vec()), inputs.get(1).and_then(|v| v.as_vec())) {
                let zipped: Vec<Value> = a.iter().zip(b.iter()).map(|(x,y)| {
                    Value::disc(&format!("{:?},{:?}", x.data, y.data)).with_confidence(conf)
                }).collect();
                return ExecResult::Ok(Value::seq(zipped));
            }
            ExecResult::Error("Need two seqs".to_string())
        }
        
        // === STRING ===
        Opcode::Upper => strop(inputs, |s| s.to_uppercase()),
        Opcode::Lower => strop(inputs, |s| s.to_lowercase()),
        Opcode::Strip => strop(inputs, |s| s.trim().to_string()),
        Opcode::Split => {
            if let (Some(s), Some(sep)) = (inputs.get(0).and_then(|v| v.as_str()), inputs.get(1).and_then(|v| v.as_str())) {
                let parts: Vec<Value> = s.split(sep).map(|p| Value::disc(p)).collect();
                return ExecResult::Ok(Value::seq(parts).with_confidence(conf));
            }
            ExecResult::Error("Need string and separator".to_string())
        }
        Opcode::Join => {
            if let (Some(list), Some(sep)) = (inputs.get(0).and_then(|v| v.as_vec()), inputs.get(1).and_then(|v| v.as_str())) {
                let joined = list.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(sep);
                return ExecResult::Ok(Value::disc(&joined).with_confidence(conf));
            }
            ExecResult::Error("Need seq of strings and separator".to_string())
        }
        Opcode::Len => {
            if let Some(v) = inputs.get(0) {
                return ExecResult::Ok(Value::quant(v.len() as f64).with_confidence(conf));
            }
            ExecResult::Error("Need a value".to_string())
        }
        Opcode::Contains => {
            if let (Some(hay), Some(needle)) = (inputs.get(0), inputs.get(1)) {
                let found = match (&hay.data, &needle.data) {
                    (DataType::Disc(h), DataType::Disc(n)) => h.contains(n.as_str()),
                    (DataType::Seq(v), _) => v.iter().any(|x| x.data == needle.data),
                    _ => false,
                };
                return ExecResult::Ok(Value::bool_(found).with_confidence(conf));
            }
            ExecResult::Error("Need haystack and needle".to_string())
        }
        
        // === STRUCT ===
        Opcode::Get => {
            if let (Some(m), Some(k)) = (inputs.get(0).and_then(|v| v.as_map()), inputs.get(1).and_then(|v| v.as_str())) {
                return match m.get(k) {
                    Some(v) => ExecResult::Ok(v.clone()),
                    None => ExecResult::Ok(Value::null()),
                };
            }
            ExecResult::Error("Need struct and key".to_string())
        }
        Opcode::Keys => {
            if let Some(m) = inputs.get(0).and_then(|v| v.as_map()) {
                let keys: Vec<Value> = m.keys().map(|k| Value::disc(k)).collect();
                return ExecResult::Ok(Value::seq(keys).with_confidence(conf));
            }
            ExecResult::Error("Need a struct".to_string())
        }
        Opcode::Merge => {
            if let (Some(a), Some(b)) = (inputs.get(0).and_then(|v| v.as_map()), inputs.get(1).and_then(|v| v.as_map())) {
                let mut merged = a.clone();
                for (k, v) in b { merged.insert(k.clone(), v.clone()); }
                return ExecResult::Ok(Value {
                    data: DataType::Struct(merged), confidence: conf,
                });
            }
            ExecResult::Error("Need two structs".to_string())
        }
        Opcode::HasKey => {
            if let (Some(m), Some(k)) = (inputs.get(0).and_then(|v| v.as_map()), inputs.get(1).and_then(|v| v.as_str())) {
                return ExecResult::Ok(Value::bool_(m.contains_key(k)).with_confidence(conf));
            }
            ExecResult::Error("Need struct and key".to_string())
        }
        
        // === AGGREGATION ===
        Opcode::Sum => {
            if let Some(list) = inputs.get(0).and_then(|v| v.as_vec()) {
                let sum: f64 = list.iter().filter_map(|v| v.as_f64()).sum();
                let ratio = list.iter().filter(|v| v.as_f64().is_some()).count() as f64 / list.len() as f64;
                return ExecResult::Ok(Value::quant_c(sum, conf * ratio));
            }
            ExecResult::Error("Need a seq".to_string())
        }
        Opcode::Mean => {
            if let Some(list) = inputs.get(0).and_then(|v| v.as_vec()) {
                let nums: Vec<f64> = list.iter().filter_map(|v| v.as_f64()).collect();
                if nums.is_empty() { return ExecResult::Ok(Value::null()); }
                let mean = nums.iter().sum::<f64>() / nums.len() as f64;
                return ExecResult::Ok(Value::quant_c(mean, conf * (nums.len() as f64 / list.len() as f64) * 0.99));
            }
            ExecResult::Error("Need a seq".to_string())
        }
        Opcode::Min => agg_f64(inputs, |nums| nums.iter().cloned().fold(f64::INFINITY, f64::min)),
        Opcode::Max => agg_f64(inputs, |nums| nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max)),
        Opcode::Median => {
            if let Some(list) = inputs.get(0).and_then(|v| v.as_vec()) {
                let mut nums: Vec<f64> = list.iter().filter_map(|v| v.as_f64()).collect();
                nums.sort_by(|a,b| a.partial_cmp(b).unwrap());
                let median = if nums.len() % 2 == 0 {
                    (nums[nums.len()/2-1] + nums[nums.len()/2]) / 2.0
                } else { nums[nums.len()/2] };
                return ExecResult::Ok(Value::quant_c(median, conf * 0.99));
            }
            ExecResult::Error("Need a seq".to_string())
        }
        Opcode::Variance => {
            if let Some(list) = inputs.get(0).and_then(|v| v.as_vec()) {
                let nums: Vec<f64> = list.iter().filter_map(|v| v.as_f64()).collect();
                if nums.is_empty() { return ExecResult::Ok(Value::null()); }
                let mean = nums.iter().sum::<f64>() / nums.len() as f64;
                let variance = nums.iter().map(|n| (n - mean).powi(2)).sum::<f64>() / nums.len() as f64;
                return ExecResult::Ok(Value::quant_c(variance, conf * 0.98));
            }
            ExecResult::Error("Need a seq".to_string())
        }
        Opcode::StdDev => {
            if let Some(list) = inputs.get(0).and_then(|v| v.as_vec()) {
                let nums: Vec<f64> = list.iter().filter_map(|v| v.as_f64()).collect();
                if nums.is_empty() { return ExecResult::Ok(Value::null()); }
                let mean = nums.iter().sum::<f64>() / nums.len() as f64;
                let variance = nums.iter().map(|n| (n - mean).powi(2)).sum::<f64>() / nums.len() as f64;
                return ExecResult::Ok(Value::quant_c(variance.sqrt(), conf * 0.97));
            }
            ExecResult::Error("Need a seq".to_string())
        }
        
        // === LOGIC ===
        Opcode::And => {
            let b = inputs.iter().all(|v| matches!(v.data, DataType::Bool(true)));
            ExecResult::Ok(Value::bool_(b).with_confidence(conf))
        }
        Opcode::Or => {
            let b = inputs.iter().any(|v| matches!(v.data, DataType::Bool(true)));
            ExecResult::Ok(Value::bool_(b).with_confidence(conf))
        }
        Opcode::Not => {
            if let Some(v) = inputs.get(0) {
                let neg = !matches!(v.data, DataType::Bool(true)) && !matches!(v.data, DataType::Quant(_)) || matches!(v.data, DataType::Quant(0.0));
                ExecResult::Ok(Value::bool_(neg).with_confidence(conf))
            } else { ExecResult::Error("Need a value".to_string()) }
        }
        Opcode::Xor => {
            if inputs.len() >= 2 {
                let a = matches!(inputs[0].data, DataType::Bool(true));
                let b = matches!(inputs[1].data, DataType::Bool(true));
                return ExecResult::Ok(Value::bool_(a ^ b).with_confidence(conf));
            }
            ExecResult::Error("Need two bools".to_string())
        }
        
        // === TYPE ===
        Opcode::TypeOf => {
            if let Some(v) = inputs.get(0) {
                let type_name = match v.data {
                    DataType::Quant(_) => "Quant",
                    DataType::Disc(_) => "Disc",
                    DataType::Seq(_) => "Seq",
                    DataType::Struct(_) => "Struct",
                    DataType::Bool(_) => "Bool",
                    DataType::Fuzz(_) => "Fuzz",
                    DataType::Null => "Null",
                };
                return ExecResult::Ok(Value::disc(type_name));
            }
            ExecResult::Error("Need a value".to_string())
        }
        
        // === PROBABILISTIC ===
        Opcode::WithConfidence => {
            if let (Some(v), Some(c)) = (inputs.get(0), inputs.get(1).and_then(|v| v.as_f64())) {
                return ExecResult::Ok(v.clone().with_confidence(c));
            }
            ExecResult::Error("Need value and confidence".to_string())
        }
        Opcode::MeasureConfidence => {
            if let Some(v) = inputs.get(0) {
                return ExecResult::Ok(Value::quant(v.confidence));
            }
            ExecResult::Error("Need a value".to_string())
        }
        Opcode::ClampConfidence => {
            if let Some(v) = inputs.get(0) {
                return ExecResult::Ok(v.clone().with_confidence(v.confidence.clamp(0.5, 0.99)));
            }
            ExecResult::Error("Need a value".to_string())
        }
        Opcode::FuzzFromThreshold => {
            if let (Some(v), Some(thresh)) = (inputs.get(0).and_then(|v| v.as_f64()), inputs.get(1).and_then(|v| v.as_f64())) {
                return ExecResult::Ok(Value::fuzz(if v > thresh { 1.0 } else { 0.0 }));
            }
            ExecResult::Error("Need two numbers".to_string())
        }
        
        Opcode::Nop => ExecResult::Ok(Value::null()),
        Opcode::Replace => ExecResult::Error("Unimplemented".to_string()),
        Opcode::Values => ExecResult::Error("Unimplemented".to_string()),
        Opcode::StartsWith => ExecResult::Error("Unimplemented".to_string()),
        Opcode::EndsWith => ExecResult::Error("Unimplemented".to_string()),
        Opcode::Substr => ExecResult::Error("Unimplemented".to_string()),
        Opcode::IsNull => ExecResult::Error("Unimplemented".to_string()),
        Opcode::IsQuant => ExecResult::Error("Unimplemented".to_string()),
        Opcode::IsDisc => ExecResult::Error("Unimplemented".to_string()),
        Opcode::FuzzCombine => ExecResult::Error("Unimplemented".to_string()),
        Opcode::FuzzNot => ExecResult::Error("Unimplemented".to_string()),
    }
}

pub enum ExecResult {
    Ok(Value),
    Error(String),
}

// Helper functions
fn binop_f64(inputs: &[Value], op: impl Fn(f64,f64) -> f64, conf: f64) -> ExecResult {
    if let (Some(a), Some(b)) = (inputs.get(0).and_then(|v| v.as_f64()), inputs.get(1).and_then(|v| v.as_f64())) {
        ExecResult::Ok(Value::quant_c(op(a, b), conf))
    } else { ExecResult::Error("Need two numbers".to_string()) }
}

fn unop_f64(inputs: &[Value], op: impl Fn(f64) -> f64, conf: f64) -> ExecResult {
    if let Some(n) = inputs.get(0).and_then(|v| v.as_f64()) {
        ExecResult::Ok(Value::quant_c(op(n), conf))
    } else { ExecResult::Error("Need a number".to_string()) }
}

fn cmpop(inputs: &[Value], op: impl Fn(f64,f64) -> bool, conf: f64) -> ExecResult {
    if let (Some(a), Some(b)) = (inputs.get(0).and_then(|v| v.as_f64()), inputs.get(1).and_then(|v| v.as_f64())) {
        ExecResult::Ok(Value::bool_(op(a, b)).with_confidence(conf))
    } else { ExecResult::Error("Need two numbers".to_string()) }
}

fn listop(inputs: &[Value], op: impl Fn(&Vec<Value>) -> Vec<Value>) -> ExecResult {
    if let Some(list) = inputs.get(0).and_then(|v| v.as_vec()) {
        ExecResult::Ok(Value::seq(op(list)))
    } else { ExecResult::Error("Need a seq".to_string()) }
}

fn strop(inputs: &[Value], op: impl Fn(&str) -> String) -> ExecResult {
    if let Some(s) = inputs.get(0).and_then(|v| v.as_str()) {
        let conf = inputs.get(0).map(|v| v.confidence).unwrap_or(1.0);
        ExecResult::Ok(Value::disc(&op(s)).with_confidence(conf))
    } else { ExecResult::Error("Need a string".to_string()) }
}

fn agg_f64(inputs: &[Value], op: impl Fn(&Vec<f64>) -> f64) -> ExecResult {
    if let Some(list) = inputs.get(0).and_then(|v| v.as_vec()) {
        let nums: Vec<f64> = list.iter().filter_map(|v| v.as_f64()).collect();
        if nums.is_empty() { return ExecResult::Ok(Value::null()); }
        ExecResult::Ok(Value::quant_c(op(&nums), inputs[0].confidence * 0.99))
    } else { ExecResult::Error("Need a seq".to_string()) }
}

// ============================================================
// COMPILER — turns intent into opcode tree
// ============================================================

#[derive(Debug, Clone)]
pub struct CompiledOp {
    pub opcode: Opcode,
    pub inputs: Vec<usize>, // indices into value stack or variable table
    pub output_var: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompiledProgram {
    pub ops: Vec<CompiledOp>,
    pub variables: HashMap<String, Value>,
    pub constraints: Vec<Constraint>,
    pub intent: String,
}

pub struct AxiomCompiler {
    intent_ops: HashMap<&'static str, Opcode>,
}

impl AxiomCompiler {
    pub fn new() -> Self {
        let mut m = HashMap::new();
        m.insert("sort", Opcode::SortAsc);
        m.insert("sort desc", Opcode::SortDesc);
        m.insert("sort asc", Opcode::SortAsc);
        m.insert("reverse", Opcode::Reverse);
        m.insert("filter", Opcode::Filter);
        m.insert("count", Opcode::Count);
        m.insert("sum", Opcode::Sum);
        m.insert("mean", Opcode::Mean);
        m.insert("average", Opcode::Mean);
        m.insert("min", Opcode::Min);
        m.insert("max", Opcode::Max);
        m.insert("median", Opcode::Median);
        m.insert("variance", Opcode::Variance);
        m.insert("stddev", Opcode::StdDev);
        m.insert("unique", Opcode::Unique);
        m.insert("flatten", Opcode::Flatten);
        m.insert("first", Opcode::First);
        m.insert("last", Opcode::Last);
        m.insert("upper", Opcode::Upper);
        m.insert("lower", Opcode::Lower);
        m.insert("strip", Opcode::Strip);
        m.insert("split", Opcode::Split);
        m.insert("join", Opcode::Join);
        m.insert("len", Opcode::Len);
        m.insert("length", Opcode::Len);
        m.insert("contains", Opcode::Contains);
        m.insert("merge", Opcode::Merge);
        m.insert("get", Opcode::Get);
        m.insert("keys", Opcode::Keys);
        m.insert("slice", Opcode::Slice);
        Self { intent_ops: m }
    }

    /// Compile natural language intent into a program
    pub fn compile_intent(&self, intent: &str) -> CompiledProgram {
        let lower = intent.to_lowercase();
        let mut ops = vec![];
        let mut constraints = vec![];

        // Extract operations from intent
        let detected: Vec<(&str, Opcode)> = self.intent_ops.iter()
            .filter(|(k, _)| lower.contains(k))
            .map(|(k, v)| (*k, v.clone()))
            .collect();

        // Deduplicate and order
        let mut seen = vec![];
        for (keyword, opcode) in &detected {
            if !seen.iter().any(|(s, _)| s.contains(keyword) || keyword.contains(s)) {
                seen.push((*keyword, opcode.clone()));
            }
        }

        // Sort direction
        if lower.contains("descending") || lower.contains("desc") || lower.contains("highest first") {
            seen = seen.into_iter().map(|(k, op)| {
                if k == "sort" { ("sort desc", Opcode::SortDesc) } else { (k, op) }
            }).collect();
        }

        for (_, opcode) in &seen {
            ops.push(CompiledOp { opcode: opcode.clone(), inputs: vec![], output_var: None });
        }

        // Extract constraints
        if lower.contains("must") { constraints.push(Constraint { kind: ConstraintKind::Must, expression: "from intent".to_string(), weight: 1.0 }); }
        if lower.contains("should") { constraints.push(Constraint { kind: ConstraintKind::Should, expression: "from intent".to_string(), weight: 0.5 }); }

        CompiledProgram { ops, variables: HashMap::new(), constraints, intent: intent.to_string() }
    }

    /// Count opcodes detected
    pub fn opcode_count(&self) -> usize { self.intent_ops.len() }
}

// ============================================================
// VM — executes compiled programs
// ============================================================

pub struct AxiomVM {
    stack: Vec<Value>,
    variables: HashMap<String, Value>,
    trace: Vec<VmTraceEntry>,
}

#[derive(Debug, Clone)]
pub struct VmTraceEntry {
    pub opcode: String,
    pub inputs_len: usize,
    pub output_confidence: f64,
}

impl AxiomVM {
    pub fn new() -> Self { Self { stack: vec![], variables: HashMap::new(), trace: vec![] } }

    pub fn push(&mut self, val: Value) { self.stack.push(val); }

    pub fn pop(&mut self) -> Option<Value> { self.stack.pop() }

    pub fn execute_program(&mut self, program: &CompiledProgram, input: Value) -> Result<Value, String> {
        self.stack.clear();
        self.stack.push(input);

        for cop in &program.ops {
            let input_count = match cop.opcode {
                Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div | Opcode::Mod | Opcode::Pow
                | Opcode::Eq | Opcode::Neq | Opcode::Lt | Opcode::Lte | Opcode::Gt | Opcode::Gte
                | Opcode::And | Opcode::Or | Opcode::Xor
                | Opcode::Split | Opcode::Contains | Opcode::Zip | Opcode::Merge
                | Opcode::HasKey | Opcode::Get | Opcode::Append => 2,
                Opcode::Slice => 3,
                _ => 1,
            };

            let mut inputs = vec![];
            for _ in 0..input_count {
                if let Some(v) = self.stack.pop() { inputs.insert(0, v); }
                else { return Err(format!("Stack underflow for {:?}", cop.opcode)); }
            }

            let result = match exec(&cop.opcode, &inputs) {
                ExecResult::Ok(v) => {
                    self.trace.push(VmTraceEntry {
                        opcode: format!("{:?}", cop.opcode),
                        inputs_len: inputs.len(),
                        output_confidence: v.confidence,
                    });
                    self.stack.push(v.clone());
                    if let Some(ref var) = cop.output_var { self.variables.insert(var.clone(), v.clone()); }
                    v
                }
                ExecResult::Error(e) => return Err(e),
            };
        }

        self.stack.pop().ok_or("No output on stack".to_string())
    }

    pub fn trace(&self) -> &[VmTraceEntry] { &self.trace }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quant_value() {
        let v = Value::quant(42.0);
        assert!(v.is_quant());
        assert_eq!(v.as_f64(), Some(42.0));
    }

    #[test]
    fn test_disc_value() {
        let v = Value::disc("hello");
        assert!(v.is_disc());
        assert_eq!(v.as_str(), Some("hello"));
    }

    #[test]
    fn test_seq_value() {
        let v = Value::seq(vec![Value::quant(1.0), Value::quant(2.0)]);
        assert!(v.is_seq());
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn test_struct_value() {
        let mut m = HashMap::new();
        m.insert("key".to_string(), Value::quant(42.0));
        let v = Value { data: DataType::Struct(m), confidence: 0.9 };
        assert!(v.is_struct());
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_bayesian() {
        let c = bayesian(0.5, 0.5);
        assert!((c - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_add() {
        let r = exec(&Opcode::Add, &[Value::quant(3.0), Value::quant(4.0)]);
        match r { ExecResult::Ok(v) => assert_eq!(v.as_f64(), Some(7.0)), _ => panic!("expected ok"), }
    }

    #[test]
    fn test_div_by_zero() {
        let r = exec(&Opcode::Div, &[Value::quant(1.0), Value::quant(0.0)]);
        assert!(matches!(r, ExecResult::Error(_)));
    }

    #[test]
    fn test_sort_desc() {
        let input = Value::seq(vec![Value::quant(3.0), Value::quant(1.0), Value::quant(5.0)]);
        let r = exec(&Opcode::SortDesc, &[input]);
        match r {
            ExecResult::Ok(v) => {
                let nums: Vec<f64> = v.as_vec().unwrap().iter().filter_map(|v| v.as_f64()).collect();
                assert_eq!(nums, vec![5.0, 3.0, 1.0]);
            }
            _ => panic!("expected ok"),
        }
    }

    #[test]
    fn test_filter() {
        let input = Value::seq(vec![Value::quant(1.0), Value::quant(5.0), Value::quant(3.0), Value::quant(8.0)]);
        let r = exec(&Opcode::Filter, &[input, Value::quant(4.0)]);
        match r {
            ExecResult::Ok(v) => {
                let nums: Vec<f64> = v.as_vec().unwrap().iter().filter_map(|v| v.as_f64()).collect();
                assert_eq!(nums, vec![5.0, 8.0]);
            }
            _ => panic!("expected ok"),
        }
    }

    #[test]
    fn test_mean() {
        let input = Value::seq(vec![Value::quant(10.0), Value::quant(20.0), Value::quant(30.0)]);
        let r = exec(&Opcode::Mean, &[input]);
        match r { ExecResult::Ok(v) => assert!((v.as_f64().unwrap() - 20.0).abs() < 0.01), _ => panic!("expected ok"), }
    }

    #[test]
    fn test_median() {
        let input = Value::seq(vec![Value::quant(3.0), Value::quant(1.0), Value::quant(2.0)]);
        let r = exec(&Opcode::Median, &[input]);
        match r { ExecResult::Ok(v) => assert!((v.as_f64().unwrap() - 2.0).abs() < 0.01), _ => panic!("expected ok"), }
    }

    #[test]
    fn test_variance_stddev() {
        let input = Value::seq(vec![Value::quant(2.0), Value::quant(4.0), Value::quant(4.0), Value::quant(4.0), Value::quant(5.0), Value::quant(5.0), Value::quant(7.0), Value::quant(9.0)]);
        let r = exec(&Opcode::Variance, &[input]);
        match r { ExecResult::Ok(v) => assert!((v.as_f64().unwrap() - 4.0).abs() < 0.1), _ => panic!("expected ok"), }
        let r2 = exec(&Opcode::StdDev, &[input.clone()]);
        match r2 { ExecResult::Ok(v) => assert!((v.as_f64().unwrap() - 2.0).abs() < 0.1), _ => panic!("expected ok"), }
    }

    #[test]
    fn test_type_check() {
        let t = AxiomType::Quant { min: 0.0, max: 100.0 };
        assert!(matches!(t.check(&Value::quant(50.0)), TypeResult::Pass));
        assert!(matches!(t.check(&Value::quant(200.0)), TypeResult::Fail(_)));
        assert!(matches!(t.check(&Value::disc("hi")), TypeResult::Fail(_)));
    }

    #[test]
    fn test_seq_type_check() {
        let t = AxiomType::Seq { element: Box::new(AxiomType::Quant { min: 0.0, max: 100.0 }), max_len: Some(5) };
        let good = Value::seq(vec![Value::quant(1.0), Value::quant(2.0)]);
        assert!(matches!(t.check(&good), TypeResult::Pass));
        let bad = Value::seq(vec![Value::quant(1.0), Value::disc("nope")]);
        assert!(matches!(t.check(&bad), TypeResult::Fail(_)));
    }

    #[test]
    fn test_struct_type_check() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), AxiomType::Disc { max_len: Some(50) });
        fields.insert("age".to_string(), AxiomType::Quant { min: 0.0, max: 150.0 });
        let t = AxiomType::Struct { fields };
        let mut m = HashMap::new();
        m.insert("name".to_string(), Value::disc("Casey"));
        m.insert("age".to_string(), Value::quant(25.0));
        let good = Value { data: DataType::Struct(m), confidence: 0.9 };
        assert!(matches!(t.check(&good), TypeResult::Pass));
    }

    #[test]
    fn test_compiler_intent() {
        let compiler = AxiomCompiler::new();
        let program = compiler.compile_intent("sort numbers descending and filter values greater than 10");
        assert!(!program.ops.is_empty());
    }

    #[test]
    fn test_vm_execution() {
        let compiler = AxiomCompiler::new();
        let program = compiler.compile_intent("sort desc");
        let mut vm = AxiomVM::new();
        let input = Value::seq(vec![Value::quant(3.0), Value::quant(1.0), Value::quant(5.0)]);
        let result = vm.execute_program(&program, input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_string_ops() {
        let r = exec(&Opcode::Upper, &[Value::disc("hello")]);
        match r { ExecResult::Ok(v) => assert_eq!(v.as_str(), Some("HELLO")), _ => panic!(), }
        let r2 = exec(&Opcode::Split, &[Value::disc("a,b,c"), Value::disc(",")]);
        match r2 { ExecResult::Ok(v) => assert_eq!(v.as_vec().unwrap().len(), 3), _ => panic!(), }
    }

    #[test]
    fn test_unique() {
        let input = Value::seq(vec![Value::quant(1.0), Value::quant(2.0), Value::quant(2.0), Value::quant(1.0)]);
        let r = exec(&Opcode::Unique, &[input]);
        match r { ExecResult::Ok(v) => assert_eq!(v.as_vec().unwrap().len(), 2), _ => panic!(), }
    }

    #[test]
    fn test_flatten() {
        let inner1 = Value::seq(vec![Value::quant(1.0), Value::quant(2.0)]);
        let inner2 = Value::seq(vec![Value::quant(3.0)]);
        let input = Value::seq(vec![inner1, inner2]);
        let r = exec(&Opcode::Flatten, &[input]);
        match r {
            ExecResult::Ok(v) => {
                let nums: Vec<f64> = v.as_vec().unwrap().iter().filter_map(|v| v.as_f64()).collect();
                assert_eq!(nums, vec![1.0, 2.0, 3.0]);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_struct_ops() {
        let mut m = HashMap::new();
        m.insert("x".to_string(), Value::quant(42.0));
        let s = Value { data: DataType::Struct(m), confidence: 0.9 };
        let r = exec(&Opcode::Get, &[s.clone(), Value::disc("x")]);
        match r { ExecResult::Ok(v) => assert_eq!(v.as_f64(), Some(42.0)), _ => panic!(), }
        let r2 = exec(&Opcode::HasKey, &[s, Value::disc("x")]);
        match r2 { ExecResult::Ok(v) => assert!(matches!(v.data, DataType::Bool(true))), _ => panic!(), }
    }

    #[test]
    fn test_measure_confidence() {
        let v = Value::quant_c(42.0, 0.75);
        let r = exec(&Opcode::MeasureConfidence, &[v]);
        match r { ExecResult::Ok(v) => assert!((v.as_f64().unwrap() - 0.75).abs() < 0.01), _ => panic!(), }
    }

    #[test]
    fn test_xor() {
        let r = exec(&Opcode::Xor, &[Value::bool_(true), Value::bool_(false)]);
        match r { ExecResult::Ok(v) => assert!(matches!(v.data, DataType::Bool(true))), _ => panic!(), }
    }
}
