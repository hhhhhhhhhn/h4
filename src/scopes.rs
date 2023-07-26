use crate::H4;
use std::rc::Rc;
use std::collections::HashMap;
use std::cell::RefCell;

#[derive(Clone)]
pub enum Value<'a> {
    JS(rquickjs::Value<'a>),
    Plain(String),
    Builtin(fn(&mut H4<'a>, &Vec<String>) -> String),
}

#[derive(Clone)]
pub struct Scopes<'a> {
    pub scopes: Rc<RefCell<
        Vec<HashMap<String, Rc<RefCell<Value<'a>>>>>
    >>,
}

impl<'a> Scopes<'a> {
    pub fn new() -> Scopes<'a> {
        Scopes{scopes: Rc::new(RefCell::new(vec![HashMap::new()]))}
    }

    pub fn push_scope(&self) {
        let scopes = Rc::clone(&self.scopes);
        let scopes = &mut scopes.borrow_mut();
        scopes.push(HashMap::new());
    }

    pub fn pop_scope(&self) {
        let scopes = Rc::clone(&self.scopes);
        let scopes = &mut scopes.borrow_mut();
        assert!(scopes.len() > 1, "Cannot pop the global scope, push a new one first!");
        scopes.pop();
    }

    pub fn get_variable(&self, name: &String) -> Option<Rc<RefCell<Value<'a>>>> {
        let scopes = Rc::clone(&self.scopes);
        let scopes = &mut scopes.borrow_mut();
        for scope in scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                let value = Rc::clone(value);
                return Some(value);
            }
        }
        return None;
    }

    pub fn set_variable(&self, name: &String, value: Value<'a>) -> Option<()> {
        let var = self.get_variable(name)?;
        *var.borrow_mut() = value;
        return Some(())
    }

    pub fn let_variable(&self, name: &String, value: Value<'a>) {
        let scopes = &mut self.scopes.borrow_mut();
        let scope = scopes.last_mut().expect("The scope stack is empty");
        scope.insert(name.clone(), Rc::new(RefCell::new(value)));
    }
}
