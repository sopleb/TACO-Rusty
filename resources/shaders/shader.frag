#version 330

uniform sampler2D tex;

uniform int hlpoints[10];
uniform float hlcolors[40];
uniform vec4 ncolor;

flat in int foundpoint;
in vec4 vColor;  // Per-vertex color from vertex shader

out vec4 fragColor;

void main()
{
    int found = foundpoint;

    vec4 drawColor;

    if (found == -1)
    {
        // Use per-vertex color for non-highlighted systems
        // vColor contains RGBA packed as int, we need to unpack it
        drawColor = vColor;
    }
    else
    {
        drawColor = vec4(hlcolors[found * 4], hlcolors[found * 4 + 1], hlcolors[found * 4 + 2], hlcolors[found * 4 + 3]);
    }

    vec4 texOut = texture(tex, gl_PointCoord);
    fragColor =  drawColor * texOut.a;
}