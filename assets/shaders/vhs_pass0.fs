#version 330

/*
   VHS (pasada 1 de 2): baja la imagen a la resolución de una cinta VHS, en YIQ.
   Adaptado de libretro/slang-shaders (vhs/shaders/vhs_mpalko/vhs_mpalko_pass0.slang) con sus
   valores por defecto (PAL). Basado en "VHS Compression" de mpalko:
   https://www.shadertoy.com/view/tsfXWj (licencia de Shadertoy: CC BY-NC-SA 3.0).

   Escribe en la esquina inferior izquierda de un búfer de coma flotante (I y Q son negativos):
   la luminancia ocupa maxResLuminance y el color maxResChroma; la pasada 2 los vuelve a escalar.
*/

in vec2 fragTexCoord;
in vec4 fragColor;

uniform sampler2D texture0;     // la imagen del juego, con filtro bilineal
uniform vec2 outputSize;        // tamaño de este búfer: el de la ventana
uniform float intensity;        // VHS_INTENSITY de vhs.rs; con 1 queda como el original

out vec4 finalColor;

const vec2 maxResLuminance = vec2(335.0, 576.0);
const vec2 maxResChroma = vec2(40.0, 240.0);
const vec2 blurAmount = vec2(0.2, 0.2);

const mat3 rgb2yiq = mat3(0.299, 0.596, 0.211,
                          0.587, -0.274, -0.523,
                          0.114, -0.322, 0.312);

vec3 downsampleVideo(vec2 uv, vec2 pixelSize, ivec2 samples)
{
    vec2 uvStart = uv - pixelSize/2.0;
    vec2 uvEnd = uv + pixelSize;

    vec3 result = vec3(0.0);
    for (int i_u = 0; i_u < samples.x; i_u++) {
        float u = mix(uvStart.x, uvEnd.x, float(i_u)/float(samples.x));
        for (int i_v = 0; i_v < samples.y; i_v++) {
            float v = mix(uvStart.y, uvEnd.y, float(i_v)/float(samples.y));
            result += texture(texture0, vec2(u, v)).rgb;
        }
    }
    return (result/float(samples.x*samples.y))*rgb2yiq;
}

vec3 downsampleVideo(vec2 fragCoord, vec2 downsampledRes)
{
    if (fragCoord.x > downsampledRes.x || fragCoord.y > downsampledRes.y) return vec3(0.0);

    vec2 uv = fragCoord/downsampledRes;
    vec2 pixelSize = 1.0/downsampledRes;
    pixelSize *= 1.0 + blurAmount*intensity;     // un poco de desenfoque de caja para no tener aliasing
    return downsampleVideo(uv, pixelSize, ivec2(8, 3));
}

void main()
{
    // Las mismas cuentas que en la pasada 2: si no coinciden, la imagen se descuadra
    vec2 resLuminance = min(maxResLuminance/max(intensity, 0.001), outputSize);
    vec2 resChroma = min(maxResChroma/max(intensity, 0.001), outputSize);

    float luminance = downsampleVideo(gl_FragCoord.xy, resLuminance).r;
    vec2 chroma = downsampleVideo(gl_FragCoord.xy, resChroma).gb;
    finalColor = vec4(luminance, chroma, 1.0);
}
