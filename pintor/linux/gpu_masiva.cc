# include "gpu_masiva.h"

# include <atomic>

# include <chrono>

# include <cstdlib>

# include <memory>

# include <vector>

# include "banco_gpu.h"

# include "gpu_surface.h"

# include "pintor/src/puente_masiva.rs.h"

using namespace Diligent ;

namespace {

constexpr std :: uint32_t ancho_lienzo = 1280 ;

constexpr std :: uint32_t alto_lienzo = 720 ;

constexpr float fondo_lienzo[4] = { 0.02f , 0.02f , 0.05f , 1.0f } ;

using Reloj = std :: chrono :: steady_clock ;

struct Uniformes {

	float tiempo ;

	float ancho ;

	float alto ;

	float reservado ;

} ;

constexpr const char * elementos_vs = R"(
cbuffer Escena { float tiempo; float ancho; float alto; float reservado; };
struct Instancia {
  float2 Centro : ATTRIB0;
  float Fase : ATTRIB1;
  float Radio : ATTRIB2;
  float4 Color : ATTRIB3;
};
struct Salida { float4 Position : SV_POSITION; float2 UV : TEXCOORD0; float4 Color : COLOR0; };
void main(in Instancia i, in uint vid : SV_VertexID, out Salida o) {
  float2 esquinas[6] = { float2(-1,-1), float2(1,-1), float2(1,1),
                         float2(-1,-1), float2(1,1), float2(-1,1) };
  float2 esquina = esquinas[vid];
  // gira más deprisa cuanto más cerca del centro, y cada uno oscila a su ritmo
  float r = length(i.Centro);
  float giro = tiempo * (0.08 + 0.22 / (0.22 + r));
  float s = sin(giro), c = cos(giro);
  float2 p = float2(c * i.Centro.x - s * i.Centro.y, s * i.Centro.x + c * i.Centro.y);
  p *= 1.0 + 0.012 * sin(tiempo * 1.7 + i.Fase);
  float2 aspecto = float2(alto / ancho, 1.0);
  float2 ndc = p * aspecto * 0.96 + esquina * (i.Radio + 1.0) * 2.0 / float2(ancho, alto);
  o.Position = float4(ndc.x, -ndc.y, 0.0, 1.0);
  o.UV = esquina * (i.Radio + 1.0) / i.Radio;
  o.Color = i.Color;
})" ;

constexpr const char * elementos_ps = R"(
struct Entrada { float4 Position : SV_POSITION; float2 UV : TEXCOORD0; float4 Color : COLOR0; };
float4 main(in Entrada i) : SV_TARGET {
  float d = length(i.UV);
  float aa = max(fwidth(d), 0.002);
  float cobertura = 1.0 - smoothstep(1.0 - aa, 1.0 + aa, d);
  return float4(i.Color.rgb, i.Color.a * cobertura);
})" ;

// El pipeline y la geometría : uno por dispositivo

struct Recursos {

	RefCntAutoPtr<IPipelineState> pipeline ;

	RefCntAutoPtr<IShaderResourceBinding> binding ;

	RefCntAutoPtr<IBuffer> uniforme ;

	RefCntAutoPtr<IBuffer> instancias ;

	std :: uint32_t cuenta = 0 ;

	bool crear(IRenderDevice * device , GError ** error) {

		if (pipeline) { return true ; }

		return crear_uniforme(device , error) && crear_instancias(device , error) && crear_pipeline(device , error) ;

	}

	bool crear_uniforme(IRenderDevice * device , GError ** error) {

		BufferDesc desc ;

		desc.Name = "LTX uniformes" ;

		desc.Size = sizeof(Uniformes) ;

		desc.Usage = USAGE_DEFAULT ;

		desc.BindFlags = BIND_UNIFORM_BUFFER ;

		desc.ImmediateContextMask = gpu_mascara() ;

		device->CreateBuffer(desc , nullptr , &uniforme) ;

		return uniforme || gpu_fallo(error , "No se pudo crear el uniforme de LTX") ;

	}

	// Rust escribe los elementos en memoria de C++ por cxx y se suben una vez

	bool crear_instancias(IRenderDevice * device , GError ** error) {

		cuenta = ltx :: elementos_pedidos() ;

		std :: vector<ltx :: Elemento> elementos(cuenta) ;

		ltx :: llenar_elementos(rust :: Slice<ltx :: Elemento>{ elementos.data() , elementos.size() }) ;

		BufferDesc desc ;

		desc.Name = "LTX elementos" ;

		desc.Size = elementos.size() * sizeof(ltx :: Elemento) ;

		desc.Usage = USAGE_IMMUTABLE ;

		desc.BindFlags = BIND_VERTEX_BUFFER ;

		desc.ImmediateContextMask = gpu_mascara() ;

		BufferData datos ;

		datos.pData = elementos.data() ;

		datos.DataSize = desc.Size ;

		device->CreateBuffer(desc , &datos , &instancias) ;

		return instancias || gpu_fallo(error , "No se pudieron crear los elementos de LTX") ;

	}

	bool crear_pipeline(IRenderDevice * device , GError ** error) {

		ShaderCreateInfo shader ;

		shader.SourceLanguage = SHADER_SOURCE_LANGUAGE_HLSL ;

		shader.Desc.UseCombinedTextureSamplers = true ;

		shader.EntryPoint = "main" ;

		RefCntAutoPtr<IShader> vs , ps ;

		shader.Desc.Name = "LTX elementos VS" ;

		shader.Desc.ShaderType = SHADER_TYPE_VERTEX ;

		shader.Source = elementos_vs ;

		device->CreateShader(shader , &vs) ;

		shader.Desc.Name = "LTX elementos PS" ;

		shader.Desc.ShaderType = SHADER_TYPE_PIXEL ;

		shader.Source = elementos_ps ;

		device->CreateShader(shader , &ps) ;

		if (!vs || !ps) { return gpu_fallo(error , "No se pudieron compilar los shaders de LTX") ; }

		GraphicsPipelineStateCreateInfo pso ;

		pso.PSODesc.Name = "LTX elementos" ;

		pso.PSODesc.PipelineType = PIPELINE_TYPE_GRAPHICS ;

		pso.GraphicsPipeline.NumRenderTargets = 1 ;

		pso.GraphicsPipeline.RTVFormats[0] = TEX_FORMAT_RGBA8_UNORM ;

		pso.GraphicsPipeline.PrimitiveTopology = PRIMITIVE_TOPOLOGY_TRIANGLE_LIST ;

		pso.GraphicsPipeline.RasterizerDesc.CullMode = CULL_MODE_NONE ;

		pso.GraphicsPipeline.DepthStencilDesc.DepthEnable = False ;

		auto & mezcla = pso.GraphicsPipeline.BlendDesc.RenderTargets[0] ;

		mezcla.BlendEnable = True ;

		mezcla.SrcBlend = BLEND_FACTOR_SRC_ALPHA ;

		mezcla.DestBlend = BLEND_FACTOR_INV_SRC_ALPHA ;

		mezcla.SrcBlendAlpha = BLEND_FACTOR_ONE ;

		mezcla.DestBlendAlpha = BLEND_FACTOR_INV_SRC_ALPHA ;

		LayoutElement layout[] = {

			{ 0 , 0 , 2 , VT_FLOAT32 , False } ,

			{ 1 , 0 , 1 , VT_FLOAT32 , False } ,

			{ 2 , 0 , 1 , VT_FLOAT32 , False } ,

			{ 3 , 0 , 4 , VT_UINT8 , True }

		} ;

		for (auto & elemento : layout) { elemento.Frequency = INPUT_ELEMENT_FREQUENCY_PER_INSTANCE ; }

		pso.GraphicsPipeline.InputLayout.LayoutElements = layout ;

		pso.GraphicsPipeline.InputLayout.NumElements = 4 ;

		pso.pVS = vs ;

		pso.pPS = ps ;

		pso.PSODesc.ImmediateContextMask = gpu_mascara() ;

		device->CreateGraphicsPipelineState(pso , &pipeline) ;

		if (!pipeline) { return gpu_fallo(error , "No se pudo crear el pipeline de LTX") ; }

		for (auto etapa : { SHADER_TYPE_VERTEX , SHADER_TYPE_PIXEL }) {

			if (auto * v = pipeline->GetStaticVariableByName(etapa , "Escena")) { v->Set(uniforme) ; }

		}

		pipeline->CreateShaderResourceBinding(&binding , true) ;

		return true ;

	}

} ;

}  // namespace

struct GpuMasiva :: Impl {

	GpuSurface surface ;

	std :: atomic<bool> pendiente{ false } ;

	Reloj :: time_point inicio = Reloj :: now() ;

	const char * volcado = std :: getenv("NLTX_VOLCAR_GPU") ;

	// NODOS_BANCO_GPU : mide cada fotograma en CPU y en GPU, esperando a la GPU

	std :: unique_ptr<BancoGpu> banco ;

	// Del carril

	std :: uint32_t fotogramas = 0 ;

	Reloj :: time_point ventana = Reloj :: now() ;

	explicit Impl(Carril & carril) : surface{ "LTX superficie" , carril } {

		if (std :: getenv("NODOS_BANCO_GPU")) { banco = std :: make_unique<BancoGpu>("LTX" , std :: vector<std :: string>{ "elementos" }) ; }

	}

	bool dibujar(GError ** error) {

		if (banco) { banco->empezar() ; }

		const auto entrada = Reloj :: now() ;

		GpuSurface :: Acomodo acomodo = GpuSurface :: Acomodo :: esperando ;

		if (!surface.acomodar(ancho_lienzo , alto_lienzo , &acomodo , error)) { return false ; }

		if (acomodo == GpuSurface :: Acomodo :: esperando) { return true ; }

		auto & r = surface.estado<Recursos>() ;

		if (!r.crear(surface.device() , error)) { return false ; }

		if (banco) { banco->comenzar() ; }

		bool hay = false ;

		if (!surface.comenzar(&hay , error)) { return false ; }

		if (!hay) { return true ; }

		IDeviceContext * ctx = surface.context() ;

		const std :: chrono :: duration<float> pasado = Reloj :: now() - inicio ;

		const Uniformes u{ pasado.count() , float(ancho_lienzo) , float(alto_lienzo) , 0.0f } ;

		ctx->UpdateBuffer(r.uniforme , 0 , sizeof(u) , &u , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		surface.fijar_objetivo(fondo_lienzo) ;

		{

			Tramo tramo(banco.get() , 0 , Etapa :: dibujar , surface.device() , ctx) ;

			const Uint64 offset = 0 ;

			IBuffer * buffers[] = { r.instancias } ;

			ctx->SetVertexBuffers(0 , 1 , buffers , &offset , RESOURCE_STATE_TRANSITION_MODE_TRANSITION , SET_VERTEX_BUFFERS_FLAG_RESET) ;

			ctx->SetPipelineState(r.pipeline) ;

			ctx->CommitShaderResources(r.binding , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

			DrawAttribs draw ;

			draw.NumVertices = 6 ;

			draw.NumInstances = r.cuenta ;

			ctx->Draw(draw) ;

		}

		if (volcado) { surface.volcar(volcado) ; }

		// lo que gasta el carril, antes de esperar a la GPU en terminar()

		const std :: chrono :: duration<double , std :: milli> gastado = Reloj :: now() - entrada ;

		if (!surface.terminar(error)) { return false ; }

		if (banco) { banco->terminar(ctx , surface.width() , surface.height()) ; }

		apuntar(r.cuenta , gastado.count()) ;

		return true ;

	}

	void apuntar(std :: uint32_t cuenta , double gastado) {

		fotogramas++ ;

		const std :: chrono :: duration<double> tramo = Reloj :: now() - ventana ;

		if (tramo.count() >= 1.0) {

			ltx :: anotar_masiva(ltx :: MedidasMasiva{ cuenta , fotogramas / tramo.count() , gastado }) ;

			fotogramas = 0 ;

			ventana = Reloj :: now() ;

		}

	}

} ;

GpuMasiva :: GpuMasiva(Carril & carril) : impl_(new Impl(carril)) {

	impl_->surface.al_pintar([this](GError ** error) { return !impl_->pendiente || impl_->dibujar(error) ; }) ;

}

GpuMasiva :: ~GpuMasiva() { delete impl_ ; }

void GpuMasiva :: solicitar() {

	impl_->pendiente = true ;

	impl_->surface.pedir() ;

}

void GpuMasiva :: reiniciar(std :: uint32_t) { impl_->pendiente = false ; }

void GpuMasiva :: avisar_con(std :: function<void()> aviso) { impl_->surface.avisar_con(std :: move(aviso)) ; }

bool GpuMasiva :: texture_info(

	std :: uint32_t * target ,

	std :: uint32_t * name ,

	std :: uint32_t * width ,

	std :: uint32_t * height ,

	GError ** error

) {

	bool nueva = false ;

	if (!impl_->surface.mostrar(target , name , width , height , &nueva , error)) { return false ; }

	// lo que se mueve solo : al coger un fotograma, el siguiente

	if (nueva && impl_->pendiente) { impl_->surface.pedir() ; }

	return true ;

}
