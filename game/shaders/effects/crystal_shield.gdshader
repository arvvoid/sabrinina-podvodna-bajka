// SPDX-License-Identifier: MIT
shader_type spatial;
render_mode unshaded, blend_add, depth_draw_never, cull_disabled;
uniform float opacity = 1.;
void fragment() {
    vec2 p=(UV-.5)*2.;
    float r=length(p);
    float rim=exp(-pow((r-.86)/.035,2.));
    float shine=exp(-dot(p-vec2(-.45,-.55),p-vec2(-.45,-.55))*160.);
    ALBEDO=vec3(.35,.88,1.);
    ALPHA=(rim*.4+shine*.7)*opacity;
}
