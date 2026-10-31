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

# include "ejecucion_lienzo.h"

# include "gpu_surface.h"

# include "recursos_lienzo.h"

using namespace Diligent ;

namespace {

constexpr float fondo_lienzo[4] = { 1.0f , 1.0f , 1.0f , 1.0f } ;

using Reloj = std :: chrono :: steady_clock ;

ITextureView * vista_srv(ITexture * t) { return t->GetDefaultView(TEXTURE_VIEW_SHADER_RESOURCE) ; }

}  // namespace

struct GpuLienzo :: Impl {

	GpuSurface surface ;

	std :: atomic<bool> pendiente{ false } ;

	std :: atomic<bool> animada{ false } ;

	// Un fotograma que no se pudo pintar : hay que volver

	std :: atomic<bool> repetir{ false } ;

	std :: atomic<std :: uint32_t> id{ 0 } ;

	Reloj :: time_point inicio = Reloj :: now() ;

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

	bool subir(const Gpu & gpu , Recursos & r , const ltx :: Marco & marco , GError ** error) {

		IDeviceContext * ctx = gpu.ctx ;

		if (marco.generacion() != r.generacion) {

			if (!r.base.subir(gpu , Geometria{ marco.vertices_base() , marco.indices_base() } , error)) { return false ; }

			r.generacion = marco.generacion() ;

		}

		if (!r.vivos.subir(gpu , Geometria{ marco.vertices_vivos() , marco.indices_vivos() } , error)) { return false ; }

		const auto tintas = marco.tintas() ;

		ctx->UpdateBuffer(r.tintas , 0 , tintas.size() * sizeof(ltx :: Tinta) , tintas.data() , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		const ltx :: Limites limites = ltx :: limites() ;

		subir_filas(ctx , r.atlas , marco.atlas() , marco.atlas_y() , limites.lado_atlas) ;

		subir_filas(ctx , r.rampas , marco.rampas() , marco.rampas_y() , limites.ancho_rampa * 4) ;

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

		if (!r.crear(surface.device() , error)) { return false ; }

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

		const Gpu gpu{ surface.device() , ctx } ;

		if (!subir(gpu , r , * marco , error)) { return false ; }

		const UniformesVista u{ float(w) , float(h) , 0.0f , float(desplazado) } ;

		ctx->UpdateBuffer(r.uniforme_vista , 0 , sizeof(u) , &u , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		const float y0 = float(desplazado) ;

		const float y1 = float(desplazado + h) ;

		Ejecucion ejecucion(gpu , r , Recuadro{ w , h , y0 , y1 }) ;

		ITexture * resuelta = nullptr ;

		{

			Tramo tramo(banco.get() , 0 , Etapa :: dibujar , surface.device() , ctx) ;

			if (!ejecucion.abrir_base(error)) { return false ; }

			if (

				!ejecucion.lista(marco->comandos_base() , r.base , error) ||

				!ejecucion.lista(marco->comandos_vivos() , r.vivos , error)

			) { return false ; }

			resuelta = ejecucion.resolver_base() ;

		}

		surface.fijar_region(w , h , fondo_lienzo) ;

		r.copia.enlace->GetVariableByName(SHADER_TYPE_PIXEL , "Capa")->Set(vista_srv(resuelta)) ;

		ctx->SetPipelineState(r.copia.pipeline) ;

		ctx->CommitShaderResources(r.copia.enlace , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

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
