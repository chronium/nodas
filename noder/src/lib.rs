pub mod codegen;
pub mod node;

pub use codegen::Codegen;

#[cfg(test)]
mod tests {
    use crate::{
        node::{Layout, Program, Storage, StorageQualifier, Type},
        Codegen,
    };

    #[test]
    fn codegen_layout() {
        let location = Layout::Location(123);
        let uniform = Layout::UniformBinding {
            set: 456,
            binding: 789,
        };

        assert_eq!(location.codegen(), String::from("layout(location = 123)"));
        assert_eq!(uniform.codegen(), "layout(set = 456, binding = 789)");
    }

    #[test]
    fn codegen_type() {
        let input = [
            Type::Vec2,
            Type::Vec3,
            Type::Vec4,
            Type::Texture2D,
            Type::Sampler,
        ];
        let expected = ["vec2", "vec3", "vec4", "texture2D", "sampler"]
            .iter()
            .map(|s| String::from(*s));

        let result = input.iter().map(|ty| ty.codegen());

        assert!(result.eq(expected));
    }

    #[test]
    fn codegen_storage_qualifier() {
        let input = [
            StorageQualifier::Uniform,
            StorageQualifier::In,
            StorageQualifier::Out,
        ];
        let expected = ["uniform", "in", "out"].iter().map(|s| String::from(*s));

        let result = input.iter().map(|ty| ty.codegen());

        assert!(result.eq(expected));
    }

    #[test]
    fn codegen_uniform() {
        let layout = Layout::UniformBinding {
            set: 12,
            binding: 128,
        };
        let uniform = Storage {
            layout,
            storage_qualifier: StorageQualifier::Uniform,
            ty: Type::Texture2D,
            binding: String::from("t_diffuse"),
        };

        assert_eq!(
            uniform.codegen(),
            "layout(set = 12, binding = 128) uniform texture2D t_diffuse"
        )
    }

    #[test]
    fn codegen_program_storage_order() {
        let program = Program {
            version: 450,
            storage: vec![
                Storage {
                    layout: Layout::Location(0),
                    storage_qualifier: StorageQualifier::Out,
                    ty: Type::Vec4,
                    binding: String::from("f_color"),
                },
                Storage {
                    layout: Layout::UniformBinding { set: 0, binding: 1 },
                    storage_qualifier: StorageQualifier::Uniform,
                    ty: Type::Sampler,
                    binding: String::from("s_diffuse"),
                },
                Storage {
                    layout: Layout::UniformBinding { set: 0, binding: 0 },
                    storage_qualifier: StorageQualifier::Uniform,
                    ty: Type::Texture2D,
                    binding: String::from("t_diffuse"),
                },
                Storage {
                    layout: Layout::Location(0),
                    storage_qualifier: StorageQualifier::In,
                    ty: Type::Vec2,
                    binding: String::from("v_tex_coords"),
                },
            ],
        };

        assert_eq!(
            program.codegen(),
            r#"#version 450

layout(location = 0) in vec2 v_tex_coords;

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_diffuse;
layout(set = 0, binding = 1) uniform sampler s_diffuse;
"#
        );
    }
}
