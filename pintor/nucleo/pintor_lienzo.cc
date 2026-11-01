# include "pintor_lienzo.h"

# include <algorithm>

# include "ejecucion_lienzo.h"

using namespace Diligent ;

namespace ltx {

bool Pintor :: preparar(const Gpu & gpu , std :: string & error) { return recursos_.crear(gpu , error) ; }

bool Pintor :: subir(const Gpu & gpu , const Marco & marco , std :: string & error) {

	Recursos & r = recursos_ ;

	const bool base = marco.generacion_base() != r.generacion_base ;

	const bool vivos = marco.generacion_vivos() != r.generacion_vivos ;

	if (base && !r.base.subir(gpu , Geometria{ marco.vertices_base() , marco.indices_base() } , error)) { return false ; }

	if (vivos && !r.vivos.subir(gpu , Geometria{ marco.vertices_vivos() , marco.indices_vivos() } , error)) { return false ; }

	if (base || vivos) {

		const auto tintas = marco.tintas() ;

		gpu.ctx->UpdateBuffer(r.tintas , 0 , tintas.size() * sizeof(Tinta) , tintas.data() , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	}

	r.generacion_base = marco.generacion_base() ;

	r.generacion_vivos = marco.generacion_vivos() ;

	if ((base || vivos) && !r.subir_imagenes(gpu , marco , error)) { return false ; }

	const Limites medidas = limites() ;

	for (std :: size_t i = 0 ; i < marco.parches_atlas() ; i++) {

		subir_filas(gpu , r.atlas , marco.parche_atlas(i) , marco.fila_atlas(i) , medidas.lado_atlas) ;

	}

	for (std :: size_t i = 0 ; i < marco.parches_rampas() ; i++) {

		subir_filas(gpu , r.rampas , marco.parche_rampas(i) , marco.fila_rampas(i) , medidas.ancho_rampa * 4) ;

	}

	return true ;

}

void Pintor :: subir_filas(const Gpu & gpu , ITexture * textura , rust :: Slice<const std :: uint8_t> bytes , std :: uint32_t y0 , std :: uint32_t largo_fila) {

	if (bytes.empty()) { return ; }

	const auto & desc = textura->GetDesc() ;

	const std :: uint32_t filas = static_cast<std :: uint32_t>(bytes.size() / largo_fila) ;

	Diligent :: Box caja(0 , desc.Width , y0 , y0 + filas , 0 , 1) ;

	TextureSubResData sub ;

	sub.pData = bytes.data() ;

	sub.Stride = largo_fila ;

	gpu.ctx->UpdateTexture(textura , 0 , 0 , caja , sub , RESOURCE_STATE_TRANSITION_MODE_NONE , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

}

Pintado Pintor :: pintar(const Gpu & gpu , const Marco & marco , std :: uint32_t ancho_maximo , std :: uint32_t alto_maximo , std :: string & error) {

	const Vista vista = marco.vista() ;

	const std :: uint32_t w = std :: clamp(vista.ancho , 1u , ancho_maximo) ;

	const std :: uint32_t h = std :: clamp(vista.alto , 1u , alto_maximo) ;

	recursos_.acomodar(w , h) ;

	const UniformesVista u{ float(w) , float(h) , 0.0f , float(vista.y) } ;

	gpu.ctx->UpdateBuffer(recursos_.uniforme_vista , 0 , sizeof(u) , &u , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	Ejecucion ejecucion(gpu , recursos_ , Recuadro{ w , h , float(vista.y) , float(vista.y + h) }) ;

	if (
		!ejecucion.abrir_base(error) ||
		!ejecucion.lista(marco.comandos_base() , recursos_.base , error) ||
		!ejecucion.lista(marco.comandos_vivos() , recursos_.vivos , error)
	) { return Pintado{} ; }

	return Pintado{ ejecucion.resolver_base() , w , h } ;

}

void Pintor :: copiar(const Gpu & gpu , ITexture * imagen) {

	const Pasada & copia = recursos_.copia ;

	copia.enlace->GetVariableByName(SHADER_TYPE_PIXEL , "Capa")->Set(imagen->GetDefaultView(TEXTURE_VIEW_SHADER_RESOURCE)) ;

	gpu.ctx->SetPipelineState(copia.pipeline) ;

	gpu.ctx->CommitShaderResources(copia.enlace , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

	DrawAttribs draw ;

	draw.NumVertices = 3 ;

	gpu.ctx->Draw(draw) ;

}

}  // namespace ltx
