#version 330

in vec2 fragTexCoord;
in vec3 fragNormal;
in vec3 fragPosition;

uniform sampler2D texture0;     // albedo (the painted 3dixel texture)
uniform sampler2D emissionMap;  // glowing texels, black elsewhere
uniform vec4 colDiffuse;
uniform vec3 lightDir;          // direction the light travels
uniform float flash;            // 0..1, turns the model white when hit
uniform vec3 viewPos;
uniform vec3 fogColor;

out vec4 finalColor;

void main()
{
    vec4 base = texture(texture0, fragTexCoord) * colDiffuse;
    vec3 glow = texture(emissionMap, fragTexCoord).rgb;
    float diffuse = max(dot(normalize(fragNormal), -lightDir), 0.0);
    // Cool ambient from the sky, warm direct light, like the 3dixel previews.
    vec3 lit = base.rgb * (vec3(0.50, 0.53, 0.60) + vec3(0.80, 0.77, 0.70) * diffuse);
    // Glowing texels ignore the light.
    vec3 color = max(lit, glow * 1.15);
    color = mix(color, vec3(1.0), flash);
    // Distance fog; glow cuts through it a little.
    float d = length(fragPosition - viewPos) * 0.011;
    float fog = 1.0 - exp(-d * d);
    fog *= 1.0 - 0.4 * max(max(glow.r, glow.g), glow.b);
    finalColor = vec4(mix(color, fogColor, fog), base.a);
}
