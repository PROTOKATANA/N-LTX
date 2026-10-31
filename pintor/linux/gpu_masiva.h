# pragma once

# include <cstdint>

# include <functional>

# include "carril.h"

# include "gpu_renderer.h"

// La prueba masiva : cientos de miles de elementos que el vertex shader mueve
// en cada fotograma, sin que su geometría vuelva a cruzar de la CPU a la GPU

class GpuMasiva final : public GpuRenderer {

public:

	explicit GpuMasiva(Carril & carril) ;

	~GpuMasiva() override ;

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
