#version 330

uniform sampler2D tex;

out vec4 fragColor;

void main()
{
    vec4 texOut = texture(tex, gl_PointCoord);
    if (texOut.a < 0.1)
        discard;
    // Dim white accents to gray
    float mx = max(texOut.r, max(texOut.g, texOut.b));
    float mn = min(texOut.r, min(texOut.g, texOut.b));
    float sat = mx - mn;
    if (sat < 0.15 && mx > 0.7)
        texOut.rgb *= 0.75;
    // Tone down bright saturated colors
    if (sat > 0.15 && texOut.r > 0.5 && texOut.g > 0.5)
        texOut.rgb *= 0.85;  // yellow (high R + high G)
    else if (sat > 0.15 && texOut.r > 0.5)
        texOut.rgb *= 0.7;   // red (high R, low G)
    if (sat > 0.15 && texOut.g > 0.5 && texOut.r <= 0.5)
        texOut.rgb *= 0.8;   // green
    fragColor = texOut;
}
