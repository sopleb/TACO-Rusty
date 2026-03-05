#version 330

layout(location = 0) in vec4 vert;
layout(location = 1) in vec4 vertColor;  // Per-vertex color from ColorVao

uniform mat4 projection;
uniform mat4 modelView;
uniform float pointsize;
uniform int hlpoints[10];
uniform float hlsizes[10];

flat out int foundpoint;
out vec4 vColor;  // Pass per-vertex color to fragment shader

void main()
{
    int found = -1;

    for(int i = 0; i < 10; i++)
    {
        if(gl_VertexID == hlpoints[i])
        {
            found = i;
        }
    }

    if (found == -1)
    {
        gl_PointSize = pointsize;
    }
    else
    {
        gl_PointSize = pointsize + hlsizes[found];
    }

    foundpoint = found;
    vColor = vertColor;  // Pass color to fragment shader
    gl_Position = projection * modelView * vert;

}