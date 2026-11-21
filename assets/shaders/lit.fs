#version 330

in vec2 fragTexCoord;
in vec3 fragNormal;

uniform sampler2D texture0;     // albedo (the painted 3dixel texture)
uniform sampler2D emissionMap;  // glowing texels, black elsewhere
uniform vec4 colDiffuse;
uniform vec3 lightDir;          // direction the light travels
uniform float flash;            // 0..1, turns the model white when hit

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
    finalColor = vec4(mix(color, vec3(1.0), flash), base.a);
}
