use maos_spirit_sdk::spirit;

pub struct Foo;

#[spirit]
impl Foo {
    fn on_idel(&self) {}
}

fn main() {}
