pub mod inspect_transform;

pub use inspect_transform::InspectTransform;

pub trait IntoInspect {
    type Output;

    fn into_inspect(&self) -> Self::Output;
}
