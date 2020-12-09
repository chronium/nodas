use std::unimplemented;

pub trait Codegen {
    fn codegen(&self) -> String {
        unimplemented!()
    }
}
