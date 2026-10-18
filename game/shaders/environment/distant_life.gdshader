// SPDX-License-Identifier: Apache-2.0
shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, fog_disabled;
uniform int kind = 0;
uniform float seed = 0.0;
uniform float movement = 1.0;
uniform vec4 silhouette_color : source_color = vec4(.025, .28, .38, 1.0);
float ellipse(vec2 p, vec2 r) {
    return (length(p / r) - 1.0) * min(r.x, r.y);
}
void fragment() {
    vec2 p = UV - .5;
    p.y += sin(TIME * 1.7 * movement + seed + p.x * 7.) * .018 * (1. - smoothstep(-.45, .1, p.x));
    float d = ellipse(p, vec2(.29, .095));
    float tail = max(abs(p.y) - (.035 + max(0., -p.x - .29) * .95), max(p.x + .27, -p.x - .46));
    d = min(d, tail);
    float dorsal = max(abs(p.x + .02) * .7 - (.16 - p.y), max(.07 - p.y, p.y - .19));
    if (kind == 1) {
        d = min(d, dorsal);
        d = min(d, ellipse(p - vec2(.13, -.07), vec2(.13, .035)));
    }
    if (kind == 2) {
        d = ellipse(p, vec2(.16, .09));
        float wing = abs(p.y) - .30 * (1. - smoothstep(.0, .28, abs(p.x + .025)));
        d = min(d, max(wing, abs(p.x) - .25));
        d = min(d, max(abs(p.y) - .012, max(p.x + .1, -p.x - .46)));
    }
    if (kind == 3) {
        d = ellipse(p, vec2(.32, .125));
        d = min(d, tail);
        d = min(d, ellipse(p - vec2(.06, .105), vec2(.10, .045)));
    }
    float alpha = (1. - smoothstep(-.008, .025, d)) * .22;
    ALBEDO = silhouette_color.rgb;
    ALPHA = alpha;
}
