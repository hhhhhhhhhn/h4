use rquickjs::{Runtime, Context, Function, Ctx};
use std::process::Command;
use std::iter::Peekable;
use std::boxed::Box;
use std::collections::HashMap;
use std::time;

// NOTE: Read https://stackoverflow.com/questions/9781285/specify-scope-for-eval-in-javascript

#[derive(Clone)]
enum Value<'a> {
    JS(rquickjs::Value<'a>),
    Plain(String),
    Builtin(fn(&mut H4<'a>, &Vec<String>) -> String),
}


#[derive(PartialEq, Eq, Debug)]
enum AdvanceResult {
    EnterQuote,
    QuoteChar,
    Macro,
    Normal,
    CallEnd,
    NextArg,
}

struct H4<'a> {
    iter: Peekable<Box<dyn Iterator<Item = char>>>,
    outputs: HashMap<String, String>,
    current_output: String,
    scopes: Vec<HashMap<String, Value<'a>>>,
    ctx: Ctx<'a>,
    quote_level: usize,
    in_call: bool,

    name_chars: String,
    quote_start: char,
    quote_end: char,
}

fn new_id() -> String {
    // TODO: Ensure they are unique
    let timestamp = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("Cannot get system time");
    return format!("temp{}", timestamp.as_micros())
}

fn builtin_define(h4: &mut H4, args: &Vec<String>) -> String {
    h4.let_variable(&args[0], Value::Plain(args[1].clone()));
    h4.iter.next();
    return String::new()
}

fn builtin_push_scope(h4: &mut H4, _args: &Vec<String>) -> String {
    h4.push_scope();
    h4.iter.next();
    return String::new()
}

fn builtin_pop_scope(h4: &mut H4, _args: &Vec<String>) -> String {
    h4.pop_scope();
    h4.iter.next();
    return String::new()
}

fn builtin_skip(h4: &mut H4, _args: &Vec<String>) -> String {
    h4.iter.next();
    return String::new()
}

impl<'h> H4<'h> {
    fn new<'a>(iter: Box<dyn Iterator<Item = char>>, ctx: Ctx<'a>) -> H4<'a> {
            let iter = iter.peekable();
            let outputs = HashMap::new();
            let mut global_scope = HashMap::new();

            global_scope.insert("@define".to_string(), Value::Builtin(builtin_define));
            global_scope.insert("@pushScope".to_string(), Value::Builtin(builtin_push_scope));
            global_scope.insert("@popScope".to_string(), Value::Builtin(builtin_pop_scope));
            global_scope.insert("@skip".to_string(), Value::Builtin(builtin_skip));

            let scopes = vec![global_scope];
            
            // TODO: Intialize javascript proxy stuff

            return H4{
                iter,
                outputs,
                scopes,
                ctx,

                current_output: "stdout".to_string(),
                name_chars: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_@".to_string(),
                quote_start: '`',
                quote_end: '\'',
                quote_level: 0,
                in_call: false,
            }
    }

    fn get_variable(&self, name: &String) -> Option<&Value<'h>> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value);
            }
        }
        return None;
    }
    
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        assert!(self.scopes.len() > 1, "Cannot pop the global scope, push a new one first!");
        self.scopes.pop();
    }

    fn get_variable_mut(&mut self, name: &String) -> Option<&mut Value<'h>> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(value) = scope.get_mut(name) {
                return Some(value);
            }
        }
        return None;
    }

    fn set_variable(&mut self, name: &String, value: Value<'h>) -> Option<()> {
        let var = self.get_variable_mut(name)?;
        *var = value;
        return Some(())
    }

    fn let_variable(&mut self, name: &String, value: Value<'h>) {
        let scope = self.scopes.last_mut().expect("The scope stack is empty");
        scope.insert(name.clone(), value);
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

    fn eval_macro(&mut self, value: &Value<'h>, args: &Vec<String>) -> String {
        match value {
            Value::Plain(str) => {
                return str.clone()
            }
            Value::Builtin(func) => {
                return func(self, args)
            }
            _ => unimplemented!(),
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
            let variable = self.get_variable(&name).map(|x| x.clone());
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
                    let evaluated = self.eval_macro(&value, &args);
                    self.write_string(evaluated);
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

const TEST: &str = r#"word()
@define(`word', `AMAZING')
word
word
@pushScope
@define(`word', `COOL')
word word()
asd
@popScope
word(asdasd, asdasd),
`quoted `quotes word `quotes'' text'
"#;

fn main() {
    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();
    let iter = TEST.chars();
    context.with(|ctx| {
        let mut h4 = H4::new(Box::new(iter), ctx);
        h4.let_variable(&"word".to_string(), Value::Plain("WORD".to_string()));
        for _ in 0..1000 {
            h4.advance();
        }
    })
}
