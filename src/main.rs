use std::{io::Read, sync::Arc};

use noir_verifier::{vir_optimizers::optimize_vir_crate, vstd_utils::get_imported_krates, verify_crate::verify_crate};
use rust_verify::user_filter::UserFilter;
use vir::{ast::Krate, messages::Span};

fn main() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read from stdin");

    let vir_crate: Krate = serde_json::from_str(&input).expect("Failed to deserialize");
    // println!("{:#?}", vir_krate.functions);

    let build_test_mode = false; // Probably not needed

    // We need the verus standard library to verify Noir code
    let mut vstd = None;
    let verus_root = if !build_test_mode {
        let verus_root = rust_verify::driver::find_verusroot();
        if let Some(rust_verify::driver::VerusRoot {
            path: verusroot, ..
        }) = &verus_root
        {
            let vstd_path = verusroot.join("vstd.vir").to_str().unwrap().to_string();
            vstd = Some((format!("vstd"), vstd_path));
        }
        verus_root
    } else {
        None
    };
    println!("{:?}", verus_root.unwrap().path);
    let (our_args, _) =
        rust_verify::config::parse_args_with_imports(&String::from(""), std::env::args(), vstd);
    let mut verifier = rust_verify::verifier::Verifier::new(our_args);

    // UserFilter is needed
    let user_filter_result = UserFilter::from_args(&verifier.args, &vir_crate);
    verifier.user_filter = match user_filter_result {
        Ok(user_filter) => Some(user_filter),
        Err(msg) => panic!("{}", msg.note),
    };

    let imported = get_imported_krates(&verifier);
    // println!("Imported {:?}", imported.crate_names);

    optimize_vir_crate(&mut verifier, vir_crate, imported);
    println!("Finished optimizing");
    println!("Start verifying crate");
    // Is it needed?
    let air_no_span: Option<Span> = Some(Span {
        raw_span: Arc::new(()),
        id: 0,
        data: vec![],
        as_string: "no location".to_string(),
    }); // We can hack it with rustc if it is mandatory

    verify_crate(&mut verifier, air_no_span).unwrap();

    println!("If no errors were reported, then we have successfully verified the code");
}
