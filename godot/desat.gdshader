shader_type canvas_item;

uniform float saturation : hint_range(0, 1) = 1.0;

void fragment() {
    vec4 tex_color = texture(TEXTURE, UV);
    float luminance = 0.2126 * tex_color.r + 0.7152 * tex_color.g + 0.0722 * tex_color.b;
    vec4 desat_color = mix(vec4(luminance, luminance, luminance, tex_color.a), tex_color, saturation);
    COLOR = desat_color;
}
