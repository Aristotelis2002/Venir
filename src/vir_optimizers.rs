use std::sync::Arc;
use crate::consts::VIR_CRATE_NAME;
use rust_verify::{verifier::Verifier, import_export::ImportOutput};
use vir::ast::{Krate, VirErrAs};

/// Sorts the vir_crate, merges it with the imported crates (vstd vir crate),
/// prunes the result, processes traits, simplifies functions.
/// The finalized vir_crate gets added to the verifier structure.
pub fn optimize_vir_crate(verifier: &mut Verifier, vir_crate: Krate, imported: ImportOutput) {
    // We don't collect external traits. I assume that we may not need them
    // collect_external_trait_impls(). It's hard to recreate it

    let vir_crate = vir::ast_sort::sort_krate(&vir_crate);

    verifier.current_crate_modules = Some(vir_crate.modules.clone());
    // I assume that we don't need the item to module maps
    // verifier.item_to_module_map = Some(Arc::new(item_to_module_map));

    let mut current_vir_crate = vir_crate.clone();
    // Merge
    let mut vir_crates: Vec<Krate> = imported.vir_crates;
    vir_crates.push(vir_crate);
    let unpruned_crate = vir::ast_simplify::merge_krates(vir_crates).unwrap(); //TODO Remove unwrap
    let (vir_crate, _, _, _, _) = vir::prune::prune_krate_for_module_or_krate(
        &unpruned_crate,
        &Arc::new(VIR_CRATE_NAME.to_string()),
        Some(&current_vir_crate),
        None,
        None,
        false,
    );

    let vir_crate = vir::traits::merge_external_traits(vir_crate).unwrap(); //TODO Remove unwrap

    Arc::make_mut(&mut current_vir_crate).arch.word_bits = vir_crate.arch.word_bits;

    // If we want to export
    // rust_verify::import_export::export_crate(&verifier.args, crate_metadata, current_vir_crate.clone()).unwrap();
    let vir_crate = vir::traits::inherit_default_bodies(&vir_crate).unwrap();
    let vir_crate = vir::traits::fixup_ens_has_return_for_trait_method_impls(vir_crate).unwrap();
    let mut diags: Vec<VirErrAs> = Vec::new();
    let check_crate_result1 = vir::well_formed::check_one_crate(&current_vir_crate);
    let check_crate_result = vir::well_formed::check_crate(
        &vir_crate,
        unpruned_crate,
        &mut diags,
        verifier.args.no_verify,
    );
    for diag in diags.drain(..) {
        match diag {
            vir::ast::VirErrAs::Warning(err) => {
                println!("Warning: {}", err.note);
                // diagnostics.report_as(&err.to_any(), MessageLevel::Warning)
                //TODO There is a air::message::Diagnostics trait which can be implemented for better error reporting.
                // In the future we will implement a Reporter struct in the style which Verus does it.
            }
            vir::ast::VirErrAs::Note(err) => {
                println!("Note: {}", err.note);
                // diagnostics.report_as(&err.to_any(), MessageLevel::Note)
            }
        }
    }
    check_crate_result1.unwrap(); //TODO Better error handling
    check_crate_result.unwrap(); //TODO Better error handling

    let vir_crate = vir::autospec::resolve_autospec(&vir_crate).unwrap();
    let (vir_crate, _erasure_modes) = vir::modes::check_crate(&vir_crate).unwrap();

    verifier.vir_crate = Some(vir_crate.clone());
    verifier.crate_name = Some(VIR_CRATE_NAME.to_string());
    let mut crate_names: Vec<String> = vec![VIR_CRATE_NAME.to_string().clone()];
    crate_names.extend(imported.crate_names.into_iter());
    verifier.crate_names = Some(crate_names);

    verifier.erasure_hints = None; // Not a priority as of currently
}