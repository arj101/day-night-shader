#version 120

#ifdef GL_ES
precision mediump float;
#endif

#define WIDTH 4.0
#define MAX_ITER 1000
#define PI 3.14159265359

uniform vec2 u_resolution;
uniform vec2 u_mouse;
uniform float u_time;


int mandlebrot_set(float px, float py) {
    float x = 0.0, y = 0.0;
    float x2 = 0.0, y2 = 0.0;
    bool set_iter = false;
    if (pow(px+1.0, 2) + pow(py, 2) <= 1.0/16.0) return MAX_ITER;
    for(int i=0;i<MAX_ITER;++i){
        y = (x + x) * y + py;
        x = x2 - y2 + px;
        x2 = x * x;
        y2 = y * y;
        if (x2 + y2 > 4.0) return i;
    }

    return MAX_ITER;
}

void main() {
    vec2 st = gl_FragCoord.xy/u_resolution.xy;
    vec2 mouse = u_mouse.xy / u_resolution.xy - 1.0;
    float scale = pow(2.0, u_time*(-0.1));
    #define center  vec2(-1.4050, 0);
    vec2 pos = WIDTH * scale * (st - 0.5) + center;
    
    
    float i =float(mandlebrot_set(pos.x, pos.y)) * 0.001 *  max(50.0 - 0.5*u_time, 1.0);
 
    gl_FragColor = vec4(sin(i) , sin(i), sin(i), 1.0);
    if (i>=0.999999) gl_FragColor = vec4(0.);
}
