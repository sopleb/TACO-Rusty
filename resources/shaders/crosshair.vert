#version 330

layout(location = 0) in vec3 vert;

uniform mat4 projection;
uniform mat4 modelView;
uniform float pointsize;

void main()
{
    gl_PointSize = pointsize;
    gl_Position = projection * modelView * vec4(vert, 1);
}