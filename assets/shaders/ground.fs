#version 330

in vec2 fragTexCoord;
in vec3 fragNormal;
in vec3 fragPosition;

uniform sampler2D texture0;  // ground tile (art/terreno/ground.py), repeats every 6.4 m
uniform vec4 colDiffuse;
uniform vec3 lightDir;
uniform vec3 viewPos;
uniform vec3 fogColor;

out vec4 finalColor;

void main()
{
    // Texture coordinates from the world position, so the tile keeps one texel = 0.1 m.
    vec2 uv = fragPosition.xz / 6.4;
    vec3 base = texture(texture0, uv).rgb * colDiffuse.rgb;
    // Broad light and dark areas so the repeat does not show from far away.
    float macro = sin(fragPosition.x * 0.043) * sin(fragPosition.z * 0.037 + 1.3)
                + 0.5 * sin((fragPosition.x + fragPosition.z) * 0.021);
    base *= 1.0 + 0.10 * macro;
    float diffuse = max(dot(vec3(0.0, 1.0, 0.0), -lightDir), 0.0);
    vec3 color = base * (vec3(0.50, 0.53, 0.60) + vec3(0.80, 0.77, 0.70) * diffuse);
    float d = length(fragPosition - viewPos) * 0.011;
    float fog = 1.0 - exp(-d * d);
    finalColor = vec4(mix(color, fogColor, fog), 1.0);
}
