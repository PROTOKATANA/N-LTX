# include "gpu_lienzo.h"

# include <algorithm>

# include <atomic>

# include <chrono>

# include <cstdlib>

# include <memory>

# include <mutex>

# include <string>

# include <vector>

# include "banco_gpu.h"

# include "gpu_surface.h"

# include "pintor/src/puente.rs.h"

using namespace Diligent ;

namespace {

constexpr float fondo_lienzo[4] = { 1.0f , 1.0f , 1.0f , 1.0f } ;

constexpr float transparente[4] = { 0.0f , 0.0f , 0.0f , 0.0f } ;

constexpr TEXTURE_FORMAT formato_color = TEX_FORMAT_RGBA8_UNORM ;

// Lo mismo que las constantes de pintor/src/atlas.rs y lista.rs

constexpr std :: uint32_t lado_atlas = 2048 ;

constexpr std :: uint32_t ancho_rampa = 256 ;

constexpr std :: uint32_t filas_rampas = 512 ;

constexpr std :: uint32_t max_tintas = 4096 ;

using Reloj = std :: chrono :: steady_clock ;

struct UniformesVista {

	float ancho ;

	float alto ;

	float x ;

	float y ;

} ;

struct UniformesCapa {

	float opacidad ;

	std :: uint32_t mezcla ;

	std :: uint32_t con_mascara ;

	std :: uint32_t reservado ;

} ;

constexpr const char * lista_vs = R"(
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

constexpr const char * lista_ps = R"(
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
  if (t.tipo == 1) {
    c.a *= Atlas.SampleLevel(Atlas_sampler, i.UV, 0.0).r;
  } else if (t.tipo == 4) {
    // lo que se solapan el píxel y el trazo, medidos en perpendicular al trazo
    float d = abs(i.UV.x);
    c.a *= saturate(min(d + 0.5, i.UV.y) - max(d - 0.5, -i.UV.y));
  } else if (t.tipo >= 2) {
    float2 g = float2(t.a * i.Doc.x + t.c * i.Doc.y + t.e, t.b * i.Doc.x + t.d * i.Doc.y + t.f);
    float u = t.tipo == 2 ? dot(g - float2(t.p, t.q), float2(t.r, t.s)) : radial(t, g);
    u = (saturate(u) * 255.0 + 0.5) / 256.0;
    c.rgb = Rampas.SampleLevel(Rampas_sampler, float2(u, t.fila), 0.0).rgb;
  }
  return float4(c.rgb * c.a, c.a);
})" ;

constexpr const char * marca_ps = R"(
float4 main() : SV_TARGET { return float4(0.0, 0.0, 0.0, 0.0); })" ;

// Un triángulo que cubre todo : las capas se leen píxel a píxel

constexpr const char * pantalla_vs = R"(
void main(in uint vid : SV_VertexID, out float4 Position : SV_POSITION) {
  float2 p = float2((vid << 1) & 2, vid & 2);
  Position = float4(p * 2.0 - 1.0, 0.0, 1.0);
})" ;

constexpr const char * copia_ps = R"(
Texture2D Capa;
float4 main(in float4 Position : SV_POSITION) : SV_TARGET {
  return Capa.Load(int3(int2(Position.xy), 0));
})" ;

constexpr const char * capa_ps = R"(
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

// Los modos de mezcla de PDF y CSS : necesitan leer lo que hay debajo

constexpr const char * mezcla_ps = R"(
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

RefCntAutoPtr<IShader> compilar(IRenderDevice * device , const char * nombre , SHADER_TYPE tipo , const char * fuente) {

	ShaderCreateInfo info ;

	info.SourceLanguage = SHADER_SOURCE_LANGUAGE_HLSL ;

	info.Desc.UseCombinedTextureSamplers = true ;

	info.EntryPoint = "main" ;

	info.Desc.Name = nombre ;

	info.Desc.ShaderType = tipo ;

	info.Source = fuente ;

	RefCntAutoPtr<IShader> shader ;

	device->CreateShader(info , &shader) ;

	return shader ;

}

void fijar_estatica(IPipelineState * pipeline , SHADER_TYPE etapa , const char * nombre , IDeviceObject * objeto) {

	if (auto * variable = pipeline->GetStaticVariableByName(etapa , nombre)) { variable->Set(objeto) ; }

}

// Solo hace falta el stencil, y D24 ocupa la mitad, pero no todas las GPU lo tienen

TEXTURE_FORMAT elegir_plantilla(IRenderDevice * device , std :: uint32_t muestras) {

	const auto & info = device->GetTextureFormatInfoExt(TEX_FORMAT_D24_UNORM_S8_UINT) ;

	const bool sirve = info.Supported && (info.BindFlags & BIND_DEPTH_STENCIL) && (info.SampleCounts & muestras) ;

	return sirve ? TEX_FORMAT_D24_UNORM_S8_UINT : TEX_FORMAT_D32_FLOAT_S8X24_UINT ;

}

// Una textura de color con MSAA, su stencil y dónde se resuelve : una por cada
// capa abierta a la vez

struct Objetivo {

	RefCntAutoPtr<ITexture> color ;

	RefCntAutoPtr<ITexture> plantilla ;

	RefCntAutoPtr<ITexture> resuelta ;

	bool crear(

		IRenderDevice * device ,

		std :: uint32_t ancho ,

		std :: uint32_t alto ,

		std :: uint32_t muestras ,

		TEXTURE_FORMAT formato_plantilla ,

		GError ** error

	) {

		TextureDesc desc ;

		desc.Name = "LTX objetivo" ;

		desc.Type = RESOURCE_DIM_TEX_2D ;

		desc.Width = ancho ;

		desc.Height = alto ;

		desc.Format = formato_color ;

		desc.SampleCount = muestras ;

		desc.BindFlags = BIND_RENDER_TARGET ;

		desc.ImmediateContextMask = gpu_mascara() ;

		device->CreateTexture(desc , nullptr , &color) ;

		desc.Format = formato_plantilla ;

		desc.BindFlags = BIND_DEPTH_STENCIL ;

		device->CreateTexture(desc , nullptr , &plantilla) ;

		desc.Format = formato_color ;

		desc.SampleCount = 1 ;

		desc.BindFlags = BIND_RENDER_TARGET | BIND_SHADER_RESOURCE ;

		device->CreateTexture(desc , nullptr , &resuelta) ;

		return (color && plantilla && resuelta) || gpu_fallo(error , "No se pudo crear un objetivo de LTX") ;

	}

} ;

// Los búferes de una lista

struct Malla {

	RefCntAutoPtr<IBuffer> vertices ;

	RefCntAutoPtr<IBuffer> indices ;

	std :: uint64_t capacidad_vertices = 0 ;

	std :: uint64_t capacidad_indices = 0 ;

	bool subir(

		IRenderDevice * device ,

		IDeviceContext * ctx ,

		rust :: Slice<const ltx :: Vertice> v ,

		rust :: Slice<const std :: uint32_t> i ,

		GError ** error

	) {

		if (v.empty() || i.empty()) { return true ; }

		const std :: uint64_t bytes_v = v.size() * sizeof(ltx :: Vertice) ;

		const std :: uint64_t bytes_i = i.size() * sizeof(std :: uint32_t) ;

		if (

			!gpu_asegurar_buffer(device , vertices , capacidad_vertices , bytes_v , BIND_VERTEX_BUFFER , "LTX vértices" , error) ||

			!gpu_asegurar_buffer(device , indices , capacidad_indices , bytes_i , BIND_INDEX_BUFFER , "LTX índices" , error)

		) { return false ; }

		ctx->UpdateBuffer(vertices , 0 , bytes_v , v.data() , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		ctx->UpdateBuffer(indices , 0 , bytes_i , i.data() , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		return true ;

	}

} ;

// Todo lo que vive en la GPU : uno por dispositivo

struct Recursos {

	RefCntAutoPtr<IBuffer> uniforme_vista ;

	RefCntAutoPtr<IBuffer> uniforme_capa ;

	RefCntAutoPtr<IBuffer> tintas ;

	RefCntAutoPtr<ITexture> atlas ;

	RefCntAutoPtr<ITexture> rampas ;

	RefCntAutoPtr<IPipelineState> dibujo ;

	RefCntAutoPtr<IPipelineState> unica ;

	RefCntAutoPtr<IPipelineState> recorte ;

	RefCntAutoPtr<IPipelineState> soltar ;

	RefCntAutoPtr<IPipelineState> capa ;

	RefCntAutoPtr<IPipelineState> mezcla ;

	RefCntAutoPtr<IPipelineState> copia ;

	RefCntAutoPtr<IShaderResourceBinding> enlace_dibujo ;

	RefCntAutoPtr<IShaderResourceBinding> enlace_unica ;

	RefCntAutoPtr<IShaderResourceBinding> enlace_recorte ;

	RefCntAutoPtr<IShaderResourceBinding> enlace_soltar ;

	RefCntAutoPtr<IShaderResourceBinding> enlace_capa ;

	RefCntAutoPtr<IShaderResourceBinding> enlace_mezcla ;

	RefCntAutoPtr<IShaderResourceBinding> enlace_copia ;

	std :: vector<std :: unique_ptr<Objetivo>> objetivos ;

	std :: uint32_t ancho = 0 ;

	std :: uint32_t alto = 0 ;

	TEXTURE_FORMAT formato_plantilla = TEX_FORMAT_UNKNOWN ;

	Malla base ;

	Malla vivos ;

	std :: uint64_t generacion = 0 ;

	bool listos = false ;

	bool crear(IRenderDevice * device , std :: uint32_t muestras , GError ** error) {

		if (listos) { return true ; }

		formato_plantilla = elegir_plantilla(device , muestras) ;

		if (

			!crear_datos(device , error) ||

			!crear_pipelines_lista(device , muestras , error) ||

			!crear_pipelines_capa(device , muestras , error)

		) { return false ; }

		listos = true ;

		return true ;

	}

	bool crear_datos(IRenderDevice * device , GError ** error) {

		BufferDesc desc ;

		desc.Name = "LTX uniformes" ;

		desc.Size = sizeof(UniformesVista) ;

		desc.Usage = USAGE_DEFAULT ;

		desc.BindFlags = BIND_UNIFORM_BUFFER ;

		desc.ImmediateContextMask = gpu_mascara() ;

		device->CreateBuffer(desc , nullptr , &uniforme_vista) ;

		desc.Size = sizeof(UniformesCapa) ;

		device->CreateBuffer(desc , nullptr , &uniforme_capa) ;

		desc.Name = "LTX tintas" ;

		desc.Size = max_tintas * sizeof(ltx :: Tinta) ;

		desc.BindFlags = BIND_SHADER_RESOURCE ;

		desc.Mode = BUFFER_MODE_STRUCTURED ;

		desc.ElementByteStride = sizeof(ltx :: Tinta) ;

		device->CreateBuffer(desc , nullptr , &tintas) ;

		TextureDesc textura ;

		textura.Name = "LTX atlas" ;

		textura.Type = RESOURCE_DIM_TEX_2D ;

		textura.Width = lado_atlas ;

		textura.Height = lado_atlas ;

		textura.Format = TEX_FORMAT_R8_UNORM ;

		textura.Usage = USAGE_DEFAULT ;

		textura.BindFlags = BIND_SHADER_RESOURCE ;

		textura.ImmediateContextMask = gpu_mascara() ;

		device->CreateTexture(textura , nullptr , &atlas) ;

		textura.Name = "LTX rampas" ;

		textura.Width = ancho_rampa ;

		textura.Height = filas_rampas ;

		textura.Format = formato_color ;

		device->CreateTexture(textura , nullptr , &rampas) ;

		return (uniforme_vista && uniforme_capa && tintas && atlas && rampas) ||

			gpu_fallo(error , "No se pudieron crear los datos de LTX") ;

	}

	bool crear_pipeline_lista(

		IRenderDevice * device ,

		const char * nombre ,

		IShader * vs ,

		IShader * ps ,

		bool pinta ,

		STENCIL_OP paso ,

		std :: uint32_t muestras ,

		RefCntAutoPtr<IPipelineState> & pipeline ,

		RefCntAutoPtr<IShaderResourceBinding> & enlace ,

		GError ** error

	) {

		GraphicsPipelineStateCreateInfo pso ;

		pso.PSODesc.Name = nombre ;

		pso.PSODesc.PipelineType = PIPELINE_TYPE_GRAPHICS ;

		pso.PSODesc.ImmediateContextMask = gpu_mascara() ;

		auto & g = pso.GraphicsPipeline ;

		g.NumRenderTargets = 1 ;

		g.RTVFormats[0] = formato_color ;

		g.DSVFormat = formato_plantilla ;

		g.SmplDesc.Count = static_cast<Uint8>(muestras) ;

		g.PrimitiveTopology = PRIMITIVE_TOPOLOGY_TRIANGLE_LIST ;

		g.RasterizerDesc.CullMode = CULL_MODE_NONE ;

		g.DepthStencilDesc.DepthEnable = False ;

		g.DepthStencilDesc.DepthWriteEnable = False ;

		g.DepthStencilDesc.StencilEnable = True ;

		g.DepthStencilDesc.FrontFace = StencilOpDesc{ STENCIL_OP_KEEP , STENCIL_OP_KEEP , paso , COMPARISON_FUNC_EQUAL } ;

		g.DepthStencilDesc.BackFace = g.DepthStencilDesc.FrontFace ;

		auto & rt = g.BlendDesc.RenderTargets[0] ;

		rt.RenderTargetWriteMask = pinta ? COLOR_MASK_ALL : COLOR_MASK_NONE ;

		rt.BlendEnable = pinta ? True : False ;

		rt.SrcBlend = BLEND_FACTOR_ONE ;

		rt.DestBlend = BLEND_FACTOR_INV_SRC_ALPHA ;

		rt.SrcBlendAlpha = BLEND_FACTOR_ONE ;

		rt.DestBlendAlpha = BLEND_FACTOR_INV_SRC_ALPHA ;

		LayoutElement layout[] = {

			{ 0 , 0 , 2 , VT_FLOAT32 , False } ,

			{ 1 , 0 , 2 , VT_FLOAT32 , False } ,

			{ 2 , 0 , 4 , VT_UINT8 , True } ,

			{ 3 , 0 , 1 , VT_UINT32 , False }

		} ;

		g.InputLayout.LayoutElements = layout ;

		g.InputLayout.NumElements = 4 ;

		SamplerDesc lineal ;

		lineal.MinFilter = lineal.MagFilter = lineal.MipFilter = FILTER_TYPE_LINEAR ;

		lineal.AddressU = lineal.AddressV = lineal.AddressW = TEXTURE_ADDRESS_CLAMP ;

		ImmutableSamplerDesc inmutables[] = {

			{ SHADER_TYPE_PIXEL , "Atlas" , lineal } ,

			{ SHADER_TYPE_PIXEL , "Rampas" , lineal }

		} ;

		pso.PSODesc.ResourceLayout.ImmutableSamplers = inmutables ;

		pso.PSODesc.ResourceLayout.NumImmutableSamplers = 2 ;

		pso.pVS = vs ;

		pso.pPS = ps ;

		device->CreateGraphicsPipelineState(pso , &pipeline) ;

		if (!pipeline) { return gpu_fallo(error , "No se pudo crear un pipeline de LTX") ; }

		fijar_estatica(pipeline , SHADER_TYPE_VERTEX , "Vista" , uniforme_vista) ;

		fijar_estatica(pipeline , SHADER_TYPE_PIXEL , "Tintas" , tintas->GetDefaultView(BUFFER_VIEW_SHADER_RESOURCE)) ;

		fijar_estatica(pipeline , SHADER_TYPE_PIXEL , "Atlas" , atlas->GetDefaultView(TEXTURE_VIEW_SHADER_RESOURCE)) ;

		fijar_estatica(pipeline , SHADER_TYPE_PIXEL , "Rampas" , rampas->GetDefaultView(TEXTURE_VIEW_SHADER_RESOURCE)) ;

		pipeline->CreateShaderResourceBinding(&enlace , true) ;

		return enlace || gpu_fallo(error , "No se pudo enlazar un pipeline de LTX") ;

	}

	bool crear_pipelines_lista(IRenderDevice * device , std :: uint32_t muestras , GError ** error) {

		auto vs = compilar(device , "LTX lista VS" , SHADER_TYPE_VERTEX , lista_vs) ;

		auto ps = compilar(device , "LTX lista PS" , SHADER_TYPE_PIXEL , lista_ps) ;

		auto marca = compilar(device , "LTX marca PS" , SHADER_TYPE_PIXEL , marca_ps) ;

		if (!vs || !ps || !marca) { return gpu_fallo(error , "No se pudieron compilar los shaders de LTX") ; }

		return

			crear_pipeline_lista(device , "LTX dibujo" , vs , ps , true , STENCIL_OP_KEEP , muestras , dibujo , enlace_dibujo , error) &&

			crear_pipeline_lista(device , "LTX unica" , vs , ps , true , STENCIL_OP_INCR_SAT , muestras , unica , enlace_unica , error) &&

			crear_pipeline_lista(device , "LTX recorte" , vs , marca , false , STENCIL_OP_INCR_SAT , muestras , recorte , enlace_recorte , error) &&

			crear_pipeline_lista(device , "LTX soltar" , vs , marca , false , STENCIL_OP_DECR_SAT , muestras , soltar , enlace_soltar , error) ;

	}

	// `muestras` a 1 : lo que pinta en la superficie de Flutter, sin stencil

	bool crear_pipeline_capa(

		IRenderDevice * device ,

		const char * nombre ,

		IShader * vs ,

		IShader * ps ,

		const char * segunda ,

		bool mezclada ,

		std :: uint32_t muestras ,

		RefCntAutoPtr<IPipelineState> & pipeline ,

		RefCntAutoPtr<IShaderResourceBinding> & enlace ,

		GError ** error

	) {

		GraphicsPipelineStateCreateInfo pso ;

		pso.PSODesc.Name = nombre ;

		pso.PSODesc.PipelineType = PIPELINE_TYPE_GRAPHICS ;

		pso.PSODesc.ImmediateContextMask = gpu_mascara() ;

		auto & g = pso.GraphicsPipeline ;

		g.NumRenderTargets = 1 ;

		g.RTVFormats[0] = formato_color ;

		g.SmplDesc.Count = static_cast<Uint8>(muestras) ;

		g.PrimitiveTopology = PRIMITIVE_TOPOLOGY_TRIANGLE_LIST ;

		g.RasterizerDesc.CullMode = CULL_MODE_NONE ;

		g.DepthStencilDesc.DepthEnable = False ;

		g.DepthStencilDesc.DepthWriteEnable = False ;

		if (muestras > 1) {

			g.DSVFormat = formato_plantilla ;

			g.DepthStencilDesc.StencilEnable = True ;

			g.DepthStencilDesc.FrontFace = StencilOpDesc{ STENCIL_OP_KEEP , STENCIL_OP_KEEP , STENCIL_OP_KEEP , COMPARISON_FUNC_EQUAL } ;

			g.DepthStencilDesc.BackFace = g.DepthStencilDesc.FrontFace ;

		}

		auto & rt = g.BlendDesc.RenderTargets[0] ;

		rt.BlendEnable = mezclada ? True : False ;

		rt.SrcBlend = BLEND_FACTOR_ONE ;

		rt.DestBlend = BLEND_FACTOR_INV_SRC_ALPHA ;

		rt.SrcBlendAlpha = BLEND_FACTOR_ONE ;

		rt.DestBlendAlpha = BLEND_FACTOR_INV_SRC_ALPHA ;

		ShaderResourceVariableDesc variables[] = {

			{ SHADER_TYPE_PIXEL , "Capa" , SHADER_RESOURCE_VARIABLE_TYPE_DYNAMIC } ,

			{ SHADER_TYPE_PIXEL , segunda ? segunda : "" , SHADER_RESOURCE_VARIABLE_TYPE_DYNAMIC }

		} ;

		pso.PSODesc.ResourceLayout.Variables = variables ;

		pso.PSODesc.ResourceLayout.NumVariables = segunda ? 2 : 1 ;

		pso.pVS = vs ;

		pso.pPS = ps ;

		device->CreateGraphicsPipelineState(pso , &pipeline) ;

		if (!pipeline) { return gpu_fallo(error , "No se pudo crear un pipeline de capa de LTX") ; }

		fijar_estatica(pipeline , SHADER_TYPE_PIXEL , "Composicion" , uniforme_capa) ;

		pipeline->CreateShaderResourceBinding(&enlace , true) ;

		return enlace || gpu_fallo(error , "No se pudo enlazar un pipeline de capa de LTX") ;

	}

	bool crear_pipelines_capa(IRenderDevice * device , std :: uint32_t muestras , GError ** error) {

		auto vs = compilar(device , "LTX pantalla VS" , SHADER_TYPE_VERTEX , pantalla_vs) ;

		auto ps_capa = compilar(device , "LTX capa PS" , SHADER_TYPE_PIXEL , capa_ps) ;

		auto ps_mezcla = compilar(device , "LTX mezcla PS" , SHADER_TYPE_PIXEL , mezcla_ps) ;

		auto ps_copia = compilar(device , "LTX copia PS" , SHADER_TYPE_PIXEL , copia_ps) ;

		if (!vs || !ps_capa || !ps_mezcla || !ps_copia) { return gpu_fallo(error , "No se pudieron compilar los shaders de capa de LTX") ; }

		return

			crear_pipeline_capa(device , "LTX capa" , vs , ps_capa , "Mascara" , true , muestras , capa , enlace_capa , error) &&

			crear_pipeline_capa(device , "LTX mezcla" , vs , ps_mezcla , "Fondo" , false , muestras , mezcla , enlace_mezcla , error) &&

			crear_pipeline_capa(device , "LTX copia" , vs , ps_copia , nullptr , false , 1 , copia , enlace_copia , error) ;

	}

	// Solo crecen, y a saltos : encoger o estirar la ventana no crea texturas
	// en cada paso, y una ventana pequeña no paga el monitor entero

	void acomodar(std :: uint32_t w , std :: uint32_t h) {

		if (w <= ancho && h <= alto) { return ; }

		const auto salto = [](std :: uint32_t v) { return (v + 255u) & ~255u ; } ;

		objetivos.clear() ;

		ancho = std :: max(ancho , salto(w)) ;

		alto = std :: max(alto , salto(h)) ;

	}

	Objetivo * objetivo(IRenderDevice * device , std :: size_t i , std :: uint32_t muestras , GError ** error) {

		while (objetivos.size() <= i) {

			auto nuevo = std :: make_unique<Objetivo>() ;

			if (!nuevo->crear(device , ancho , alto , muestras , formato_plantilla , error)) { return nullptr ; }

			objetivos.push_back(std :: move(nuevo)) ;

		}

		return objetivos[i].get() ;

	}

} ;

ITextureView * vista_rt(ITexture * t) { return t->GetDefaultView(TEXTURE_VIEW_RENDER_TARGET) ; }

ITextureView * vista_srv(ITexture * t) { return t->GetDefaultView(TEXTURE_VIEW_SHADER_RESOURCE) ; }

// Ejecuta las listas de un fotograma, con su pila de capas

class Ejecucion {

public:

	Ejecucion(

		IRenderDevice * device ,

		IDeviceContext * ctx ,

		Recursos & r ,

		std :: uint32_t muestras ,

		std :: uint32_t ancho ,

		std :: uint32_t alto

	) :

		device_(device) , ctx_(ctx) , r_(r) , muestras_(muestras) , ancho_(ancho) , alto_(alto) {}

	bool abrir_base(GError ** error) {

		if (!r_.objetivo(device_ , 0 , muestras_ , error)) { return false ; }

		fijar(0 , fondo_lienzo) ;

		return true ;

	}

	bool lista(rust :: Slice<const ltx :: Comando> comandos , Malla & malla , float y0 , float y1 , GError ** error) {

		if (comandos.empty() || !malla.vertices) { return true ; }

		const Uint64 offset = 0 ;

		IBuffer * buffers[] = { malla.vertices } ;

		ctx_->SetVertexBuffers(0 , 1 , buffers , &offset , RESOURCE_STATE_TRANSITION_MODE_TRANSITION , SET_VERTEX_BUFFERS_FLAG_RESET) ;

		ctx_->SetIndexBuffer(malla.indices , 0 , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		for (std :: size_t i = 0 ; i < comandos.size() ; i++) {

			const ltx :: Comando & c = comandos[i] ;

			switch (c.orden) {

				case ltx :: Orden :: Dibujar : indices(r_.dibujo , r_.enlace_dibujo , c) ; break ;

				case ltx :: Orden :: Unica : indices(r_.unica , r_.enlace_unica , c) ; break ;

				case ltx :: Orden :: Recortar : indices(r_.recorte , r_.enlace_recorte , c) ; break ;

				case ltx :: Orden :: Soltar : indices(r_.soltar , r_.enlace_soltar , c) ; break ;

				case ltx :: Orden :: AbrirCapa :

					// lo que no se ve no gasta una textura

					if (c.y1 < y0 || c.y0 > y1) {

						i = c.salto ;

						break ;

					}

					if (!abrir_capa(error)) { return false ; }

					break ;

				case ltx :: Orden :: CerrarMascara : cerrar_mascara() ; break ;

				case ltx :: Orden :: CerrarCapa : cerrar_capa(c) ; break ;

				default : break ;

			}

		}

		return true ;

	}

	ITexture * resolver_base() {

		resolver(0) ;

		return r_.objetivos[0]->resuelta ;

	}

private:

	void fijar(std :: size_t i , const float * limpiar) {

		Objetivo & o = * r_.objetivos[i] ;

		ITextureView * rtv = vista_rt(o.color) ;

		ITextureView * dsv = o.plantilla->GetDefaultView(TEXTURE_VIEW_DEPTH_STENCIL) ;

		ctx_->SetRenderTargets(1 , &rtv , dsv , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		if (limpiar) {

			ctx_->ClearRenderTarget(rtv , limpiar , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

			ctx_->ClearDepthStencil(dsv , CLEAR_STENCIL_FLAG , 1.0f , 0 , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		}

		Viewport vista ;

		vista.Width = static_cast<float>(ancho_) ;

		vista.Height = static_cast<float>(alto_) ;

		ctx_->SetViewports(1 , &vista , r_.ancho , r_.alto) ;

		actual_ = nullptr ;

	}

	void usar(IPipelineState * pipeline , IShaderResourceBinding * enlace) {

		if (pipeline == actual_) { return ; }

		ctx_->SetPipelineState(pipeline) ;

		ctx_->CommitShaderResources(enlace , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		actual_ = pipeline ;

	}

	void indices(IPipelineState * pipeline , IShaderResourceBinding * enlace , const ltx :: Comando & c) {

		if (c.cuenta == 0) { return ; }

		usar(pipeline , enlace) ;

		ctx_->SetStencilRef(c.nivel) ;

		DrawIndexedAttribs draw ;

		draw.NumIndices = c.cuenta ;

		draw.IndexType = VT_UINT32 ;

		draw.FirstIndexLocation = c.inicio ;

		ctx_->DrawIndexed(draw) ;

	}

	bool abrir_capa(GError ** error) {

		if (!r_.objetivo(device_ , profundidad_ + 1 , muestras_ , error)) { return false ; }

		profundidad_++ ;

		fijar(profundidad_ , transparente) ;

		return true ;

	}

	void resolver(std :: size_t i) {

		Objetivo & o = * r_.objetivos[i] ;

		ctx_->SetRenderTargets(0 , nullptr , nullptr , RESOURCE_STATE_TRANSITION_MODE_NONE) ;

		ResolveTextureSubresourceAttribs resolucion ;

		resolucion.SrcTextureTransitionMode = RESOURCE_STATE_TRANSITION_MODE_TRANSITION ;

		resolucion.DstTextureTransitionMode = RESOURCE_STATE_TRANSITION_MODE_TRANSITION ;

		ctx_->ResolveTextureSubresource(o.color , o.resuelta , resolucion) ;

	}

	void cerrar_mascara() {

		resolver(profundidad_) ;

		mascara_ = r_.objetivos[profundidad_]->resuelta ;

		profundidad_-- ;

	}

	void cerrar_capa(const ltx :: Comando & c) {

		resolver(profundidad_) ;

		ITexture * capa = r_.objetivos[profundidad_]->resuelta ;

		profundidad_-- ;

		const bool mezclada = c.mezcla != 0 ;

		if (mezclada) { resolver(profundidad_) ; }

		const UniformesCapa u{ c.opacidad , c.mezcla , c.con_mascara ? 1u : 0u , 0u } ;

		ctx_->UpdateBuffer(r_.uniforme_capa , 0 , sizeof(u) , &u , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		ITexture * segunda = mezclada ? r_.objetivos[profundidad_]->resuelta.RawPtr() : (mascara_ && c.con_mascara ? mascara_ : capa) ;

		IShaderResourceBinding * enlace = mezclada ? r_.enlace_mezcla : r_.enlace_capa ;

		enlace->GetVariableByName(SHADER_TYPE_PIXEL , "Capa")->Set(vista_srv(capa)) ;

		enlace->GetVariableByName(SHADER_TYPE_PIXEL , mezclada ? "Fondo" : "Mascara")->Set(vista_srv(segunda)) ;

		ctx_->SetPipelineState(mezclada ? r_.mezcla : r_.capa) ;

		ctx_->CommitShaderResources(enlace , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		fijar(profundidad_ , nullptr) ;

		ctx_->SetPipelineState(mezclada ? r_.mezcla : r_.capa) ;

		ctx_->SetStencilRef(c.nivel) ;

		DrawAttribs draw ;

		draw.NumVertices = 3 ;

		ctx_->Draw(draw) ;

		mascara_ = nullptr ;

		actual_ = nullptr ;

	}

	IRenderDevice * device_ ;

	IDeviceContext * ctx_ ;

	Recursos & r_ ;

	std :: uint32_t muestras_ ;

	std :: uint32_t ancho_ ;

	std :: uint32_t alto_ ;

	std :: size_t profundidad_ = 0 ;

	ITexture * mascara_ = nullptr ;

	IPipelineState * actual_ = nullptr ;

} ;

// Los trazos y los glifos no dependen del MSAA : solo los rellenos, y con 8× sus
// curvas pierden los dientes. Si la GPU no llega, lo que tenga

std :: uint32_t muestras_posibles(IRenderDevice * device) {

	const char * pedidas = std :: getenv("NLTX_MSAA") ;

	const std :: uint32_t n = pedidas ? static_cast<std :: uint32_t>(std :: atoi(pedidas)) : 8u ;

	const auto cuentas = device->GetTextureFormatInfoExt(formato_color).SampleCounts ;

	for (std :: uint32_t m = std :: min(n , 8u) ; m > 1 ; m /= 2) {

		if (cuentas & m) { return m ; }

	}

	return 2 ;

}

}  // namespace

struct GpuLienzo :: Impl {

	GpuSurface surface ;

	std :: atomic<bool> pendiente{ false } ;

	std :: atomic<bool> animada{ false } ;

	// Un fotograma que no se pudo pintar : hay que volver

	std :: atomic<bool> repetir{ false } ;

	std :: atomic<std :: uint32_t> id{ 0 } ;

	Reloj :: time_point inicio = Reloj :: now() ;

	std :: uint32_t muestras = 0 ;

	// El tamaño de la superficie, que no cambia. Sin monitor, 0 : la de la vista

	std :: uint32_t lienzo_ancho ;

	std :: uint32_t lienzo_alto ;

	// NODOS_BANCO_GPU : mide cada fotograma en CPU y en GPU, esperando a la GPU

	std :: unique_ptr<BancoGpu> banco ;

	// Lo que llega del hilo de plataforma

	mutable std :: mutex cerrojo ;

	std :: uint32_t ancho = 0 ;

	std :: uint32_t alto = 0 ;

	double escala = 1.0 ;

	double y = 0.0 ;

	std :: string volcado ;

	// Lo que se lee desde el hilo de plataforma

	std :: atomic<std :: int64_t> vertices{ 0 } ;

	std :: atomic<double> ms_carril{ 0 } ;

	Impl(Carril & carril , std :: uint32_t ancho_lienzo , std :: uint32_t alto_lienzo) :

		surface{ "LTX lienzo" , carril } , lienzo_ancho(ancho_lienzo) , lienzo_alto(alto_lienzo) {

		if (std :: getenv("NODOS_BANCO_GPU")) { banco = std :: make_unique<BancoGpu>("LTX" , std :: vector<std :: string>{ "lienzo" }) ; }

	}

	bool subir(IDeviceContext * ctx , Recursos & r , const ltx :: Marco & marco , GError ** error) {

		if (marco.generacion() != r.generacion) {

			if (!r.base.subir(surface.device() , ctx , marco.vertices_base() , marco.indices_base() , error)) { return false ; }

			r.generacion = marco.generacion() ;

		}

		if (!r.vivos.subir(surface.device() , ctx , marco.vertices_vivos() , marco.indices_vivos() , error)) { return false ; }

		const auto tintas = marco.tintas() ;

		ctx->UpdateBuffer(r.tintas , 0 , tintas.size() * sizeof(ltx :: Tinta) , tintas.data() , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		subir_filas(ctx , r.atlas , marco.atlas() , marco.atlas_y() , lado_atlas) ;

		subir_filas(ctx , r.rampas , marco.rampas() , marco.rampas_y() , ancho_rampa * 4) ;

		return true ;

	}

	void subir_filas(IDeviceContext * ctx , ITexture * textura , rust :: Slice<const std :: uint8_t> bytes , std :: uint32_t y0 , std :: uint32_t largo_fila) {

		if (bytes.empty()) { return ; }

		const auto & desc = textura->GetDesc() ;

		const std :: uint32_t filas = static_cast<std :: uint32_t>(bytes.size() / largo_fila) ;

		Box caja(0 , desc.Width , y0 , y0 + filas , 0 , 1) ;

		TextureSubResData sub ;

		sub.pData = bytes.data() ;

		sub.Stride = largo_fila ;

		ctx->UpdateTexture(textura , 0 , 0 , caja , sub , RESOURCE_STATE_TRANSITION_MODE_NONE , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	}

	bool dibujar(GError ** error) {

		if (banco) { banco->empezar() ; }

		const auto entrada = Reloj :: now() ;

		std :: uint32_t w , h ;

		double e , desplazado ;

		std :: string ruta ;

		{

			std :: lock_guard<std :: mutex> guardia(cerrojo) ;

			w = ancho ;

			h = alto ;

			e = escala ;

			desplazado = y ;

			ruta.swap(volcado) ;

		}

		if (w == 0 || h == 0) { return true ; }

		const std :: uint32_t superficie_w = lienzo_ancho ? lienzo_ancho : w ;

		const std :: uint32_t superficie_h = lienzo_alto ? lienzo_alto : h ;

		w = std :: min(w , superficie_w) ;

		h = std :: min(h , superficie_h) ;

		GpuSurface :: Acomodo acomodo = GpuSurface :: Acomodo :: esperando ;

		if (!surface.acomodar(superficie_w , superficie_h , &acomodo , error)) { return false ; }

		if (acomodo == GpuSurface :: Acomodo :: esperando) {

			devolver(ruta) ;

			repetir = true ;

			return true ;

		}

		auto & r = surface.estado<Recursos>() ;

		const bool nuevos = !r.listos ;

		if (muestras == 0) { muestras = muestras_posibles(surface.device()) ; }

		if (!r.crear(surface.device() , muestras , error)) { return false ; }

		r.acomodar(w , h) ;

		if (banco) { banco->comenzar() ; }

		bool hay = false ;

		if (!surface.comenzar(&hay , error)) { return false ; }

		if (!hay) {

			devolver(ruta) ;

			repetir = true ;

			return true ;

		}

		IDeviceContext * ctx = surface.context() ;

		// después de comenzar : lo que Rust marca como subido se sube de verdad

		const std :: chrono :: duration<double> pasado = Reloj :: now() - inicio ;

		const ltx :: Vista vista{ double(h) , e , desplazado , pasado.count() } ;

		const rust :: Box<ltx :: Marco> marco = ltx :: preparar(id , vista , nuevos) ;

		if (!subir(ctx , r , * marco , error)) { return false ; }

		const UniformesVista u{ float(w) , float(h) , 0.0f , float(desplazado) } ;

		ctx->UpdateBuffer(r.uniforme_vista , 0 , sizeof(u) , &u , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		Ejecucion ejecucion(surface.device() , ctx , r , muestras , w , h) ;

		ITexture * resuelta = nullptr ;

		{

			Tramo tramo(banco.get() , 0 , Etapa :: dibujar , surface.device() , ctx) ;

			if (!ejecucion.abrir_base(error)) { return false ; }

			const float y0 = float(desplazado) ;

			const float y1 = float(desplazado + h) ;

			if (

				!ejecucion.lista(marco->comandos_base() , r.base , y0 , y1 , error) ||

				!ejecucion.lista(marco->comandos_vivos() , r.vivos , y0 , y1 , error)

			) { return false ; }

			resuelta = ejecucion.resolver_base() ;

		}

		surface.fijar_region(w , h , fondo_lienzo) ;

		r.enlace_copia->GetVariableByName(SHADER_TYPE_PIXEL , "Capa")->Set(vista_srv(resuelta)) ;

		ctx->SetPipelineState(r.copia) ;

		ctx->CommitShaderResources(r.enlace_copia , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		DrawAttribs draw ;

		draw.NumVertices = 3 ;

		ctx->Draw(draw) ;

		if (!ruta.empty()) { surface.volcar(ruta.c_str() , w , h) ; }

		animada = marco->animada() ;

		vertices = std :: int64_t(marco->vertices_base().size() + marco->vertices_vivos().size()) ;

		// lo que gasta el carril, antes de esperar a la GPU en terminar()

		const std :: chrono :: duration<double , std :: milli> gastado = Reloj :: now() - entrada ;

		ms_carril = gastado.count() ;

		if (!surface.terminar(error)) { return false ; }

		if (banco) { banco->terminar(ctx , surface.width() , surface.height()) ; }

		return true ;

	}

	// Un volcado pedido en un fotograma que no se llegó a pintar

	void devolver(const std :: string & ruta) {

		if (ruta.empty()) { return ; }

		std :: lock_guard<std :: mutex> guardia(cerrojo) ;

		if (volcado.empty()) { volcado = ruta ; }

	}

} ;

GpuLienzo :: GpuLienzo(Carril & carril , std :: uint32_t lienzo_ancho , std :: uint32_t lienzo_alto) :

	impl_(new Impl(carril , lienzo_ancho , lienzo_alto)) {

	impl_->surface.al_pintar([this](GError ** error) { return !impl_->pendiente || impl_->dibujar(error) ; }) ;

}

GpuLienzo :: ~GpuLienzo() { delete impl_ ; }

void GpuLienzo :: solicitar() {

	impl_->pendiente = true ;

	impl_->surface.pedir() ;

}

void GpuLienzo :: reiniciar(std :: uint32_t id) {

	impl_->id = id ;

	impl_->pendiente = false ;

}

void GpuLienzo :: avisar_con(std :: function<void()> aviso) { impl_->surface.avisar_con(std :: move(aviso)) ; }

bool GpuLienzo :: texture_info(

	std :: uint32_t * target ,

	std :: uint32_t * name ,

	std :: uint32_t * width ,

	std :: uint32_t * height ,

	GError ** error

) {

	bool nueva = false ;

	if (!impl_->surface.mostrar(target , name , width , height , &nueva , error)) { return false ; }

	// lo que se mueve solo : al coger un fotograma, el siguiente

	const bool repetir = impl_->repetir.exchange(false) ;

	if ((nueva && impl_->animada) || repetir) { impl_->surface.pedir() ; }

	return true ;

}

void GpuLienzo :: fijar_vista(std :: uint32_t ancho , std :: uint32_t alto , double escala , double y) {

	std :: lock_guard<std :: mutex> guardia(impl_->cerrojo) ;

	impl_->ancho = ancho ;

	impl_->alto = alto ;

	impl_->escala = escala ;

	impl_->y = y ;

}

void GpuLienzo :: volcar(const char * ruta) {

	std :: lock_guard<std :: mutex> guardia(impl_->cerrojo) ;

	impl_->volcado = ruta ;

}

MedidasLienzo GpuLienzo :: medidas() const { return { impl_->vertices , impl_->ms_carril } ; }
