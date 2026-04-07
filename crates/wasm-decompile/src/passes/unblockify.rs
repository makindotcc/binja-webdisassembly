use crate::{Block, Module, Pass, PassContext};

pub struct UnblockifyPass;

impl Pass for UnblockifyPass {
    fn name(&self) -> &'static str {
        "unblockify"
    }

    fn run(&self, module: &mut Module, _ctx: &mut PassContext) {
        for func in &mut module.functions {
            if !func.is_import {
                if func.name.as_ref().is_some_and(|name| name == "factorial") {
                    unblockify(&mut func.body);
                }
            }
        }
    }
}

/// Convert:
/// ```js
/// function factorial(p0) {
///   let l0, l1, l2, l3, l4;
///   l0 = 1;
///   block_0: {
///     if (p0 < 2) break block_0;
///     l1 = (p0 - 1);
///     l2 = (l1 & 7);
///     l0 = 1;
///     if (u32(p0 - 2) >= 7) {
///       l3 = 0;
///       l4 = (0 - (l1 & -8));
///       l0 = 1;
///       do {
///         l1 = (p0 + l3);
///         l0 = ((((((((l0 * l1) * (l1 - 1)) * (l1 - 2)) * (l1 - 3)) * (l1 - 4)) * (l1 - 5)) * (l1 - 6)) * (l1 - 7));
///         l3 = (l3 - 8);
///       } while (l4 !== l3);
///       if ((l2 === 0)) break block_0;
///       p0 = (p0 + l3);
///     }
///     do {
///       l0 = (l0 * p0);
///       p0 = (p0 - 1);
///       l2 = (l2 - 1);
///     } while (l2);
///   }
///   return l0;
/// }
/// ```
/// into:
/// ```js
///
/// ```
fn unblockify(_block: &mut Block) {
    // TODO: implement unblockify pass
}
