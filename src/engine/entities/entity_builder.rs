use crate::engine::entities::{BgEntity, GeoEntity};
use crate::engine::gpu;
use lyon::tessellation as Tes;
use lyon::path::math as LyM;
use lyon::tessellation::{FillTessellator, StrokeTessellator, VertexBuffers, FillRule, FillOptions, BuffersBuilder, TessellationError};
use lyon::path::{Path};
use lyon::path::builder::{SvgPathBuilder};

pub enum BluePrint {
    SvgPath(Box<dyn Fn(&mut dyn SvgPathBuilder)>),
    //Rect(LyM::Rect)
    Square(f32)
}

pub struct EntityBuilder {
    fill_tess: Tes::FillTessellator,
    stroke_tess: Tes::StrokeTessellator,
    fill_opts: FillOptions,
    stroke_opts: Tes::StrokeOptions,
}


const TOLERANCE: f32 = 0.02;


impl EntityBuilder {

    // TODO: BackgroundFillOptions: &FillOptions::DEFAULT

    pub fn new() -> EntityBuilder {
        EntityBuilder{
            fill_tess: FillTessellator::new(),
            stroke_tess: StrokeTessellator::new(),
            fill_opts: Tes::FillOptions::tolerance(TOLERANCE)
                .with_fill_rule(FillRule::NonZero),
            stroke_opts: Tes::StrokeOptions::tolerance(TOLERANCE),
        }
    }

    pub(crate) fn build_bg(&mut self, rect: LyM::Rect) -> Result<BgEntity, TessellationError> {
        let mut geometry = VertexBuffers::new(); //<gpu::BgPoint, u16>

        self.fill_tess.tessellate_rectangle(
            &rect,
            &FillOptions::DEFAULT,
            &mut BuffersBuilder::new(&mut geometry, gpu::Custom)
        )?;

        Ok(BgEntity::new(geometry))
    }

    pub fn tes_fill(&mut self, path: &Path, buf: &mut VertexBuffers<gpu::GpuVertex, u16>, entity_id: u32) -> Result<(), TessellationError> {
        let mut buf_builder = BuffersBuilder::new(
            buf,
            gpu::WithId(entity_id as i32));
        self.fill_tess.tessellate_path(path,&self.fill_opts, &mut buf_builder)?;
        Ok(())
    }

    pub fn tes_stroke(&mut self, path: &Path, buf: &mut VertexBuffers<gpu::GpuVertex, u16>, entity_id: u32) -> Result<(), TessellationError> {
        let mut buf_builder = BuffersBuilder::new(
            buf,
            gpu::WithId(entity_id as i32));
        self.stroke_tess.tessellate_path(path,&self.stroke_opts, &mut buf_builder)?;
        Ok(())
    }

    pub fn build_geo(&mut self, path: &Path, instances: usize, scale: f32, fill: bool, stroke: bool) -> Result<GeoEntity, TessellationError> {
        let mut geometry = VertexBuffers::new();
        let entity_id = 0; // TODO: This needs to be changed for multiple entities in the same buffer
        let fill_id = 0;
        if fill {
            self.tes_fill(path, &mut geometry, entity_id)?;
        }
        let range_fill = fill_id..geometry.indices.len() as u32;
        let stroke_id = range_fill.end;
        if stroke {
            self.tes_stroke(path, &mut geometry, entity_id)?;
        }
        let range_stroke = stroke_id..geometry.indices.len() as u32;
        Ok(GeoEntity::new(geometry, range_fill, range_stroke, scale, instances))
    }

}