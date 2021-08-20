use lyon::svg::path::builder::{PathBuilder, SvgPathBuilder};
use lyon::math::point;

fn build_arrow_path<Builder: PathBuilder>(builder: &mut Builder) {
    builder.begin(point(-1.0, -0.3));
    builder.line_to(point(0.0, -0.3));
    builder.line_to(point(0.0, -1.0));
    builder.line_to(point(1.5, 0.0));
    builder.line_to(point(0.0, 1.0));
    builder.line_to(point(0.0, 0.3));
    builder.line_to(point(-1.0, 0.3));
    builder.close();
}


pub fn build_tetrion_path<Builder: SvgPathBuilder>(builder: &mut Builder) {
    builder.move_to(point(0.0, 0.0));
    //builder.begin(point(-1.0, -0.3));
    builder.line_to(point(0.0, 10.0));
    builder.line_to(point(10.0, 10.0));
    builder.line_to(point(10.0, 0.0));
    // builder.line_to(point(1.5, 0.0));
    // builder.line_to(point(0.0, 1.0));
    // builder.line_to(point(0.0, 0.3));
    // builder.line_to(point(-1.0, 0.3));
    builder.close();
}