# include "gpu_lienzo.h"

# include <algorithm>

# include <atomic>

# include <chrono>

# include <cstdlib>

# include <memory>

# include <string>

# include <thread>

# include <vector>

# include "banco_gpu.h"

# include "ejecucion_lienzo.h"

# include "gpu_surface.h"

# include "recursos_lienzo.h"

using namespace Diligent ;

namespace {

constexpr float fondo_lienzo[4] = { 1.0f , 1.0f , 1.0f , 1.0f } ;

// Sin monitor no hay lienzo que medir

constexpr std :: uint32_t ancho_sin_monitor = 1920 ;

constexpr std :: uint32_t alto_sin_monitor = 1080 ;

// Cada cuánto mira el vigía si debe parar, aunque nadie le despierte

constexpr std :: uint32_t espera_vigia_ms = 200 ;

using Reloj = std :: chrono :: steady_clock ;

ITextureView * vista_srv(ITexture * t) { return t->GetDefaultView(TEXTURE_VIEW_SHADER_RESOURCE) ; }

}  // namespace

struct GpuLienzo :: Impl {

	GpuSurface surface ;

	std :: atomic<bool> pendiente{ false } ;

	// Un fotograma que no se pudo pintar : hay que volver

	std :: atomic<bool> repetir{ false } ;

	std :: atomic<std :: uint32_t> id{ 0 } ;

	// El tamaño de la superficie, que no cambia

	std :: uint32_t lienzo_ancho ;

	std :: uint32_t lienzo_alto ;

	// NODOS_BANCO_GPU : mide cada fotograma en CPU y en GPU, esperando a la GPU

	std :: unique_ptr<BancoGpu> banco ;

	// NLTX_VOLCAR_LIENZO : escribe cada fotograma en ese archivo, en PPM

	const char * volcado = std :: getenv("NLTX_VOLCAR_LIENZO") ;

	std :: thread vigia ;

	std :: atomic<bool> parar{ false } ;

	Impl(Carril & carril , std :: uint32_t ancho_lienzo , std :: uint32_t alto_lienzo) :

		surface{ "LTX lienzo" , carril } ,

		lienzo_ancho(ancho_lienzo ? ancho_lienzo : ancho_sin_monitor) ,

		lienzo_alto(alto_lienzo ? alto_lienzo : alto_sin_monitor) {

		if (std :: getenv("NODOS_BANCO_GPU")) { banco = std :: make_unique<BancoGpu>("LTX" , std :: vector<std :: string>{ "lienzo" }) ; }

	}

	// Hasta que Rust deja algo nuevo no hay nada que pintar

	void vigilar() {

		std :: uint64_t vistas = 0 ;

		while (!parar) {

			const std :: uint64_t entregas = ltx :: esperar_entrega(id , vistas , espera_vigia_ms) ;

			if (entregas == vistas) { continue ; }

			vistas = entregas ;

			pendiente = true ;

			surface.pedir() ;

		}

	}

	bool subir(const Gpu & gpu , Recursos & r , const ltx :: Marco & marco , GError ** error) {

		IDeviceContext * ctx = gpu.ctx ;

		const bool base = marco.generacion_base() != r.generacion_base ;

		const bool vivos = marco.generacion_vivos() != r.generacion_vivos ;

		if (base && !r.base.subir(gpu , Geometria{ marco.vertices_base() , marco.indices_base() } , error)) { return false ; }

		if (vivos && !r.vivos.subir(gpu , Geometria{ marco.vertices_vivos() , marco.indices_vivos() } , error)) { return false ; }

		if (base || vivos) {

			const auto tintas = marco.tintas() ;

			ctx->UpdateBuffer(r.tintas , 0 , tintas.size() * sizeof(ltx :: Tinta) , tintas.data() , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		}

		r.generacion_base = marco.generacion_base() ;

		r.generacion_vivos = marco.generacion_vivos() ;

		const ltx :: Limites limites = ltx :: limites() ;

		for (std :: size_t i = 0 ; i < marco.parches_atlas() ; i++) {

			subir_filas(ctx , r.atlas , marco.parche_atlas(i) , marco.fila_atlas(i) , limites.lado_atlas) ;

		}

		for (std :: size_t i = 0 ; i < marco.parches_rampas() ; i++) {

			subir_filas(ctx , r.rampas , marco.parche_rampas(i) , marco.fila_rampas(i) , limites.ancho_rampa * 4) ;

		}

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

		GpuSurface :: Acomodo acomodo = GpuSurface :: Acomodo :: esperando ;

		if (!surface.acomodar(lienzo_ancho , lienzo_alto , &acomodo , error)) { return false ; }

		if (acomodo == GpuSurface :: Acomodo :: esperando) {

			repetir = true ;

			return true ;

		}

		auto & r = surface.estado<Recursos>() ;

		const bool nuevos = !r.listos ;

		if (!r.crear(surface.device() , error)) { return false ; }

		if (banco) { banco->comenzar() ; }

		bool hay = false ;

		if (!surface.comenzar(&hay , error)) { return false ; }

		if (!hay) {

			repetir = true ;

			return true ;

		}

		IDeviceContext * ctx = surface.context() ;

		// después de comenzar : los parches que se recogen hay que subirlos

		const rust :: Box<ltx :: Marco> marco = ltx :: tomar(id , nuevos) ;

		const ltx :: Vista vista = marco->vista() ;

		const std :: uint32_t w = std :: clamp(vista.ancho , 1u , lienzo_ancho) ;

		const std :: uint32_t h = std :: clamp(vista.alto , 1u , lienzo_alto) ;

		r.acomodar(w , h) ;

		const Gpu gpu{ surface.device() , ctx } ;

		if (!subir(gpu , r , * marco , error)) { return false ; }

		const UniformesVista u{ float(w) , float(h) , 0.0f , float(vista.y) } ;

		ctx->UpdateBuffer(r.uniforme_vista , 0 , sizeof(u) , &u , RESOURCE_STATE_TRANSITION_MODE_TRANSITION) ;

		Ejecucion ejecucion(gpu , r , Recuadro{ w , h , float(vista.y) , float(vista.y + h) }) ;

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

		if (volcado) { surface.volcar(volcado , w , h) ; }

		// lo que gasta el carril, antes de esperar a la GPU en terminar()

		const std :: chrono :: duration<double , std :: milli> gastado = Reloj :: now() - entrada ;

		ltx :: anotar_lienzo(gastado.count()) ;

		if (!surface.terminar(error)) { return false ; }

		if (banco) { banco->terminar(ctx , surface.width() , surface.height()) ; }

		return true ;

	}

} ;

GpuLienzo :: GpuLienzo(Carril & carril , std :: uint32_t lienzo_ancho , std :: uint32_t lienzo_alto) :

	impl_(new Impl(carril , lienzo_ancho , lienzo_alto)) {

	impl_->surface.al_pintar([this](GError ** error) { return !impl_->pendiente || impl_->dibujar(error) ; }) ;

}

// El vigía se para antes de soltar lo que usa

GpuLienzo :: ~GpuLienzo() {

	impl_->parar = true ;

	ltx :: despertar() ;

	if (impl_->vigia.joinable()) { impl_->vigia.join() ; }

	delete impl_ ;

}

void GpuLienzo :: solicitar() {

	impl_->pendiente = true ;

	impl_->surface.pedir() ;

}

void GpuLienzo :: reiniciar(std :: uint32_t id) {

	impl_->id = id ;

	impl_->pendiente = false ;

	if (!impl_->vigia.joinable()) { impl_->vigia = std :: thread([this] { impl_->vigilar() ; }) ; }

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

	if (impl_->repetir.exchange(false)) { impl_->surface.pedir() ; }

	return true ;

}
