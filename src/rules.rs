//! Bounded interpreter for recovered scoring-rule IR. No file, network, import or process APIs.
use anyhow::{Result, anyhow, bail};
use serde_json::Value as J;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
type Env = BTreeMap<String, V>;
#[derive(Clone, Debug)]
pub enum V {
    Nil,
    N(f64),
    S(String),
    L(Rc<RefCell<Vec<V>>>),
    M(Rc<RefCell<Env>>),
    D(Rc<RefCell<Env>>),
    Type(Vec<String>),
    Fn(J, Env),
}
impl V {
    pub fn from_json(j: &J) -> V {
        match j {
            J::Null => V::Nil,
            J::Bool(x) => V::N(if *x { 1. } else { 0. }),
            J::Number(x) => V::N(x.as_f64().unwrap()),
            J::String(x) => V::S(x.clone()),
            J::Array(a) => list(a.iter().map(Self::from_json).collect()),
            J::Object(m) => map(m
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        if k == "scoreTypes" {
                            match Self::from_json(v) {
                                V::M(m) => V::D(m),
                                v => v,
                            }
                        } else {
                            Self::from_json(v)
                        },
                    )
                })
                .collect()),
        }
    }
    pub fn json(&self) -> J {
        match self {
            V::Nil => J::Null,
            V::N(x) => serde_json::json!(*x),
            V::S(x) => J::String(x.clone()),
            V::L(a) => J::Array(a.borrow().iter().map(V::json).collect()),
            V::M(a) | V::D(a) => J::Object(
                a.borrow()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.json()))
                    .collect(),
            ),
            _ => J::Null,
        }
    }
    fn num(&self) -> Result<f64> {
        match self {
            V::N(x) => Ok(*x),
            _ => bail!("expected number, got {self:?}"),
        }
    }
    fn text(&self) -> String {
        match self {
            V::S(s) => s.clone(),
            V::N(n) => format!("{n}"),
            _ => format!("{self:?}"),
        }
    }
    fn truth(&self) -> bool {
        match self {
            V::Nil => false,
            V::N(n) => *n != 0.,
            V::S(s) => !s.is_empty(),
            V::L(a) => !a.borrow().is_empty(),
            V::M(m) | V::D(m) => !m.borrow().is_empty(),
            _ => true,
        }
    }
    fn seq(&self) -> Result<Vec<V>> {
        match self {
            V::L(a) => Ok(a.borrow().clone()),
            V::M(m) | V::D(m) => Ok(m.borrow().keys().map(|s| V::S(s.clone())).collect()),
            _ => bail!("expected sequence"),
        }
    }
    fn get(&self, k: &V) -> Result<V> {
        match self {
            V::D(m) => Ok(m.borrow().get(&k.text()).cloned().unwrap_or(V::N(0.))),
            V::M(m) => m
                .borrow()
                .get(&k.text())
                .cloned()
                .ok_or_else(|| anyhow!("missing key {}", k.text())),
            V::L(a) => {
                let n = k.num()? as isize;
                let a = a.borrow();
                let i = if n < 0 { a.len() as isize + n } else { n };
                a.get(i as usize)
                    .cloned()
                    .ok_or_else(|| anyhow!("index {n}"))
            }
            _ => bail!("cannot index"),
        }
    }
}
fn list(v: Vec<V>) -> V {
    V::L(Rc::new(RefCell::new(v)))
}
fn map(v: Env) -> V {
    V::M(Rc::new(RefCell::new(v)))
}
fn s(n: &J, k: &str) -> String {
    n[k].as_str().unwrap_or("").into()
}
fn arr(n: &J, k: &str) -> Vec<J> {
    n[k].as_array().cloned().unwrap_or_default()
}
#[derive(Debug)]
enum Flow {
    Next,
    Return(V),
    Continue,
}
pub struct Machine {
    pub env: Env,
    budget: usize,
}
impl Machine {
    pub fn new() -> Self {
        Self {
            env: Env::new(),
            budget: 500_000,
        }
    }
    fn tick(&mut self) -> Result<()> {
        if self.budget == 0 {
            bail!("scoring operation limit")
        };
        self.budget -= 1;
        Ok(())
    }
    pub fn load(&mut self, module: &J) -> Result<()> {
        let mut e = self.env.clone();
        self.block(&arr(module, "body"), &mut e)?;
        self.env = e;
        Ok(())
    }
    pub fn call(&mut self, name: &str, args: Vec<V>) -> Result<V> {
        let f = self
            .env
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("unknown rule {name}"))?;
        self.invoke(f, args, Env::new())
    }
    pub fn expression(&mut self, n: &J, vars: &J) -> Result<J> {
        let mut e = match V::from_json(vars) {
            V::M(m) => m.borrow().clone(),
            _ => Env::new(),
        };
        Ok(self.eval(n, &mut e)?.json())
    }
    fn assign(&mut self, t: &J, v: V, e: &mut Env) -> Result<()> {
        match s(t, "node").as_str() {
            "Name" => {
                e.insert(s(t, "id"), v);
            }
            "Tuple" | "List" => {
                let a = v.seq()?;
                let ts = arr(t, "elts");
                if a.len() != ts.len() {
                    bail!("unpack length")
                };
                for (t, v) in ts.iter().zip(a) {
                    self.assign(t, v, e)?;
                }
            }
            "Attribute" => {
                let o = self.eval(&t["value"], e)?;
                if let V::M(m) | V::D(m) = o {
                    m.borrow_mut().insert(s(t, "attr"), v);
                } else {
                    bail!("attribute assignment")
                }
            }
            "Subscript" => {
                let o = self.eval(&t["value"], e)?;
                let k = self.eval(&t["slice"], e)?;
                match o {
                    V::M(m) | V::D(m) => {
                        m.borrow_mut().insert(k.text(), v);
                    }
                    V::L(a) => {
                        let i = k.num()? as usize;
                        a.borrow_mut()[i] = v;
                    }
                    _ => bail!("index assignment"),
                }
            }
            _ => bail!("unsupported assignment"),
        };
        Ok(())
    }
    fn block(&mut self, b: &[J], e: &mut Env) -> Result<Flow> {
        for n in b {
            self.tick()?;
            let flow = match s(n, "node").as_str() {
                "Import" | "Pass" => Flow::Next,
                "FunctionDef" => {
                    e.insert(s(n, "name"), V::Fn(n.clone(), e.clone()));
                    Flow::Next
                }
                "Assign" => {
                    let v = self.eval(&n["value"], e)?;
                    for t in arr(n, "targets") {
                        self.assign(&t, v.clone(), e)?;
                    }
                    Flow::Next
                }
                "AugAssign" => {
                    let a = self.eval(&n["target"], e)?;
                    let b = self.eval(&n["value"], e)?;
                    let v = self.bin(&s(&n["op"], "node"), a, b)?;
                    self.assign(&n["target"], v, e)?;
                    Flow::Next
                }
                "Expr" => {
                    self.eval(&n["value"], e)?;
                    Flow::Next
                }
                "Return" => Flow::Return(if n["value"].is_null() {
                    V::Nil
                } else {
                    self.eval(&n["value"], e)?
                }),
                "Continue" => Flow::Continue,
                "If" => {
                    let yes = self.eval(&n["test"], e)?.truth();
                    self.block(&arr(n, if yes { "body" } else { "orelse" }), e)?
                }
                "For" => {
                    let vals = self.eval(&n["iter"], e)?.seq()?;
                    let mut ret = Flow::Next;
                    for v in vals {
                        self.assign(&n["target"], v, e)?;
                        if let Flow::Return(v) = self.block(&arr(n, "body"), e)? {
                            ret = Flow::Return(v);
                            break;
                        }
                    }
                    ret
                }
                "Try" => match self.block(&arr(n, "body"), e) {
                    Ok(f) => f,
                    Err(_) => {
                        let hs = arr(n, "handlers");
                        if hs.is_empty() {
                            bail!("rule exception")
                        };
                        self.block(&arr(&hs[0], "body"), e)?
                    }
                },
                x => bail!("unsupported rule statement {x}"),
            };
            if !matches!(flow, Flow::Next) {
                return Ok(flow);
            }
        }
        Ok(Flow::Next)
    }
    fn bin(&self, op: &str, a: V, b: V) -> Result<V> {
        if op == "Add" {
            if let (V::L(x), V::L(y)) = (&a, &b) {
                let mut v = x.borrow().clone();
                v.extend(y.borrow().clone());
                return Ok(list(v));
            }
            if let (V::S(x), V::S(y)) = (&a, &b) {
                return Ok(V::S(format!("{x}{y}")));
            }
        }
        let x = a.num()?;
        let y = b.num()?;
        Ok(V::N(match op {
            "Add" => x + y,
            "Sub" => x - y,
            "Mult" => x * y,
            "Div" => {
                if y == 0. {
                    bail!("division by zero")
                };
                (x / y).floor()
            }
            "Pow" => x.powf(y),
            "BitOr" => (x as i64 | y as i64) as f64,
            "LShift" => (x as i64)
                .checked_shl(y as u32)
                .ok_or_else(|| anyhow!("shift overflow"))? as f64,
            _ => bail!("unsupported operator {op}"),
        }))
    }
    fn eval(&mut self, n: &J, e: &mut Env) -> Result<V> {
        self.tick()?;
        Ok(match s(n, "node").as_str() {
            "Constant" => V::from_json(&n["value"]),
            "Name" => e
                .get(&s(n, "id"))
                .cloned()
                .ok_or_else(|| anyhow!("unknown variable {}", s(n, "id")))?,
            "List" | "Tuple" => {
                let mut a = vec![];
                for x in arr(n, "elts") {
                    a.push(self.eval(&x, e)?)
                }
                list(a)
            }
            "Dict" => {
                let mut m = Env::new();
                for (k, v) in arr(n, "keys").iter().zip(arr(n, "values")) {
                    m.insert(self.eval(k, e)?.text(), self.eval(&v, e)?);
                }
                map(m)
            }
            "Attribute" => self.eval(&n["value"], e)?.get(&V::S(s(n, "attr")))?,
            "Subscript" => {
                let a = self.eval(&n["value"], e)?;
                let k = self.eval(&n["slice"], e)?;
                a.get(&k)?
            }
            "BinOp" => {
                let a = self.eval(&n["left"], e)?;
                let b = self.eval(&n["right"], e)?;
                self.bin(&s(&n["op"], "node"), a, b)?
            }
            "UnaryOp" => {
                let v = self.eval(&n["operand"], e)?;
                match s(&n["op"], "node").as_str() {
                    "Not" => V::N((!v.truth()) as u8 as f64),
                    "USub" => V::N(-v.num()?),
                    _ => bail!("unary"),
                }
            }
            "BoolOp" => {
                let and = s(&n["op"], "node") == "And";
                let mut v = V::Nil;
                for x in arr(n, "values") {
                    v = self.eval(&x, e)?;
                    if v.truth() != and {
                        break;
                    }
                }
                v
            }
            "IfExp" => {
                let yes = self.eval(&n["test"], e)?.truth();
                self.eval(&n[if yes { "body" } else { "orelse" }], e)?
            }
            "Compare" => {
                let mut a = self.eval(&n["left"], e)?;
                let mut yes = true;
                for (op, b) in arr(n, "ops").iter().zip(arr(n, "comparators")) {
                    let b = self.eval(&b, e)?;
                    let eq = a.json() == b.json();
                    let order = if let (V::N(x), V::N(y)) = (&a, &b) {
                        x.partial_cmp(y)
                    } else {
                        a.text().partial_cmp(&b.text())
                    };
                    use std::cmp::Ordering::*;
                    yes &= match s(op, "node").as_str() {
                        "Eq" => eq,
                        "NotEq" => !eq,
                        "Gt" => order == Some(Greater),
                        "GtE" => order != Some(Less),
                        "Lt" => order == Some(Less),
                        "LtE" => order != Some(Greater),
                        "IsNot" => !eq,
                        "Is" => eq,
                        "In" => b.seq()?.iter().any(|x| x.json() == a.json()),
                        x => bail!("comparison {x}"),
                    };
                    a = b;
                    if !yes {
                        break;
                    }
                }
                V::N(yes as u8 as f64)
            }
            "GeneratorExp" | "ListComp" => {
                let mut out = vec![];
                self.comp(
                    &arr(n, "generators"),
                    0,
                    &n["elt"],
                    &mut e.clone(),
                    &mut out,
                )?;
                list(out)
            }
            "Lambda" => V::Fn(n.clone(), e.clone()),
            "Call" => {
                let mut args = vec![];
                for x in arr(n, "args") {
                    args.push(self.eval(&x, e)?)
                }
                let mut kw = Env::new();
                for x in arr(n, "keywords") {
                    kw.insert(s(&x, "arg"), self.eval(&x["value"], e)?);
                }
                let f = &n["func"];
                if s(f, "node") == "Attribute" {
                    let attr = s(f, "attr");
                    if s(&f["value"], "id") == "collections" && attr == "namedtuple" {
                        V::Type(args[1].seq()?.iter().map(V::text).collect())
                    } else {
                        let o = self.eval(&f["value"], e)?;
                        self.method(o, &attr, args, kw)?
                    }
                } else {
                    let name = s(f, "id");
                    if let Some(v) = e.get(&name).cloned() {
                        self.invoke(v, args, kw)?
                    } else {
                        self.builtin(&name, args)?
                    }
                }
            }
            x => bail!("unsupported rule expression {x}"),
        })
    }
    fn comp(&mut self, gs: &[J], i: usize, elt: &J, e: &mut Env, out: &mut Vec<V>) -> Result<()> {
        if i == gs.len() {
            out.push(self.eval(elt, e)?);
            return Ok(());
        }
        let g = &gs[i];
        for v in self.eval(&g["iter"], e)?.seq()? {
            self.assign(&g["target"], v, e)?;
            let mut yes = true;
            for cond in arr(g, "ifs") {
                if !self.eval(&cond, e)?.truth() {
                    yes = false;
                    break;
                }
            }
            if yes {
                self.comp(gs, i + 1, elt, e, out)?
            }
        }
        Ok(())
    }
    fn invoke(&mut self, f: V, args: Vec<V>, kw: Env) -> Result<V> {
        match f {
            V::Type(fields) => {
                let mut m = Env::new();
                for (i, k) in fields.iter().enumerate() {
                    m.insert(
                        k.clone(),
                        kw.get(k)
                            .cloned()
                            .or_else(|| args.get(i).cloned())
                            .ok_or_else(|| anyhow!("missing constructor arg {k}"))?,
                    );
                }
                Ok(map(m))
            }
            V::Fn(n, mut e) => {
                let params = arr(&n["args"], "args");
                for (i, p) in params.iter().enumerate() {
                    let k = s(p, "arg");
                    e.insert(
                        k.clone(),
                        kw.get(&k)
                            .cloned()
                            .or_else(|| args.get(i).cloned())
                            .ok_or_else(|| anyhow!("missing function arg {k}"))?,
                    );
                }
                if s(&n, "node") == "Lambda" {
                    self.eval(&n["body"], &mut e)
                } else {
                    match self.block(&arr(&n, "body"), &mut e)? {
                        Flow::Return(v) => Ok(v),
                        _ => Ok(V::Nil),
                    }
                }
            }
            _ => bail!("not callable"),
        }
    }
    fn method(&mut self, o: V, name: &str, args: Vec<V>, kw: Env) -> Result<V> {
        match name {
            "get" => Ok(o.get(&args[0]).unwrap_or_else(|_| {
                args.get(1)
                    .cloned()
                    .or_else(|| kw.get("default").cloned())
                    .unwrap_or(V::Nil)
            })),
            "append" | "add" => {
                if let V::L(a) = o {
                    let mut a = a.borrow_mut();
                    if name == "append" || !a.iter().any(|v| v.json() == args[0].json()) {
                        a.push(args[0].clone())
                    }
                    Ok(V::Nil)
                } else {
                    bail!("append target")
                }
            }
            "update" => {
                if let V::L(a) = o {
                    for v in args[0].seq()? {
                        let mut a = a.borrow_mut();
                        if !a.iter().any(|x| x.json() == v.json()) {
                            a.push(v)
                        }
                    }
                    Ok(V::Nil)
                } else {
                    bail!("update target")
                }
            }
            "sort" => {
                if let V::L(a) = o {
                    let mut vals = vec![];
                    for v in a.borrow().iter() {
                        let key = if let Some(f) = kw.get("key") {
                            self.invoke(f.clone(), vec![v.clone()], Env::new())?
                        } else {
                            v.clone()
                        };
                        vals.push((key.num()?, v.clone()));
                    }
                    vals.sort_by(|a, b| a.0.total_cmp(&b.0));
                    if kw.get("reverse").is_some_and(V::truth) {
                        vals.reverse()
                    }
                    *a.borrow_mut() = vals.into_iter().map(|x| x.1).collect();
                    Ok(V::Nil)
                } else {
                    bail!("sort target")
                }
            }
            "format" => {
                let mut text = o.text();
                for a in args {
                    text = text.replacen("{}", &a.text(), 1)
                }
                Ok(V::S(text))
            }
            _ => bail!("unsupported rule method {name}"),
        }
    }
    fn builtin(&mut self, name: &str, args: Vec<V>) -> Result<V> {
        let a = args.first().cloned().unwrap_or(V::Nil);
        Ok(match name {
            "IsMatchElim" => V::N(
                ((3..=9).contains(&(a.get(&V::S("round".into()))?.num()? as i64))) as u8 as f64,
            ),
            "IsMatchRoundRobin" => V::N((a.get(&V::S("round".into()))?.num()? == 16.) as u8 as f64),
            "IsMatchTopNFinals" => V::N((a.get(&V::S("round".into()))?.num()? == 15.) as u8 as f64),
            "IsMatchTieredTopNFinals" => V::N(
                ((40..=42).contains(&(a.get(&V::S("round".into()))?.num()? as i64))) as u8 as f64,
            ),
            "int" => V::N(a.num()?.trunc()),
            "bool" => V::N(a.truth() as u8 as f64),
            "list" | "set" => list(if matches!(a, V::Nil) {
                vec![]
            } else {
                a.seq()?
            }),
            "len" => V::N(a.seq()?.len() as f64),
            "any" => V::N(a.seq()?.iter().any(V::truth) as u8 as f64),
            "next" => a
                .seq()?
                .first()
                .cloned()
                .or_else(|| args.get(1).cloned())
                .ok_or_else(|| anyhow!("empty next"))?,
            "sum" => V::N(
                a.seq()?
                    .iter()
                    .try_fold(0., |a, v| Ok::<_, anyhow::Error>(a + v.num()?))?,
            ),
            "min" | "max" => {
                let vals = if args.len() == 1 { a.seq()? } else { args };
                let nums = vals.iter().map(V::num).collect::<Result<Vec<_>>>()?;
                V::N(
                    nums.into_iter()
                        .reduce(if name == "min" { f64::min } else { f64::max })
                        .ok_or_else(|| anyhow!("empty min/max"))?,
                )
            }
            "enumerate" => list(
                a.seq()?
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| list(vec![V::N(i as f64), v]))
                    .collect(),
            ),
            "zip" => list(
                a.seq()?
                    .into_iter()
                    .zip(args[1].seq()?)
                    .map(|(a, b)| list(vec![a, b]))
                    .collect(),
            ),
            _ => bail!("unsupported rule call {name}"),
        })
    }
}
