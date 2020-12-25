use imgui::im_str;
use imgui_inspect::InspectRenderStruct;
use nalgebra::{Isometry3, Translation3, UnitQuaternion, Vector3};

#[derive(Debug, Clone, PartialEq)]
pub struct InspectTransform {
    position: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3],
}

impl InspectRenderStruct<InspectTransform> for InspectTransform {
    fn render(
        _data: &[&InspectTransform],
        _label: &'static str,
        _ui: &imgui::Ui,
        _args: &imgui_inspect::InspectArgsStruct,
    ) {
        todo!()
    }

    fn render_mut(
        data: &mut [&mut InspectTransform],
        label: &'static str,
        ui: &imgui::Ui,
        _args: &imgui_inspect::InspectArgsStruct,
    ) -> bool {
        ui.text(im_str!("{}", label));
        ui.input_float3(im_str!("position"), &mut data[0].position)
            .build();
        ui.input_float3(im_str!("rotation"), &mut data[0].rotation)
            .build();
        ui.input_float3(im_str!("scale"), &mut data[0].scale)
            .build();

        true
    }
}

impl InspectTransform {
    pub fn new(isometry: Isometry3<f32>, scale: Vector3<f32>) -> Self {
        let (x, y, z) = isometry.rotation.euler_angles();
        Self {
            position: isometry.translation.vector.into(),
            rotation: [x.to_degrees(), y.to_degrees(), z.to_degrees()],
            scale: scale.into(),
        }
    }

    pub fn position(&self) -> Translation3<f32> {
        Translation3::from(Vector3::new(
            self.position[0],
            self.position[1],
            self.position[2],
        ))
    }

    pub fn rotation(&self) -> UnitQuaternion<f32> {
        UnitQuaternion::from_euler_angles(
            self.rotation[0].to_radians(),
            self.rotation[1].to_radians(),
            self.rotation[2].to_radians(),
        )
    }

    pub fn scale(&self) -> Vector3<f32> {
        Vector3::new(self.scale[0], self.scale[1], self.scale[2])
    }
}
