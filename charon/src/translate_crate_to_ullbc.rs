use crate::get_mir::{extract_constants_at_top_level, MirLevel};
use crate::meta;
use crate::names::{hir_item_to_name, item_def_id_to_name};
use crate::reorder_decls as rd;
use crate::translate_ctx::*;
use crate::translate_functions_to_ullbc;
use crate::types as ty;
use crate::ullbc_ast as ast;
use linked_hash_set::LinkedHashSet;
use rustc_hir::{Defaultness, ImplItem, ImplItemKind, Item, ItemKind};
use rustc_middle::ty::TyCtxt;
use rustc_session::Session;
use std::collections::HashMap;

impl<'tcx, 'ctx> TransCtx<'tcx, 'ctx> {
    fn register_local_hir_impl_item(&mut self, _top_item: bool, impl_item: &ImplItem) {
        // TODO: make a proper error message
        assert!(impl_item.defaultness == Defaultness::Final);

        // Match on the impl item kind
        match &impl_item.kind {
            ImplItemKind::Const(_, _) => unimplemented!(),
            ImplItemKind::Type(_) => {
                // Note sure what to do with associated types yet
                unimplemented!();
            }
            ImplItemKind::Fn(_, _) => {
                let local_id = impl_item.owner_id.to_def_id().as_local().unwrap();
                let _ = self.translate_fun_decl_id(local_id.to_def_id());
            }
        }
    }

    /// General function to register a MIR item. It is called on all the top-level
    /// items. This includes: crate inclusions and `use` instructions (which are
    /// ignored), but also type and functions declarations.
    /// Note that this function checks if the item has been registered, and adds
    /// its def_id to the list of registered items otherwise.
    ///
    /// `stack`: the stack of definitions we explored before reaching this one.
    /// This is useful for debugging purposes, to check how we reached a point
    /// (in particular if we want to figure out where we failed to consider a
    /// definition as opaque).
    fn register_local_hir_item(&mut self, top_item: bool, item: &Item) {
        trace!("{:?}", item);

        // The annoying thing is that when iterating over the items in a crate, we
        // iterate over *all* the items, which is a problem with regards to the
        // *opaque* modules: we see all the definitions which are in there, and
        // not only those which are transitively reachable from the root.
        // Because of this, we need the following check: if the item is a "top"
        // item (not an item transitively reachable from an item which is not
        // opaque) and inside an opaque module (or sub-module), we ignore it.
        if top_item {
            match hir_item_to_name(self.tcx, item) {
                Option::None => {
                    // This kind of item is to be ignored
                    return;
                }
                Option::Some(item_name) => {
                    if self.crate_info.is_opaque_decl(&item_name) {
                        return;
                    }
                    // Continue
                }
            }
        }

        // Case disjunction on the item kind.
        let def_id = item.owner_id.to_def_id();
        match &item.kind {
            ItemKind::TyAlias(_, _) => {
                // We ignore the type aliases - it seems they are inlined
            }
            ItemKind::OpaqueTy(_) => unimplemented!(),
            ItemKind::Union(_, _) => unimplemented!(),
            ItemKind::Enum(_, _) | ItemKind::Struct(_, _) => {
                let _ = self.translate_type_decl_id(def_id);
            }
            ItemKind::Fn(_, _, _) => {
                let _ = self.translate_fun_decl_id(def_id);
            }
            ItemKind::Const(_, _) | ItemKind::Static(_, _, _) => {
                if extract_constants_at_top_level(self.mir_level) {
                    let _ = self.translate_global_decl_id(def_id);
                } else {
                    // Avoid registering globals in optimized MIR (they will be inlined)
                }
            }

            ItemKind::Impl(impl_block) => {
                trace!("impl");
                // Sanity checks - TODO: remove?
                translate_functions_to_ullbc::check_impl_item(impl_block);

                // Explore the items
                let hir_map = self.tcx.hir();
                for impl_item_ref in impl_block.items {
                    // impl_item_ref only gives the reference of the impl item:
                    // we need to look it up
                    let impl_item = hir_map.impl_item(impl_item_ref.id);

                    self.register_local_hir_impl_item(false, impl_item);
                }
            }
            ItemKind::Use(_, _) => {
                // Ignore
            }
            ItemKind::ExternCrate(_) => {
                // Ignore
            }
            ItemKind::Mod(module) => {
                trace!("module");

                // Explore the module, only if it was not marked as "opaque"
                // TODO: we may want to accumulate the set of modules we found,
                // to check that all the opaque modules given as arguments actually
                // exist
                trace!("{:?}", def_id);
                let module_name = item_def_id_to_name(self.tcx, def_id);
                let opaque = self.id_is_opaque(def_id);
                if opaque {
                    // Ignore
                    trace!("Ignoring module [{}] because marked as opaque", module_name);
                } else {
                    trace!("Diving into module [{}]", module_name);
                    let hir_map = self.tcx.hir();
                    for item_id in module.item_ids {
                        // Lookup and register the item
                        let item = hir_map.item(*item_id);
                        self.register_local_hir_item(false, item);
                    }
                }
            }
            _ => {
                unimplemented!("{:?}", item.kind);
            }
        }
    }
}

/// Translate all the declarations in the crate.
pub fn translate<'tcx, 'ctx>(
    crate_info: CrateInfo,
    sess: &'ctx Session,
    tcx: TyCtxt<'tcx>,
    mir_level: MirLevel,
) -> TransCtx<'tcx, 'ctx> {
    let mut ctx = TransCtx {
        sess,
        tcx,
        mir_level,
        crate_info,
        all_ids: LinkedHashSet::new(),
        stack: LinkedHashSet::new(),
        file_to_id: HashMap::new(),
        id_to_file: HashMap::new(),
        real_file_counter: meta::LocalFileId::Generator::new(),
        virtual_file_counter: meta::VirtualFileId::Generator::new(),
        type_id_map: ty::TypeDeclId::MapGenerator::new(),
        type_defs: ty::TypeDeclId::Map::new(),
        fun_id_map: ast::FunDeclId::MapGenerator::new(),
        fun_defs: ast::FunDeclId::Map::new(),
        global_id_map: ast::GlobalDeclId::MapGenerator::new(),
        global_defs: ast::GlobalDeclId::Map::new(),
    };

    // First push all the items in the stack of items to translate.
    //
    // The way rustc works is as follows:
    // - we call it on the root of the crate (for instance "main.rs"), and it
    //   explores all the files from there (typically listed through statements
    //   of the form "mod MODULE_NAME")
    // - the other files in the crate are Module items in the HIR graph
    let hir = tcx.hir();
    for item_id in hir.items() {
        let item_id = item_id.hir_id();
        let node = hir.find(item_id).unwrap();
        let item = match node {
            rustc_hir::Node::Item(item) => item,
            _ => unreachable!(),
        };
        ctx.register_local_hir_item(true, item);
    }

    // Translate.
    //
    // For as long as the stack of items to translate is not empty, we pop the top item
    // and translate it. Note that we transitively translate items: if an item refers to
    // non-translated (potentially external) items, we add them to the stack.
    //
    // Note that the order in which we translate the definitions doesn't matter:
    // we never need to lookup a translated definition, and only use the map
    // from Rust ids to translated ids.
    while let Some(id) = ctx.stack.pop_front() {
        match id {
            rd::AnyDeclId::Type(id) => ctx.translate_type(id),
            rd::AnyDeclId::Fun(id) => ctx.translate_function(id),
            rd::AnyDeclId::Global(id) => ctx.translate_global(id),
        }
    }

    // Return the context
    ctx
}
