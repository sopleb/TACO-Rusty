#version 330

layout(location = 0) in vec3 vert;
layout(location = 1) in vec4 in_color;

uniform mat4 projection;
uniform mat4 modelView;

out vec4 out_color;

void main()
{
    gl_Position = projection * modelView * vec4(vert, 1);
	out_color = in_color;
}