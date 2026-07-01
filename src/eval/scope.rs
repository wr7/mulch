use mulch_macros::{GCDebug, GCProject, GCPtr};

use crate::{
    eval::Set,
    gc::{
        GCBox,
        safety::{GC, GCCtx, Projected},
    },
};

#[derive(Clone, Copy, GCPtr, GCDebug, GCProject)]
pub struct Scope {
    pub parent: Option<GCBox<Scope>>,
    pub variables: Set,
}

impl Scope {
    pub fn new_global<'c>(ctx: &'c GCCtx) -> GC<'c, Scope> {
        let parent = unsafe { GC::new(ctx, None) }; // TODO: make this safe

        Projected::<Scope> {
            parent,
            variables: Set::new_empty(ctx),
        }
        .into()
    }
}
