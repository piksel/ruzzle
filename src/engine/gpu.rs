use lyon::tessellation as Tes;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Globals {
    pub(crate) resolution: [f32; 2],
    pub(crate) scroll_offset: [f32; 2],
    pub(crate) zoom: f32,
    pub(crate) _pad: f32,
}

unsafe impl bytemuck::Pod for Globals {}
unsafe impl bytemuck::Zeroable for Globals {}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GpuVertex {
    position: [f32; 2],
    normal: [f32; 2],
    prim_id: i32,
}
unsafe impl bytemuck::Pod for GpuVertex {}
unsafe impl bytemuck::Zeroable for GpuVertex {}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Primitive {
    pub color: [f32; 4],
    pub color_stroke: [f32; 4],

    pub translate: [f32; 2],
    pub z_index: i32,
    pub width: f32,

    pub angle: f32,
    pub scale: f32,
    _pad1: i32,
    _pad2: i32,
}

impl Primitive {
    pub(crate) fn new_with_scale(scale: f32) -> Self {
        Primitive {
            color: [0.0, 0.0, 0.0, 1.0],
            scale,
            ..Primitive::DEFAULT
        }
    }
}

impl Primitive {
    const DEFAULT: Self = Primitive {
        color: [0.0; 4],
        color_stroke: [0.0; 4],
        translate: [0.0; 2],
        z_index: 0,
        width: 0.0,
        angle: 0.0,
        scale: 1.0,
        _pad1: 0,
        _pad2: 0,
    };
}

unsafe impl bytemuck::Pod for Primitive {}
unsafe impl bytemuck::Zeroable for Primitive {}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BgPoint {
    pub(crate) point: [f32; 2],
}
unsafe impl bytemuck::Pod for BgPoint {}
unsafe impl bytemuck::Zeroable for BgPoint {}

pub const PRIM_BUFFER_LEN: usize = 180;

/// This vertex constructor forwards the positions and normals provided by the
/// tessellators and add a shape id.
pub struct WithId(pub i32);

impl Tes::FillVertexConstructor<GpuVertex> for WithId {
    fn new_vertex(&mut self, vertex: Tes::FillVertex) -> GpuVertex {
        GpuVertex {
            position: vertex.position().to_array(),
            normal: [0.0, 0.0],
            prim_id: self.0,
        }
    }
}

impl Tes::StrokeVertexConstructor<GpuVertex> for WithId {
    fn new_vertex(&mut self, vertex: Tes::StrokeVertex) -> GpuVertex {
        GpuVertex {
            position: vertex.position_on_path().to_array(),
            normal: vertex.normal().to_array(),
            prim_id: self.0,
        }
    }
}

pub struct Custom;

impl Tes::FillVertexConstructor<BgPoint> for Custom {
    fn new_vertex(&mut self, vertex: Tes::FillVertex) -> BgPoint {
        BgPoint {
            point: vertex.position().to_array(),
        }
    }
}

impl Tes::StrokeVertexConstructor<BgPoint> for Custom {
    fn new_vertex(&mut self, vertex: Tes::StrokeVertex) -> BgPoint {
        BgPoint {
            point: vertex.position_on_path().to_array(),
        }
    }
}