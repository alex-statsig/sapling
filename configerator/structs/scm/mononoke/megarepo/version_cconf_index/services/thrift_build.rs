// @generated by autocargo

use std::env;
use std::fs;
use std::path::Path;
use thrift_compiler::Config;
use thrift_compiler::GenContext;
const CRATEMAP: &str = "\
configerator/structs/scm/mononoke/megarepo/megarepo_configs.thrift megarepo_configs //configerator/structs/scm/mononoke/megarepo:megarepo_configs-rust
configerator/structs/scm/mononoke/megarepo/version_cconf_index.thrift crate //configerator/structs/scm/mononoke/megarepo:version_cconf_index-rust
thrift/annotation/rust.thrift megarepo_configs->rust //thrift/annotation:rust-rust
thrift/annotation/scope.thrift megarepo_configs->rust->scope //thrift/annotation:scope-rust
";
#[rustfmt::skip]
fn main() {
    println!("cargo:rerun-if-changed=thrift_build.rs");
    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR env not provided");
    let cratemap_path = Path::new(&out_dir).join("cratemap");
    fs::write(cratemap_path, CRATEMAP).expect("Failed to write cratemap");
    Config::from_env(GenContext::Services)
        .expect("Failed to instantiate thrift_compiler::Config")
        .base_path("../../../../../../..")
        .types_crate("version_cconf_index__types")
        .clients_crate("version_cconf_index__clients")
        .options("serde")
        .run(["../../version_cconf_index.thrift"])
        .expect("Failed while running thrift compilation");
}
