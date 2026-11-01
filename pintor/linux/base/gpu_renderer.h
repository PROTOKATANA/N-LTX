# pragma once

# include <flutter_linux/flutter_linux.h>

# include <cstdint>

# include <functional>

// Lo que el registro de texturas de Flutter necesita de cualquier superficie
// GPU, sin conocer Diligent. Cada renderizador pinta en su
// carril (carril.h) y el raster solo coge lo ya pintado.
class GpuRenderer {

public:

	virtual ~GpuRenderer() = default ;

	// Hilo de plataforma: hay una escena nueva que dibujar.
	virtual void solicitar() = 0 ;

	// Hilo de plataforma: la superficie se asigna a la textura `id` (puede ser
	// una superficie reutilizada); no dibujar hasta el siguiente solicitar().
	virtual void reiniciar(std :: uint32_t id) = 0 ;

	// Hilo de plataforma, al montarse : cómo pedirle a Flutter que vuelva a
	// pintar la textura. El renderizador lo llama desde su carril al terminar
	// un fotograma (o lo que el raster deba importar).
	virtual void avisar_con(std :: function<void()> aviso) = 0 ;

	// Hilo de plataforma, al registrar su familia (si precalienta, ver
	// FamiliaTexturas) : que su carril prepare lo que pueda (shaders,
	// simulaciones) antes de que Flutter pida la textura.
	virtual void precalentar() {}

	// Hilo raster (populate de FlTextureGL): la textura GL que Flutter debe
	// muestrear, la última terminada. Nunca espera a la GPU.
	virtual bool texture_info(

		std :: uint32_t * target ,

		std :: uint32_t * name ,

		std :: uint32_t * width ,

		std :: uint32_t * height ,

		GError ** error

	) = 0 ;

} ;
