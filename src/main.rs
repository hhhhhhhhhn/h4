mod h4;
mod insertable;
mod scopes;
mod builtin;

use h4::H4;
use rquickjs::{Runtime, Context};
use insertable::InsertableIterator;
use arg::{Args, parse_args};

#[derive(Args, Debug)]
///h4
struct Arguments {
    ///File to be processed. If not specified, stdin is used.
    file: String
}

fn main() {
    let args: Arguments = parse_args();
    let input = if args.file == "" {
        std::io::read_to_string(std::io::stdin()).unwrap()
    } else {
        std::io::read_to_string(std::fs::File::open(args.file).unwrap()).unwrap()
    };

    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();
    let boxed: Box<dyn Iterator<Item = char>> = Box::new(input.chars());
    let insertable = InsertableIterator::from(boxed);

    context.with(move |ctx| {
        let mut h4 = H4::new(insertable, ctx);
        h4.consume();
    })
}
