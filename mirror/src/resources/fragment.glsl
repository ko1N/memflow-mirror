#version 140

in vec2 v_tex_coords;
out vec4 color;

uniform sampler2D tex;

void main() {
    //color = texture(tex, v_tex_coords);

    // map dxgi bgra to rgba
    color.zyxw = texture(tex, v_tex_coords);
}
