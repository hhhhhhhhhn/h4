use std::rc::Rc;
use crate::H4;
use crate::scopes::Value;
use std::process::Command;
use std::fs;
use rquickjs;

pub fn builtin_define(h4: &mut H4, args: &Vec<String>) -> String {
    let scopes = Rc::clone(&mut h4.scopes);
    scopes.let_variable(&args[0], Value::Plain(args[1].clone()));
    h4.iter.next();
    return String::new()
}

pub fn builtin_let(h4: &mut H4, args: &Vec<String>) -> String {
    h4.scopes.let_variable(&args[0], Value::JS(h4.eval_js(args[1].clone())));
    h4.iter.next();
    return String::new()
}

pub fn builtin_for(h4: &mut H4, args: &Vec<String>) -> String {
    if args.len() < 3 {
        panic!("Not enough arguments")
    }

    let name = &args[0];
    let list = h4.scopes.get_variable(&args[1]);
    let body = &args[2];


    match list {
        None => panic!("Error: {} does not exist", &args[0]),
        Some(value) => {
            let value = Rc::clone(&value);
            let value = value.borrow().clone();

            match value {
                Value::Plain(_) => {
                    panic!("Error: {} is a string, not a list.", &args[0])
                }
                Value::JS(js_value) => {
                    if !js_value.is_array() {
                        panic!("Error: {} is not a list.", &args[0])
                    }
                    let list = js_value.as_array().unwrap();

                    let mut evaluated = String::new();

                    for element in list.iter() {
                        match element {
                            Ok(v) => {
                                let v: rquickjs::Value = v;
                                let str = h4.js_value_to_string(v);
                                evaluated.push_str("`'@pushScope\n");
                                evaluated.push_str(&format!("@define(`{}', `{}')\n", name, str));
                                evaluated.push_str(body);
                                evaluated.push_str("`'@popScope\n");
                            },
                            Err(e) => {
                                panic!("Error: {}", e);
                            }
                        }
                    }

                    return evaluated
                }
                Value::Builtin(_) => {
                    panic!("Error: {} is a builtin function, not a list.", &args[0])
                }
            }
        }
    }
}

pub fn builtin_set(h4: &mut H4, args: &Vec<String>) -> String {
    h4.scopes.set_variable(&args[0], Value::JS(h4.eval_js(args[1].clone())));
    h4.iter.next();
    return String::new()
}

pub fn builtin_get(h4: &mut H4, args: &Vec<String>) -> String {
    return builtin_jseval(h4, args)
}

pub fn builtin_push_scope(h4: &mut H4, _args: &Vec<String>) -> String {
    h4.scopes.push_scope();
    h4.iter.next();
    return String::new()
}

pub fn builtin_pop_scope(h4: &mut H4, _args: &Vec<String>) -> String {
    h4.scopes.pop_scope();
    h4.iter.next();
    return String::new()
}

pub fn builtin_skip(h4: &mut H4, args: &Vec<String>) -> String {
    let mut times = 1;
    if let Some(arg) = args.get(0) {
        times = arg.parse::<i32>().unwrap_or(0);
    }
    for _ in 0..times {
        h4.iter.next();
    }
    return String::new()
}

pub fn builtin_jseval(h4: &mut H4, args: &Vec<String>) -> String {
    let value = h4.eval_js(args[0].clone());
    let value = h4.js_value_to_string(value);
    return value
}

pub fn builtin_import(h4: &mut H4, args: &Vec<String>) -> String {
    let value = match args.get(0) {
        Some(file) => {
            fs::read_to_string(file)
                .unwrap_or_else(|e| format!("`Could not read file {e}'"))
        }
        None => "`No file provided'".to_string(),
    };
    h4.iter.next();
    return value
}

pub fn builtin_dump(h4: &mut H4, _args: &Vec<String>) -> String {
    let scopes = Rc::clone(&h4.scopes.scopes);
    let scopes = scopes.borrow();
    for (i, stack) in scopes.iter().enumerate() {
        println!("Stack {i}:");
        for (key, value) in stack {
            let value = Rc::clone(value);
            let value = value.borrow().clone();
            match value {
                Value::Plain(str) => {
                    println!("{key}: {}", str.clone());
                },
                Value::Builtin(_) => {
                    println!("{key}: <Builtin>");
                }
                _ => unimplemented!()
            }
        }
    }
    h4.iter.next();
    return String::new()
}

pub fn run_shell(command: String) -> String {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output();
    return output
        .map(|o| {
            let mut stdout = String::from_utf8_lossy(&o.stdout).to_string();
            if stdout.chars().last().unwrap() == '\n' {
                stdout.pop();
            }
            return stdout
        })
        .unwrap_or_else(|e| format!("`Program exited with error: {e}'"));
}

pub fn builtin_shell(_h4: &mut H4, args: &Vec<String>) -> String {
    match args.get(0) {
        Some(command) => return run_shell(command.clone()),
        None => return "`Command not found'".to_string()
    }
}
