# include "gpu_lienzo.h"

# include <atomic>

# include <chrono>

# include <cstdlib>

# include <fstream>

# include <memory>

# include <string>

# include <thread>

# include <vector>

# include "banco_gpu.h"

# include "gpu_surface.h"

# include "pintor_lienzo.h"

using namespace Diligent ;

namespace {

// Sin monitor no hay lienzo que medir

constexpr std :: uint32_t ancho_sin_monitor = 1920 ;

constexpr std :: uint32_t alto_sin_monitor = 1080 ;

// Cada cuánto mira el vigía si debe parar, aunque nadie le despierte

constexpr std :: uint32_t espera_vigia_ms = 200 ;

using Reloj = std :: chrono :: steady_clock ;

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

	// NLTX_BANCO_GPU : mide cada fotograma en CPU y en GPU, esperando a la GPU

	std :: unique_ptr<BancoGpu> banco ;

	// NLTX_VOLCAR_LIENZO : escribe cada fotograma en ese archivo, en PPM

	const char * volcado = std :: getenv("NLTX_VOLCAR_LIENZO") ;

	// NLTX_HORNEAR : escribe ahí los shaders compilados (ver pintor/README.md)

	const char * horno = std :: getenv("NLTX_HORNEAR") ;

	std :: thread vigia ;

	std :: atomic<bool> parar{ false } ;

	Impl(Carril & carril , std :: uint32_t ancho_lienzo , std :: uint32_t alto_lienzo) :

		surface{ "LTX lienzo" , carril } ,

		lienzo_ancho(ancho_lienzo ? ancho_lienzo : ancho_sin_monitor) ,

		lienzo_alto(alto_lienzo ? alto_lienzo : alto_sin_monitor) {

		if (std :: getenv("NLTX_BANCO_GPU")) { banco = std :: make_unique<BancoGpu>("LTX" , std :: vector<std :: string>{ "lienzo" }) ; }

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

	bool dibujar(GError ** error) {

		if (banco) { banco->empezar() ; }

		const auto entrada = Reloj :: now() ;

		GpuSurface :: Acomodo acomodo = GpuSurface :: Acomodo :: esperando ;

		if (!surface.acomodar(lienzo_ancho , lienzo_alto , &acomodo , error)) { return false ; }

		if (acomodo == GpuSurface :: Acomodo :: esperando) {

			repetir = true ;

			return true ;

		}

		auto & pintor = surface.estado<ltx :: Pintor>() ;

		const bool nuevos = !pintor.listo() ;

		const ltx :: Gpu gpu{ surface.device() , surface.context() } ;

		std :: string fallo ;

		if (!pintor.preparar(gpu , fallo)) { return gpu_fallo(error , fallo.c_str()) ; }

		if (horno) {

			const std :: string horneado = ltx :: hornear(gpu , fallo) ;

			if (horneado.empty()) { return gpu_fallo(error , fallo.c_str()) ; }

			std :: ofstream(horno) << horneado ;

			horno = nullptr ;

		}

		if (banco) { banco->comenzar() ; }

		bool hay = false ;

		if (!surface.comenzar(&hay , error)) { return false ; }

		if (!hay) {

			repetir = true ;

			return true ;

		}

		// después de comenzar : los parches que se recogen hay que subirlos

		const rust :: Box<ltx :: Marco> marco = ltx :: tomar(id , nuevos) ;

		if (!pintor.subir(gpu , * marco , fallo)) { return gpu_fallo(error , fallo.c_str()) ; }

		ltx :: Pintado pintado ;

		{

			Tramo tramo(banco.get() , 0 , Etapa :: dibujar , gpu.device , gpu.ctx) ;

			pintado = pintor.pintar(gpu , * marco , lienzo_ancho , lienzo_alto , fallo) ;

		}

		if (!pintado.imagen) { return gpu_fallo(error , fallo.c_str()) ; }

		const ltx :: Fondo color = marco->fondo() ;

		const float fondo[4] = { color.r , color.g , color.b , 1.0f } ;

		surface.fijar_region(pintado.ancho , pintado.alto , fondo) ;

		pintor.copiar(gpu , pintado.imagen) ;

		if (volcado) { surface.volcar(volcado , pintado.ancho , pintado.alto) ; }

		// lo que gasta el carril, antes de esperar a la GPU en terminar()

		const std :: chrono :: duration<double , std :: milli> gastado = Reloj :: now() - entrada ;

		ltx :: anotar_lienzo(gastado.count()) ;

		if (!surface.terminar(error)) { return false ; }

		if (banco) { banco->terminar(gpu.ctx , surface.width() , surface.height()) ; }

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
