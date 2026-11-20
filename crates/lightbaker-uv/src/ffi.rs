//! xatlas' C API (`vendor/xatlas/xatlas_c.h`), declared by hand (ADR-0006: no bindgen). Layouts are checked
//! against the C++ compiler's `sizeof` in the tests.

#![allow(non_snake_case, dead_code)]

use std::ffi::{c_char, c_void};

pub const INDEX_FORMAT_UINT32: u32 = 1;

#[repr(C)]
pub struct Chart {
    pub faceArray: *mut u32,
    pub atlasIndex: u32,
    pub faceCount: u32,
    pub r#type: u32,
    pub material: u32,
}

#[repr(C)]
pub struct Vertex {
    pub atlasIndex: i32,
    pub chartIndex: i32,
    pub uv: [f32; 2],
    pub xref: u32,
}

#[repr(C)]
pub struct Mesh {
    pub chartArray: *mut Chart,
    pub indexArray: *mut u32,
    pub vertexArray: *mut Vertex,
    pub chartCount: u32,
    pub indexCount: u32,
    pub vertexCount: u32,
}

#[repr(C)]
pub struct Atlas {
    pub image: *mut u32,
    pub meshes: *mut Mesh,
    pub utilization: *mut f32,
    pub width: u32,
    pub height: u32,
    pub atlasCount: u32,
    pub chartCount: u32,
    pub meshCount: u32,
    pub texelsPerUnit: f32,
}

#[repr(C)]
pub struct MeshDecl {
    pub vertexPositionData: *const c_void,
    pub vertexNormalData: *const c_void,
    pub vertexUvData: *const c_void,
    pub indexData: *const c_void,
    pub faceIgnoreData: *const bool,
    pub faceMaterialData: *const u32,
    pub faceVertexCount: *const u8,
    pub vertexCount: u32,
    pub vertexPositionStride: u32,
    pub vertexNormalStride: u32,
    pub vertexUvStride: u32,
    pub indexCount: u32,
    pub indexOffset: i32,
    pub faceCount: u32,
    pub indexFormat: u32,
    pub epsilon: f32,
}

#[repr(C)]
pub struct ChartOptions {
    pub paramFunc: *const c_void,
    pub maxChartArea: f32,
    pub maxBoundaryLength: f32,
    pub normalDeviationWeight: f32,
    pub roundnessWeight: f32,
    pub straightnessWeight: f32,
    pub normalSeamWeight: f32,
    pub textureSeamWeight: f32,
    pub maxCost: f32,
    pub maxIterations: u32,
    pub useInputMeshUvs: bool,
    pub fixWinding: bool,
}

#[repr(C)]
pub struct PackOptions {
    pub maxChartSize: u32,
    pub padding: u32,
    pub texelsPerUnit: f32,
    pub resolution: u32,
    pub bilinear: bool,
    pub blockAlign: bool,
    pub bruteForce: bool,
    pub createImage: bool,
    pub rotateChartsToAxis: bool,
    pub rotateCharts: bool,
}

unsafe extern "C" {
    pub fn xatlasCreate() -> *mut Atlas;
    pub fn xatlasDestroy(atlas: *mut Atlas);
    pub fn xatlasAddMesh(atlas: *mut Atlas, decl: *const MeshDecl, mesh_count_hint: u32) -> u32;
    pub fn xatlasComputeCharts(atlas: *mut Atlas, options: *const ChartOptions);
    pub fn xatlasPackCharts(atlas: *mut Atlas, options: *const PackOptions);
    pub fn xatlasAddMeshErrorString(error: u32) -> *const c_char;
    pub fn xatlasMeshDeclInit(decl: *mut MeshDecl);
    pub fn xatlasChartOptionsInit(options: *mut ChartOptions);
    pub fn xatlasPackOptionsInit(options: *mut PackOptions);
    /// the probe (src/probe.cpp)
    pub fn lbuv_sizeof(which: i32) -> usize;
}
