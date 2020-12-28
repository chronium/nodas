use std::sync::Arc;

use log::info;
use nalgebra::Vector3;
use ncollide3d::{bounding_volume::BoundingVolume, query::RayCast};
use ncollide3d::{
    bounding_volume::AABB,
    math::Isometry,
    partitioning::{BVHImpl, BVT},
    query::{ContactPrediction, ContactPreprocessor},
    shape::{CompositeShape, Shape, TriMesh},
};

use crate::render::{
    binding::{Buffer, BufferUsage},
    state,
};

use super::ModelVertex;

#[derive(Clone)]
pub struct Mesh {
    pub(super) name: String,
    pub(super) vertex_buffer: Arc<Buffer>,
    pub(super) index_buffer: Arc<Buffer>,
    pub(super) num_elements: u32,
    pub(super) material: usize,
}

#[derive(Clone)]
pub struct Geometry {
    pub(super) meshes: Vec<Mesh>,
    pub(super) colliders: Vec<TriMesh<f32>>,
    pub(super) bvt: BVT<usize, AABB<f32>>,
}

impl Geometry {
    pub fn new(state: &state::WgpuState, obj_models: Vec<tobj::Model>) -> Self {
        let mut meshes = Vec::new();

        let mut colliders = Vec::new();
        for m in obj_models {
            info!("Load mesh {:?}", m.name);
            let mut vertices = Vec::new();
            for i in 0..m.mesh.positions.len() / 3 {
                vertices.push(ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ]
                    .into(),
                    tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]].into(),
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ]
                    .into(),
                    tangent: [0.0; 3].into(),
                    bitangent: [0.0; 3].into(),
                });
            }

            let indices = &m.mesh.indices;

            for c in indices.chunks(3) {
                let v0 = vertices[c[0] as usize];
                let v1 = vertices[c[1] as usize];
                let v2 = vertices[c[2] as usize];

                let delta_pos1 = v1.position - v0.position;
                let delta_pos2 = v2.position - v0.position;

                let delta_uv1 = v1.tex_coords - v0.tex_coords;
                let delta_uv2 = v2.tex_coords - v0.tex_coords;

                let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
                let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
                let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * r;

                vertices[c[0] as usize].tangent = tangent.into();
                vertices[c[1] as usize].tangent = tangent.into();
                vertices[c[2] as usize].tangent = tangent.into();

                vertices[c[0] as usize].bitangent = bitangent.into();
                vertices[c[1] as usize].bitangent = bitangent.into();
                vertices[c[2] as usize].bitangent = bitangent.into();
            }

            let shape = TriMesh::new(
                vertices
                    .iter()
                    .map(|v| v.position.into())
                    .collect::<Vec<_>>(),
                indices
                    .chunks(3)
                    .map(|c| [c[0] as usize, c[1] as usize, c[2] as usize].into())
                    .collect::<Vec<_>>(),
                Some(vertices.iter().map(|v| v.tex_coords).collect::<Vec<_>>()),
            );

            colliders.push(shape);

            let vertex_buffer =
                Buffer::new_init(state, m.name.as_str(), &vertices, BufferUsage::Vertex);
            let index_buffer =
                Buffer::new_init(state, m.name.as_str(), &m.mesh.indices, BufferUsage::Index);

            meshes.push(Mesh {
                name: m.name,
                vertex_buffer: Arc::new(vertex_buffer),
                index_buffer: Arc::new(index_buffer),
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            });
        }

        let bvt = BVT::new_balanced(
            colliders
                .iter()
                .enumerate()
                .map(|(index, collider)| (index, collider.aabb().clone()))
                .collect(),
        );

        Self {
            meshes,
            colliders,
            bvt,
        }
    }

    pub fn scaled(&self, scale: Vector3<f32>) -> Self {
        let mut clone = self.clone();
        let colliders = clone
            .colliders
            .iter()
            .map(|c| c.clone().scaled(&scale))
            .collect::<Vec<_>>();
        let bvt = BVT::new_balanced(
            colliders
                .iter()
                .enumerate()
                .map(|(index, collider)| (index, collider.aabb().clone()))
                .collect(),
        );

        clone.colliders = colliders;
        clone.bvt = bvt;

        clone
    }
}

impl CompositeShape<f32> for Geometry {
    fn nparts(&self) -> usize {
        self.meshes.len()
    }

    fn map_part_at(
        &self,
        i: usize,
        m: &Isometry<f32>,
        f: &mut dyn FnMut(&Isometry<f32>, &dyn Shape<f32>),
    ) {
        self.colliders[i].map_part_at(i, m, f)
    }

    fn map_part_and_preprocessor_at(
        &self,
        i: usize,
        m: &Isometry<f32>,
        prediction: &ContactPrediction<f32>,
        f: &mut dyn FnMut(&Isometry<f32>, &dyn Shape<f32>, &dyn ContactPreprocessor<f32>),
    ) {
        self.colliders[i].map_part_and_preprocessor_at(i, m, prediction, f)
    }

    fn aabb_at(&self, i: usize) -> AABB<f32> {
        *self.bvt.leaf(i).bounding_volume()
    }

    fn bvh(&self) -> BVHImpl<f32, usize, AABB<f32>> {
        BVHImpl::BVT(&self.bvt)
    }
}

impl Shape<f32> for Geometry {
    fn aabb(&self, m: &Isometry<f32>) -> AABB<f32> {
        fn merged(a: AABB<f32>, b: &AABB<f32>) -> AABB<f32> {
            a.merged(b)
        }

        self.colliders
            .iter()
            .map(|m| m.aabb())
            .fold(AABB::new_invalid(), merged)
            .transform_by(m)
    }

    fn tangent_cone_contains_dir(
        &self,
        _feature: ncollide3d::shape::FeatureId,
        _m: &Isometry<f32>,
        _deformations: Option<&[f32]>,
        _dir: &nalgebra::Unit<ncollide3d::math::Vector<f32>>,
    ) -> bool {
        todo!()
    }

    fn as_ray_cast(&self) -> Option<&dyn RayCast<f32>> {
        Some(self)
    }
}

impl RayCast<f32> for Geometry {
    fn toi_and_normal_with_ray(
        &self,
        m: &Isometry<f32>,
        ray: &ncollide3d::query::Ray<f32>,
        max_toi: f32,
        solid: bool,
    ) -> Option<ncollide3d::query::RayIntersection<f32>> {
        for collider in self.colliders.iter() {
            if let Some(intersection) = collider.toi_and_normal_with_ray(m, ray, max_toi, solid) {
                return Some(intersection);
            }
        }
        None
    }
}
