use crate::{
    Consumes, Converter, ConvertingFn, FunctionInfo, FunctionVisitor, GobbindFunctions, Provides,
};
use gobbind_macros::{Gobbind, gobbind};

// TODO: remove this after writing actual docs etc

#[derive(Debug)]
struct ExampleAnnotation(u64);

#[derive(Gobbind)]
struct TestType;

#[gobbind]
impl TestType {
    #[gobbind(rename = "test")]
    #[gobbind(@ExampleAnnotation(10))]
    pub fn example(a: &i64, b: i64) -> String {
        (*a + b).to_string()
    }

    #[gobbind(skip)]
    pub fn example_two(a: &i64, b: i64) -> String {
        (*a + b).to_string()
    }
}

#[derive(Debug)]
enum Variant {
    Nil,
    Int(i64),
    String(String),
}

#[test]
pub fn x() {
    struct ExampleVisitor<'env, 'converter> {
        thunks: Vec<Box<dyn Fn(&'converter [Variant]) -> Variant + 'env>>,
    }

    struct ExampleConverter<'a> {
        idx: usize,
        inputs: &'a [Variant],
        output: Variant,
    }

    impl Converter<'_> for ExampleConverter<'_> {
        type Error = ();
    }

    impl Provides<'_, i64> for ExampleConverter<'_> {
        fn provide(&mut self) -> Result<i64, Self::Error> {
            let Variant::Int(x) = &self.inputs[self.idx] else {
                return Err(());
            };

            self.idx += 1;
            Ok(*x)
        }
    }

    impl<'converter> Provides<'converter, &'converter i64> for ExampleConverter<'converter> {
        fn provide(&mut self) -> Result<&'converter i64, Self::Error> {
            let Variant::Int(x) = &self.inputs[self.idx] else {
                return Err(());
            };

            self.idx += 1;
            Ok(x)
        }
    }

    impl Consumes<'_, String> for ExampleConverter<'_> {
        fn consume(&mut self, value: String) -> Result<(), Self::Error> {
            self.output = Variant::String(value);
            Ok(())
        }
    }

    impl<'env, 'converter> FunctionVisitor<'env, 'converter> for ExampleVisitor<'env, 'converter> {
        type Converter = ExampleConverter<'converter>;

        fn visit<Imp, ImpDummy>(&mut self, function: FunctionInfo<Imp, (Self::Converter, ImpDummy)>)
        where
            Imp: ConvertingFn<'env, 'converter, Self::Converter, ImpDummy>,
        {
            println!("{}", &function.name());

            if let Some(example_attribute) = function.custom_attributes().get::<ExampleAnnotation>()
            {
                println!("test {example_attribute:?}");
            }

            let thunk = move |arguments: &'converter [Variant]| -> Variant {
                let mut converter = ExampleConverter {
                    idx: 0,
                    inputs: arguments,
                    output: Variant::Nil,
                };

                function.imp.converting_call(&mut converter).unwrap();
                converter.output
            };

            self.thunks.push(Box::new(thunk));
        }
    }

    let mut visitor = ExampleVisitor { thunks: Vec::new() };

    TestType::walk_functions(&mut visitor);

    for thunk in &visitor.thunks {
        let result = thunk(&[Variant::Int(10), Variant::Int(15)]);
        dbg!(result);
    }
}
