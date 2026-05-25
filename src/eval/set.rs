use mulch_macros::{GCDebug, GCPtr};

use crate::{
    error::{DResult, FullSpan, Spanned},
    eval::{self, Evaluator, MValue, lazyvalue::LazyValue},
    gc::{GCString, GCVec},
    parser::ast::{self, MemberAccess, NamedValue},
};

#[derive(Clone, GCDebug, GCPtr)]
pub struct Set {
    values: GCVec<NamedMValue>,
}

#[derive(Clone, GCDebug, GCPtr)]
struct NamedMValue {
    name: GCString,
    value: LazyValue,
}

impl<'gc> Evaluator<'gc> {
    pub(super) fn evaluate_set(&self, ast: Spanned<ast::Set>) -> DResult<MValue> {
        let ast_attributes: GCVec<NamedValue> = ast.0.0.0.values;

        let output_vec =
            unsafe { GCVec::<NamedMValue>::new_uninit(self.gc, ast_attributes.len(self.gc)) };

        let output_vec_ptr = unsafe { output_vec.as_mut_ptr(self.gc) };

        // NOTE: the following is sound because we're not using any functions that can trigger a
        // garbage-collection cycle. Normally, you would have to worry about a garbage-collection
        // cycle invalidating `ast_attributes`.

        for (i, attr_definition) in unsafe { ast_attributes.as_slice(self.gc) }
            .iter()
            .enumerate()
        {
            let attr_name = unsafe { attr_definition.name.0.0.get(self.gc) };

            // We need to insert into the `Set` value so that the names are ordered. We also need to
            // ensure that there are no duplicate attributes.

            let output_slice = unsafe { std::slice::from_raw_parts(output_vec_ptr, i) };
            let binary_search_result =
                unsafe { output_slice.binary_search_by_key(&attr_name, |a| a.name.get(self.gc)) };

            let idx = match binary_search_result {
                Err(idx) => idx,
                Ok(idx) => {
                    let first_attribute_definition =
                        unsafe { output_slice[idx].value.to_ast(self.gc).unwrap().1 };

                    return Err(eval::error::attribute_defined_multiple_times(
                        first_attribute_definition,
                        FullSpan::new(attr_definition.value.1, ast.1.file_id),
                    ));
                }
            };

            // Then we must manually use memcpy to insert the attribute into the `GCVec`
            unsafe {
                std::ptr::copy(
                    output_vec_ptr.add(idx),
                    output_vec_ptr.add(idx + 1),
                    i - idx,
                );
                output_vec_ptr.add(idx).write(NamedMValue {
                    name: attr_definition.name.0.0.clone(),
                    value: LazyValue::from_ast(
                        self.gc,
                        attr_definition.value.clone().with_file_id(ast.1.file_id),
                    ),
                });
            }
        }

        Ok(MValue::Set(Set { values: output_vec }))
    }

    pub(super) fn evaluate_member_access(&self, ast: Spanned<MemberAccess>) -> DResult<MValue> {
        unsafe {
            let ast_span = ast.1;
            let ast = self.gc.push_root(ast.0);

            let lhs = ast.get().lhs.get(self.gc).with_file_id(ast_span.file_id);
            let lhs = self.evaluate(lhs)?;

            let MValue::Set(lhs) = lhs else {
                return Err(eval::error::member_access_on_non_set(ast_span));
            };

            let ast_val = ast.get();
            let rhs = ast_val.rhs.0.get(self.gc);

            let matching_attr = match lhs
                .values
                .as_slice(self.gc)
                .binary_search_by_key(&rhs, |a| a.name.get(self.gc))
            {
                Ok(idx) => lhs.values.as_slice(self.gc)[idx].clone(),
                Err(_) => return Err(eval::error::no_attribute_with_name(ast_span, rhs)),
            };

            std::mem::drop(ast);

            matching_attr.value.get_or_evaluate(self, ast_span)
        }
    }
}
