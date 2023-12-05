extern crate enumify;

enumify::enumify! {
    #[derive(Debug)]
    pub enum Term;

    #[derive(Debug)]
    pub enum Var {
        Free(String),
        Bound(usize),
    }

    #[enumify(Box)]
    #[allow(dead_code)]
    #[derive(Debug)]
    pub struct App {
        function: Term,
        argument: Term,
    }

    #[enumify(Box)]
    #[derive(Debug)]
    pub struct Abs {
        #[allow(dead_code)]
        variable: String,

        #[allow(dead_code)]
        body: Term,
    }
}

impl From<String> for Term {
    fn from(value: String) -> Self {
        Self::from(Var::Free(value))
    }
}

impl From<&str> for Term {
    fn from(value: &str) -> Self {
        Self::from(Var::Free(value.to_owned()))
    }
}

#[test]
fn it_works() {
    let _ = Term::from(App {
        function: Term::from(Abs {
            variable: String::from("x"),
            body: Term::from("x"),
        }),
        argument: Term::from("y"),
    });
}
