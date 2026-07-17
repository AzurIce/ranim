pub struct ComplexStruct {
    a: u32,
    b: String,
    c: bool,
}

pub trait FooTrait {
    fn foo(c: ComplexStruct) {}
}

struct S;

impl FooTrait for S {
    fn foo(ComplexStruct { a, b, c }: ComplexStruct) {}
}
