// SPDX-License-Identifier: Apache-2.0
shader_type spatial;
render_mode unshaded, blend_add, depth_draw_never, cull_disabled;
uniform float phase = 0.;
uniform float visibility = 1.;
void fragment() {
    float x=UV.x;
    float y=(UV.y-.5)*2.;
    float bend=sin(x*14.-phase*9.)*(1.-x)*.45;
    float ribbon=exp(-pow((y-bend)/.075,2.))+exp(-pow((y+bend)/.09,2.))*.65;
    float mist=exp(-y*y*12.)*.20;
    float fade=smoothstep(0.,.3,x)*(1.-smoothstep(.83,1.,x));
    ALBEDO=mix(vec3(.16,.62,1.),vec3(.65,1.,.93),x);
    ALPHA=(ribbon+mist)*fade*visibility*.8;
}
