use crate::gdextension::api::GDExtensionAPI;
use gobbind::{DynamicFn, FunctionInfo, FunctionVisitor, GobbindFunctions};

#[derive(Debug)]
pub struct GDExtensionConverter {}

struct GDExtensionVisitor<'api> {
    api: &'api GDExtensionAPI,
}

impl<'env, 'converter> FunctionVisitor<'env, 'converter> for GDExtensionVisitor<'_> {
    type Converter = GDExtensionConverter;

    fn visit<Imp, ImpDummy>(&mut self, function: FunctionInfo<Imp, (Self::Converter, ImpDummy)>)
    where
        Imp: DynamicFn<'env, 'converter, Self::Converter, ImpDummy> + 'static,
    {
        // generate `call` and `ptrcall` stubs
    }
}

pub fn generate_binding<T>(api: &GDExtensionAPI)
where
    T: GobbindFunctions<'_, '_, GDExtensionConverter>,
{
    let mut visitor = GDExtensionVisitor { api };

    T::walk_functions(&mut visitor);
}
