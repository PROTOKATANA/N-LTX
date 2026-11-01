# pragma once

# include <cstdint>

# include <string>

# include "recursos_lienzo.h"

// El pintor de N-LTX, lo único que necesita quien lo integra. No crea el
// dispositivo ni conoce la ventana : recibe un `Gpu` y deja lo pintado en una
// textura. Una vez por fotograma, en el hilo de ese contexto, y después de
// pedirle a Rust el marco (`ltx :: tomar`) :
//
//   preparar  una vez por dispositivo, antes de nada
//   subir     lo que cambió desde el marco anterior
//   pintar    da la textura pintada
//   copiar    la copia al objetivo fijado, si no se quiere usar la textura
//
// Los fallos llegan en `error` : quien integra decide cómo contarlos

namespace ltx {

// Una textura RGBA8, de una muestra, y cuánto de ella se ha pintado desde su
// esquina de arriba a la izquierda. `imagen` nula : no se pudo

struct Pintado {

	Diligent :: ITexture * imagen = nullptr ;

	std :: uint32_t ancho = 0 ;

	std :: uint32_t alto = 0 ;

} ;

class Pintor {

public:

	// Mientras no lo esté, la GPU no tiene nada subido : hay que pedir a Rust
	// el marco completo

	bool listo() const { return recursos_.listos ; }

	bool preparar(const Gpu & gpu , std :: string & error) ;

	bool subir(const Gpu & gpu , const Marco & marco , std :: string & error) ;

	// Lo que se ve en `marco`, hasta `ancho_maximo` × `alto_maximo`, sobre su
	// fondo (`Taller :: fondo`)

	Pintado pintar(const Gpu & gpu , const Marco & marco , std :: uint32_t ancho_maximo , std :: uint32_t alto_maximo , std :: string & error) ;

	// Pinta `imagen` en el objetivo RGBA8 de una muestra que tenga fijado el
	// contexto, con su viewport

	void copiar(const Gpu & gpu , Diligent :: ITexture * imagen) ;

private:

	void subir_filas(const Gpu & gpu , Diligent :: ITexture * textura , rust :: Slice<const std :: uint8_t> bytes , std :: uint32_t y0 , std :: uint32_t largo_fila) ;

	Recursos recursos_ ;

} ;

}  // namespace ltx
