use crate::{Consumes, Converter, DynamicFn, Provides};
use gobbind::FunctionInfo;
use gobbind_macros::gobbind;

// TODO: remove this after writing actual docs etc

#[derive(Debug)]
struct ExampleAnnotation(u64);

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

    let info = FunctionInfo::of_val(TestType::example).unwrap();
    println!("{}", info.name());

    if let Some(attr) = info.custom_attributes().get::<ExampleAnnotation>() {
        println!("{attr:?}");
    }

    let thunk = move |arguments: &[Variant]| -> Variant {
        let mut converter = ExampleConverter {
            idx: 0,
            inputs: arguments,
            output: Variant::Nil,
        };

        TestType::example.call_dynamic(&mut converter).unwrap();
        converter.output
    };

    let result = thunk(&[Variant::Int(10), Variant::Int(15)]);
    dbg!(result);
}
