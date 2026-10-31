# pragma once

# include <cstdint>

# include <functional>

# include "carril.h"

# include "gpu_renderer.h"

// Lo que se puede preguntar desde Flutter

struct MedidasLienzo {

	std :: int64_t vertices = 0 ;

	// Lo que gasta el carril en un fotograma, sin esperar a la GPU

	double ms_carril = 0 ;

} ;

// Pinta el lienzo de N-LTX : texto, fórmulas, figuras y animaciones en una sola
// superficie, con lo que Rust prepara en cada fotograma (pintor/src/lienzo.rs).
// Pinta con MSAA en una textura propia y resuelve al final sobre la superficie
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

	// Qué parte del documento se ve : el tamaño de la superficie y el
	// desplazamiento en píxeles físicos, y los píxeles físicos por punto

	void fijar_vista(std :: uint32_t ancho , std :: uint32_t alto , double escala , double y) ;

	// Depuración : escribe el próximo fotograma en `ruta`, en PPM

	void volcar(const char * ruta) ;

	MedidasLienzo medidas() const ;

private:

	struct Impl ;

	Impl * impl_ ;

} ;
