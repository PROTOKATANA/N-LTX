# pragma once

# include <cstdint>

# include <functional>

# include "carril.h"

# include "gpu_renderer.h"

// Pinta el lienzo de N-LTX : texto, fórmulas, figuras y animaciones en una sola
// superficie. Rust lleva el ritmo y lo hace todo en su taller (pintor/src/taller.rs);
// aquí solo se copia lo último hecho, y solo lo que cambió. Un hilo vigía espera en
// Rust a que haya algo nuevo y pide el fotograma. Pinta con MSAA en una textura propia y resuelve al final sobre la superficie
// compartida con Flutter, que mide lo que el lienzo (lienzo.h) : al redimensionar
// la ventana no se recrea nada y solo cambia la parte que se pinta

class GpuLienzo final : public GpuRenderer {

public:

	GpuLienzo(Carril & carril , std :: uint32_t lienzo_ancho , std :: uint32_t lienzo_alto) ;

	~GpuLienzo() override ;

	void solicitar() override ;

	void reiniciar(std :: uint32_t id) override ;

	void avisar_con(std :: function<void()> aviso) override ;

	bool texture_info(

		std :: uint32_t * target ,

		std :: uint32_t * name ,

		std :: uint32_t * width ,

		std :: uint32_t * height ,

		GError ** error

	) override ;

private:

	struct Impl ;

	Impl * impl_ ;

} ;
