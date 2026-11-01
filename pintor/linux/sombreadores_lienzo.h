# pragma once

// Los shaders del lienzo, en HLSL. `lista_ps` usa los TIPO_* de la tinta, que
// define quien lo compila a partir de `ltx :: TipoTinta`

inline constexpr const char * hlsl_lista_vs = R"(
cbuffer Vista { float2 tam; float2 desplazamiento; };
struct Entrada {
  float2 Pos : ATTRIB0;
  float2 UV : ATTRIB1;
  float4 Color : ATTRIB2;
  uint Tinta : ATTRIB3;
};
struct Salida {
  float4 Position : SV_POSITION;
  float2 Doc : TEXCOORD0;
  float2 UV : TEXCOORD1;
  float4 Color : COLOR0;
  nointerpolation uint Tinta : TEXCOORD2;
};
void main(in Entrada e, out Salida s) {
  float2 p = (e.Pos - desplazamiento) / tam * 2.0 - 1.0;
  s.Position = float4(p.x, -p.y, 0.0, 1.0);
  s.Doc = e.Pos;
  s.UV = e.UV;
  s.Color = e.Color;
  s.Tinta = e.Tinta;
})" ;

inline constexpr const char * hlsl_lista_ps = R"(
struct Tinta {
  uint tipo; float fila; float radio; float reservado;
  float a; float b; float c; float d; float e; float f;
  float p; float q; float r; float s;
};
StructuredBuffer<Tinta> Tintas;
Texture2D Atlas;
SamplerState Atlas_sampler;
Texture2D Rampas;
SamplerState Rampas_sampler;
struct Entrada {
  float4 Position : SV_POSITION;
  float2 Doc : TEXCOORD0;
  float2 UV : TEXCOORD1;
  float4 Color : COLOR0;
  nointerpolation uint Tinta : TEXCOORD2;
};
float radial(Tinta t, float2 g) {
  // de un rayo que sale del foco, qué fracción de su camino al borde es g
  float2 f = float2(t.p, t.q);
  float2 d = g - f;
  float A = dot(d, d);
  float B = dot(f, d);
  float C = dot(f, f) - t.radio * t.radio;
  float raiz = sqrt(max(B * B - A * C, 0.0));
  return A <= 0.0 ? 0.0 : A / max(raiz - B, 1e-6);
}
float4 main(in Entrada i) : SV_TARGET {
  Tinta t = Tintas[i.Tinta];
  float4 c = i.Color;
  if (t.tipo == TIPO_GLIFO) {
    c.a *= Atlas.SampleLevel(Atlas_sampler, i.UV, 0.0).r;
  } else if (t.tipo == TIPO_TRAZO) {
    // lo que se solapan el píxel y el trazo, medidos en perpendicular al trazo
    float d = abs(i.UV.x);
    c.a *= saturate(min(d + 0.5, i.UV.y) - max(d - 0.5, -i.UV.y));
  } else if (t.tipo == TIPO_LINEAL || t.tipo == TIPO_RADIAL) {
    float2 g = float2(t.a * i.Doc.x + t.c * i.Doc.y + t.e, t.b * i.Doc.x + t.d * i.Doc.y + t.f);
    float u = t.tipo == TIPO_LINEAL ? dot(g - float2(t.p, t.q), float2(t.r, t.s)) : radial(t, g);
    u = (saturate(u) * 255.0 + 0.5) / 256.0;
    c.rgb = Rampas.SampleLevel(Rampas_sampler, float2(u, t.fila), 0.0).rgb;
  }
  return float4(c.rgb * c.a, c.a);
})" ;

// Las imágenes llegan con el alfa ya multiplicado : al reducirlas, los mipmaps no
// oscurecen los bordes

inline constexpr const char * hlsl_imagen_ps = R"(
Texture2D Imagen;
SamplerState Imagen_sampler;
struct Entrada {
  float4 Position : SV_POSITION;
  float2 Doc : TEXCOORD0;
  float2 UV : TEXCOORD1;
  float4 Color : COLOR0;
  nointerpolation uint Tinta : TEXCOORD2;
};
float4 main(in Entrada i) : SV_TARGET {
  return Imagen.Sample(Imagen_sampler, i.UV) * i.Color.a;
})" ;

inline constexpr const char * hlsl_marca_ps = R"(
float4 main() : SV_TARGET { return float4(0.0, 0.0, 0.0, 0.0); })" ;

// Un triángulo que cubre todo : las capas se leen píxel a píxel

inline constexpr const char * hlsl_pantalla_vs = R"(
void main(in uint vid : SV_VertexID, out float4 Position : SV_POSITION) {
  float2 p = float2((vid << 1) & 2, vid & 2);
  Position = float4(p * 2.0 - 1.0, 0.0, 1.0);
})" ;

inline constexpr const char * hlsl_copia_ps = R"(
Texture2D Capa;
float4 main(in float4 Position : SV_POSITION) : SV_TARGET {
  return Capa.Load(int3(int2(Position.xy), 0));
})" ;

inline constexpr const char * hlsl_capa_ps = R"(
cbuffer Composicion { float opacidad; uint mezcla; uint con_mascara; uint reservado; };
Texture2D Capa;
Texture2D Mascara;
float4 main(in float4 Position : SV_POSITION) : SV_TARGET {
  int3 p = int3(int2(Position.xy), 0);
  float4 c = Capa.Load(p) * opacidad;
  if (con_mascara != 0) {
    c *= dot(Mascara.Load(p).rgb, float3(0.2126, 0.7152, 0.0722));
  }
  return c;
})" ;

// Los modos de mezcla de PDF y CSS : necesitan leer lo que hay debajo. Sus
// números son la posición más uno en MEZCLAS, de pintor/src/compilador.rs

inline constexpr const char * hlsl_mezcla_ps = R"(
cbuffer Composicion { float opacidad; uint mezcla; uint con_mascara; uint reservado; };
Texture2D Capa;
Texture2D Fondo;
float maximo(float3 c) { return max(c.r, max(c.g, c.b)); }
float minimo(float3 c) { return min(c.r, min(c.g, c.b)); }
float lum(float3 c) { return dot(c, float3(0.3, 0.59, 0.11)); }
float sat(float3 c) { return maximo(c) - minimo(c); }
float3 recortar(float3 c) {
  float l = lum(c);
  float n = minimo(c);
  float x = maximo(c);
  if (n < 0.0) c = l + (c - l) * l / (l - n);
  if (x > 1.0) c = l + (c - l) * (1.0 - l) / (x - l);
  return c;
}
float3 con_lum(float3 c, float l) { return recortar(c + (l - lum(c))); }
float3 con_sat(float3 c, float s) {
  float n = minimo(c);
  float x = maximo(c);
  return x > n ? (c - n) * s / (x - n) : float3(0.0, 0.0, 0.0);
}
float3 duro(float3 b, float3 s) {
  return lerp(b * 2.0 * s, 1.0 - (1.0 - b) * (2.0 - 2.0 * s), step(0.5, s));
}
float suave(float b, float s) {
  float d = b <= 0.25 ? ((16.0 * b - 12.0) * b + 4.0) * b : sqrt(b);
  return s <= 0.5 ? b - (1.0 - 2.0 * s) * b * (1.0 - b) : b + (2.0 * s - 1.0) * (d - b);
}
float esquivar(float b, float s) { return b <= 0.0 ? 0.0 : s >= 1.0 ? 1.0 : min(1.0, b / (1.0 - s)); }
float quemar(float b, float s) { return b >= 1.0 ? 1.0 : s <= 0.0 ? 0.0 : 1.0 - min(1.0, (1.0 - b) / s); }
float3 mezclar(float3 b, float3 s) {
  switch (mezcla) {
    case 1: return b * s;
    case 2: return b + s - b * s;
    case 3: return duro(s, b);
    case 4: return min(b, s);
    case 5: return max(b, s);
    case 6: return float3(esquivar(b.r, s.r), esquivar(b.g, s.g), esquivar(b.b, s.b));
    case 7: return float3(quemar(b.r, s.r), quemar(b.g, s.g), quemar(b.b, s.b));
    case 8: return duro(b, s);
    case 9: return float3(suave(b.r, s.r), suave(b.g, s.g), suave(b.b, s.b));
    case 10: return abs(b - s);
    case 11: return b + s - 2.0 * b * s;
    case 12: return con_lum(con_sat(s, sat(b)), lum(b));
    case 13: return con_lum(con_sat(b, sat(s)), lum(b));
    case 14: return con_lum(s, lum(b));
    case 15: return con_lum(b, lum(s));
  }
  return s;
}
float4 main(in float4 Position : SV_POSITION) : SV_TARGET {
  int3 p = int3(int2(Position.xy), 0);
  float4 s = Capa.Load(p) * opacidad;
  float4 b = Fondo.Load(p);
  float3 cs = s.a > 0.0 ? s.rgb / s.a : float3(0.0, 0.0, 0.0);
  float3 cb = b.a > 0.0 ? b.rgb / b.a : float3(0.0, 0.0, 0.0);
  float3 c = (1.0 - s.a) * b.rgb + (1.0 - b.a) * s.rgb + s.a * b.a * mezclar(cb, cs);
  return float4(c, s.a + b.a * (1.0 - s.a));
})" ;
