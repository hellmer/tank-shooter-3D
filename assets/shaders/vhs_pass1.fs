#version 330

/*
   VHS (pasada 2 de 2): vuelve a escalar luminancia y color con filtro bicúbico y pasa a RGB.
   Al tener el color mucho menos resolución horizontal que la luminancia, se corre hacia los lados.
   Adaptado de libretro/slang-shaders (vhs/shaders/vhs_mpalko/vhs_mpalko_pass1.slang).
   Basado en "VHS Compression" de mpalko: https://www.shadertoy.com/view/tsfXWj
   (licencia de Shadertoy: CC BY-NC-SA 3.0). El filtro bicúbico es de
   http://www.java-gaming.org/index.php?topic=35123.0
   Al final añade las scanlines del metroidvania (scanlines.fs): oscurece el hueco entre 'lines' líneas,
   con el brillo medio de la franja que cubre cada píxel para que no salgan bandas.
*/

in vec2 fragTexCoord;
in vec4 fragColor;

uniform sampler2D texture0;     // la salida de la pasada 1 (YIQ), con filtro bilineal
uniform vec2 outputSize;        // tamaño de la ventana: el mismo que el del búfer de la pasada 1
uniform float intensity;        // VHS_INTENSITY de vhs.rs; con 1 queda como el original
uniform float lines;            // SCANLINES de vhs.rs: líneas del televisor en toda la imagen
uniform float strength;         // SCANLINE_STRENGTH de vhs.rs: 0 = sin scanlines

out vec4 finalColor;

const vec2 maxResLuminance = vec2(335.0, 576.0);
const vec2 maxResChroma = vec2(40.0, 240.0);

const mat3 yiq2rgb = mat3(1.0, 1.0, 1.0,
                          0.956, -0.272, -1.106,
                          0.621, -0.647, 1.703);

vec4 cubic(float v)
{
    vec4 n = vec4(1.0, 2.0, 3.0, 4.0) - v;
    vec4 s = n*n*n;
    float x = s.x;
    float y = s.y - 4.0*s.x;
    float z = s.z - 4.0*s.y + 6.0*s.x;
    float w = 6.0 - x - y - z;
    return vec4(x, y, z, w)*(1.0/6.0);
}

vec4 textureBicubic(sampler2D tex, vec2 texCoords)
{
    vec2 texSize = vec2(textureSize(tex, 0));
    vec2 invTexSize = vec2(1.0)/texSize;

    texCoords = texCoords*texSize - 0.5;
    vec2 fxy = fract(texCoords);
    texCoords -= fxy;

    vec4 xcubic = cubic(fxy.x);
    vec4 ycubic = cubic(fxy.y);

    vec4 c = texCoords.xxyy + vec2(-0.5, +1.5).xyxy;
    vec4 s = vec4(xcubic.xz + xcubic.yw, ycubic.xz + ycubic.yw);
    vec4 offset = c + vec4(xcubic.yw, ycubic.yw)/s;
    offset *= invTexSize.xxyy;

    vec4 sample0 = texture(tex, offset.xz);
    vec4 sample1 = texture(tex, offset.yz);
    vec4 sample2 = texture(tex, offset.xw);
    vec4 sample3 = texture(tex, offset.yw);

    float sx = s.x/(s.x + s.y);
    float sy = s.z/(s.z + s.w);
    return mix(mix(sample3, sample2, sx), mix(sample1, sample0, sx), sy);
}

void main()
{
    vec2 uv = gl_FragCoord.xy/outputSize;

    vec2 resLuminance = min(maxResLuminance/max(intensity, 0.001), outputSize);
    vec2 resChroma = min(maxResChroma/max(intensity, 0.001), outputSize);
    vec2 uvLuminance = uv*(resLuminance/outputSize);
    vec2 uvChroma = uv*(resChroma/outputSize);

    float luminance = textureBicubic(texture0, uvLuminance).x;
    vec2 chroma = textureBicubic(texture0, uvChroma).yz;
    vec3 color = vec3(luminance, chroma)*yiq2rgb;

    float y = uv.y*lines;                                   // en líneas: cada entero, una línea
    float d = max(fwidth(y), 1e-4);                         // líneas que cubre este píxel
    float blur = max(sin(3.14159265*d)/(3.14159265*d), 0.0);    // media del coseno sobre el píxel
    float line = 0.5 + 0.5*cos(6.2831853*y)*blur;           // 1 en el centro de cada línea, 0 entre dos
    finalColor = vec4(color*(1.0 - strength*(1.0 - line)), 1.0);
}
