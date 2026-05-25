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

pub fn member_access_on_non_set(span: FullSpan) -> Diagnostic {
    error!("EE0003", "The member access operator can only be used on sets", [
        {"Here", span, primary},
    ])
}

pub fn no_attribute_with_name(span: FullSpan, name: &str) -> Diagnostic {
    error!("EE0004", format!("No attribute found with name `{name}`"), [
        {"Here", span, primary},
    ])
}
