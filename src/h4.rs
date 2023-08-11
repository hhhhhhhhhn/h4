use rquickjs::Ctx;
use std::collections::HashMap;
use std::time;
use crate::insertable::InsertableIterator;
use std::rc::Rc;
use crate::scopes::{Scopes, Value};
use crate::builtin::*;

#[derive(PartialEq, Eq, Debug)]
enum AdvanceResult {
    EnterQuote,
    QuoteChar,
    Macro,
    Normal,
    CallEnd,
    NextArg,
}

pub struct H4<'a, 'b> {
    pub iter: InsertableIterator<'b, char>,
    pub outputs: HashMap<String, String>,
    pub current_output: String,
    pub scopes: Rc<Scopes<'a>>,
    pub ctx: Rc<Ctx<'a>>,
    pub quote_level: usize,
    pub in_call: bool,

    pub name_chars: String,
    pub quote_start: char,
    pub quote_end: char,
}

fn new_id() -> String {
    // TODO: Ensure they are unique
    let timestamp = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("Cannot get system time");
    return format!("temp{}", timestamp.as_micros())
}

impl<'h, 'b> H4<'h, 'b> {
    pub fn new(iter: InsertableIterator<'b, char>, ctx: Ctx<'h>) -> H4<'h, 'b> {
            let outputs = HashMap::new();

            let scopes = Scopes::new();
            scopes.let_variable(&"@define".to_string(), Value::Builtin(builtin_define));
            scopes.let_variable(&"@dump".to_string(), Value::Builtin(builtin_dump));
            scopes.let_variable(&"@pushScope".to_string(), Value::Builtin(builtin_push_scope));
            scopes.let_variable(&"@popScope".to_string(), Value::Builtin(builtin_pop_scope));
            scopes.let_variable(&"@skip".to_string(), Value::Builtin(builtin_skip));
            scopes.let_variable(&"@jsEval".to_string(), Value::Builtin(builtin_jseval));
            scopes.let_variable(&"@let".to_string(), Value::Builtin(builtin_let));
            scopes.let_variable(&"@set".to_string(), Value::Builtin(builtin_set));
            scopes.let_variable(&"@get".to_string(), Value::Builtin(builtin_get));
            scopes.let_variable(&"@shell".to_string(), Value::Builtin(builtin_shell));
            scopes.let_variable(&"@import".to_string(), Value::Builtin(builtin_import));
            scopes.let_variable(&"@for".to_string(), Value::Builtin(builtin_for));
            scopes.let_variable(&"@newline".to_string(), Value::Plain("\n".to_string()));

            let h4 = H4{
                iter,
                outputs,
                scopes: Rc::new(scopes),
                ctx: Rc::new(ctx),

                current_output: "stdout".to_string(),
                name_chars: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_@".to_string(),
                quote_start: '`',
                quote_end: '\'',
                quote_level: 0,
                in_call: false,
            };

            h4.setup_quickjs();
            return h4
    }

    pub fn consume(&mut self) {
        while let Some(_) = self.advance() {}
    }

    fn setup_quickjs(&self) {
        let scopes = self.scopes.clone();
        let ctx = Rc::clone(&self.ctx);

        ctx.globals()
            .set("h4GetVariable", rquickjs::Function::new(*ctx.clone(), move |name: String| -> rquickjs::Value {
                let var = scopes.get_variable(&name);
                let undefined = rquickjs::Undefined;
                let undefined = undefined.into_value(*ctx.clone());
                let value: rquickjs::Value = match var {
                    None => undefined,
                    Some(value) => {
                        let value = Rc::clone(&value);
                        let value = value.borrow().clone();
                        match value {
                            Value::Plain(str) => {
                                rquickjs::String::from_str(*ctx.clone(), &str).unwrap().into_value()
                            },
                            Value::Builtin(..) => {
                                rquickjs::String::from_str(*ctx.clone(), "<Builtin>").unwrap().into_value()
                            },
                            Value::JS(val) => {
                                val
                            }
                        }
                    }
                };
                return value
            })).ok();

        let ctx = self.ctx.clone();
        ctx.globals()
            .set("debugPrint", rquickjs::Function::new(*ctx.clone(), |value: String| {
                eprintln!("{}", value)
            })).ok();
        ctx.globals()
            .set("shell", rquickjs::Function::new(*ctx.clone(), run_shell)).ok();

        let ctx = self.ctx.clone();
        _ = ctx.eval::<rquickjs::Value, &str>(r#"
            let h4Handler = {
                get(_target, key) {
                    key = key.toString()
                    return h4GetVariable(key)
                },

                has(_target, key) {
                    key = key.toString()
                    return h4GetVariable(key) !== undefined
                }
            }

            let h4Proxy = new Proxy({}, h4Handler)

            let values = []

            function h4Eval(script) {
                return Function("h4Proxy", 'with(h4Proxy) {return (' + script + ')}')(h4Proxy)
            }
        "#).expect("Cannot intialize QuickJS variables")
    }

    fn write(&mut self, chr: char) {
        if self.current_output == "stdout" {
            print!("{}", chr); // TODO: Make faster
        }
        if self.current_output == "stderr" {
            eprint!("{}", chr); // TODO: Make faster
        }
        if !self.outputs.contains_key(&self.current_output) {
            self.outputs.insert(self.current_output.clone(), String::new());
        }
        let output = self.outputs.get_mut(&self.current_output).expect("The value was not inserted");
        output.push(chr);
    }

    pub fn eval_js(&self, js: String) -> rquickjs::Value<'h> {
        let value = self.ctx.eval::<rquickjs::Function, &str>("h4Eval").expect("h4Eval not found");
        let result: rquickjs::Value = value.call((&js,)).expect("Could not evaluate");
        return result
    }

    pub fn js_value_to_string(&self, value: rquickjs::Value<'h>) -> String {
        let str = self.ctx.eval::<rquickjs::Function, &str>("String").expect("String not found");
        let result: rquickjs::String = str.call((value,)).expect("Could not evaluate");
        return result.to_string().unwrap_or_else(|e| format!("`QuickJS Error: {e}'"));
    }

    fn write_string(&mut self, str: String) {
        if self.current_output == "stdout" {
            print!("{}", str); // TODO: Make faster
        }
        if self.current_output == "stderr" {
            eprint!("{}", str); // TODO: Make faster
        }
        if !self.outputs.contains_key(&self.current_output) {
            self.outputs.insert(self.current_output.clone(), String::new());
        }
        let output = self.outputs.get_mut(&self.current_output).expect("The value was not inserted");
        output.push_str(&str);
    }

    fn insert_input(&mut self, str: String) {
        self.iter.insert_elements(str.chars().collect());
    }

    fn eval_macro(&mut self, value: &Value<'h>, args: &Vec<String>) -> String {
        match value {
            Value::Plain(str) => {
                let mut evaluated = "`'@pushScope\n".to_string();
                for (i, arg) in args.iter().enumerate() {
                    evaluated.push_str(&format!("@define(`@arg{}', `{}')\n", i, arg).to_string());
                }
                evaluated.push_str(str);
                evaluated.push_str("`'@popScope\n");
                return evaluated
            }
            Value::Builtin(func) => {
                return func(self, args)
            }
            Value::JS(value) => {
                if value.is_function() {
                    unimplemented!()
                }
                return self.js_value_to_string(value.clone());
            }
        }
    }

    fn advance(&mut self) -> Option<AdvanceResult> {
        let chr = *self.iter.peek()?;
        if self.quote_level > 0 {
            if chr == self.quote_start {
                self.quote_level += 1
            } else if chr == self.quote_end {
                self.quote_level -= 1
            }
            if self.quote_level > 0 {
                self.write(chr);
            }
            self.iter.next();
            return Some(AdvanceResult::QuoteChar)
        }
        if chr == self.quote_start {
            self.quote_level += 1;
            self.iter.next();
            return Some(AdvanceResult::EnterQuote);
        }
        if self.name_chars.contains(chr) {
            let name = self.consume_name();
            let variable = self.scopes.get_variable(&name).map(|x| x.clone());
            match variable {
                None => {
                    self.write_string(name);
                    return Some(AdvanceResult::Normal)
                }
                Some(value) => {
                    let mut args: Vec<String> = Vec::new();
                    if self.iter.peek() == Some(&'(') {
                        self.in_call = true;
                        self.iter.next();
                        let mut id = new_id();
                        let previous_output = self.current_output.clone();
                        self.current_output = id.clone();
                        loop {
                            let reason = self.advance().expect("Did not close call before EOF");
                            if reason == AdvanceResult::CallEnd || reason == AdvanceResult::NextArg {
                                args.push(
                                    self.outputs.get(&id)
                                        .map(|x| x.clone())
                                        .unwrap_or_else(|| String::new())
                                );
                                self.outputs.remove(&id); // TODO: Not borrow // TODO: Not borrow
                                id = new_id();
                                self.current_output = id.clone();
                            }
                            if reason == AdvanceResult::CallEnd {
                                break
                            }
                        }
                        self.current_output = previous_output;
                    }
                    let evaluated = self.eval_macro(&value.borrow(), &args);
                    self.insert_input(evaluated);
                    return Some(AdvanceResult::Macro)
                }
            }
        }
        if self.in_call {
            if chr == ')' {
                self.in_call = false;
                self.iter.next();
                return Some(AdvanceResult::CallEnd)
            }
            else if chr == ','{
                self.iter.next();
                self.iter.next(); // Skips space
                return Some(AdvanceResult::NextArg)
            }
        }
        self.write(chr);
        self.iter.next();
        return Some(AdvanceResult::Normal)
    }

    fn consume_name(&mut self) -> String {
        let mut name = String::new();
        loop {
            let chr = self.iter.peek();
            if chr.is_none() {
                break
            }
            let chr = *chr.unwrap();
            if !self.name_chars.contains(chr) {
                break
            }
            name.push(chr);
            self.iter.next();
        }
        return name
    }
}

