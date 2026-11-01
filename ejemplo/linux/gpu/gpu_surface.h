# pragma once

# include <flutter_linux/flutter_linux.h>

# include <atomic>

# include <cstddef>

# include <cstdint>

# include <functional>

# include <memory>

# include "DeviceContext.h"

# include "RefCntAutoPtr.hpp"

# include "RenderDevice.h"

# include "carril.h"

// Superficie de render compartida entre Diligent/Vulkan y el contexto EGL de
// Flutter : dos VkImage exportadas por memoria externa e importadas como
// texturas GL, que se relevan (relevo.h) : el carril pinta una mientras
// Flutter muestra la otra. Cada imagen lleva dos semáforos para cederse el
// turno con GL. Son dos hilos : lo de Vulkan, en el del carril (carril.h); lo
// de GL, en el raster de Flutter, que nunca espera a la GPU : si un shader se
// alarga, Flutter sigue mostrando el último fotograma terminado y la interfaz
// no se frena.

class GpuSurface {

public:

	GpuSurface(const char * nombre , Carril & carril) ;

	// Sin llamadas GL ni Vulkan : los handles externos los libera el proceso
	~GpuSurface() ;

	GpuSurface(const GpuSurface &) = delete ;

	GpuSurface & operator=(const GpuSurface &) = delete ;

	// --- Hilo de plataforma, al montar el renderizador

	// Lo que pinta en el carril cada vez que se pide (el fallo lo cuenta el
	// raster)
	void al_pintar(std :: function<bool(GError **)> pintar) ;

	// Cómo pedirle a Flutter que vuelva a pintar la textura (se llama desde
	// cualquier hilo)
	void avisar_con(std :: function<void()> aviso) ;

	// --- Cualquier hilo

	// Que el carril pinte (una vez, aunque se pida varias)
	void pedir() ;

	// --- Hilo raster (populate de FlTextureGL)

	// Coge la última imagen terminada, si hay, y da la textura GL que Flutter
	// debe muestrear (la provisional, transparente, mientras no hay ninguna).
	// Importa en GL las imágenes nuevas y suelta las que se retiran. Nunca
	// espera a la GPU. `nueva` : si cambió la imagen a la vista
	bool mostrar(

		std :: uint32_t * target ,

		std :: uint32_t * name ,

		std :: uint32_t * width ,

		std :: uint32_t * height ,

		bool * nueva ,

		GError ** error

	) ;

	// --- Hilo del carril

	Diligent :: IRenderDevice * device() const ;

	Diligent :: IDeviceContext * context() const ;

	// Estado propio de un renderizador que debe existir una vez por dispositivo
	// (pipelines, atlas…). Se crea en el primer uso, con el candado : dos
	// carriles a la vez no lo crean dos veces. Vive lo que el dispositivo
	template <class T>
	T & estado() {

		static char etiqueta ;

		return * static_cast<T *>(ranura_estado(&etiqueta , [] { return std :: shared_ptr<void>(std :: make_shared<T>()) ; })) ;

	}

	// Las imágenes al tamaño pedido. `lista` : ya lo tenían; `nueva` :
	// recién hechas, hay que pintar aunque la escena no cambie; `esperando` :
	// el raster las está importando o retirando, el carril vuelve cuando se
	// le pida
	enum class Acomodo { lista , nueva , esperando } ;

	bool acomodar(std :: uint32_t w , std :: uint32_t h , Acomodo * acomodo , GError ** error) ;

	// Fotograma : comenzar() toma una imagen libre y espera (en la GPU) a que
	// GL la suelte; entre medias van las subidas de datos y los draw calls;
	// terminar() la publica. `hay` : false si no hay imagen libre (la lista
	// aún no la cogió el raster) : el carril vuelve cuando se le pida
	bool comenzar(bool * hay , GError ** error) ;

	// Fija la imagen como objetivo, la limpia y pone el viewport a textura
	// completa (si es mayor que el panel, la escena se dibuja pre-deformada y el
	// estirado de Flutter la deja con las proporciones correctas).
	void fijar_objetivo(const float fondo[4]) ;

	// Fija la imagen como objetivo con el viewport en su esquina de arriba a
	// la izquierda, de `ancho` × `alto` : quien pinta cubre esa región entera.
	// Si la región cambió desde la última vez que se pintó esta imagen, antes
	// la limpia entera a `fondo` : fuera de la región (el resto del lienzo, ver
	// lienzo.h) siempre hay fondo, nunca lo de otro tamaño
	void fijar_region(std :: uint32_t ancho , std :: uint32_t alto , const float fondo[4]) ;

	// Publica la imagen : espera en el carril a que la GPU la termine (el
	// raster nunca espera) y avisa a Flutter
	bool terminar(GError ** error) ;

	// Depuración : al mostrarse la imagen de este fotograma, el raster la lee
	// por GL y la escribe como PPM tal como se ve en pantalla. Antes de
	// terminar(). Con `ancho` y `alto`, solo su esquina de arriba a la
	// izquierda (la región de fijar_region); con 0, la textura entera. Solo
	// con NLTX_VOLCAR_LIENZO
	void volcar(const char * ruta , std :: uint32_t ancho = 0 , std :: uint32_t alto = 0) ;

	std :: uint32_t width() const ;

	std :: uint32_t height() const ;

private:

	// El de `etiqueta`, creado con `crear` si aún no existe (bajo el candado)
	void * ranura_estado(const void * etiqueta , std :: shared_ptr<void> (* crear)()) ;

	struct Impl ;

	Impl * impl_ ;

} ;

// --- El dispositivo de la base : uno, con un contexto inmediato por carril

// Hilo de plataforma, al registrar las familias : cuántos carriles habrá
void gpu_carriles(std :: size_t carriles) ;

// Hilo del carril : crea el dispositivo (el primer carril que llega, al
// arrancar la app, sin esperar a GL) o espera a que esté. False si no se
// pudo, o si `parar` se puso mientras esperaba
bool gpu_esperar_dispositivo(const std :: atomic<bool> & parar) ;

// Despierta a los carriles que esperan el dispositivo (para que miren `parar`)
void gpu_despertar() ;

Diligent :: IRenderDevice * gpu_device() ;

Diligent :: IDeviceContext * gpu_context(std :: size_t carril) ;

// --- El contexto de los recursos
//
// Diligent solo deja usar un recurso (buffer, textura, pipeline) en los
// contextos de su ImmediateContextMask, y lo libera cuando todos ellos
// avanzan : cada recurso lleva el bit de su carril (con más bits, lo que
// suelte un carril esperaría a que otro carril quieto envíe algo). El carril
// marca su hilo al arrancar; quien cree recursos desde otro hilo le pasa la
// marca con CarrilDelHilo

// El ImmediateContextMask de lo que se crea en este hilo. Fuera de un carril
// lo dice (g_critical) y marca todos, para que funcione igual
Diligent :: Uint64 gpu_mascara() ;

// El carril de este hilo, o SIZE_MAX
std :: size_t gpu_carril_del_hilo() ;

// Mientras viva, este hilo crea recursos para `carril`
class CarrilDelHilo {

public:

	explicit CarrilDelHilo(std :: size_t carril) ;

	~CarrilDelHilo() ;

	CarrilDelHilo(const CarrilDelHilo &) = delete ;

	CarrilDelHilo & operator=(const CarrilDelHilo &) = delete ;

private:

	std :: size_t antes_ ;

} ;

bool gpu_fallo(GError ** error , const char * mensaje) ;


// 0xRRGGBBAA → float[4].
void gpu_color(std :: uint32_t rgba , float out[4]) ;
