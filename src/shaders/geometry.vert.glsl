#version 450

#define PRIM_BUFFER_LEN 180

layout(std140, binding = 0)
uniform Globals {
    vec2 u_resolution;
    vec2 u_scroll_offset;
    float u_zoom;
};

struct Primitive {
    vec4 color;
    vec4 color_stroke;
    vec2 translate;
    int z_index;
    float width;
    float angle;
    float scale;
    int _pad1;
    int _pad2;
};

layout(std140, binding = 1)
uniform u_primitives { Primitive primitives[PRIM_BUFFER_LEN]; };

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_normal;
layout(location = 2) in int a_prim_id;

layout(location = 0) out vec4 v_color;

void main() {
    int id = a_prim_id + gl_InstanceIndex;
    Primitive prim = primitives[id];

    vec2 invert_y = vec2(1.0, -1.0);

    mat2 rotation = mat2(
        cos(prim.angle), -sin(prim.angle),
        sin(prim.angle), cos(prim.angle)
    );

    vec2 local_pos = (a_position * prim.scale + a_normal * prim.width) * rotation;
    vec2 world_pos = local_pos - u_scroll_offset + prim.translate;
    vec2 transformed_pos = world_pos * u_zoom / (vec2(0.5, 0.5) * u_resolution) * invert_y;

    float z = float(prim.z_index) / 4096.0;


    if (a_normal[0] == 0) {
        v_color = prim.color;
        float fact = 1 - ((a_position[0] / 20) + (a_position[1] / 20));
        v_color[0] *= fact;
        v_color[1] *= fact;
        v_color[2] *= fact;
        if (prim.color[3] == 0.0) {
            z = 0.0;
        }
    } else {
        // v_color = prim.color;
        v_color = prim.color_stroke;
        if (prim.color_stroke[3] == 0.0) {
            z = 0.0;
        }
    }

    gl_Position = vec4(transformed_pos, z / 1000.0, 1.0);
}
