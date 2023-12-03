# enumify

A Rust macro that declares an `enum` (and a bunch of `impl From`s) based on a set of `struct`s.

## Examples

There are two poster child cases where the code generation of `enumify` can cut down on a lot of boilerplate: **dispatching on an `enum`** and **abstract syntax trees**.

### Dispatching on an `enum`

When working with threads, a very common approach is to organize them like **agents**: the thread communicates with other threads through **message-passing** using asynchronous channels such as `crossbeam::channel` or `tokio::sync::mpsc` and the thread has a pre-determined set of messages that it can handle. This approach makes use of a "main loop" that may resemble the following code snippet:

```rust
while let Ok(message) = receiver.recv() {
    match message {
        Message::Foo(foo) => self.handle_foo(foo),
        Message::Bar(bar) => self.handle_bar(bar),
        Message::Oof(oof) => self.handle_oof(oof),
    }
}
```

While this may look harmless and quite pleasant to look at at the surface level, the *o*blivious *o*nlooker has no idea of the amount of *b*elligerent *b*oilerplate that was manually written in order to both declare a `struct` for each of the variants and then an `enum Message` containing a variant for each of the `struct`s. This is motivating enough, but 9 times out of 10 there's also a barrage of `impl From` waiting just ahead.

`enumify` solves this problem by generating the repetitive part automatically. For example, declaring `Message` and the associated conversions is as easy declaring a `struct` for each of the variants:

```rust
enumify::enumify! {
    #[derive(Debug)]
    pub enum Message;

    #[derive(Debug)]
    pub struct Foo {
        here: usize,
        there: usize,
    }

    #[derive(Debug, Deserialize)]
    pub struct Bar {
        one: usize,

        #[serde(rename = "another")]
        other: usize,
    }

    #[derive(Debug)]
    pub struct Oof(String);
}
```

You can declare `impl`s targeting the `enum` and the `struct`s as if you had declared them yourself:

```rust
impl Foo {
    pub fn sum(&self) -> usize {
        self.here + self.there
    }
}
```

The automatically derived conversions make constructing a new message just as easy:

```rust
pub fn send(sender: Sender<Message>, message: impl Into<Message>) {
    sender.send(message.into()).expect("looks good");
}

impl Foo {
    pub fn do_the_thing(&self, sender: Sender<Message>) {
        let message = Message::from(Bar {
            one: 1 + self.here,
            other: 11 + self.there,
        });

        sender.send(message).expect("looks good");
    }

    pub fn do_another_thing(&self, sender: Sender<Message>) {
        send(sender, Bar {
            one: 1 + self.here,
            other: 11 + self.there,
        });
    }
}
```

### Abstract syntax trees

Working with recursive types in Rust is ever so very slightly annoying. One `Box` here, another `Rc` there and it's generally fine. The problem is that the annoyance scales linearly with the amount of wrapping that needs to be done. One of the cases where this annoyance reaches critical mass is with abstract syntax trees. Fortunately, `enumify` can help us here too: declaring an `#[enumify(Wrapper)]` attribute on top of some `struct` wraps the corresponding variant in the `enum` with the type specificed in the attribute. For example, declaring a type for the untyped lambda calculus looks similar to the following:

```rust
enumify::enumify! {
    #[derive(Debug)]
    pub enum Term<V>;

    #[derive(Debug)]
    pub struct Var<V>(V);

    #[enumify(Box)]
    #[derive(Debug)]
    pub struct App<V> {
        function: Term<V>,
        argument: Term<V>,
    }

    #[enumify(Box)]
    #[derive(Debug)]
    pub struct Abs<V> {
        variable: V,
        body: Term<V>,
    }
}

impl From<usize> for Term<usize> {
    fn from(value: usize) -> Self {
        Self::from(Var(value))
    }
}

#[test]
fn it_works() {
    let _ = Term::from(App {
        function: Term::from(Abs {
            variable: 0,
            body: Term::from(0),
        }),
        argument: Term::from(1),
    });
}
```
