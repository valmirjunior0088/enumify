extern crate enumify;

enumify::enumify! {
    #[derive(Debug)]
    pub enum Term<V>;

    #[derive(Debug)]
    pub struct Var<V>(V);

    #[enumify(Box)]
    #[allow(dead_code)]
    #[derive(Debug)]
    pub struct App<V> {
        function: Term<V>,
        argument: Term<V>,
    }

    #[enumify(Box)]
    #[allow(dead_code)]
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
