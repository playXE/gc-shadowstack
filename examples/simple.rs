use gc_shadowstack::gc_shadowstack;
pub trait Traceable: core::fmt::Display {
    fn trace(&self) {
        println!("Trace '{}'", self);
    }
}
impl<T: Traceable> Rootable for T {}
gc_shadowstack!(ShadowStack, Traceable, Rootable, Root, Handle, letroot);

impl<T: core::fmt::Display> Traceable for T {}

fn main() {
    let stack = ShadowStack::new();
    letroot!(int: i64 = stack, 42);
    letroot!(_str = stack, "Hello,World!");
    {
        letroot!(_will_not_be_traced = stack, "hi!");
        // will_not_be_traced is unlinked from shadow stack when reaches end of its lifetime.
    }
    unsafe {
        stack.walk(|value| {
            value.trace();
        });

        *int = 44;
        stack.walk(|value| {
            value.trace();
        });
    }
}
