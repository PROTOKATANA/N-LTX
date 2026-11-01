# pragma once

# include <atomic>

# include <condition_variable>

# include <cstddef>

# include <functional>

# include <mutex>

# include <thread>

# include <vector>

// declarados, no incluidos : el runner (texturas.h) no ve Diligent

namespace Diligent {

struct IRenderDevice ;

struct IDeviceContext ;

}  // namespace Diligent

// Un carril : un hilo de render propio, con su contexto inmediato de Diligent
// y su cola de Vulkan, para los renderizadores de una familia de texturas.
// Pinta cuando se le pide, sin esperar a Flutter : el hilo raster solo coge lo
// ya pintado (ver gpu_surface.h), y si un fotograma se alarga, la interfaz de
// Flutter no espera.
//
// El dispositivo (uno, con un contexto por carril) lo crea el primer carril
// que arranca, al arrancar la app (ver gpu_surface.h).

class Carril {

public:

	// `indice` : su contexto inmediato, de 0 a los carriles que haya
	// (gpu_carriles) menos 1
	explicit Carril(std :: size_t indice) ;

	// Para el hilo y lo espera (al apagar)
	~Carril() ;

	Carril(const Carril &) = delete ;

	Carril & operator=(const Carril &) = delete ;

	// Hilo de plataforma, antes de pedirlo : lo que corre en el hilo del
	// carril cada vez que se pide. Devuelve su número
	std :: size_t registrar(std :: function<void()> trabajo) ;

	// Cualquier hilo : que el trabajo corra en el carril (una vez, aunque se
	// pida varias antes)
	void pedir(std :: size_t trabajo) ;

	// Hilo del carril, ya con el dispositivo creado
	Diligent :: IRenderDevice * device() const ;

	Diligent :: IDeviceContext * context() const ;

	std :: size_t indice() const { return indice_ ; }

private:

	void bucle() ;

	const std :: size_t indice_ ;

	std :: mutex mutex_ ;

	std :: condition_variable cv_ ;

	std :: vector<std :: function<void()>> trabajos_ ;

	std :: vector<bool> pedidos_ ;

	std :: atomic<bool> parar_{ false } ;

	std :: thread hilo_ ;

} ;
