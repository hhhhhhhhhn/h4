mod h4;
mod insertable;
mod scopes;

use h4::H4;
use rquickjs::{Runtime, Context};
use insertable::InsertableIterator;

fn main() {
    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();
    let stdin = std::io::read_to_string(std::io::stdin()).unwrap();
    let str = stdin.to_string();
    let boxed: Box<dyn Iterator<Item = char>> = Box::new(str.chars());
    let insertable = InsertableIterator::from(boxed);

    context.with(move |ctx| {
        let mut h4 = H4::new(insertable, ctx);
        h4.consume();
    })
}
