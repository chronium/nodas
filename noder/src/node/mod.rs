use crate::Codegen;

pub struct Program {
    pub(crate) version: usize,
    pub(crate) storage: Vec<Storage>,
}

impl Codegen for Program {
    fn codegen(&self) -> String {
        let mut storage = self.storage.clone();
        storage.sort();
        let ins = storage
            .iter()
            .filter(|st| st.storage_qualifier == StorageQualifier::In);
        let outs = storage
            .iter()
            .filter(|st| st.storage_qualifier == StorageQualifier::Out);
        let uniforms = storage
            .iter()
            .filter(|st| st.storage_qualifier == StorageQualifier::Uniform);

        let mut ins_comp = ins
            .map(|i| format!("\n{}", i.codegen()))
            .collect::<Vec<_>>()
            .join(";");
        ins_comp.push_str(";\n");

        let mut outs_comp = outs
            .map(|i| format!("\n{}", i.codegen()))
            .collect::<Vec<_>>()
            .join(";\n");
        outs_comp.push_str(";\n");

        let mut uniforms_comp = uniforms
            .map(|i| format!("\n{}", i.codegen()))
            .collect::<Vec<_>>()
            .join(";");
        uniforms_comp.push_str(";\n");

        format!(
            "#version {}\n{}{}{}",
            self.version, ins_comp, outs_comp, uniforms_comp
        )
    }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Storage {
    pub(crate) ty: Type,
    pub(crate) layout: Layout,
    pub(crate) storage_qualifier: StorageQualifier,
    pub(crate) binding: String,
}

impl Codegen for Storage {
    fn codegen(&self) -> String {
        format!(
            "{} {} {} {}",
            self.layout.codegen(),
            self.storage_qualifier.codegen(),
            self.ty.codegen(),
            self.binding
        )
    }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum StorageQualifier {
    In,
    Out,
    Uniform,
}

impl Codegen for StorageQualifier {
    fn codegen(&self) -> String {
        match self {
            Self::Uniform => format!("uniform"),
            Self::In => format!("in"),
            Self::Out => format!("out"),
        }
    }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Type {
    Vec2,
    Vec3,
    Vec4,
    Texture2D,
    Sampler,
}

impl Codegen for Type {
    fn codegen(&self) -> String {
        match self {
            Self::Vec2 => format!("vec2"),
            Self::Vec3 => format!("vec3"),
            Self::Vec4 => format!("vec4"),
            Self::Texture2D => format!("texture2D"),
            Self::Sampler => format!("sampler"),
        }
    }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Layout {
    UniformBinding { set: usize, binding: usize },
    Location(usize),
}

impl Codegen for Layout {
    fn codegen(&self) -> String {
        match self {
            Self::Location(loc) => format!("layout(location = {})", loc),
            Self::UniformBinding { set, binding } => {
                format!("layout(set = {}, binding = {})", set, binding)
            }
        }
    }
}
