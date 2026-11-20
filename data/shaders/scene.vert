#version 300 es
// One shape: the mesh vertex placed by the instance's three scaled axes and
// translation (iA, iB, iC; translation in .w), coloured by iD (rgb, and in
// .a the style, or for a see-through shape its opacity). Every value
// arrives as the core's Q16 integer and is scaled by uQ. No number lives
// here: the passes, widths and limits are uniforms from the core.
in vec3 aPos;
in vec3 aNormal;
in vec4 iA;
in vec4 iB;
in vec4 iC;
in vec4 iD;
uniform mat4 uVP;
uniform float uPass;
// solid, ink and tint pass numbers, and a half for comparing them
uniform vec4 uPasses;
// the ink line: least width, growth per unit of the shape, most width
uniform vec3 uInkSize;
uniform float uQ;
uniform float uEps;
// T16: the wind (stage.rs): each wave's direction (x, z), radians a metre
// and share; the four phases; the sway styles, a half, where a crown
// starts to bend; the bend at a full gust for a tuft's tip and a crown's top
uniform vec4 uWave0;
uniform vec4 uWave1;
uniform vec4 uWave2;
uniform vec4 uWave3;
uniform vec4 uPhase;
uniform vec4 uSway;
uniform vec2 uBend;
vec2 wave(vec4 w, float phase, vec2 at) {
  vec4 q = w * uQ;
  return q.xy * q.w * sin(q.z * dot(q.xy, at) - phase);
}
out vec3 vWorld;
out vec3 vNormal;
out vec4 vColor;
// the mesh's own point, for patterns that follow the shape (the corduroy)
out vec3 vLocal;
void main() {
  vec3 a = iA.xyz * uQ;
  vec3 b = iB.xyz * uQ;
  vec3 c = iC.xyz * uQ;
  vec3 t = vec3(iA.w, iB.w, iC.w) * uQ;
  vec3 p = aPos * uQ;
  vec3 n = aNormal * uQ;
  vec3 size = vec3(length(a), length(b), length(c));
  if (abs(uPass - uPasses.y) < uPasses.w) {
    // the inverted hull: each axis grown by the line's width on both sides
    float big = max(size.x, max(size.y, size.z));
    float w = clamp(uInkSize.x + big * uInkSize.y, uInkSize.x, uInkSize.z);
    p *= (size + w + w) / max(size, vec3(uEps));
  }
  vec3 world = a * p.x + b * p.y + c * p.z + t;
  // T16: a swaying shape bends with the wind from its foot (a tuft) or its
  // middle (a crown) up; the whole shape reads the wind at its own place
  float style = iD.w * uQ;
  float within = uSway.z * uQ;
  float tuft = step(abs(style - uSway.x * uQ), within);
  float crown = step(abs(style - uSway.y * uQ), within);
  if (tuft + crown > 0.0) {
    vec4 ph = uPhase * uQ;
    vec2 g = wave(uWave0, ph.x, t.xz) + wave(uWave1, ph.y, t.xz) + wave(uWave2, ph.z, t.xz) + wave(uWave3, ph.w, t.xz);
    float up = tuft * clamp(aPos.y * uQ, 0.0, 1.0) * uBend.x + crown * clamp(aPos.y * uQ + uSway.w * uQ, 0.0, 1.0) * uBend.y;
    world.xz += g * up * uQ;
  }
  vNormal = a * (n.x / max(dot(a, a), uEps)) + b * (n.y / max(dot(b, b), uEps)) + c * (n.z / max(dot(c, c), uEps));
  vColor = iD * uQ;
  vWorld = world;
  vLocal = aPos * uQ;
  gl_Position = uVP * vec4(world, 1.0);
}
