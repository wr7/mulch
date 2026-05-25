use crate::error::{Diagnostic, FullSpan, error};

pub fn illegal_recursively_defined_value(
    definition_span: FullSpan,
    usage_span: FullSpan,
) -> Diagnostic {
    if definition_span == usage_span {
        error!("EE0001", "Illegal recursively defined value", [{"here", definition_span, primary}])
    } else {
        error!("EE0001", "Illegal recursively defined value", [
            {"Value is defined here", definition_span, secondary},
            {"The value depends on itself here", usage_span, primary},
        ])
    }
}

pub fn attribute_defined_multiple_times(
    first_definition: FullSpan,
    second_definition: FullSpan,
) -> Diagnostic {
    error!("EE0002", "Attribute defined multiple times", [
        {"First defined here", first_definition, secondary},
        {"Then defined here", second_definition, primary},
    ])
}
